use async_bb8_diesel::AsyncConnection;
use common_enums::EntryStatus;
use common_types::{
    entry::{
        BalanceDirectionMismatchData, BalanceDirectionMismatchDataV1, CurrencyMismatchData,
        CurrencyMismatchDataV1, EntryAmountMismatchData, EntryAmountMismatchDataV1,
        MetadataMismatchData, MetadataMismatchDataV1,
    },
    metadata_schema::UniqueConstraintProvider,
    recon_rule::{
        entry_field::ExtractFromEntryField,
        strategies::one_to_one_strategy::OneToOneSingleSingleV1, tolerance_config::ToleranceConfig,
    },
    staging_entry::{StagingEntryData, StagingEntryDataV2},
    transaction::{
        AmountMismatchData, AmountMismatchDataV1, MatchedData, MatchedDataV1, TransactionData,
        TransactionDataV3,
    },
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
use error_stack::ResultExt;
use futures::future;
use recon_domain_models::{
    behaviour::{Conversion, ReverseConversion},
    entry::{DomainEntry, EntryWithSearchableMetadata},
    search_entry_key::DomainSearchEntryKey,
    search_expectation::DomainSearchExpectation,
    transaction::{DomainTransaction, TransactionUpdateData},
    ApplicationEncryptionInterface, EncryptionHandler,
};
use recon_domain_models::{
    search_duplicate::{DomainSearchDuplicate, SearchDuplicateCreationData},
    staging_entry::{DomainStagingEntry, StagingEntryDomainVersionUpdate},
    transaction::{SearchExpectationCreation, TransactionCreationData},
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
        AmountMismatch, MatchRuleValidation, ProcessingError, ProcessingResult,
    },
    routes::AppState,
};
use masking::ExposeInterface;

pub struct OneToOneTransactionContext {
    staging_entry_archive: (DomainStagingEntry, StagingEntryUpdateInternal),
    processed_staging_entry: StagingEntryNew,
    new_search_duplicate: SearchDuplicateNew,
    new_entries: Vec<EntryNew>,
    new_search_expectations: Vec<SearchExpectationNew>,
    new_search_entry_keys: Vec<SearchEntryKeyNew>,
    new_transaction: TransactionNew,
}

#[derive(Debug)]
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
    staging_entry: DomainStagingEntry,
    rule_id: String,
    profile_id: String,
    merchant_id: MerchantId,
    metadata_schema: MetadataSchema,
}

pub struct OneToOneConfirmationPrepType {
    merchant_id: MerchantId,
    profile_id: String,
    staging_entry: DomainStagingEntry,
    search_expectation: DomainSearchExpectation,
    metadata_schema: MetadataSchema,
    rule_id: String,
}

pub struct OneToOneStatusDeterminationInput {
    source_entry: RawEntryData,
    target_entry: Option<RawEntryData>,
    target_account_id: String,
    rule_id: String,
    merchant_id: MerchantId,
    profile_id: String,
}

pub struct OneToOneStatusDeterminationOutput {
    transaction_data: TransactionData,
    entries: Vec<EntryWithSearchableMetadata>,
    search_expectation: Option<SearchExpectationCreation>,
}

#[async_trait::async_trait]
impl ReconEngine for OneToOneSingleSingleV1 {
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

