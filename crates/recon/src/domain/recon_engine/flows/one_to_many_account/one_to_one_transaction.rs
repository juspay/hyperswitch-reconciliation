use storage_impl::{
    account::AccountInterface, entry::EntryInterface, search_expectation::SearchCriteria,
    transaction::TransactionInterface,
};

use crate::{
    domain::metadata_schema::implementations::unique_constraint::UniqueConstraintExt,
    domain::recon_engine::{
        flows::{
            ReconContextConfirmation, ReconContextTransaction, ReconEngine, ReconPrepConfirmation,
            ReconPrepTransaction,
        },
        implementations::{
            match_rules::MatchRulesExt, search_identifier::SearchIdentifierExt, trigger::TriggerExt,
        },
        rule_orchestration::{ConfirmationContext, TransactionContext},
        utils::{
            couple_entries_with_searchable_metadata, extract_unique_field, hash_string,
            DomainEntryExt,
        },
        MatchRuleValidation, ProcessingError, ProcessingResult,
    },
    routes::AppState,
    utils::recon_rule::RuleAccountExt,
};
use async_bb8_diesel::AsyncConnection;
use common_enums::EntryStatus;
use common_types::{
    entry::{
        BalanceDirectionMismatchData, BalanceDirectionMismatchDataV1, CurrencyMismatchData,
        CurrencyMismatchDataV1, EntryAmountMismatchData, EntryAmountMismatchDataV1, EntryData,
        EntryDataV2, EntryMismatchedData, EntryMismatchedDataV1, MetadataMismatchData,
        MetadataMismatchDataV1,
    },
    metadata_schema::UniqueConstraintProvider,
    recon_rule::{
        entry_field::ExtractFromEntryField,
        match_rules::MatchRules,
        strategies::one_to_many_strategy::{
            FixedSplitValue, OneToManySingleSingleTarget, OneToManySingleSingleTargetType,
            OneToManySingleSingleV1,
        },
    },
    staging_entry::{StagingEntryData, StagingEntryDataV2},
    transaction::{
        status::{DomainTransactionStatus, TransactionAmountMismatchData},
        AmountMismatchData, AmountMismatchDataV1, MatchedData, MatchedDataV1, TransactionData,
        TransactionDataV3,
    },
    units::{FloatMajorUnit, MinorUnit},
};
use common_utils::{id_type::MerchantId, logging};
use diesel_models::{
    entry::EntryNew,
    errors::DatabaseError,
    metadata_schema::MetadataSchema,
    search_duplicate::SearchDuplicateNew,
    search_entry_key::SearchEntryKeyNew,
    search_expectation::SearchExpectationNew,
    staging_entry::{
        StagingEntry, StagingEntryNew, StagingEntryUpdate, StagingEntryUpdateInternal,
    },
    transaction::{Transaction, TransactionNew},
};
use error_stack::{Report, ResultExt};
use futures::future;
use masking::ExposeInterface;
use recon_domain_models::{
    behaviour::{Conversion, ReverseConversion},
    entry::DomainEntry,
    search_duplicate::{DomainSearchDuplicate, SearchDuplicateCreationData},
    search_entry_key::DomainSearchEntryKey,
    search_expectation::DomainSearchExpectation,
    staging_entry::{DomainStagingEntry, StagingEntryDomainVersionUpdate},
    transaction::{
        DirectTransactionCreationEntry, DomainTransaction, ExpectedTransactionCreationEntry,
        SearchExpectationCreation, TransactionCreationData, TransactionCreationEntry,
        TransactionUpdateData,
    },
};

pub struct OneToOneTransactionContext {
    staging_entry_archive: (DomainStagingEntry, StagingEntryUpdateInternal),
    processed_staging_entry: StagingEntryNew,
    new_search_duplicate: SearchDuplicateNew,
    new_entries: Vec<EntryNew>,
    new_search_expectations: Vec<SearchExpectationNew>,
    new_search_entry_keys: Vec<SearchEntryKeyNew>,
    new_transaction: TransactionNew,
}

pub struct OneToOneConfirmationContext {
    staging_entry_archive: (DomainStagingEntry, StagingEntryUpdateInternal),
    processed_staging_entry: StagingEntryNew,
    new_search_duplicate: SearchDuplicateNew,
    transaction_archive: DomainTransaction,
    new_transaction: TransactionNew,
    new_entries: Vec<EntryNew>,
    new_search_expectations: Vec<SearchExpectationNew>,
    new_search_entry_keys: Vec<SearchEntryKeyNew>,
}

pub struct OneToOneTransactionPrepType {
    rule_id: String,
    profile_id: String,
    merchant_id: MerchantId,
    staging_entry: DomainStagingEntry,
    metadata_schema: MetadataSchema,
}

