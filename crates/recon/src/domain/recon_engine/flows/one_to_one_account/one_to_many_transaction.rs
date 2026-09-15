use std::slice;

use async_bb8_diesel::AsyncConnection;
use common_enums::{BalanceDirection, EntryStatus};
use common_types::{
    entry::{
        CurrencyMismatchData, CurrencyMismatchDataV1, MetadataMismatchData, MetadataMismatchDataV1,
    },
    metadata_schema::UniqueConstraintProvider,
    recon_rule::{
        entry_field::ExtractFromEntryField, strategies::one_to_one_strategy::OneToOneSingleManyV1,
    },
    staging_entry::{StagingEntryData, StagingEntryDataV2},
    transaction::{
        AmountMismatchData, AmountMismatchDataV1, MatchedData, MatchedDataV1, TransactionData,
        TransactionDataV3,
    },
    units::MinorUnit,
};
use common_utils::{date_time, id_type::MerchantId, logging};
use diesel_models::{
    account::Account,
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
use error_stack::{report, ResultExt};
use futures::future;
use masking::ExposeInterface;
use recon_domain_models::{
    behaviour::Conversion,
    entry::{DomainEntry, EntryWithSearchableMetadata},
    search_duplicate::{DomainSearchDuplicate, SearchDuplicateCreationData},
    search_entry_key::DomainSearchEntryKey,
    search_expectation::DomainSearchExpectation,
    staging_entry::StagingEntryDomainVersionUpdate,
    transaction::{SearchExpectationCreation, TransactionCreationData},
    ApplicationEncryptionInterface, EncryptionHandler,
};
use recon_domain_models::{
    behaviour::ReverseConversion,
    staging_entry::DomainStagingEntry,
    transaction::{DomainTransaction, TransactionUpdateData},
};

use crate::{
    db::{
        account::AccountInterface, entry::EntryInterface, search_expectation::SearchCriteria,
        transaction::TransactionInterface,
    },
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
            collect_metadata_mismatch_fields, couple_entries_with_searchable_metadata,
            extract_unique_field, hash_string, BalanceState, EntryClassification,
            EntryDataWithClassification, EntryDataWithClassificationVecExt, MatchOutcome,
            RawEntryData,
        },
        MatchRuleValidation, ProcessingError, ProcessingResult,
    },
    routes::AppState,
};

pub struct OneToManyTransactionPrepType {
    staging_entry: DomainStagingEntry,
    rule_id: String,
    profile_id: String,
    merchant_id: MerchantId,
    metadata_schema: MetadataSchema,
    source_account: Account,
    target_account: Account,
}

pub struct OneToManyConfirmationPrepType {
    merchant_id: MerchantId,
    staging_entry: DomainStagingEntry,
    search_expectation: DomainSearchExpectation,
    metadata_schema: MetadataSchema,
    source_account: Account,
    target_account: Account,
    rule_id: String,
}

pub struct OneToManyTransactionContext {
    staging_entry_archive: (DomainStagingEntry, StagingEntryUpdateInternal),
    processed_staging_entry: StagingEntryNew,
    new_search_duplicate: SearchDuplicateNew,
    new_entries: Vec<EntryNew>,
    new_search_expectations: Vec<SearchExpectationNew>,
    new_search_entry_keys: Vec<SearchEntryKeyNew>,
    new_transaction: TransactionNew,
}

pub struct OneToManyConfirmationContext {
    staging_entry_archive: (DomainStagingEntry, StagingEntryUpdateInternal),
    processed_staging_entry: StagingEntryNew,
    new_search_duplicate: SearchDuplicateNew,
    transaction_archive: DomainTransaction,
    new_transaction: TransactionNew,
    new_entries: Vec<EntryNew>,
    new_search_expectations: Vec<SearchExpectationNew>,
    new_search_entry_keys: Vec<SearchEntryKeyNew>,
}
pub struct OneToManyStatusDeterminationInput {
    source_entry: RawEntryData,
    target_entries: Vec<RawEntryData>,
    source_account: Account,
    target_account: Account,
    merchant_id: MerchantId,
    rule_id: String,
    profile_id: String,
}
pub struct OneToManyStatusDeterminationOutput {
    transaction_data: TransactionData,
    entries: Vec<EntryWithSearchableMetadata>,
    search_expectation: Option<SearchExpectationCreation>,
}

