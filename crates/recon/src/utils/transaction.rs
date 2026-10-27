use common_utils::id_type::MerchantId;

use api_models::{
    account::AccountData,
    entry::{
        CompactEntryData, EntryData as EntryDataApi,
        MetadataMismatchData as MetadataMismatchDataApi,
    },
    recon_rule::RuleData,
    transaction::{
        ApiTransactionAccountFilters, ApiTransactionFilterStatus, ApiTransactionListFiltersV2,
        ApiTransactionStatus, ForceMatchedData as ForceMatchedDataApi, GetTransactionResponse,
        ManuallyMatchedData as ManuallyMatchedDataApi, ManuallyPostedData,
        MatchedData as MatchedDataApi, PostedData as PostedDataApi,
        TransactionData as TransactionDataApi, TransactionEntry, TransactionEntryWithData,
        TransactionWithEntriesResponse,
    },
    user::User,
};
use common_enums::{BalanceDirection, Currency, DieselTransactionStatus, EntryStatus};
use common_types::{
    amount::{AmountResponse, AmountType},
    entry::{EntryData as DieselEntryData, EntryDataV2, FieldMismatchDetail},
    metadata_schema::{ExposedKeys, ExposedKeysProvider},
    pagination::CursorSort,
    recon_rule::aging_config::AgingConfig,
    transaction::{
        status::{
            DomainTransactionStatus, TransactionAmountMismatchData, TransactionMatchedData,
            TransactionPostedData,
        },
        AmountMismatchData, AmountMismatchDataV1, ForceMatchedData, ForceMatchedDataV1,
        ManualPostedData, ManuallyMatchedData, ManuallyMatchedDataV1, MatchedData, MatchedDataV1,
        PartiallyReconciledData, PartiallyReconciledDataV1, PostedData, PostedDataV1,
        TransactionAccountFilters, TransactionData as DieselTransactionData, TransactionDataV3,
        TransactionEffectiveAtSortField, TransactionListEntryFilters, TransactionListFiltersV2,
        TransactionListMetadataFilters, TransactionListSortField, TransactionListStatusFilter,
        VoidData, VoidDataV1,
    },
    units::{FloatMajorUnit, MinorUnit},
};
use common_utils::{auth::AuthContext, date_time};
use diesel_models::{
    account::Account,
    entry::Entry as DieselEntry,
    recon_rule::ReconRule,
    transaction::{TransactionAccountView, TransactionRuleView},
};
use error_stack::{report, ResultExt};
use futures::{future, future::Either};
use masking::ExposeInterface;
use recon_domain_models::{
    behaviour::ReverseConversion,
    encryption::ApplicationEncryptionInterface,
    encryption_handler::EncryptionHandler,
    entry::EntryWithSearchableMetadata,
    transaction::{
        DirectTransactionCreationEntry, DomainTransaction, ExpectedTransactionCreationEntry,
        SearchExpectationCreation, TransactionCreationEntry,
    },
};
use std::collections::{HashMap, HashSet};
use storage_impl::{
    account::AccountInterface, entry::EntryInterface, metadata_schema::MetadataSchemaInterface,
    transformation::TransformationConfigInterface,
};
use time::PrimitiveDateTime;

use crate::{
    errors::transaction::{TransactionError, TransactionResult},
    routes::AppState,
    types::transformers::ForeignFrom,
    utils::{
        entry::{EntryDataExt, FieldMismatchDetailExt},
        metadata_schema::project_metadata,
        recon_engine,
    },
};
use external_services::user::interfaces::UserInterface;

pub trait TransactionEntryExt {
    fn get_amount(&self) -> FloatMajorUnit;
    fn get_entry_type(&self) -> BalanceDirection;
    fn get_account_id(&self) -> &str;
    fn get_currency(&self) -> Currency;
    fn get_effective_at(&self) -> PrimitiveDateTime;

    async fn to_creation_entry(
        self,
        direct_entry_data: DieselEntryData,
        merchant_id: MerchantId,
        exposed_keys: ExposedKeys,
        encryptor: &EncryptionHandler<impl ApplicationEncryptionInterface>,
    ) -> TransactionResult<(
        EntryWithSearchableMetadata,
        Option<SearchExpectationCreation>,
    )>;
}

impl TransactionEntryExt for TransactionEntry {
    fn get_amount(&self) -> FloatMajorUnit {
        match self {
            Self::Direct(direct_entry) => direct_entry.amount,
            Self::Expected(expected_entry) => expected_entry.amount,
        }
    }
    fn get_entry_type(&self) -> BalanceDirection {
        match self {
            Self::Direct(direct_entry) => direct_entry.entry_type,
            Self::Expected(expected_entry) => expected_entry.entry_type,
        }
    }
    fn get_account_id(&self) -> &str {
        match self {
            Self::Direct(direct_entry) => &direct_entry.account_id,
            Self::Expected(expected_entry) => &expected_entry.account_id,
        }
    }

    fn get_currency(&self) -> Currency {
        match self {
            Self::Direct(direct_entry) => direct_entry.currency,
            Self::Expected(expected_entry) => expected_entry.currency,
        }
    }

    fn get_effective_at(&self) -> PrimitiveDateTime {
        match self {
            Self::Direct(direct_entry) => direct_entry.effective_at,
            Self::Expected(expected_entry) => expected_entry.effective_at,
        }
    }