pub struct OneToOneConfirmationPrepType {
    merchant_id: MerchantId,
    staging_entry: DomainStagingEntry,
    search_expectation: DomainSearchExpectation,
    metadata_schema: MetadataSchema,
}

pub struct OneToOneStatusDeterminationInput {
    staging_entry: DomainStagingEntry,
    expected_entry: DomainEntry,
    old_transaction_status: DomainTransactionStatus,
    remaining_entries: Vec<DomainEntry>,
}

pub struct OneToOneStatusDeterminationOutput {
    transaction_data: TransactionData,
    current_entry_data: EntryData,
}

#[async_trait::async_trait]
impl ReconEngine for OneToManySingleSingleV1 {
    type PrepTypeTransaction = OneToOneTransactionPrepType;
    type PrepTypeConfirmation = OneToOneConfirmationPrepType;
    type ContextTypeTransaction = OneToOneTransactionContext;
    type ContextTypeConfirmation = OneToOneConfirmationContext;
    type StatusDeterminationInput = OneToOneStatusDeterminationInput;
    type StatusDeterminationOutput = OneToOneStatusDeterminationOutput;

    async fn prep_transaction(
        self,
        state: &AppState,
        staging_entry: DomainStagingEntry,
        transaction_context: TransactionContext,
    ) -> ProcessingResult<ReconPrepTransaction> {
        let source_account = state
            .store
            .find_account_by_id(&self.source_account.account_id)
            .await
            .change_context(ProcessingError::DatabaseError)?;

        if staging_entry.currency != source_account.currency {
            return Err(ProcessingError::StagingEntryCurrencyMismatch {
                staging_currency: staging_entry.currency,
                account_currency: source_account.currency,
            }
            .into());
        }

        Ok(ReconPrepTransaction::OneToManySingleSingleV1 {
            engine: self,
            prep: OneToOneTransactionPrepType {
                rule_id: transaction_context.rule_id,
                profile_id: staging_entry.profile_id.clone(),
                merchant_id: staging_entry.merchant_id.clone(),
                staging_entry,
                metadata_schema: transaction_context.metadata_schema,
            },
        })
    }

    async fn prep_confirmation(
        self,
        state: &AppState,
        staging_entry: DomainStagingEntry,
        confirmation_context: ConfirmationContext,
    ) -> ProcessingResult<ReconPrepConfirmation> {
        let target_accounts = future::try_join_all(
            self.target_accounts
                .get_rule_accounts()
                .iter()
                .map(|(acc, _)| state.store.find_account_by_id(acc)),
        )
        .await
        .change_context(ProcessingError::DatabaseError)?;

        let target_account = target_accounts
            .into_iter()
            .find(|account| account.id == staging_entry.account_id)
            .ok_or(ProcessingError::InternalError(format!(
                "Account not found for id: {}",
                staging_entry.account_id
            )))?;

        if staging_entry.currency != target_account.currency {
            return Err(ProcessingError::StagingEntryCurrencyMismatch {
                staging_currency: staging_entry.currency,
                account_currency: target_account.currency,
            }
            .into());
        }

        Ok(ReconPrepConfirmation::OneToManySingleSingleV1 {
            engine: self,
            prep: OneToOneConfirmationPrepType {
                merchant_id: staging_entry.merchant_id.clone(),
                staging_entry,
                search_expectation: confirmation_context.domain_search_expectation,
                metadata_schema: confirmation_context.metadata_schema,
            },
        })
    }