#[async_trait::async_trait]
impl ReconEngine for OneToOneSingleManyV1 {
    type PrepTypeTransaction = OneToManyTransactionPrepType;
    type PrepTypeConfirmation = OneToManyConfirmationPrepType;
    type ContextTypeTransaction = OneToManyTransactionContext;
    type ContextTypeConfirmation = OneToManyConfirmationContext;
    type StatusDeterminationInput = OneToManyStatusDeterminationInput;
    type StatusDeterminationOutput = OneToManyStatusDeterminationOutput;

    async fn prep_transaction(
        self,
        state: &AppState,
        staging_entry: DomainStagingEntry,
        transaction_context: TransactionContext,
    ) -> ProcessingResult<ReconPrepTransaction> {
        let source_account = state
            .store
            .find_account_by_id(&staging_entry.account_id)
            .await
            .change_context(ProcessingError::DatabaseError)?;

        let target_account = state
            .store
            .find_account_by_id(&self.target_account.account_id)
            .await
            .change_context(ProcessingError::DatabaseError)?;

        if staging_entry.currency != source_account.currency {
            return Err(ProcessingError::StagingEntryCurrencyMismatch {
                staging_currency: staging_entry.currency,
                account_currency: source_account.currency,
            }
            .into());
        }
        Ok(ReconPrepTransaction::OneToOneSingleManyV1 {
            engine: self,
            prep: OneToManyTransactionPrepType {
                rule_id: transaction_context.rule_id,
                profile_id: staging_entry.profile_id.clone(),
                merchant_id: staging_entry.merchant_id.clone(),
                staging_entry,
                metadata_schema: transaction_context.metadata_schema,
                source_account,
                target_account,
            },
        })
    }

    async fn prep_confirmation(
        self,
        state: &AppState,
        staging_entry: DomainStagingEntry,
        confirmation_context: ConfirmationContext,
    ) -> ProcessingResult<ReconPrepConfirmation> {
        let source_account = state
            .store
            .find_account_by_id(&self.source_account.account_id)
            .await
            .change_context(ProcessingError::DatabaseError)?;

        let target_account = state
            .store
            .find_account_by_id(&staging_entry.account_id)
            .await
            .change_context(ProcessingError::DatabaseError)?;

        if staging_entry.currency != target_account.currency {
            return Err(ProcessingError::StagingEntryCurrencyMismatch {
                staging_currency: staging_entry.currency,
                account_currency: target_account.currency,
            }
            .into());
        }

        Ok(ReconPrepConfirmation::OneToOneSingleManyV1 {
            engine: self,
            prep: OneToManyConfirmationPrepType {
                merchant_id: staging_entry.merchant_id.clone(),
                staging_entry,
                search_expectation: confirmation_context.domain_search_expectation,
                metadata_schema: confirmation_context.metadata_schema,
                source_account,
                target_account,
                rule_id: confirmation_context.rule_id,
            },
        })
    }