    async fn to_creation_entry(
        self,
        direct_entry_data: DieselEntryData,
        merchant_id: MerchantId,
        exposed_keys: ExposedKeys,
        encryptor: &EncryptionHandler<impl ApplicationEncryptionInterface>,
    ) -> TransactionResult<(
        EntryWithSearchableMetadata,
        Option<SearchExpectationCreation>,
    )> {
        match self {
            Self::Direct(direct_entry) => {
                let raw_metadata = direct_entry.metadata.clone().expose();

                let exposed_metadata = project_metadata(&raw_metadata, &exposed_keys.exposed);

                let encrypted_metadata = encryptor
                    .encrypt_json_value(raw_metadata)
                    .await
                    .change_context(TransactionError::InternalServerError)?;

                let amount =
                    AmountType::from_major_unit(direct_entry.amount, direct_entry.currency)
                        .change_context(TransactionError::InvalidAmount {
                            details: "Invalid amount provided".to_string(),
                        })?
                        .value();

                let creation_entry =
                    TransactionCreationEntry::Direct(DirectTransactionCreationEntry {
                        account_id: direct_entry.account_id.clone(),
                        entry_type: direct_entry.entry_type,
                        amount,
                        currency: direct_entry.currency,
                        effective_at: direct_entry.effective_at,
                        merchant_id,
                        metadata: encrypted_metadata,
                        data: direct_entry_data,
                        primary_staging_entry_id: direct_entry.staging_entry_id.clone(),
                        transformation_id: direct_entry.transformation_id.clone(),
                        order_id: direct_entry.order_id.clone(),
                        exposed_metadata,
                    });

                Ok((
                    creation_entry.into_entry_with_searchable_metadata(&exposed_keys.searchable),
                    None,
                ))
            }
            Self::Expected(expected_entry) => {
                let encrypted_metadata = encryptor
                    .encrypt_json_value(expected_entry.metadata.clone().expose())
                    .await
                    .change_context(TransactionError::InternalServerError)?;

                let amount =
                    AmountType::from_major_unit(expected_entry.amount, expected_entry.currency)
                        .change_context(TransactionError::InvalidAmount {
                            details: "Invalid amount provided".to_string(),
                        })?
                        .value();

                let encrypted_source_value = encryptor
                    .encrypt_string(expected_entry.search_expectation.source_value.clone())
                    .await
                    .change_context(TransactionError::InternalServerError)?;

                let hashed_source_value =
                    recon_engine::hash_string(&expected_entry.search_expectation.source_value);

                let creation_entry =
                    TransactionCreationEntry::Expected(ExpectedTransactionCreationEntry {
                        account_id: expected_entry.account_id.clone(),
                        entry_type: expected_entry.entry_type,
                        amount,
                        currency: expected_entry.currency,
                        effective_at: expected_entry.effective_at,
                        merchant_id: merchant_id.clone(),
                        metadata: encrypted_metadata,
                        data: DieselEntryData::V2(EntryDataV2::Expected),
                        order_id: expected_entry.order_id.clone(),
                    });

                let expectation = SearchExpectationCreation {
                    rule_id: expected_entry.search_expectation.rule_id.clone(),
                    target_account_id: expected_entry.account_id.clone(),
                    source_value: encrypted_source_value,
                    hashed_source_value,
                    target_key: expected_entry.search_expectation.target_key.clone(),
                    merchant_id,
                };

                Ok((
                    creation_entry.into_entry_with_searchable_metadata(&exposed_keys.searchable),
                    Some(expectation),
                ))
            }
        }
    }
}

/// Validation and utility functions for transaction entries
pub trait TransactionEntryVecExt {
    fn validate_credit_debit_balance(&self) -> TransactionResult<()>;

    /// Validate all amounts are positive
    fn validate_positive_amounts(&self) -> TransactionResult<()>;

    /// Validate that all accounts exist, belong to the profile, and entry currencies match account currencies
    async fn validate_accounts_and_currencies(
        &self,
        auth: &AuthContext,
        store: &AppState,
    ) -> TransactionResult<()>;

    async fn to_creation_data(
        self,
        state: &AppState,
        merchant_id: MerchantId,
        profile_id: &str,
        encryptor: &EncryptionHandler<impl ApplicationEncryptionInterface>,
    ) -> TransactionResult<(
        Vec<EntryWithSearchableMetadata>,
        DieselTransactionData,
        Vec<SearchExpectationCreation>,
    )>;
}

fn exposed_keys_for_entry(
    entry: TransactionEntry,
    schema_id_by_config: &HashMap<String, String>,
    exposed_keys_by_schema: &HashMap<String, ExposedKeys>,
) -> TransactionResult<(TransactionEntry, ExposedKeys)> {
    let TransactionEntry::Direct(direct_entry) = entry else {
        return Ok((entry, ExposedKeys::default()));
    };
    let Some(config_id) = direct_entry.transformation_id.as_deref() else {
        return Ok((
            TransactionEntry::Direct(direct_entry),
            ExposedKeys::default(),
        ));
    };
    let metadata_schema_id = schema_id_by_config.get(config_id).ok_or(
        TransactionError::TransformationConfigNotFound {
            config_id: config_id.to_string(),
        },
    )?;
    let exposed_keys = exposed_keys_by_schema
        .get(metadata_schema_id)
        .ok_or(TransactionError::InternalServerError)?;
    Ok((TransactionEntry::Direct(direct_entry), exposed_keys.clone()))
}

impl TransactionEntryVecExt for Vec<TransactionEntry> {
    fn validate_credit_debit_balance(&self) -> TransactionResult<()> {
        let sum_of = |direction| -> TransactionResult<MinorUnit> {
            self.iter()
                .filter(|entry| entry.get_entry_type() == direction)
                .map(|entry| {
                    entry
                        .get_amount()
                        .to_minor_unit_as_i64(entry.get_currency())
                        .change_context(TransactionError::InvalidAmount {
                            details: "Invalid amount provided".to_string(),
                        })
                })
                .sum()
        };

        let credit_sum = sum_of(BalanceDirection::Credit)?;
        let debit_sum = sum_of(BalanceDirection::Debit)?;

        if credit_sum != debit_sum {
            return Err(TransactionError::UnbalancedTransaction {
                details: format!(
                    "Total credits ({credit_sum}) do not equal total debits ({debit_sum}) in minor units"
                ),
            }
            .into());
        }
        Ok(())
    }

    fn validate_positive_amounts(&self) -> TransactionResult<()> {
        let is_positive = self
            .iter()
            .map(|entry| entry.get_amount())
            .all(|amount| amount.is_positive());
        if !is_positive {
            return Err(TransactionError::InvalidAmount {
                details: "Amount must be greater than zero".to_string(),
            }
            .into());
        }
        Ok(())
    }