    async fn create_transaction_context(
        self,
        state: &AppState,
        data: Self::PrepTypeTransaction,
    ) -> ProcessingResult<ReconContextTransaction> {
        let encryptor = state
            .encryption_client
            .get_encryptor_for_merchant(data.merchant_id.clone());

        let staging_entry_archive = (
            data.staging_entry.clone(),
            StagingEntryUpdate::DiscardUpdate {
                original_data: data.staging_entry.data.clone(),
            }
            .into(),
        );

        let new_staging_entry = data.staging_entry.new_versioned(
            StagingEntryDomainVersionUpdate::ProcessedUpdate {
                data: StagingEntryData::V2(StagingEntryDataV2::Processed),
            },
            None,
        );

        let unique_constraint = data.metadata_schema.schema_data.get_unique_constraint();
        let (unique_field_name, unique_field_value) = extract_unique_field(
            unique_constraint.extract_unique_value(&new_staging_entry),
            unique_constraint.get_unique_field_delimiter(),
        )?;

        let hashed_unique_field = hash_string(&unique_field_value);

        let encrypted_unique_field = encryptor
            .encrypt_string(unique_field_value)
            .await
            .change_context(ProcessingError::InternalError(
                "Failed to encrypt unique field".to_string(),
            ))?;

        let new_search_duplicate = SearchDuplicateCreationData {
            staging_entry_id: new_staging_entry.staging_entry_id.clone(),
            account_id: new_staging_entry.account_id.clone(),
            merchant_id: new_staging_entry.merchant_id.clone(),
            transformation_id: new_staging_entry.transformation_id.clone(),
            field_path: unique_field_name,
            encrypted_field_value: encrypted_unique_field,
            hashed_field_value: hashed_unique_field,
            primary_staging_entry_id: new_staging_entry.id.clone(),
            staging_entry_version: new_staging_entry.version,
        };

        let new_search_duplicate = DomainSearchDuplicate::new(new_search_duplicate);

        // This logic can be abstracted out into the target type if more variants are introduced
        let targets =
            match self.target_accounts.clone() {
                OneToManySingleSingleTargetType::Percentage{ targets } => targets
                    .into_iter()
                    .map(|(target, percentage)| -> ProcessingResult<(OneToManySingleSingleTarget, MinorUnit)> {
                        let split_amount = new_staging_entry
                            .amount
                            .to_major_unit_as_f64(new_staging_entry.currency)
                            .change_context(ProcessingError::InternalError(
                                "Amount could not be converted to major unit".to_string(),
                            ))?;
                        let split_amount = FloatMajorUnit::new(split_amount.0 * percentage.value / 100.0)
                            .to_minor_unit_as_i64(new_staging_entry.currency)
                            .change_context(ProcessingError::InternalError(
                                "Amount could not be converted to minor unit".to_string(),
                            ))?;
                        Ok((target, split_amount))
                    })
                    .collect::<ProcessingResult<Vec<(OneToManySingleSingleTarget, MinorUnit)>>>()?,
                OneToManySingleSingleTargetType::Fixed{ targets } => {
                    let fixed_sum = targets
                        .iter()
                        .filter_map(|(_, fixed_split_value)| match fixed_split_value {
                            FixedSplitValue::Amount { value } => Some(value.get_amount_as_i64()),
                            FixedSplitValue::Remaining => None,
                        })
                        .sum::<i64>();
                    targets
                        .into_iter()
                        .map(|(target, fixed_split_value)| {
                            let split_amount = match fixed_split_value {
                                FixedSplitValue::Amount { value } => value,
                                FixedSplitValue::Remaining => {
                                    let remaining_amount =
                                        new_staging_entry.amount.get_amount_as_i64() - fixed_sum;
                                    MinorUnit::new(remaining_amount)
                                }
                            };
                            (target, split_amount)
                        })
                        .collect::<Vec<_>>()
                }
            };

        let expected_sum: i64 = targets
            .iter()
            .map(|(_, amount)| amount.get_amount_as_i64())
            .sum();

        if new_staging_entry.amount.get_amount_as_i64() != expected_sum {
            let splits_debug = targets
                .iter()
                .map(|(target, amount)| format!("({:?}, {})", target, amount.get_amount_as_i64()))
                .collect::<Vec<_>>()
                .join(", ");
            return Err(ProcessingError::InternalError(
                format!(
                    "The split amounts should add up to the staging entry amount. splits=[{}], sum={}, expected={}",
                    splits_debug,
                    expected_sum,
                    new_staging_entry.amount.get_amount_as_i64()
                ),
            )
            .into());
        }

        let expected_entries_with_expectations =
            future::try_join_all(
                targets.iter().map(|(target_account, split_amount)| async {
                    let source_value = target_account
                        .search_identifier
                        .extract_source_value(&new_staging_entry)?;

                    let encrypted_source_value = encryptor
                        .encrypt_string(source_value.clone())
                        .await
                        .change_context(ProcessingError::InternalError(
                            "Failed to encrypt metadata".to_string(),
                        ))?;

                    let search_expectation = SearchExpectationCreation {
                        rule_id: data.rule_id.clone(),
                        target_account_id: target_account.account_id.clone(),
                        source_value: encrypted_source_value,
                        hashed_source_value: hash_string(&source_value),
                        target_key: target_account.search_identifier.get_target_field(),
                        merchant_id: new_staging_entry.merchant_id.clone(),
                    };

                    let expected_metadata = target_account
                        .match_rules
                        .extract_expected_entry_metadata_from_source(&new_staging_entry)?;

                    let encrypted_expected_metadata = encryptor
                        .encrypt_json_value(expected_metadata.into())
                        .await
                        .change_context(ProcessingError::InternalError(
                            "Failed to encrypt metadata".to_string(),
                        ))?;

                    Ok::<
                        (TransactionCreationEntry, SearchExpectationCreation),
                        Report<ProcessingError>,
                    >((
                        TransactionCreationEntry::Expected(ExpectedTransactionCreationEntry {
                            account_id: target_account.account_id.clone(),
                            entry_type: new_staging_entry.entry_type.opposite(),
                            amount: *split_amount,
                            currency: new_staging_entry.currency,
                            effective_at: new_staging_entry.effective_at,
                            merchant_id: new_staging_entry.merchant_id.clone(),
                            metadata: encrypted_expected_metadata,
                            data: EntryData::V2(EntryDataV2::Expected),
                            order_id: new_staging_entry.order_id.clone(),
                        }),
                        search_expectation,
                    ))
                }),
            )
            .await?;

        let (expected_entries, search_expectations): (Vec<_>, Vec<_>) =
            expected_entries_with_expectations.into_iter().unzip();

        let direct_entry = TransactionCreationEntry::Direct(DirectTransactionCreationEntry {
            account_id: new_staging_entry.account_id.clone(),
            entry_type: new_staging_entry.entry_type,
            amount: new_staging_entry.amount,
            currency: new_staging_entry.currency,
            effective_at: new_staging_entry.effective_at,
            merchant_id: new_staging_entry.merchant_id.clone(),
            metadata: new_staging_entry.metadata.clone(),
            data: EntryData::V2(EntryDataV2::Pending),
            primary_staging_entry_id: Some(new_staging_entry.id.clone()),
            transformation_id: Some(new_staging_entry.transformation_id.clone()),
            order_id: new_staging_entry.order_id.clone(),
            exposed_metadata: new_staging_entry.exposed_metadata.clone(),
        });

        let entries = expected_entries
            .into_iter()
            .chain(std::iter::once(direct_entry))
            .collect();

        let entries_with_search_keys =
            couple_entries_with_searchable_metadata(state, entries, &data.profile_id).await?;

        let transaction_creation_data = TransactionCreationData {
            entries: entries_with_search_keys,
            profile_id: data.profile_id,
            rule_id: data.rule_id,
            currency: new_staging_entry.currency,
            data: TransactionData::V3(TransactionDataV3::Expected),
            search_group: None,
            search_expectations,
            modified_by: None,
        };

        let transaction_data = transaction_creation_data.get_new_transaction_data();

        let new_search_entry_keys = transaction_data
            .search_entry_keys
            .into_iter()
            .map(DomainSearchEntryKey::construct_new)
            .collect();

        let processed_staging_entry = new_staging_entry.construct_new();

        let new_search_duplicate = new_search_duplicate.construct_new();

        let new_entries = transaction_data
            .entries
            .into_iter()
            .map(|entry| entry.construct_new())
            .collect();

        let new_search_expectations = transaction_data
            .search_expectations
            .into_iter()
            .map(|exp| exp.construct_new())
            .collect();

        let new_transaction = transaction_data.transaction.construct_new();
        Ok(ReconContextTransaction::OneToManySingleSingleV1 {
            engine: self,
            context: Box::new(OneToOneTransactionContext {
                staging_entry_archive,
                processed_staging_entry,
                new_search_duplicate,
                new_entries,
                new_search_expectations,
                new_search_entry_keys,
                new_transaction,
            }),
        })
    }