    async fn create_transaction_context(
        self,
        state: &AppState,
        data: OneToManyTransactionPrepType,
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

        let (source_entry, target_entries) = build_image(
            vec![],
            &new_staging_entry,
            &self.source_account.account_id,
            &self.target_account.account_id,
        )?;

        let status_output = self
            .determine_status(
                state,
                OneToManyStatusDeterminationInput {
                    source_entry,
                    target_entries,
                    source_account: data.source_account,
                    target_account: data.target_account,
                    merchant_id: data.merchant_id,
                    rule_id: data.rule_id.clone(),
                    profile_id: data.profile_id.clone(),
                },
            )
            .await?;

        let transaction_creation_data = TransactionCreationData {
            entries: status_output.entries,
            profile_id: data.profile_id,
            rule_id: data.rule_id,
            currency: new_staging_entry.currency,
            data: status_output.transaction_data,
            search_group: None,
            search_expectations: status_output.search_expectation.into_iter().collect(),
            modified_by: None,
        };

        let transaction_data = transaction_creation_data.get_new_transaction_data();

        let processed_staging_entry = new_staging_entry.construct_new();
        let new_search_duplicate = new_search_duplicate.construct_new();

        let new_entries: Vec<EntryNew> = transaction_data
            .entries
            .into_iter()
            .map(|entry| entry.construct_new())
            .collect();

        let new_search_entry_keys = transaction_data
            .search_entry_keys
            .into_iter()
            .map(DomainSearchEntryKey::construct_new)
            .collect();

        let new_search_expectations: Vec<SearchExpectationNew> = transaction_data
            .search_expectations
            .into_iter()
            .map(|exp| exp.construct_new())
            .collect();

        let new_transaction = transaction_data.transaction.construct_new();

        Ok(ReconContextTransaction::OneToOneSingleManyV1 {
            engine: self,
            context: Box::new(OneToManyTransactionContext {
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
        data: OneToManyConfirmationPrepType,
    ) -> ProcessingResult<ReconContextConfirmation> {
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

        let old_transaction = state
            .store
            .find_by_id_and_profile(
                &data.search_expectation.primary_transaction_id,
                &data.source_account.profile_id,
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

        let (source_entry, target_entries) = build_image(
            old_entries,
            &new_staging_entry,
            &self.source_account.account_id,
            &data.target_account.id,
        )?;

        let status_output = self
            .determine_status(
                state,
                OneToManyStatusDeterminationInput {
                    source_entry,
                    target_entries,
                    source_account: data.source_account,
                    target_account: data.target_account,
                    merchant_id: data.merchant_id,
                    rule_id: data.rule_id,
                    profile_id: old_domain_transaction.profile_id.clone(),
                },
            )
            .await?;

        let transaction_archive = old_domain_transaction.clone();

        let transaction_update_data = TransactionUpdateData {
            entries: status_output.entries,
            profile_id: old_domain_transaction.profile_id,
            rule_id: old_domain_transaction.rule_id,
            currency: old_domain_transaction.currency,
            data: status_output.transaction_data,
            transaction_id: old_domain_transaction.transaction_id,
            old_transaction_version: old_domain_transaction.version,
            search_group: None,
            search_expectations: status_output.search_expectation.into_iter().collect(),
        };

        let transaction_data = transaction_update_data.get_new_transaction_data();

        let processed_staging_entry = new_staging_entry.construct_new();
        let new_search_duplicate = new_search_duplicate.construct_new();

        let new_entries: Vec<EntryNew> = transaction_data
            .entries
            .into_iter()
            .map(|entry| entry.construct_new())
            .collect();

        let new_search_entry_keys = transaction_data
            .search_entry_keys
            .into_iter()
            .map(DomainSearchEntryKey::construct_new)
            .collect();

        let new_search_expectations: Vec<SearchExpectationNew> = transaction_data
            .search_expectations
            .into_iter()
            .map(|exp| exp.construct_new())
            .collect();

        let new_transaction = transaction_data.transaction.construct_new();

        Ok(ReconContextConfirmation::OneToOneSingleManyV1 {
            engine: self,
            context: Box::new(OneToManyConfirmationContext {
                staging_entry_archive,
                processed_staging_entry,
                new_search_duplicate,
                transaction_archive,
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
        context: OneToManyConfirmationContext,
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
        let source_value = self.search_identifier.extract_target_value(data)?;
        let hashed_source_value = hash_string(&source_value);

        Ok(vec![SearchCriteria {
            rule_id: rule_id.to_string(),
            target_key: self.search_identifier.get_target_field(),
            hashed_source_value: hashed_source_value.expose().to_owned(),
        }])
    }

    fn evaluate_trigger(&self, data: &impl ExtractFromEntryField) -> bool {
        self.source_account.trigger.validate(data)
    }

    async fn determine_status(
        &self,
        state: &AppState,
        data: OneToManyStatusDeterminationInput,
    ) -> ProcessingResult<OneToManyStatusDeterminationOutput> {
        let encryptor = state
            .encryption_client
            .get_encryptor_for_merchant(data.merchant_id.clone());

        let (classified_entries, match_outcome) = validate_entries(
            self,
            &data.source_entry,
            data.target_entries,
            &data.source_account,
            &data.target_account,
            &encryptor,
        )
        .await?;
        classified_entries.validate_transaction_entry_count()?;

        let transaction_data = TransactionData::V3(match &match_outcome {
            MatchOutcome::CurrencyMismatch => TransactionDataV3::CurrencyMismatch,
            MatchOutcome::NoTarget => TransactionDataV3::Expected,
            MatchOutcome::DataMismatch(balance_state) => match balance_state {
                BalanceState::Balanced | BalanceState::BalancedWithTolerance => {
                    TransactionDataV3::DataMismatch(MetadataMismatchDataV1 {
                        mismatched_fields: collect_metadata_mismatch_fields(
                            classified_entries.iter().map(|e| &e.classification),
                        ),
                    })
                }
                BalanceState::UnderAmount => TransactionDataV3::UnderAmount(
                    AmountMismatchData::V1(AmountMismatchDataV1::Mismatch),
                ),
                BalanceState::OverAmount => TransactionDataV3::OverAmount(AmountMismatchData::V1(
                    AmountMismatchDataV1::Mismatch,
                )),
            },
            MatchOutcome::Good(balance_state) => match balance_state {
                BalanceState::Balanced => {
                    TransactionDataV3::Matched(MatchedData::V1(MatchedDataV1::Auto))
                }
                BalanceState::BalancedWithTolerance => {
                    TransactionDataV3::Matched(MatchedData::V1(MatchedDataV1::WithTolerance))
                }
                BalanceState::UnderAmount => TransactionDataV3::UnderAmount(
                    AmountMismatchData::V1(AmountMismatchDataV1::Expected),
                ),
                BalanceState::OverAmount => TransactionDataV3::OverAmount(AmountMismatchData::V1(
                    AmountMismatchDataV1::Expected,
                )),
            },
        });

        let search_expectation = match match_outcome {
            MatchOutcome::CurrencyMismatch => None,
            MatchOutcome::DataMismatch(BalanceState::UnderAmount | BalanceState::OverAmount)
            | MatchOutcome::Good(BalanceState::Balanced | BalanceState::BalancedWithTolerance)
            | MatchOutcome::DataMismatch(
                BalanceState::Balanced | BalanceState::BalancedWithTolerance,
            )
            | MatchOutcome::NoTarget
            | MatchOutcome::Good(BalanceState::OverAmount | BalanceState::UnderAmount) => {
                let source_value = self
                    .search_identifier
                    .extract_source_value(&data.source_entry)?;
                let encrypted_source_value = encryptor
                    .encrypt_string(source_value.clone())
                    .await
                    .change_context(ProcessingError::InternalError(
                    "Failed to encrypt source value".to_string(),
                ))?;

                Some(SearchExpectationCreation {
                    rule_id: data.rule_id.clone(),
                    target_account_id: data.target_account.id.clone(),
                    source_value: encrypted_source_value,
                    hashed_source_value: hash_string(&source_value),
                    target_key: self.search_identifier.get_target_field(),
                    merchant_id: data.merchant_id.clone(),
                })
            }
        };

        let entries = classified_entries
            .into_iter()
            .map(|data| data.to_transaction_creation_entry(&match_outcome))
            .collect();

        let entries_with_search_keys =
            couple_entries_with_searchable_metadata(state, entries, &data.profile_id).await?;

        Ok(OneToManyStatusDeterminationOutput {
            transaction_data,
            entries: entries_with_search_keys,
            search_expectation,
        })
    }
}

fn build_image(
    existing_entries: Vec<DomainEntry>,
    staging_entry: &DomainStagingEntry,
    source_account_id: &str,
    target_account_id: &str,
) -> ProcessingResult<(RawEntryData, Vec<RawEntryData>)> {
    let staging_raw_entry = RawEntryData::from_staging_entry(staging_entry);

    let (mut source_entries, mut target_entries): (Vec<_>, Vec<_>) = existing_entries
        .into_iter()
        .filter(|entry| entry.status != EntryStatus::Expected)
        .filter(|entry| {
            entry.account_id == source_account_id || entry.account_id == target_account_id
        })
        .map(RawEntryData::from_domain_entry)
        .partition(|entry| entry.account_id == source_account_id);

    if staging_entry.account_id == source_account_id {
        source_entries.push(staging_raw_entry);
    } else if staging_entry.account_id == target_account_id {
        target_entries.push(staging_raw_entry);
    } else {
        return Err(ProcessingError::InternalError(format!(
            "Staging entry account {} does not match source {} or target {}",
            staging_entry.account_id, source_account_id, target_account_id
        ))
        .into());
    }

    if source_entries.len() > 1 {
        return Err(ProcessingError::InternalError(format!(
            "More than one entry found for source. Source entries: {}, Target entries: {}",
            source_entries.len(),
            target_entries.len()
        ))
        .into());
    }

    let source_entry = source_entries.pop().ok_or_else(|| {
        ProcessingError::InternalError(
            "SingleMany requires exactly one source entry, found none".to_string(),
        )
    })?;

    Ok((source_entry, target_entries))
}

async fn validate_entries(
    engine: &OneToOneSingleManyV1,
    source_entry: &RawEntryData,
    target_entries: Vec<RawEntryData>,
    source_account: &Account,
    target_account: &Account,
    encryptor: &EncryptionHandler<impl ApplicationEncryptionInterface>,
) -> ProcessingResult<(Vec<EntryDataWithClassification>, MatchOutcome)> {
    let classified_source = if source_entry.currency == source_account.currency {
        EntryDataWithClassification {
            entry: source_entry.clone(),
            classification: EntryClassification::Good,
        }
    } else {
        return Err(report!(ProcessingError::InternalError(format!(
            "Currency mismatch in source entry: expected currency {}, found {}",
            source_account.currency, source_entry.currency
        ))));
    };

    let mut has_data_mismatches = false;
    let mut has_currency_mismatches = false;
    let has_targets = !target_entries.is_empty();

    let classified_targets: Vec<_> = target_entries
        .into_iter()
        .map(|entry| {
            let classification = if entry.currency != target_account.currency
                || entry.currency != source_entry.currency
            {
                has_currency_mismatches = true;
                EntryClassification::CurrencyMismatch(CurrencyMismatchData::V1(
                    CurrencyMismatchDataV1 {
                        expected_currency: target_account.currency,
                        actual_currency: entry.currency,
                    },
                ))
            } else {
                match engine
                    .match_rules
                    .validate_metadata_with_source(source_entry, &entry)?
                {
                    MatchRuleValidation::Passed => EntryClassification::Good,
                    MatchRuleValidation::Failed(mismatch_data) => {
                        has_data_mismatches = true;
                        EntryClassification::MetadataMismatch(MetadataMismatchData::V1(
                            MetadataMismatchDataV1 {
                                mismatched_fields: mismatch_data.mismatched_fields,
                            },
                        ))
                    }
                }
            };
            Ok(EntryDataWithClassification {
                entry,
                classification,
            })
        })
        .collect::<ProcessingResult<Vec<_>>>()?;

    let match_outcome = match (has_currency_mismatches, has_targets, has_data_mismatches) {
        (true, _, _) => MatchOutcome::CurrencyMismatch,
        (false, false, _) => MatchOutcome::NoTarget,
        (false, true, true) => {
            let balance_state = determine_balance(&classified_source, &classified_targets);
            MatchOutcome::DataMismatch(balance_state)
        }
        (false, true, false) => {
            let balance_state = determine_balance(&classified_source, &classified_targets);
            MatchOutcome::Good(balance_state)
        }
    };

    let actual_entries: Vec<_> = std::iter::once(classified_source)
        .chain(classified_targets)
        .collect();

    let expected_entry = match match_outcome {
        MatchOutcome::Good(BalanceState::Balanced)
        | MatchOutcome::Good(BalanceState::BalancedWithTolerance)
        | MatchOutcome::CurrencyMismatch
        | MatchOutcome::DataMismatch(
            BalanceState::Balanced | BalanceState::BalancedWithTolerance,
        ) => None,
        MatchOutcome::DataMismatch(BalanceState::OverAmount | BalanceState::UnderAmount)
        | MatchOutcome::NoTarget
        | MatchOutcome::Good(BalanceState::OverAmount | BalanceState::UnderAmount) => {
            let (expected_direction, expected_amount) = expected_entry_balance(&actual_entries);

            let expected_metadata = engine
                .match_rules
                .extract_expected_entry_metadata_from_source(source_entry)?;

            let encrypted_metadata = encryptor
                .encrypt_json_value(expected_metadata.into())
                .await
                .change_context(ProcessingError::InternalError(
                    "Failed to encrypt metadata".to_string(),
                ))?;

            Some(EntryDataWithClassification {
                entry: RawEntryData {
                    account_id: target_account.id.clone(),
                    amount: expected_amount,
                    entry_type: expected_direction,
                    currency: target_account.currency,
                    effective_at: date_time::now(),
                    order_id: source_entry.order_id.clone(),
                    metadata: encrypted_metadata,
                    merchant_id: source_entry.merchant_id.clone(),
                    primary_staging_entry_id: None,
                    transformation_id: None,
                    exposed_metadata: serde_json::json!({}),
                },
                classification: EntryClassification::Expected,
            })
        }
    };

    Ok((
        actual_entries.into_iter().chain(expected_entry).collect(),
        match_outcome,
    ))
}

fn expected_entry_balance(
    entries: &[EntryDataWithClassification],
) -> (BalanceDirection, MinorUnit) {
    let credit_sum: MinorUnit = entries
        .iter()
        .filter(|data| data.entry.entry_type() == BalanceDirection::Credit)
        .map(|data| data.entry.amount())
        .sum();

    let debit_sum: MinorUnit = entries
        .iter()
        .filter(|data| data.entry.entry_type() == BalanceDirection::Debit)
        .map(|data| data.entry.amount())
        .sum();

    if credit_sum >= debit_sum {
        (BalanceDirection::Debit, credit_sum - debit_sum)
    } else {
        (BalanceDirection::Credit, debit_sum - credit_sum)
    }
}

fn determine_balance(
    source_entry: &EntryDataWithClassification,
    target_entries: &[EntryDataWithClassification],
) -> BalanceState {
    let (source_balance_direction, source_amount) =
        expected_entry_balance(slice::from_ref(source_entry));
    let (target_balance_direction, target_amount) = expected_entry_balance(target_entries);

    if source_balance_direction == target_balance_direction {
        BalanceState::UnderAmount
    } else if source_amount == target_amount {
        BalanceState::Balanced
    } else if source_amount > target_amount {
        BalanceState::UnderAmount
    } else {
        BalanceState::OverAmount
    }
}