    async fn validate_accounts_and_currencies(
        &self,
        auth: &AuthContext,
        store: &AppState,
    ) -> TransactionResult<()> {
        let account_ids: Vec<String> = self
            .iter()
            .map(|entry| entry.get_account_id().to_string())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        let accounts = store
            .store
            .list_accounts_by_profile_id_and_account_ids(&auth.profile_id, &account_ids)
            .await
            .change_context(TransactionError::InternalServerError)?;

        let account_map: HashMap<String, _> = accounts
            .into_iter()
            .map(|account| (account.id.clone(), account))
            .collect();

        let transaction_currency = self
            .first()
            .ok_or(TransactionError::InsufficientEntries)?
            .get_currency();

        for entry in self {
            let account_id = entry.get_account_id();
            let entry_currency = entry.get_currency();

            if entry_currency != transaction_currency {
                return Err(TransactionError::EntryCurrencyMismatch.into());
            }

            let account =
                account_map
                    .get(account_id)
                    .ok_or_else(|| TransactionError::AccountNotFound {
                        account_id: account_id.to_string(),
                    })?;

            if entry_currency != account.currency {
                return Err(TransactionError::EntryCurrencyMismatch.into());
            }
        }

        Ok(())
    }

    async fn to_creation_data(
        self,
        state: &AppState,
        merchant_id: MerchantId,
        profile_id: &str,
        encryptor: &EncryptionHandler<impl ApplicationEncryptionInterface>,
    ) -> TransactionResult<(
        Vec<EntryWithSearchableMetadata>,
        DieselTransactionData,
        Vec<SearchExpectationCreation>,
    )> {
        let has_expected = self
            .iter()
            .any(|e| matches!(e, TransactionEntry::Expected(_)));

        let (direct_entry_data, transaction_data) = if has_expected {
            (
                DieselEntryData::V2(EntryDataV2::Pending),
                DieselTransactionData::V3(TransactionDataV3::Expected),
            )
        } else {
            (
                DieselEntryData::V2(EntryDataV2::Matched),
                DieselTransactionData::V3(TransactionDataV3::Matched(MatchedData::V1(
                    MatchedDataV1::Auto,
                ))),
            )
        };

        let config_ids: HashSet<String> = self
            .iter()
            .filter_map(|entry| match entry {
                TransactionEntry::Direct(direct_entry) => direct_entry.transformation_id.clone(),
                TransactionEntry::Expected(_) => None,
            })
            .collect();

        let transformation_configs = state
            .store
            .list_transformation_configs_by_ids_and_profile_id(
                config_ids.into_iter().collect(),
                profile_id,
            )
            .await
            .change_context(TransactionError::InternalServerError)?;

        let config_schema_map: HashMap<String, String> = transformation_configs
            .iter()
            .map(|config| (config.id.clone(), config.metadata_schema_id.clone()))
            .collect();

        let schemas = state
            .store
            .list_metadata_schemas_by_ids_and_profile_id(
                transformation_configs
                    .iter()
                    .map(|config| config.metadata_schema_id.clone())
                    .collect(),
                profile_id,
            )
            .await
            .change_context(TransactionError::InternalServerError)?;

        let exposed_keys_by_schema: HashMap<String, ExposedKeys> = schemas
            .into_iter()
            .map(|schema| (schema.id, schema.schema_data.exposed_keys()))
            .collect();

        let entries_with_exposed_keys = self
            .into_iter()
            .map(|entry| exposed_keys_for_entry(entry, &config_schema_map, &exposed_keys_by_schema))
            .collect::<TransactionResult<Vec<_>>>()?;

        let entry_results = future::try_join_all(entries_with_exposed_keys.into_iter().map(
            |(entry, exposed_keys)| {
                entry.to_creation_entry(
                    direct_entry_data.clone(),
                    merchant_id.clone(),
                    exposed_keys,
                    encryptor,
                )
            },
        ))
        .await?;

        let (creation_entries, expectations): (Vec<_>, Vec<_>) = entry_results.into_iter().unzip();
        let search_expectations: Vec<_> = expectations.into_iter().flatten().collect();

        Ok((creation_entries, transaction_data, search_expectations))
    }
}

/// Trait for converting transaction models to responses
pub trait TransactionExt {
    /// Convert a Transaction database model to GetTransactionResponse API model
    fn to_response(
        self,
        aging_config: &AgingConfig,
        modified_by: Option<User>,
    ) -> TransactionResult<GetTransactionResponse>;

    /// Validate the integrity of ledger principles for this transaction
    fn validate_ledger_integrity(&self) -> TransactionResult<()>;

    /// Get user details for the modified_by field by fetching from user_client
    async fn get_modified_by(
        &self,
        state: &AppState,
        merchant_id: MerchantId,
        profile_id: String,
    ) -> TransactionResult<Option<User>>;
}

/// Implementation of TransactionResponseConverter for Transaction
impl TransactionExt for DomainTransaction {
    fn to_response(
        self,
        aging_config: &AgingConfig,
        modified_by: Option<User>,
    ) -> TransactionResult<GetTransactionResponse> {
        // Convert amounts from minor units to AmountResponse for API response
        let credit_amount = AmountResponse::from_minor_unit(self.credit_amount, self.currency)
            .change_context(TransactionError::InternalServerError)?;
        let debit_amount = AmountResponse::from_minor_unit(self.debit_amount, self.currency)
            .change_context(TransactionError::InternalServerError)?;

        Ok(GetTransactionResponse {
            id: self.id,
            transaction_id: self.transaction_id,
            entry_id: self.entry_id,
            status: ApiTransactionStatus::from_domain_transaction_status(
                &DomainTransactionStatus::from(&self.data),
                aging_config,
                self.effective_at,
                self.discarded_at.unwrap_or(date_time::now()),
            ),
            version: self.version,
            profile_id: self.profile_id,
            rule_id: self.rule_id,
            credit_amount,
            debit_amount,
            discarded_at: self.discarded_at,
            discarded_status: self.discarded_data.as_ref().map(|transaction_data| {
                ApiTransactionStatus::from_domain_transaction_status(
                    &DomainTransactionStatus::from(transaction_data),
                    aging_config,
                    self.effective_at,
                    self.discarded_at.unwrap_or(date_time::now()),
                )
            }),
            discarded_data: self.discarded_data.map(|d| d.to_api_response()),
            created_at: self.created_at,
            effective_at: self.effective_at,
            data: self.data.to_api_response(),
            modified_by,
        })
    }