    async fn create_confirmation_context(
        self,
        state: &AppState,
        data: Self::PrepTypeConfirmation,
    ) -> ProcessingResult<ReconContextConfirmation> {
        let encryptor = state
            .encryption_client
            .get_encryptor_for_merchant(data.merchant_id);

        let old_transaction = state
            .store
            .find_by_id_and_profile(
                &data.search_expectation.primary_transaction_id,
                &data.staging_entry.profile_id,
            )
            .await
            .change_context(ProcessingError::DatabaseError)?;

        let old_domain_transaction: DomainTransaction = old_transaction
            .convert_back(&encryptor)
            .await
            .change_context(ProcessingError::InternalError(
                "Failed to convert diesel transaction to domain transaction".to_string(),
            ))?;

        let diesel_old_entries = state
            .store
            .list_entries_by_ids(old_domain_transaction.entry_id.clone())
            .await
            .change_context(ProcessingError::DatabaseError)?;

        let old_entries = future::try_join_all(
            diesel_old_entries
                .into_iter()
                .map(|entry| entry.convert_back(&encryptor)),
        )
        .await
        .change_context(ProcessingError::InternalError(
            "Failed to convert diesel entries to domain entries".to_string(),
        ))?;

        let (domain_expected_entry, remaining_entries) = data
            .search_expectation
            .find_expected_entry(old_entries)
            .ok_or(ProcessingError::InternalError(
                "Failed to find expected entry in transaction".to_string(),
            ))?;

        let status_determination_input = OneToOneStatusDeterminationInput {
            staging_entry: data.staging_entry.clone(),
            expected_entry: domain_expected_entry,
            old_transaction_status: old_domain_transaction.status.clone(),
            remaining_entries: remaining_entries.clone(),
        };

        let OneToOneStatusDeterminationOutput {
            transaction_data,
            current_entry_data,
        } = self
            .determine_status(state, status_determination_input)
            .await?;

        let staging_entry_archive = (
            data.staging_entry.clone(),
            StagingEntryUpdate::DiscardUpdate {
                original_data: data.staging_entry.data.clone(),
            }
            .into(),
        );

        let new_staging_entry = data.staging_entry.new_versioned(
            StagingEntryDomainVersionUpdate::ProcessedUpdate {
                data: StagingEntryData::V2(StagingEntryDataV2::Processed),
            },
            None,
        );

        let unique_constraint = data.metadata_schema.schema_data.get_unique_constraint();
        let (unique_field_name, unique_field_value) = extract_unique_field(
            unique_constraint.extract_unique_value(&new_staging_entry),
            unique_constraint.get_unique_field_delimiter(),
        )?;

        let hashed_unique_field = hash_string(&unique_field_value);

        let encrypted_unique_field = encryptor
            .encrypt_string(unique_field_value)
            .await
            .change_context(ProcessingError::InternalError(
                "Failed to encrypt unique field".to_string(),
            ))?;

        let new_search_duplicate = SearchDuplicateCreationData {
            staging_entry_id: new_staging_entry.staging_entry_id.clone(),
            account_id: new_staging_entry.account_id.clone(),
            merchant_id: new_staging_entry.merchant_id.clone(),
            transformation_id: new_staging_entry.transformation_id.clone(),
            field_path: unique_field_name,
            encrypted_field_value: encrypted_unique_field,
            hashed_field_value: hashed_unique_field,
            primary_staging_entry_id: new_staging_entry.id.clone(),
            staging_entry_version: new_staging_entry.version,
        };

        let new_search_duplicate = DomainSearchDuplicate::new(new_search_duplicate);

        let current_entry = TransactionCreationEntry::Direct(DirectTransactionCreationEntry {
            account_id: new_staging_entry.account_id.clone(),
            entry_type: new_staging_entry.entry_type,
            amount: new_staging_entry.amount,
            currency: new_staging_entry.currency,
            effective_at: new_staging_entry.effective_at,
            merchant_id: new_staging_entry.merchant_id.clone(),
            metadata: new_staging_entry.metadata.clone(),
            data: current_entry_data,
            primary_staging_entry_id: Some(new_staging_entry.id.clone()),
            transformation_id: Some(new_staging_entry.transformation_id.clone()),
            order_id: new_staging_entry.order_id.clone(),
            exposed_metadata: new_staging_entry.exposed_metadata.clone(),
        });

        let remaining_results = future::try_join_all(remaining_entries.into_iter().map(|entry| {
            entry.get_creation_data_using_db(
                state,
                &old_domain_transaction,
                (&transaction_data).into(),
            )
        }))
        .await?;

        let (new_remaining_entries, remaining_expectations): (Vec<_>, Vec<_>) =
            remaining_results.into_iter().unzip();
        let remaining_expectations: Vec<_> = remaining_expectations.into_iter().flatten().collect();

        let entries = new_remaining_entries
            .into_iter()
            .chain(std::iter::once(current_entry))
            .collect();

        let entries_with_search_keys = couple_entries_with_searchable_metadata(
            state,
            entries,
            &old_domain_transaction.profile_id,
        )
        .await?;

        let transaction_update_data = TransactionUpdateData {
            entries: entries_with_search_keys,
            profile_id: old_domain_transaction.profile_id.clone(),
            rule_id: old_domain_transaction.rule_id.clone(),
            currency: old_domain_transaction.currency,
            data: transaction_data,
            transaction_id: old_domain_transaction.transaction_id.clone(),
            old_transaction_version: old_domain_transaction.version,
            search_group: None,
            search_expectations: remaining_expectations,
        };

        let transaction_data = transaction_update_data.get_new_transaction_data();

        let new_search_entry_keys = transaction_data
            .search_entry_keys
            .into_iter()
            .map(DomainSearchEntryKey::construct_new)
            .collect();

        let processed_staging_entry = new_staging_entry.construct_new();

        let new_search_duplicate = new_search_duplicate.construct_new();

        let new_entries = transaction_data
            .entries
            .into_iter()
            .map(|entry| entry.construct_new())
            .collect();

        let new_search_expectations = transaction_data
            .search_expectations
            .into_iter()
            .map(|exp| exp.construct_new())
            .collect();

        let new_transaction = transaction_data.transaction.construct_new();

        Ok(ReconContextConfirmation::OneToManySingleSingleV1 {
            engine: self,
            context: Box::new(OneToOneConfirmationContext {
                staging_entry_archive,
                processed_staging_entry,
                new_search_duplicate,
                transaction_archive: old_domain_transaction,
                new_transaction,
                new_entries,
                new_search_expectations,
                new_search_entry_keys,
            }),
        })
    }