        Ok(ReconPrepTransaction::OneToOneSingleSingleV1 {
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
        let target_account = state
            .store
            .find_account_by_id(&self.target_account.account_id)
            .await
            .change_context(ProcessingError::DatabaseError)?;

        if staging_entry.currency != target_account.currency {
            return Err(ProcessingError::StagingEntryCurrencyMismatch {
                staging_currency: staging_entry.currency,
                account_currency: target_account.currency,
            }
            .into());
        }

        Ok(ReconPrepConfirmation::OneToOneSingleSingleV1 {
            engine: self,
            prep: OneToOneConfirmationPrepType {
                merchant_id: staging_entry.merchant_id.clone(),
                profile_id: staging_entry.profile_id.clone(),
                staging_entry,
                search_expectation: confirmation_context.domain_search_expectation,
                metadata_schema: confirmation_context.metadata_schema,
                rule_id: confirmation_context.rule_id,
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

        let (source_entry, target_entry) = build_single_image(
            vec![],
            &new_staging_entry,
            &self.source_account.account_id,
            &self.target_account.account_id,
        )?;

        let OneToOneStatusDeterminationOutput {
            transaction_data,
            entries,
            search_expectation,
        } = self
            .determine_status(
                state,
                OneToOneStatusDeterminationInput {
                    source_entry,
                    target_entry,
                    target_account_id: self.target_account.account_id.clone(),
                    rule_id: data.rule_id.clone(),
                    merchant_id: data.merchant_id.clone(),
                    profile_id: data.profile_id.clone(),
                },
            )
            .await?;

        let transaction_creation_data = TransactionCreationData {
            entries,
            profile_id: data.profile_id,
            rule_id: data.rule_id,
            currency: new_staging_entry.currency,
            data: transaction_data,
            search_group: None,
            search_expectations: search_expectation.into_iter().collect(),
            modified_by: None,
        };

        let transaction_data = transaction_creation_data.get_new_transaction_data();

        let processed_staging_entry = new_staging_entry.construct_new();

        let new_search_duplicate = new_search_duplicate.construct_new();

        let new_entries = transaction_data
            .entries
            .into_iter()
            .map(|entry| entry.construct_new())
            .collect();

        let new_search_entry_keys = transaction_data
            .search_entry_keys
            .into_iter()
            .map(DomainSearchEntryKey::construct_new)
            .collect();

        let new_search_expectations = transaction_data
            .search_expectations
            .into_iter()
            .map(|exp| exp.construct_new())
            .collect();

        let new_transaction = transaction_data.transaction.construct_new();

        Ok(ReconContextTransaction::OneToOneSingleSingleV1 {
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
                &data.profile_id,
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

        let (source_entry, target_entry) = build_single_image(
            old_entries,
            &new_staging_entry,
            &self.source_account.account_id,
            &self.target_account.account_id,
        )?;

        let status_determination_input = OneToOneStatusDeterminationInput {
            source_entry,
            target_entry,
            target_account_id: self.target_account.account_id.clone(),
            rule_id: data.rule_id.clone(),
            merchant_id: data.merchant_id.clone(),
            profile_id: data.profile_id.clone(),
        };

        let OneToOneStatusDeterminationOutput {
            transaction_data,
            entries,
            search_expectation,
        } = self
            .determine_status(state, status_determination_input)
            .await?;

        let transaction_update_data = TransactionUpdateData {
            entries,
            profile_id: old_domain_transaction.profile_id.clone(),
            rule_id: old_domain_transaction.rule_id.clone(),
            currency: old_domain_transaction.currency,
            data: transaction_data,
            transaction_id: old_domain_transaction.transaction_id.clone(),
            old_transaction_version: old_domain_transaction.version,
            search_group: None,
            search_expectations: search_expectation.into_iter().collect(),
        };

        let transaction_data = transaction_update_data.get_new_transaction_data();

        let processed_staging_entry = new_staging_entry.construct_new();

        let new_search_duplicate = new_search_duplicate.construct_new();

        let new_entries = transaction_data
            .entries
            .into_iter()
            .map(|entry| entry.construct_new())
            .collect();

        let new_search_entry_keys = transaction_data
            .search_entry_keys
            .into_iter()
            .map(DomainSearchEntryKey::construct_new)
            .collect();

        let new_search_expectations = transaction_data
            .search_expectations
            .into_iter()
            .map(|exp| exp.construct_new())
            .collect();

        let new_transaction = transaction_data.transaction.construct_new();

        Ok(ReconContextConfirmation::OneToOneSingleSingleV1 {
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
        context: OneToOneConfirmationContext,
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
            hashed_source_value: hashed_source_value.expose(),
        }])
    }

    fn evaluate_trigger(&self, data: &impl ExtractFromEntryField) -> bool {
        self.source_account.trigger.validate(data)
    }

    async fn determine_status(
        &self,
        state: &AppState,
        data: OneToOneStatusDeterminationInput,
    ) -> ProcessingResult<OneToOneStatusDeterminationOutput> {
        let encryptor = state
            .encryption_client
            .get_encryptor_for_merchant(data.merchant_id.clone());

        let ClassifyEntriesOutput {
            source_classification,
            target_classification,
            tolerance_entry,
            match_outcome,
            search_expectation,
        } = classify_entries(
            self,
            &data.rule_id,
            data.source_entry,
            data.target_entry,
            &data.target_account_id,
            &data.merchant_id,
            &encryptor,
            self.target_account.tolerance_config.as_ref(),
        )
        .await?;

        let transaction_data = TransactionData::V3(match &match_outcome {
            MatchOutcome::CurrencyMismatch => TransactionDataV3::CurrencyMismatch,
            MatchOutcome::NoTarget => TransactionDataV3::Expected,
            MatchOutcome::DataMismatch(balance_state) => match balance_state {
                BalanceState::Balanced | BalanceState::BalancedWithTolerance => {
                    TransactionDataV3::DataMismatch(MetadataMismatchDataV1 {
                        mismatched_fields: collect_metadata_mismatch_fields(
                            [
                                Some(&source_classification.classification),
                                Some(&target_classification.classification),
                                tolerance_entry.as_ref().map(|e| &e.classification),
                            ]
                            .into_iter()
                            .flatten(),
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
                    AmountMismatchData::V1(AmountMismatchDataV1::Mismatch),
                ),
                BalanceState::OverAmount => TransactionDataV3::OverAmount(AmountMismatchData::V1(
                    AmountMismatchDataV1::Mismatch,
                )),
            },
        });

        let classified_entries: Vec<EntryDataWithClassification> =
            std::iter::once(source_classification)
                .chain(std::iter::once(target_classification))
                .chain(tolerance_entry)
                .collect();

        classified_entries.validate_transaction_entry_count()?;

        let creation_entries = classified_entries
            .into_iter()
            .map(|data| data.to_transaction_creation_entry(&match_outcome))
            .collect();

        let entries =
            couple_entries_with_searchable_metadata(state, creation_entries, &data.profile_id)
                .await?;

        Ok(OneToOneStatusDeterminationOutput {
            transaction_data,
            entries,
            search_expectation,
        })
    }
}

fn build_single_image(
    existing_entries: Vec<DomainEntry>,
    staging_entry: &DomainStagingEntry,
    source_account_id: &str,
    target_account_id: &str,
) -> ProcessingResult<(RawEntryData, Option<RawEntryData>)> {
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

    if source_entries.len() > 1 || target_entries.len() > 1 {
        return Err(ProcessingError::InternalError(format!(
            "More than one entry found for source or target. Source entries: {}, Target entries: {}",
            source_entries.len(),
            target_entries.len()
        ))
        .into());
    }

    let source_entry = source_entries.pop().ok_or_else(|| {
        ProcessingError::InternalError(
            "SingleSingle requires exactly one source entry, found none".to_string(),
        )
    })?;

    let target_entry = target_entries.pop();

    Ok((source_entry, target_entry))
}

struct ClassifyEntriesOutput {
    source_classification: EntryDataWithClassification,
    target_classification: EntryDataWithClassification,
    tolerance_entry: Option<EntryDataWithClassification>,
    match_outcome: MatchOutcome,
    /// Routing search expectation produced when a placeholder Expected target
    /// entry is created (only on `NoTarget`). `None` for every other branch.
    search_expectation: Option<SearchExpectationCreation>,
}

#[allow(clippy::too_many_arguments)]
async fn classify_entries(
    engine: &OneToOneSingleSingleV1,
    rule_id: &str,
    source_entry: RawEntryData,
    target_entry: Option<RawEntryData>,
    target_account_id: &str,
    merchant_id: &MerchantId,
    encryptor: &EncryptionHandler<impl ApplicationEncryptionInterface>,
    tolerance_config: Option<&ToleranceConfig>,
) -> ProcessingResult<ClassifyEntriesOutput> {
    let Some(target_entry) = target_entry else {
        let expected_metadata = engine
            .match_rules
            .extract_expected_entry_metadata_from_source(&source_entry)?;

        let encrypted_expected_metadata = encryptor
            .encrypt_json_value(expected_metadata.into())
            .await
            .change_context(ProcessingError::InternalError(
                "Failed to encrypt metadata".to_string(),
            ))?;

        let source_value = engine
            .search_identifier
            .extract_source_value(&source_entry)?;

        let encrypted_source_value = encryptor
            .encrypt_string(source_value.clone())
            .await
            .change_context(ProcessingError::InternalError(
                "Failed to encrypt source value".to_string(),
            ))?;

        let search_expectation = SearchExpectationCreation {
            rule_id: rule_id.to_string(),
            target_account_id: target_account_id.to_string(),
            source_value: encrypted_source_value,
            hashed_source_value: hash_string(&source_value),
            target_key: engine.search_identifier.get_target_field(),
            merchant_id: merchant_id.clone(),
        };

        let expected_entry = RawEntryData {
            account_id: target_account_id.to_string(),
            amount: source_entry.amount,
            entry_type: source_entry.entry_type.opposite(),
            currency: source_entry.currency,
            effective_at: source_entry.effective_at,
            order_id: source_entry.order_id.clone(),
            metadata: encrypted_expected_metadata,
            merchant_id: merchant_id.clone(),
            primary_staging_entry_id: None,
            transformation_id: None,
            exposed_metadata: serde_json::json!({}),
        };

        return Ok(ClassifyEntriesOutput {
            source_classification: EntryDataWithClassification {
                entry: source_entry,
                classification: EntryClassification::Good,
            },
            target_classification: EntryDataWithClassification {
                entry: expected_entry,
                classification: EntryClassification::Expected,
            },
            tolerance_entry: None,
            match_outcome: MatchOutcome::NoTarget,
            search_expectation: Some(search_expectation),
        });
    };

    if let MatchRuleValidation::Failed(mismatch_data) = engine
        .match_rules
        .validate_currency(&source_entry, &target_entry)
    {
        let target_account_classification = EntryClassification::CurrencyMismatch(
            CurrencyMismatchData::V1(CurrencyMismatchDataV1 {
                expected_currency: mismatch_data.expected,
                actual_currency: mismatch_data.actual,
            }),
        );

        return Ok(ClassifyEntriesOutput {
            source_classification: EntryDataWithClassification {
                entry: source_entry,
                classification: EntryClassification::Good,
            },
            target_classification: EntryDataWithClassification {
                entry: target_entry,
                classification: target_account_classification,
            },
            tolerance_entry: None,
            match_outcome: MatchOutcome::CurrencyMismatch,
            search_expectation: None,
        });
    }

    if let MatchRuleValidation::Failed(mismatch_data) = engine
        .match_rules
        .validate_balance_direction_with_source_and_target(&source_entry, &target_entry)
    {
        let target_account_classification = EntryClassification::BalanceDirectionMismatch(
            BalanceDirectionMismatchData::V1(BalanceDirectionMismatchDataV1 {
                expected_direction: mismatch_data.expected,
                actual_direction: mismatch_data.actual,
            }),
        );

        return Ok(ClassifyEntriesOutput {
            source_classification: EntryDataWithClassification {
                entry: source_entry,
                classification: EntryClassification::Good,
            },
            target_classification: EntryDataWithClassification {
                entry: target_entry,
                classification: target_account_classification,
            },
            tolerance_entry: None,
            match_outcome: MatchOutcome::Good(BalanceState::UnderAmount),
            search_expectation: None,
        });
    }

    if let MatchRuleValidation::Failed(amount_mismatch) = engine
        .match_rules
        .validate_overamount_with_source_and_target_and_tolerance(
            &source_entry,
            &target_entry,
            tolerance_config,
        )
    {
        match amount_mismatch {
            AmountMismatch::Failed(mismatch_data) => {
                let target_entry_classification = EntryClassification::AmountMismatch(
                    EntryAmountMismatchData::V1(EntryAmountMismatchDataV1 {
                        expected_amount: mismatch_data.expected,
                        actual_amount: mismatch_data.actual,
                        currency: mismatch_data.currency,
                        variance: mismatch_data.actual - mismatch_data.expected,
                    }),
                );

                return Ok(ClassifyEntriesOutput {
                    source_classification: EntryDataWithClassification {
                        entry: source_entry,
                        classification: EntryClassification::Good,
                    },
                    target_classification: EntryDataWithClassification {
                        entry: target_entry,
                        classification: target_entry_classification,
                    },
                    tolerance_entry: None,
                    match_outcome: MatchOutcome::Good(BalanceState::OverAmount),
                    search_expectation: None,
                });
            }
            AmountMismatch::WithinTolerance(mismatch_data) => {
                let tolerance_entry = EntryDataWithClassification {
                    entry: RawEntryData {
                        account_id: mismatch_data.tolerance_account_id,
                        amount: mismatch_data.tolerance_amount,
                        entry_type: mismatch_data.tolerance_balance_direction,
                        currency: mismatch_data.currency,
                        effective_at: target_entry.effective_at,
                        merchant_id: merchant_id.clone(),
                        metadata: encryptor
                            .encrypt_json_value(serde_json::json!({}))
                            .await
                            .change_context(ProcessingError::InternalError(
                                "Failed to encrypt tolerance entry metadata".to_string(),
                            ))?,
                        order_id: source_entry.order_id.clone(),
                        primary_staging_entry_id: None,
                        transformation_id: None,
                        exposed_metadata: serde_json::json!({}),
                    },
                    classification: EntryClassification::Good,
                };

                if let MatchRuleValidation::Failed(mismatch_data) = engine
                    .match_rules
                    .validate_metadata_with_source(&source_entry, &target_entry)?
                {
                    let target_entry_classification = EntryClassification::MetadataMismatch(
                        MetadataMismatchData::V1(MetadataMismatchDataV1 {
                            mismatched_fields: mismatch_data.mismatched_fields,
                        }),
                    );

                    return Ok(ClassifyEntriesOutput {
                        source_classification: EntryDataWithClassification {
                            entry: source_entry,
                            classification: EntryClassification::Good,
                        },
                        target_classification: EntryDataWithClassification {
                            entry: target_entry,
                            classification: target_entry_classification,
                        },
                        tolerance_entry: Some(tolerance_entry),
                        match_outcome: MatchOutcome::DataMismatch(
                            BalanceState::BalancedWithTolerance,
                        ),
                        search_expectation: None,
                    });
                }

                return Ok(ClassifyEntriesOutput {
                    source_classification: EntryDataWithClassification {
                        entry: source_entry,
                        classification: EntryClassification::Good,
                    },
                    target_classification: EntryDataWithClassification {
                        entry: target_entry,
                        classification: EntryClassification::Good,
                    },
                    tolerance_entry: Some(tolerance_entry),
                    match_outcome: MatchOutcome::Good(BalanceState::BalancedWithTolerance),
                    search_expectation: None,
                });
            }
        }
    }

    if let MatchRuleValidation::Failed(amount_mismatch) = engine
        .match_rules
        .validate_underamount_with_source_and_target_and_tolerance(
            &source_entry,
            &target_entry,
            tolerance_config,
        )
    {
        match amount_mismatch {
            AmountMismatch::Failed(mismatch_data) => {
                let target_entry_classification = EntryClassification::AmountMismatch(
                    EntryAmountMismatchData::V1(EntryAmountMismatchDataV1 {
                        expected_amount: mismatch_data.expected,
                        actual_amount: mismatch_data.actual,
                        currency: mismatch_data.currency,
                        variance: mismatch_data.expected - mismatch_data.actual,
                    }),
                );

                return Ok(ClassifyEntriesOutput {
                    source_classification: EntryDataWithClassification {
                        entry: source_entry,
                        classification: EntryClassification::Good,
                    },
                    target_classification: EntryDataWithClassification {
                        entry: target_entry,
                        classification: target_entry_classification,
                    },
                    tolerance_entry: None,
                    match_outcome: MatchOutcome::Good(BalanceState::UnderAmount),
                    search_expectation: None,
                });
            }
            AmountMismatch::WithinTolerance(mismatch_data) => {
                let tolerance_entry = EntryDataWithClassification {
                    entry: RawEntryData {
                        account_id: mismatch_data.tolerance_account_id,
                        amount: mismatch_data.tolerance_amount,
                        entry_type: mismatch_data.tolerance_balance_direction,
                        currency: mismatch_data.currency,
                        effective_at: target_entry.effective_at,
                        merchant_id: merchant_id.clone(),
                        metadata: encryptor
                            .encrypt_json_value(serde_json::json!({}))
                            .await
                            .change_context(ProcessingError::InternalError(
                                "Failed to encrypt tolerance entry metadata".to_string(),
                            ))?,
                        order_id: source_entry.order_id.clone(),
                        primary_staging_entry_id: None,
                        transformation_id: None,
                        exposed_metadata: serde_json::json!({}),
                    },
                    classification: EntryClassification::Good,
                };

                if let MatchRuleValidation::Failed(mismatch_data) = engine
                    .match_rules
                    .validate_metadata_with_source(&source_entry, &target_entry)?
                {
                    let target_entry_classification = EntryClassification::MetadataMismatch(
                        MetadataMismatchData::V1(MetadataMismatchDataV1 {
                            mismatched_fields: mismatch_data.mismatched_fields,
                        }),
                    );

                    return Ok(ClassifyEntriesOutput {
                        source_classification: EntryDataWithClassification {
                            entry: source_entry,
                            classification: EntryClassification::Good,
                        },
                        target_classification: EntryDataWithClassification {
                            entry: target_entry,
                            classification: target_entry_classification,
                        },
                        tolerance_entry: Some(tolerance_entry),
                        match_outcome: MatchOutcome::DataMismatch(
                            BalanceState::BalancedWithTolerance,
                        ),
                        search_expectation: None,
                    });
                }

                return Ok(ClassifyEntriesOutput {
                    source_classification: EntryDataWithClassification {
                        entry: source_entry,
                        classification: EntryClassification::Good,
                    },
                    target_classification: EntryDataWithClassification {
                        entry: target_entry,
                        classification: EntryClassification::Good,
                    },
                    tolerance_entry: Some(tolerance_entry),
                    match_outcome: MatchOutcome::Good(BalanceState::BalancedWithTolerance),
                    search_expectation: None,
                });
            }
        };
    }

    if let MatchRuleValidation::Failed(mismatch_data) = engine
        .match_rules
        .validate_metadata_with_source(&source_entry, &target_entry)?
    {
        let target_entry_classification = EntryClassification::MetadataMismatch(
            MetadataMismatchData::V1(MetadataMismatchDataV1 {
                mismatched_fields: mismatch_data.mismatched_fields,
            }),
        );

        return Ok(ClassifyEntriesOutput {
            source_classification: EntryDataWithClassification {
                entry: source_entry,
                classification: EntryClassification::Good,
            },
            target_classification: EntryDataWithClassification {
                entry: target_entry,
                classification: target_entry_classification,
            },
            tolerance_entry: None,
            match_outcome: MatchOutcome::DataMismatch(BalanceState::Balanced),
            search_expectation: None,
        });
    }

    Ok(ClassifyEntriesOutput {
        source_classification: EntryDataWithClassification {
            entry: source_entry,
            classification: EntryClassification::Good,
        },
        target_classification: EntryDataWithClassification {
            entry: target_entry,
            classification: EntryClassification::Good,
        },
        tolerance_entry: None,
        match_outcome: MatchOutcome::Good(BalanceState::Balanced),
        search_expectation: None,
    })
}