    fn validate_ledger_integrity(&self) -> TransactionResult<()> {
        let credit_amount = self.credit_amount.get_amount_as_i64();
        let debit_amount = self.debit_amount.get_amount_as_i64();

        // Check whether credit amount equals debit amount
        if credit_amount != debit_amount {
            return Err(TransactionError::UnbalancedTransaction {
                details: format!(
                    "Credit amount ({credit_amount}) does not equal debit amount ({debit_amount})"
                ),
            }
            .into());
        }

        Ok(())
    }

    async fn get_modified_by(
        &self,
        state: &AppState,
        merchant_id: MerchantId,
        profile_id: String,
    ) -> TransactionResult<Option<User>> {
        let Some(user_id) = self.modified_by.clone() else {
            return Ok(None);
        };
        let user_response = state
            .user_client
            .get_user_by_id(merchant_id, profile_id, user_id)
            .await
            .change_context(TransactionError::InternalServerError)?;
        Ok(Some(User::foreign_from(user_response)))
    }
}

/// Raw grouped transaction data (no API conversions)
/// Contains all data from TransactionAccountView grouped by (transaction_id, version)
pub struct GroupedTransactionViews {
    pub transaction: DomainTransaction,
    pub entries_with_accounts: Vec<(DieselEntry, Account)>,
    pub rule: ReconRule,
    pub has_more_entries: bool,
}

impl GroupedTransactionViews {
    pub fn to_response(
        self,
        modified_by: Option<User>,
    ) -> TransactionResult<TransactionWithEntriesResponse> {
        // Convert amounts from minor units to AmountResponse
        let credit_amount = AmountResponse::from_minor_unit(
            self.transaction.credit_amount,
            self.transaction.currency,
        )
        .change_context(TransactionError::InternalServerError)?;
        let debit_amount = AmountResponse::from_minor_unit(
            self.transaction.debit_amount,
            self.transaction.currency,
        )
        .change_context(TransactionError::InternalServerError)?;

        // Convert entries with accounts to CompactEntryData
        let entries = self
            .entries_with_accounts
            .into_iter()
            .map(|(entry, account)| {
                let mismatch = if entry.status == EntryStatus::Mismatched {
                    match entry
                        .data
                        .clone()
                        .to_api_response()
                        .change_context(TransactionError::InternalServerError)?
                    {
                        EntryDataApi::Mismatched { mismatch_type } => Some(mismatch_type),
                        _ => None,
                    }
                } else {
                    None
                };

                Ok(CompactEntryData {
                    entry_id: entry.id,
                    entry_type: entry.balance_direction,
                    status: entry.status,
                    amount: AmountResponse::from_minor_unit(entry.amount, entry.currency)
                        .change_context(TransactionError::InternalServerError)?,
                    account: AccountData {
                        account_id: account.id,
                        account_name: account.name,
                    },
                    order_id: entry.entry_identifier,
                    staging_entry_id: entry.primary_staging_entry_id,
                    mismatch,
                })
            })
            .collect::<Result<Vec<_>, error_stack::Report<TransactionError>>>()?;

        Ok(TransactionWithEntriesResponse {
            id: self.transaction.id,
            transaction_id: self.transaction.transaction_id,
            created_at: self.transaction.created_at,
            effective_at: self.transaction.effective_at,
            status: ApiTransactionStatus::from_domain_transaction_status(
                &DomainTransactionStatus::from(&self.transaction.data),
                &self.rule.aging_config,
                self.transaction.effective_at,
                self.transaction.discarded_at.unwrap_or(date_time::now()),
            ),
            version: self.transaction.version,
            profile_id: self.transaction.profile_id,
            rule: RuleData {
                rule_id: self.rule.id,
                rule_name: self.rule.name,
            },
            credit_amount,
            debit_amount,
            entries,
            has_more_entries: self.has_more_entries,
            discarded_status: self
                .transaction
                .discarded_data
                .as_ref()
                .map(|transaction_data| {
                    ApiTransactionStatus::from_domain_transaction_status(
                        &DomainTransactionStatus::from(transaction_data),
                        &self.rule.aging_config,
                        self.transaction.effective_at,
                        self.transaction.discarded_at.unwrap_or(date_time::now()),
                    )
                }),
            data: self.transaction.data.to_api_response(),
            discarded_data: self.transaction.discarded_data.map(|d| d.to_api_response()),
            discarded_at: self.transaction.discarded_at,
            modified_by,
        })
    }
}

impl CursorSort<GroupedTransactionViews> for TransactionListSortField {
    fn cursor_at(&self, item: &GroupedTransactionViews) -> Self {
        match self {
            Self::Id { .. } => Self::Id {
                id: Some(item.transaction.id.clone()),
            },
            Self::EffectiveAt { .. } => Self::EffectiveAt {
                cursor_value: Some(TransactionEffectiveAtSortField {
                    effective_at: item.transaction.effective_at,
                    id: item.transaction.id.clone(),
                }),
            },
        }
    }

    fn cursor_supplied(&self) -> bool {
        match self {
            Self::Id { id } => id.is_some(),
            Self::EffectiveAt { cursor_value } => cursor_value.is_some(),
        }
    }
}