    async fn process_transaction_context(
        self,
        state: &AppState,
        context: Self::ContextTypeTransaction,
    ) -> ProcessingResult<Transaction> {
        let conn =
            state
                .store
                .get_connection()
                .await
                .change_context(ProcessingError::InternalError(
                    "Failed to get database connection".to_string(),
                ))?;

        conn.transaction_async(|conn| async move {
            StagingEntry::update_by_id(
                &conn,
                &context.staging_entry_archive.0.id,
                context.staging_entry_archive.1,
            )
            .await
            .map_err(|e| {
                logging::error!("Failed to archive staging entry: {:?}", e);
                *e.current_context()
            })?;

            // Insert the processed staging entry
            context
                .processed_staging_entry
                .insert(&conn)
                .await
                .map_err(|e| {
                    logging::error!("Failed to insert processed staging entry: {:?}", e);
                    *e.current_context()
                })?;

            context
                .new_search_duplicate
                .insert(&conn)
                .await
                .map_err(|e| {
                    logging::error!(
                        "Failed to insert search duplicate for processed entry: {:?}",
                        e
                    );
                    *e.current_context()
                })?;

            EntryNew::batch_insert_chunked(&conn, context.new_entries)
                .await
                .map_err(|e| {
                    logging::error!("Failed to batch insert entries: {:?}", e);
                    *e.current_context()
                })?;

            SearchEntryKeyNew::batch_insert(context.new_search_entry_keys, &conn)
                .await
                .map_err(|e| {
                    logging::error!("Failed to batch insert searchable metadata: {:?}", e);
                    *e.current_context()
                })?;

            for expectation in context.new_search_expectations {
                expectation.insert(&conn).await.map_err(|e| {
                    logging::error!("Failed to insert search expectation: {:?}", e);
                    *e.current_context()
                })?;
            }

            let db_transaction = context.new_transaction.insert(&conn).await.map_err(|e| {
                logging::error!("Failed to insert transaction: {:?}", e);
                *e.current_context()
            })?;

            Result::<_, DatabaseError>::Ok(db_transaction)
        })
        .await
        .change_context(ProcessingError::DatabaseError)
    }