pub async fn hydrate_transaction_rule_views(
    state: &AppState,
    encryptor: &EncryptionHandler<impl ApplicationEncryptionInterface>,
    profile_id: &str,
    views: Vec<TransactionRuleView>,
    max_entries_per_transaction: u16,
) -> TransactionResult<Vec<GroupedTransactionViews>> {
    let entry_cap = usize::from(max_entries_per_transaction);
    let candidate_count = entry_cap.saturating_add(1);

    let candidate_entry_ids: Vec<String> = views
        .iter()
        .flat_map(|view| {
            view.transaction
                .entry_ids
                .iter()
                .take(candidate_count)
                .cloned()
        })
        .collect();

    let entries = state
        .store
        .list_entries_by_ids(candidate_entry_ids)
        .await
        .change_context(TransactionError::InternalServerError)?;

    let account_ids: Vec<String> = entries
        .iter()
        .map(|entry| entry.account_id.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    let accounts_by_id: HashMap<String, Account> = state
        .store
        .list_accounts_by_profile_id_and_account_ids(profile_id, &account_ids)
        .await
        .change_context(TransactionError::InternalServerError)?
        .into_iter()
        .map(|account| (account.id.clone(), account))
        .collect();

    let entries_by_id = entries.into_iter().try_fold(
        HashMap::<String, (DieselEntry, Account)>::new(),
        |mut acc, entry| {
            let account = match accounts_by_id.get(&entry.account_id) {
                Some(account) => account.clone(),
                None => {
                    return Err(
                        report!(TransactionError::InternalServerError).attach_printable(format!(
                            "Account {} not found for entry {} while listing transactions",
                            entry.account_id, entry.id
                        )),
                    )
                }
            };

            acc.insert(entry.id.clone(), (entry, account));
            TransactionResult::Ok(acc)
        },
    )?;

    future::try_join_all(views.into_iter().map(|view| {
        let collected =
            view.transaction
                .entry_ids
                .iter()
                .take(candidate_count)
                .map(|entry_id| {
                    let (entry, account) = match entries_by_id.get(entry_id) {
                        Some(entry_with_account) => entry_with_account,
                        None => {
                            return Err(report!(TransactionError::InternalServerError)
                                .attach_printable(format!(
                            "Entry {entry_id} referenced by transaction {} version {} not found",
                            view.transaction.transaction_id, view.transaction.version
                        )))
                        }
                    };

                    if entry.logical_transaction_id != view.transaction.transaction_id
                        || entry.transaction_version != view.transaction.version
                    {
                        return Err(report!(TransactionError::InternalServerError)
                            .attach_printable(format!(
                        "Entry {entry_id} belongs to transaction {} version {}, not {} version {}",
                        entry.logical_transaction_id,
                        entry.transaction_version,
                        view.transaction.transaction_id,
                        view.transaction.version
                    )));
                    }

                    Ok((entry.clone(), account.clone()))
                })
                .collect::<TransactionResult<Vec<_>>>();

        let mut entries_with_accounts = match collected {
            Ok(entries_with_accounts) => entries_with_accounts,
            Err(error) => return Either::Left(future::ready(Err(error))),
        };

        let has_more_entries = entries_with_accounts.len() > entry_cap;
        entries_with_accounts.truncate(entry_cap);

        Either::Right(async move {
            let transaction = view
                .transaction
                .convert_back(encryptor)
                .await
                .change_context(TransactionError::InternalServerError)?;

            TransactionResult::Ok(GroupedTransactionViews {
                transaction,
                entries_with_accounts,
                rule: view.recon_rule,
                has_more_entries,
            })
        })
    }))
    .await
}

/// Pure grouping function - groups TransactionAccountView by (transaction_id, version)
/// Returns raw diesel models without any API conversions or error handling
pub async fn group_transaction_views(
    encryptor: &EncryptionHandler<impl ApplicationEncryptionInterface>,
    views: Vec<TransactionAccountView>,
) -> TransactionResult<Vec<GroupedTransactionViews>> {
    let mut grouped: HashMap<(String, i32), GroupedTransactionViews> = HashMap::new();

    for view in views {
        let key = (
            view.transaction.transaction_id.clone(),
            view.transaction.version,
        );

        match grouped.get_mut(&key) {
            Some(group) => {
                group.entries_with_accounts.push((view.entry, view.account));
            }
            None => {
                let transaction = view
                    .transaction
                    .convert_back(encryptor)
                    .await
                    .change_context(TransactionError::InternalServerError)?;

                grouped.insert(
                    key.clone(),
                    GroupedTransactionViews {
                        transaction,
                        entries_with_accounts: vec![(view.entry, view.account)],
                        rule: view.recon_rule.clone(),
                        has_more_entries: false,
                    },
                );
            }
        }
    }

    Ok(grouped.into_values().collect())
}

/// Trait for converting Diesel transaction data to API transaction data
pub trait TransactionDataExt<T> {
    /// Convert a DieselTransactionData model to a TransactionDataApi model
    fn to_api_response(self) -> T;
}

/// Implementation of TransactionDataExt for DieselTransactionData
impl TransactionDataExt<TransactionDataApi> for DieselTransactionData {
    fn to_api_response(self) -> TransactionDataApi {
        match self {
            Self::V3(v3_data) => v3_data.to_api_response(),
        }
    }
}

/// Implementation of conversion for VoidData to API response
impl TransactionDataExt<TransactionDataApi> for VoidData {
    fn to_api_response(self) -> TransactionDataApi {
        match self {
            Self::V1(v1_data) => v1_data.to_api_response(),
        }
    }
}

/// Implementation of conversion for VoidDataV1 to API response
impl TransactionDataExt<TransactionDataApi> for VoidDataV1 {
    fn to_api_response(self) -> TransactionDataApi {
        TransactionDataApi::Void {
            reason: self.reason,
        }
    }
}

/// Implementation of conversion for TransactionDataV3 to API response
impl TransactionDataExt<TransactionDataApi> for TransactionDataV3 {
    fn to_api_response(self) -> TransactionDataApi {
        match self {
            Self::Expected => TransactionDataApi::Expected,
            Self::Posted(posted_data) => posted_data.to_api_response(),
            Self::Matched(matched_data) => matched_data.to_api_response(),
            Self::OverAmount(AmountMismatchData::V1(AmountMismatchDataV1::Mismatch)) => {
                TransactionDataApi::Mismatched
            }
            Self::OverAmount(AmountMismatchData::V1(AmountMismatchDataV1::Expected)) => {
                TransactionDataApi::Mismatched
            }
            Self::UnderAmount(AmountMismatchData::V1(AmountMismatchDataV1::Mismatch)) => {
                TransactionDataApi::Mismatched
            }
            Self::UnderAmount(AmountMismatchData::V1(AmountMismatchDataV1::Expected)) => {
                TransactionDataApi::Mismatched
            }
            Self::DataMismatch(fields) => {
                TransactionDataApi::DataMismatch(MetadataMismatchDataApi {
                    mismatched_fields: fields
                        .mismatched_fields
                        .into_iter()
                        .map(FieldMismatchDetail::to_api_response)
                        .collect(),
                })
            }
            Self::CurrencyMismatch => TransactionDataApi::Mismatched,
            Self::SplitMismatch => TransactionDataApi::Mismatched,
            Self::Archived => TransactionDataApi::Archived,
            Self::Void(void_data) => void_data.to_api_response(),
            Self::PartiallyReconciled(partially_reconciled_data) => {
                partially_reconciled_data.to_api_response()
            }
        }
    }
}

/// Implementation of conversion for MatchedData to API response
impl TransactionDataExt<TransactionDataApi> for MatchedData {
    fn to_api_response(self) -> TransactionDataApi {
        match self {
            Self::V1(v1_data) => v1_data.to_api_response(),
        }
    }
}

/// Implementation of conversion for MatchedDataV1 to API response
impl TransactionDataExt<TransactionDataApi> for MatchedDataV1 {
    fn to_api_response(self) -> TransactionDataApi {
        match self {
            Self::Force(force_matched_data) => force_matched_data.to_api_response(),
            Self::Manual(manually_matched_data) => manually_matched_data.to_api_response(),
            Self::Auto => TransactionDataApi::Matched(MatchedDataApi::Auto),
            Self::WithTolerance => TransactionDataApi::Matched(MatchedDataApi::WithTolerance),
        }
    }
}

/// Implementation of conversion for ForceMatchedData to API response
impl TransactionDataExt<TransactionDataApi> for ForceMatchedData {
    fn to_api_response(self) -> TransactionDataApi {
        match self {
            Self::V1(v1_data) => v1_data.to_api_response(),
        }
    }
}

/// Implementation of conversion for ForceMatchedDataV1 to API response
impl TransactionDataExt<TransactionDataApi> for ForceMatchedDataV1 {
    fn to_api_response(self) -> TransactionDataApi {
        TransactionDataApi::Matched(MatchedDataApi::Force(ForceMatchedDataApi {
            reason: self.reason,
        }))
    }
}

/// Implementation of conversion for ManuallyMatchedData to API response
impl TransactionDataExt<TransactionDataApi> for ManuallyMatchedData {
    fn to_api_response(self) -> TransactionDataApi {
        match self {
            Self::V1(v1_data) => v1_data.to_api_response(),
        }
    }
}

/// Implementation of conversion for ManuallyMatchedDataV1 to API response
impl TransactionDataExt<TransactionDataApi> for ManuallyMatchedDataV1 {
    fn to_api_response(self) -> TransactionDataApi {
        TransactionDataApi::Matched(MatchedDataApi::Manual(ManuallyMatchedDataApi {
            reason: self.reason,
        }))
    }
}

/// Implementation of conversion for PostedData to API response
impl TransactionDataExt<TransactionDataApi> for PostedData {
    fn to_api_response(self) -> TransactionDataApi {
        match self {
            Self::V1(v1_data) => v1_data.to_api_response(),
        }
    }
}

/// Implementation of conversion for PostedDataV1 to API response
impl TransactionDataExt<TransactionDataApi> for PostedDataV1 {
    fn to_api_response(self) -> TransactionDataApi {
        match self {
            Self::Manual(ManualPostedData::V1(v1)) => {
                TransactionDataApi::Posted(PostedDataApi::Manual(ManuallyPostedData {
                    reason: v1.reason,
                }))
            }
        }
    }
}

impl TransactionDataExt<TransactionDataApi> for PartiallyReconciledData {
    fn to_api_response(self) -> TransactionDataApi {
        match self {
            Self::V1(v1_data) => v1_data.to_api_response(),
        }
    }
}

impl TransactionDataExt<TransactionDataApi> for PartiallyReconciledDataV1 {
    fn to_api_response(self) -> TransactionDataApi {
        TransactionDataApi::PartiallyReconciled {
            reason: self.reason,
        }
    }
}

pub trait TransactionEntryWithDataVecExt {
    fn validate_credit_debit_balance(&self) -> TransactionResult<()>;

    /// Validate all amounts are positive
    fn validate_positive_amounts(&self) -> TransactionResult<()>;

    /// Validate that all accounts exist, belong to the profile, and entry currencies match account currencies
    async fn validate_accounts_and_currencies(
        &self,
        auth: &AuthContext,
        store: &AppState,
    ) -> TransactionResult<()>;
}

impl TransactionEntryWithDataVecExt for Vec<TransactionEntryWithData> {
    fn validate_credit_debit_balance(&self) -> TransactionResult<()> {
        let sum_of = |direction| -> TransactionResult<MinorUnit> {
            self.iter()
                .filter(|entry| entry.entry_type == direction)
                .map(|entry| {
                    entry
                        .amount
                        .to_minor_unit_as_i64(entry.currency)
                        .change_context(TransactionError::InvalidAmount {
                            details: "Invalid amount provided".to_string(),
                        })
                })
                .sum()
        };

        let credit_sum = sum_of(BalanceDirection::Credit)?;
        let debit_sum = sum_of(BalanceDirection::Debit)?;

        if credit_sum != debit_sum {
            return Err(TransactionError::UnbalancedTransaction {
                details: format!(
                    "Total credits ({credit_sum}) do not equal total debits ({debit_sum}) in minor units"
                ),
            }
            .into());
        }

        Ok(())
    }

    /// Validate all amounts are positive
    fn validate_positive_amounts(&self) -> TransactionResult<()> {
        let is_positive = self
            .iter()
            .map(|entry| entry.amount)
            .all(|amount| amount.is_positive());

        if !is_positive {
            return Err(TransactionError::InvalidAmount {
                details: "Amount must be greater than zero".to_string(),
            }
            .into());
        }

        Ok(())
    }

    /// Validate that all accounts exist, belong to the profile, and entry currencies match account currencies
    async fn validate_accounts_and_currencies(
        &self,
        auth: &AuthContext,
        store: &AppState,
    ) -> TransactionResult<()> {
        // Collect all unique account IDs from entries
        let account_ids: Vec<String> = self
            .iter()
            .map(|entry| entry.account_id.clone())
            .collect::<HashSet<_>>() // Remove duplicates
            .into_iter()
            .collect();

        // Fetch all accounts in a single batch query
        let accounts = store
            .store
            .list_accounts_by_profile_id_and_account_ids(&auth.profile_id, &account_ids)
            .await
            .change_context(TransactionError::InternalServerError)?;

        // Create a map for quick account lookup
        let account_map: HashMap<String, _> = accounts
            .into_iter()
            .map(|account| (account.id.clone(), account))
            .collect();

        // Validate each entry against the fetched accounts
        for entry in self {
            let account = account_map.get(&entry.account_id).ok_or_else(|| {
                TransactionError::AccountNotFound {
                    account_id: entry.account_id.clone(),
                }
            })?;

            if entry.currency != account.currency {
                return Err(TransactionError::EntryCurrencyMismatch.into());
            }
        }
        Ok(())
    }
}

pub trait DieselTransactionStatusExt {
    fn from(value: &ApiTransactionFilterStatus) -> Self;
}

impl DieselTransactionStatusExt for DieselTransactionStatus {
    fn from(value: &ApiTransactionFilterStatus) -> Self {
        match value {
            ApiTransactionFilterStatus::Expected | ApiTransactionFilterStatus::Missing => {
                Self::Expected
            }
            ApiTransactionFilterStatus::PostedManual => Self::PostedManual,
            ApiTransactionFilterStatus::MatchedAuto => Self::MatchedAuto,
            ApiTransactionFilterStatus::MatchedManual => Self::MatchedManual,
            ApiTransactionFilterStatus::MatchedForce => Self::MatchedForce,
            ApiTransactionFilterStatus::MatchedWithTolerance => Self::MatchedWithTolerance,
            ApiTransactionFilterStatus::OverAmountExpected => Self::OverAmountExpected,
            ApiTransactionFilterStatus::OverAmountMismatch => Self::OverAmountMismatch,
            ApiTransactionFilterStatus::UnderAmountExpected => Self::UnderAmountExpected,
            ApiTransactionFilterStatus::UnderAmountMismatch => Self::UnderAmountMismatch,
            ApiTransactionFilterStatus::DataMismatch => Self::DataMismatch,
            ApiTransactionFilterStatus::CurrencyMismatch => Self::CurrencyMismatch,
            ApiTransactionFilterStatus::SplitMismatch => Self::SplitMismatch,
            ApiTransactionFilterStatus::Archived => Self::Archived,
            ApiTransactionFilterStatus::Void => Self::Void,
            ApiTransactionFilterStatus::PartiallyReconciled => Self::PartiallyReconciled,
        }
    }
}

impl ForeignFrom<ApiTransactionAccountFilters> for TransactionAccountFilters {
    fn foreign_from(value: ApiTransactionAccountFilters) -> Self {
        Self {
            transaction_id: value.transaction_id,
            status: value
                .status
                .as_ref()
                .map(|s| s.iter().map(DieselTransactionStatusExt::from).collect()),
            time_range: value.time_range,
            rule_id: value.rule_id,
            credit_account: value.credit_account,
            debit_account: value.debit_account,
        }
    }
}

impl ForeignFrom<ApiTransactionListFiltersV2> for TransactionListFiltersV2 {
    fn foreign_from(value: ApiTransactionListFiltersV2) -> Self {
        Self {
            transaction_id: value.transaction_id,
            status: value.status.map(|statuses| {
                statuses
                    .into_iter()
                    .map(TransactionListStatusFilter::foreign_from)
                    .collect()
            }),
            time_range: value.time_range,
            rule_id: value.rule_id,
            entry_fields: TransactionListEntryFilters {
                order_id: value.order_id,
                metadata: value
                    .metadata
                    .into_iter()
                    .map(|metadata_filter| TransactionListMetadataFilters {
                        key: metadata_filter.key,
                        value: metadata_filter.value,
                    })
                    .collect(),
            },
        }
    }
}

impl ForeignFrom<ApiTransactionFilterStatus> for TransactionListStatusFilter {
    fn foreign_from(value: ApiTransactionFilterStatus) -> Self {
        match value {
            ApiTransactionFilterStatus::Expected => Self::Expected,
            ApiTransactionFilterStatus::PostedManual => Self::PostedManual,
            ApiTransactionFilterStatus::OverAmountExpected => Self::OverAmountExpected,
            ApiTransactionFilterStatus::OverAmountMismatch => Self::OverAmountMismatch,
            ApiTransactionFilterStatus::UnderAmountExpected => Self::UnderAmountExpected,
            ApiTransactionFilterStatus::UnderAmountMismatch => Self::UnderAmountMismatch,
            ApiTransactionFilterStatus::DataMismatch => Self::DataMismatch,
            ApiTransactionFilterStatus::CurrencyMismatch => Self::CurrencyMismatch,
            ApiTransactionFilterStatus::SplitMismatch => Self::SplitMismatch,
            ApiTransactionFilterStatus::Archived => Self::Archived,
            ApiTransactionFilterStatus::Void => Self::Void,
            ApiTransactionFilterStatus::PartiallyReconciled => Self::PartiallyReconciled,
            ApiTransactionFilterStatus::Missing => Self::Missing,
            ApiTransactionFilterStatus::MatchedAuto => Self::MatchedAuto,
            ApiTransactionFilterStatus::MatchedManual => Self::MatchedManual,
            ApiTransactionFilterStatus::MatchedForce => Self::MatchedForce,
            ApiTransactionFilterStatus::MatchedWithTolerance => Self::MatchedWithTolerance,
        }
    }
}

pub trait ApiTransactionStatusExt {
    fn from_domain_transaction_status(
        domain_transaction_status: &DomainTransactionStatus,
        aging_config: &AgingConfig,
        start_date: PrimitiveDateTime,
        end_date: PrimitiveDateTime,
    ) -> Self;
}

impl ApiTransactionStatusExt for ApiTransactionStatus {
    fn from_domain_transaction_status(
        domain_transaction_status: &DomainTransactionStatus,
        aging_config: &AgingConfig,
        start_date: PrimitiveDateTime,
        end_date: PrimitiveDateTime,
    ) -> Self {
        match domain_transaction_status {
            DomainTransactionStatus::Expected => {
                if aging_config.check_if_missing(start_date, end_date) {
                    Self::Missing
                } else {
                    Self::Expected
                }
            }
            DomainTransactionStatus::Posted(TransactionPostedData::Manual) => {
                Self::Posted(TransactionPostedData::Manual)
            }
            DomainTransactionStatus::Matched(TransactionMatchedData::Auto) => {
                Self::Matched(TransactionMatchedData::Auto)
            }
            DomainTransactionStatus::Matched(TransactionMatchedData::Manual) => {
                Self::Matched(TransactionMatchedData::Manual)
            }
            DomainTransactionStatus::Matched(TransactionMatchedData::Force) => {
                Self::Matched(TransactionMatchedData::Force)
            }
            DomainTransactionStatus::Matched(TransactionMatchedData::WithTolerance) => {
                Self::Matched(TransactionMatchedData::WithTolerance)
            }
            DomainTransactionStatus::OverAmount(TransactionAmountMismatchData::Expected) => {
                Self::OverAmount(TransactionAmountMismatchData::Expected)
            }
            DomainTransactionStatus::OverAmount(TransactionAmountMismatchData::Mismatch) => {
                Self::OverAmount(TransactionAmountMismatchData::Mismatch)
            }
            DomainTransactionStatus::UnderAmount(TransactionAmountMismatchData::Expected) => {
                Self::UnderAmount(TransactionAmountMismatchData::Expected)
            }
            DomainTransactionStatus::UnderAmount(TransactionAmountMismatchData::Mismatch) => {
                Self::UnderAmount(TransactionAmountMismatchData::Mismatch)
            }
            DomainTransactionStatus::DataMismatch => Self::DataMismatch,
            DomainTransactionStatus::CurrencyMismatch => Self::CurrencyMismatch,
            DomainTransactionStatus::SplitMismatch => Self::SplitMismatch,
            DomainTransactionStatus::Archived => Self::Archived,
            DomainTransactionStatus::Void => Self::Void,
            DomainTransactionStatus::PartiallyReconciled => Self::PartiallyReconciled,
        }
    }
}

pub trait ApiTransactionFilterStatusExt {
    fn from_domain_transaction_status(
        domain_transaction_status: &DomainTransactionStatus,
        aging_config: &AgingConfig,
        start_date: PrimitiveDateTime,
        end_date: PrimitiveDateTime,
    ) -> Self;
}

impl ApiTransactionFilterStatusExt for ApiTransactionFilterStatus {
    fn from_domain_transaction_status(
        domain_transaction_status: &DomainTransactionStatus,
        aging_config: &AgingConfig,
        start_date: PrimitiveDateTime,
        end_date: PrimitiveDateTime,
    ) -> Self {
        match domain_transaction_status {
            DomainTransactionStatus::Expected => {
                if aging_config.check_if_missing(start_date, end_date) {
                    Self::Missing
                } else {
                    Self::Expected
                }
            }
            DomainTransactionStatus::Posted(TransactionPostedData::Manual) => Self::PostedManual,
            DomainTransactionStatus::Matched(TransactionMatchedData::Auto) => Self::MatchedAuto,
            DomainTransactionStatus::Matched(TransactionMatchedData::Manual) => Self::MatchedManual,
            DomainTransactionStatus::Matched(TransactionMatchedData::Force) => Self::MatchedForce,
            DomainTransactionStatus::Matched(TransactionMatchedData::WithTolerance) => {
                Self::MatchedWithTolerance
            }
            DomainTransactionStatus::OverAmount(TransactionAmountMismatchData::Expected) => {
                Self::OverAmountExpected
            }
            DomainTransactionStatus::OverAmount(TransactionAmountMismatchData::Mismatch) => {
                Self::OverAmountMismatch
            }
            DomainTransactionStatus::UnderAmount(TransactionAmountMismatchData::Expected) => {
                Self::UnderAmountExpected
            }
            DomainTransactionStatus::UnderAmount(TransactionAmountMismatchData::Mismatch) => {
                Self::UnderAmountMismatch
            }
            DomainTransactionStatus::DataMismatch => Self::DataMismatch,
            DomainTransactionStatus::CurrencyMismatch => Self::CurrencyMismatch,
            DomainTransactionStatus::SplitMismatch => Self::SplitMismatch,
            DomainTransactionStatus::Archived => Self::Archived,
            DomainTransactionStatus::Void => Self::Void,
            DomainTransactionStatus::PartiallyReconciled => Self::PartiallyReconciled,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entries(rows: &[(BalanceDirection, f64)]) -> Vec<TransactionEntryWithData> {
        rows.iter()
            .map(|(entry_type, amount)| TransactionEntryWithData {
                account_id: "account_test".to_string(),
                entry_type: *entry_type,
                amount: FloatMajorUnit::new(*amount),
                currency: Currency::AUD,
                effective_at: date_time::now(),
                metadata: serde_json::json!({}).into(),
                staging_entry_id: None,
                data: EntryDataApi::Pending,
                order_id: "order_test".to_string(),
            })
            .collect()
    }

    #[test]
    fn balanced_amounts_that_are_unequal_as_f64_still_balance() {
        // 324.71 + 181.51 sums to a different double than 506.22 (off by ~5.7e-14).
        assert!(entries(&[
            (BalanceDirection::Credit, 324.71),
            (BalanceDirection::Debit, 506.22),
            (BalanceDirection::Credit, 181.51),
        ])
        .validate_credit_debit_balance()
        .is_ok());

        // The payload that happened to land on identical doubles must keep passing.
        assert!(entries(&[
            (BalanceDirection::Credit, 777.93),
            (BalanceDirection::Debit, 2373.06),
            (BalanceDirection::Credit, 1595.13),
        ])
        .validate_credit_debit_balance()
        .is_ok());
    }

    #[test]
    fn genuinely_unbalanced_amounts_are_rejected() {
        assert!(entries(&[
            (BalanceDirection::Credit, 324.71),
            (BalanceDirection::Debit, 506.22),
        ])
        .validate_credit_debit_balance()
        .is_err());
    }
}