    async fn process_confirmation_context(
        self,
        state: &AppState,
        context: Self::ContextTypeConfirmation,
    ) -> ProcessingResult<Transaction> {
        let conn =
            state
                .store
                .get_connection()
                .await
                .change_context(ProcessingError::InternalError(
                    "Failed to get database connection".to_string(),
                ))?;

        conn.transaction_async(|conn| async move {
            state
                .store
                .discard_transaction_with_entries_and_expectations(
                    &conn,
                    context.transaction_archive.convert(),
                )
                .await
                .map_err(|e| {
                    logging::error!("Failed to discard old transaction: {:?}", e);
                    *e.current_context()
                })?;

            StagingEntry::update_by_id(
                &conn,
                &context.staging_entry_archive.0.id,
                context.staging_entry_archive.1,
            )
            .await
            .map_err(|e| {
                logging::error!("Failed to archive staging entry: {:?}", e);
                *e.current_context()
            })?;

            // Insert the processed staging entry
            context
                .processed_staging_entry
                .insert(&conn)
                .await
                .map_err(|e| {
                    logging::error!("Failed to insert processed staging entry: {:?}", e);
                    *e.current_context()
                })?;

            context
                .new_search_duplicate
                .insert(&conn)
                .await
                .map_err(|e| {
                    logging::error!(
                        "Failed to insert search duplicate for processed entry: {:?}",
                        e
                    );
                    *e.current_context()
                })?;

            EntryNew::batch_insert_chunked(&conn, context.new_entries)
                .await
                .map_err(|e| {
                    logging::error!("Failed to batch insert entries: {:?}", e);
                    *e.current_context()
                })?;

            SearchEntryKeyNew::batch_insert(context.new_search_entry_keys, &conn)
                .await
                .map_err(|e| {
                    logging::error!("Failed to batch insert searchable metadata: {:?}", e);
                    *e.current_context()
                })?;

            for expectation in context.new_search_expectations {
                expectation.insert(&conn).await.map_err(|e| {
                    logging::error!("Failed to insert search expectation: {:?}", e);
                    *e.current_context()
                })?;
            }

            let db_transaction = context.new_transaction.insert(&conn).await.map_err(|e| {
                logging::error!("Failed to insert transaction: {:?}", e);
                *e.current_context()
            })?;

            Result::<_, DatabaseError>::Ok(db_transaction)
        })
        .await
        .change_context(ProcessingError::DatabaseError)
    }

    fn get_search_criteria(
        &self,
        data: &impl ExtractFromEntryField,
        rule_id: &str,
    ) -> ProcessingResult<Vec<SearchCriteria>> {
        let target_account = match self.target_accounts.clone() {
            OneToManySingleSingleTargetType::Percentage { targets } => targets
                .into_iter()
                .find(|(account, _)| account.account_id == data.extract_account_id())
                .map(|acc| acc.0),
            OneToManySingleSingleTargetType::Fixed { targets } => targets
                .into_iter()
                .find(|(account, _)| account.account_id == data.extract_account_id())
                .map(|acc| acc.0),
        }
        .ok_or(ProcessingError::InternalError(
            "Failed to find target account in rule".to_string(),
        ))?;

        let source_value = target_account
            .search_identifier
            .extract_target_value(data)?;
        let hashed_source_value = hash_string(&source_value);

        Ok(vec![SearchCriteria {
            rule_id: rule_id.to_string(),
            target_key: target_account.search_identifier.get_target_field(),
            hashed_source_value: hashed_source_value.expose(),
        }])
    }

    fn evaluate_trigger(&self, data: &impl ExtractFromEntryField) -> bool {
        self.source_account.trigger.validate(data)
    }

    async fn determine_status(
        &self,
        _state: &AppState,
        data: Self::StatusDeterminationInput,
    ) -> ProcessingResult<Self::StatusDeterminationOutput> {
        let target_account = match &self.target_accounts {
            OneToManySingleSingleTargetType::Percentage { targets } => {
                targets.iter().find_map(|(account, _)| {
                    (account.account_id == data.staging_entry.account_id).then(|| account.clone())
                })
            }
            OneToManySingleSingleTargetType::Fixed { targets } => {
                targets.iter().find_map(|(account, _)| {
                    (account.account_id == data.staging_entry.account_id).then(|| account.clone())
                })
            }
        }
        .ok_or(ProcessingError::InternalError(
            "Failed to find target account in rule".to_string(),
        ))?;

        let (current_transaction_data, current_entry_data) =
            determine_status_based_on_current_entry(
                target_account.match_rules,
                &data.staging_entry,
                &data.expected_entry,
            )?;

        let (source_entry, remaining_target_entries) = separate_source_and_target_entries(
            data.remaining_entries,
            &self.source_account.account_id,
        )?;

        let source_sum = source_entry.amount.get_amount_as_i64();

        let target_sum = remaining_target_entries
            .iter()
            .filter(|entry| entry.status != EntryStatus::Expected)
            .fold(
                if data.staging_entry.entry_type == source_entry.entry_type {
                    -data.staging_entry.amount.get_amount_as_i64()
                } else {
                    data.staging_entry.amount.get_amount_as_i64()
                },
                |acc, entry| {
                    if entry.entry_type == source_entry.entry_type {
                        acc - entry.amount.get_amount_as_i64()
                    } else {
                        acc + entry.amount.get_amount_as_i64()
                    }
                },
            );

        let transaction_data = match (
            data.old_transaction_status,
            current_transaction_data.clone(),
        ) {
            (DomainTransactionStatus::CurrencyMismatch, _)
            | (_, TransactionData::V3(TransactionDataV3::CurrencyMismatch)) => {
                TransactionData::V3(TransactionDataV3::CurrencyMismatch)
            }
            (DomainTransactionStatus::OverAmount(_), _)
            | (DomainTransactionStatus::UnderAmount(TransactionAmountMismatchData::Mismatch), _)
            | (_, TransactionData::V3(TransactionDataV3::OverAmount(_)))
            | (_, TransactionData::V3(TransactionDataV3::UnderAmount(_))) => {
                if target_sum > source_sum {
                    TransactionData::V3(TransactionDataV3::OverAmount(AmountMismatchData::V1(
                        AmountMismatchDataV1::Mismatch,
                    )))
                } else if target_sum < source_sum {
                    TransactionData::V3(TransactionDataV3::UnderAmount(AmountMismatchData::V1(
                        AmountMismatchDataV1::Mismatch,
                    )))
                } else {
                    TransactionData::V3(TransactionDataV3::SplitMismatch)
                }
            }
            (_, TransactionData::V3(TransactionDataV3::DataMismatch(fields))) => {
                TransactionData::V3(TransactionDataV3::DataMismatch(fields))
            }
            (DomainTransactionStatus::DataMismatch, _) => {
                TransactionData::V3(TransactionDataV3::DataMismatch(MetadataMismatchDataV1 {
                    mismatched_fields: vec![],
                }))
            }
            _ => {
                if target_sum > source_sum {
                    TransactionData::V3(TransactionDataV3::OverAmount(AmountMismatchData::V1(
                        AmountMismatchDataV1::Mismatch,
                    )))
                } else if target_sum < source_sum {
                    TransactionData::V3(TransactionDataV3::UnderAmount(AmountMismatchData::V1(
                        AmountMismatchDataV1::Expected,
                    )))
                } else {
                    TransactionData::V3(TransactionDataV3::Matched(MatchedData::V1(
                        MatchedDataV1::Auto,
                    )))
                }
            }
        };

        let current_entry_data = match (transaction_data.clone(), current_entry_data.clone()) {
            (TransactionData::V3(TransactionDataV3::Matched(_)), _) => {
                EntryData::V2(EntryDataV2::Matched)
            }
            (_, EntryData::V2(EntryDataV2::Matched)) => EntryData::V2(EntryDataV2::Pending),
            _ => current_entry_data,
        };

        Ok(OneToOneStatusDeterminationOutput {
            transaction_data,
            current_entry_data,
        })
    }
}

fn separate_source_and_target_entries(
    mut old_entries: Vec<DomainEntry>,
    source_account_id: &str,
) -> ProcessingResult<(DomainEntry, Vec<DomainEntry>)> {
    let (source_entry, remaining_entries) = old_entries
        .iter()
        .position(|entry| entry.account_id == source_account_id)
        .map(|i| (old_entries.remove(i), old_entries))
        .ok_or(ProcessingError::InternalError(
            "Failed to find source entry in transaction".to_string(),
        ))?;
    Ok((source_entry, remaining_entries))
}

fn determine_status_based_on_current_entry(
    match_rules: MatchRules,
    staging_entry: &DomainStagingEntry,
    expected_entry: &DomainEntry,
) -> ProcessingResult<(TransactionData, EntryData)> {
    if let MatchRuleValidation::Failed(mismatch_data) =
        match_rules.validate_currency(staging_entry, expected_entry)
    {
        return Ok((
            TransactionData::V3(TransactionDataV3::CurrencyMismatch),
            EntryData::V2(EntryDataV2::Mismatched(EntryMismatchedData::V1(
                EntryMismatchedDataV1::CurrencyMismatch(CurrencyMismatchData::V1(
                    CurrencyMismatchDataV1 {
                        expected_currency: mismatch_data.expected,
                        actual_currency: mismatch_data.actual,
                    },
                )),
            ))),
        ));
    }

    if let MatchRuleValidation::Failed(mismatch_data) =
        match_rules.validate_balance_direction(staging_entry, expected_entry)
    {
        return Ok((
            TransactionData::V3(TransactionDataV3::UnderAmount(AmountMismatchData::V1(
                AmountMismatchDataV1::Mismatch,
            ))),
            EntryData::V2(EntryDataV2::Mismatched(EntryMismatchedData::V1(
                EntryMismatchedDataV1::BalanceDirectionMismatch(BalanceDirectionMismatchData::V1(
                    BalanceDirectionMismatchDataV1 {
                        expected_direction: mismatch_data.expected,
                        actual_direction: mismatch_data.actual,
                    },
                )),
            ))),
        ));
    }

    if let MatchRuleValidation::Failed(amount_mismatch) =
        match_rules.validate_overamount(staging_entry, expected_entry)
    {
        return Ok((
            TransactionData::V3(TransactionDataV3::OverAmount(AmountMismatchData::V1(
                AmountMismatchDataV1::Mismatch,
            ))),
            EntryData::V2(EntryDataV2::Mismatched(EntryMismatchedData::V1(
                EntryMismatchedDataV1::AmountMismatch(EntryAmountMismatchData::V1(
                    EntryAmountMismatchDataV1 {
                        expected_amount: amount_mismatch.expected,
                        actual_amount: amount_mismatch.actual,
                        currency: amount_mismatch.currency,
                        variance: amount_mismatch.actual - amount_mismatch.expected,
                    },
                )),
            ))),
        ));
    }

    if let MatchRuleValidation::Failed(amount_mismatch) =
        match_rules.validate_underamount(staging_entry, expected_entry)
    {
        return Ok((
            TransactionData::V3(TransactionDataV3::UnderAmount(AmountMismatchData::V1(
                AmountMismatchDataV1::Mismatch,
            ))),
            EntryData::V2(EntryDataV2::Mismatched(EntryMismatchedData::V1(
                EntryMismatchedDataV1::AmountMismatch(EntryAmountMismatchData::V1(
                    EntryAmountMismatchDataV1 {
                        expected_amount: amount_mismatch.expected,
                        actual_amount: amount_mismatch.actual,
                        currency: amount_mismatch.currency,
                        variance: amount_mismatch.expected - amount_mismatch.actual,
                    },
                )),
            ))),
        ));
    }

    if let MatchRuleValidation::Failed(mismatch_data) =
        match_rules.validate_metadata(staging_entry, expected_entry)?
    {
        return Ok((
            TransactionData::V3(TransactionDataV3::DataMismatch(MetadataMismatchDataV1 {
                mismatched_fields: mismatch_data.mismatched_fields.clone(),
            })),
            EntryData::V2(EntryDataV2::Mismatched(EntryMismatchedData::V1(
                EntryMismatchedDataV1::MetadataMismatch(MetadataMismatchData::V1(
                    MetadataMismatchDataV1 {
                        mismatched_fields: mismatch_data.mismatched_fields,
                    },
                )),
            ))),
        ));
    }

    Ok((
        TransactionData::V3(TransactionDataV3::Matched(MatchedData::V1(
            MatchedDataV1::Auto,
        ))),
        EntryData::V2(EntryDataV2::Matched),
    ))
}
