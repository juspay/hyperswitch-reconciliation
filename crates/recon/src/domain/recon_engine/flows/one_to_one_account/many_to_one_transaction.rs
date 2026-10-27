use async_bb8_diesel::AsyncConnection;
use common_enums::{BalanceDirection, EntryStatus, StagingEntryProcessingMode};
use common_types::{
    entry::{
        CurrencyMismatchData, CurrencyMismatchDataV1, MetadataMismatchData, MetadataMismatchDataV1,
    },
    metadata_schema::UniqueConstraintProvider,
    recon_rule::{
        entry_field::ExtractFromEntryField,
        strategies::one_to_one_strategy::{OneToOneManySingleV1, SourceAccumulationMode},
    },
    staging_entry::{StagingEntryData, StagingEntryDataV2},
    transaction::{
        status::DomainTransactionStatus, AmountMismatchData, AmountMismatchDataV1, MatchedData,
        MatchedDataV1, TransactionData, TransactionDataV3,
    },
    units::MinorUnit,
};
use common_utils::{id_type::MerchantId, logging};
use diesel_models::{
    account::Account,
    entry::{Entry, EntryNew, ExpectedEntryUpdateInternal},
    errors::DatabaseError,
    metadata_schema::MetadataSchema,
    search_duplicate::SearchDuplicateNew,
    search_entry_key::SearchEntryKeyNew,
    search_expectation::SearchExpectationNew,
    search_group::SearchGroupNew,
    staging_entry::{
        StagingEntry, StagingEntryNew, StagingEntryUpdate, StagingEntryUpdateInternal,
    },
    transaction::{ExpectedSourceAppendUpdateInternal, Transaction, TransactionNew},
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
    search_group::DomainSearchGroup,
    staging_entry::{DomainStagingEntry, StagingEntryDomainVersionUpdate},
    transaction::{
        DomainTransaction, SearchExpectationCreation, SearchGroupCreation, TransactionCreationData,
        TransactionCreationEntry, TransactionUpdateData,
    },
    ApplicationEncryptionInterface, EncryptionHandler,
};
use storage_impl::{
    account::AccountInterface, entry::EntryInterface, search_expectation::SearchCriteria,
    search_group::SearchGroupInterface, transaction::TransactionInterface,
};

use crate::{
    domain::metadata_schema::implementations::unique_constraint::UniqueConstraintExt,
    domain::recon_engine::{
        flows::{
            ReconContextConfirmation, ReconContextTransaction, ReconEngine,
            ReconEngineIncrementalTransactionInterface, ReconEngineSearchGroupInterface,
            ReconPrepConfirmation, ReconPrepTransaction,
        },
        implementations::{
            grouping_field::GroupingFieldExt, match_rules::MatchRulesExt,
            search_identifier::SearchIdentifierExt, trigger::TriggerExt,
        },
        rule_orchestration::{ConfirmationContext, TransactionContext},
        utils::{
            collect_metadata_mismatch_fields, couple_entries_with_searchable_metadata,
            extract_common, extract_common_result, extract_unique_field, hash_string, BalanceState,
            EntryClassification, EntryDataWithClassification, EntryDataWithClassificationVecExt,
            MatchOutcome, RawEntryData,
        },
        MatchRuleValidation, ProcessingError, ProcessingResult,
    },
    routes::AppState,
};

pub struct ManyToOneTransactionPrepType {
    staging_entry: DomainStagingEntry,
    rule_id: String,
    profile_id: String,
    merchant_id: MerchantId,
    metadata_schema: MetadataSchema,
    target_account: Account,
    source_account: Account,
}

pub struct ManyToOneConfirmationPrepType {
    staging_entry: DomainStagingEntry,
    search_expectation: DomainSearchExpectation,
    merchant_id: MerchantId,
    metadata_schema: MetadataSchema,
    target_account: Account,
    source_account: Account,
}

pub enum ManyToOneTransactionContext {
    NewSearchGroup(Box<ManyToOneTransactionContextWithNewSearchGroup>),
    ExistingSearchGroup(Box<ManyToOneTransactionContextWithExistingSearchGroup>),
    ExpectedSourceAppend(Box<ManyToOneExpectedSourceAppendContext>),
}

pub struct ManyToOneTransactionContextWithNewSearchGroup {
    staging_entry_archive: (DomainStagingEntry, StagingEntryUpdateInternal),
    processed_staging_entry: StagingEntryNew,
    new_search_duplicate: SearchDuplicateNew,
    new_entries: Vec<EntryNew>,
    new_search_expectations: Vec<SearchExpectationNew>,
    new_search_entry_keys: Vec<SearchEntryKeyNew>,
    new_transaction: TransactionNew,
    new_search_group: SearchGroupNew,
}

pub struct ManyToOneTransactionContextWithExistingSearchGroup {
    staging_entry_archive: (DomainStagingEntry, StagingEntryUpdateInternal),
    processed_staging_entry: StagingEntryNew,
    new_search_duplicate: SearchDuplicateNew,
    transaction_archive: DomainTransaction,
    new_transaction: TransactionNew,
    new_entries: Vec<EntryNew>,
    new_search_expectations: Vec<SearchExpectationNew>,
    new_search_entry_keys: Vec<SearchEntryKeyNew>,
    new_search_group: Option<SearchGroupNew>,
}

/// Source-only append prepared while a ManySingle transaction has no target entry.
/// Existing source entries and routing artifacts remain live on the same transaction version.
pub struct ManyToOneExpectedSourceAppendContext {
    staging_entry_archive: (DomainStagingEntry, StagingEntryUpdateInternal),
    transaction_update: (DomainTransaction, ExpectedSourceAppendUpdateInternal),
    processed_staging_entry: StagingEntryNew,
    new_search_duplicate: SearchDuplicateNew,
    new_source_entry: EntryNew,
    new_search_entry_keys: Vec<SearchEntryKeyNew>,
    expected_entry_update: (DomainEntry, ExpectedEntryUpdateInternal),
}

pub struct ManyToOneConfirmationContext {
    staging_entry_archive: (DomainStagingEntry, StagingEntryUpdateInternal),
    processed_staging_entry: StagingEntryNew,
    new_search_duplicate: SearchDuplicateNew,
    transaction_archive: DomainTransaction,
    new_transaction: TransactionNew,
    new_entries: Vec<EntryNew>,
    new_search_expectations: Vec<SearchExpectationNew>,
    new_search_entry_keys: Vec<SearchEntryKeyNew>,
    new_search_group: Option<SearchGroupNew>,
}

pub struct ManyToOneStatusDeterminationInput {
    source_entries: Vec<RawEntryData>,
    target_entry: Option<RawEntryData>,
    source_account: Account,
    rule_id: String,
    merchant_id: MerchantId,
}

pub struct ManyToOneStatusDeterminationOutput {
    transaction_data: TransactionData,
    entries: Vec<TransactionCreationEntry>,
    search_group_creation: Option<SearchGroupCreation>,
    search_expectation: Option<SearchExpectationCreation>,
}

#[async_trait::async_trait]
impl ReconEngine for OneToOneManySingleV1 {
    type PrepTypeTransaction = ManyToOneTransactionPrepType;
    type PrepTypeConfirmation = ManyToOneConfirmationPrepType;
    type ContextTypeTransaction = ManyToOneTransactionContext;
    type ContextTypeConfirmation = ManyToOneConfirmationContext;
    type StatusDeterminationInput = ManyToOneStatusDeterminationInput;
    type StatusDeterminationOutput = ManyToOneStatusDeterminationOutput;

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

        Ok(ReconPrepTransaction::OneToOneManySingleV1 {
            engine: self,
            prep: ManyToOneTransactionPrepType {
                rule_id: transaction_context.rule_id,
                profile_id: staging_entry.profile_id.clone(),
                merchant_id: staging_entry.merchant_id.clone(),
                staging_entry,
                metadata_schema: transaction_context.metadata_schema,
                target_account,
                source_account,
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
            .find_account_by_id(&staging_entry.account_id)
            .await
            .change_context(ProcessingError::DatabaseError)?;

        let source_account = state
            .store
            .find_account_by_id(&self.source_account.account_id)
            .await
            .change_context(ProcessingError::DatabaseError)?;

        if staging_entry.currency != target_account.currency {
            return Err(ProcessingError::StagingEntryCurrencyMismatch {
                staging_currency: staging_entry.currency,
                account_currency: target_account.currency,
            }
            .into());
        }

        Ok(ReconPrepConfirmation::OneToOneManySingleV1 {
            engine: self,
            prep: ManyToOneConfirmationPrepType {
                merchant_id: staging_entry.merchant_id.clone(),
                staging_entry,
                search_expectation: confirmation_context.domain_search_expectation,
                metadata_schema: confirmation_context.metadata_schema,
                target_account,
                source_account,
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

        let grouping_value = self
            .source_account
            .grouping_field
            .extract_value(&data.staging_entry)?;

        let hashed_grouping_value = hash_string(&grouping_value);

        let search_group = state
            .store
            .find_by_rule_id_and_hashed_group_value(&data.rule_id, &hashed_grouping_value.expose())
            .await
            .change_context(ProcessingError::DatabaseError)?;

        let context = match search_group {
            Some(diesel_search_group) => {
                let domain_search_group: DomainSearchGroup = diesel_search_group
                    .convert_back(&encryptor)
                    .await
                    .change_context(ProcessingError::InternalError(
                        "Failed to convert diesel search group to domain search group".to_string(),
                    ))?;

                let transaction = state
                    .store
                    .find_by_id_and_profile(
                        &domain_search_group.primary_transaction_id,
                        &domain_search_group.profile_id,
                    )
                    .await
                    .change_context(ProcessingError::DatabaseError)?;

                let domain_transaction: DomainTransaction = transaction
                    .convert_back(&encryptor)
                    .await
                    .change_context(ProcessingError::InternalError(
                        "Failed to convert diesel transaction to domain transaction".to_string(),
                    ))?;

                if self.check_incremental_transaction_append_eligibility(
                    &domain_transaction,
                    &data.staging_entry,
                ) {
                    self.create_incremental_transaction_context(state, data, domain_transaction)
                        .await
                } else {
                    self.create_transaction_context_with_search_group(
                        state,
                        data,
                        domain_search_group,
                    )
                    .await
                }
            }
            None => {
                self.create_transaction_context_without_search_group(state, data)
                    .await
            }
        }?;

        Ok(ReconContextTransaction::OneToOneManySingleV1 {
            engine: self,
            context: Box::new(context),
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

        let new_search_duplicate = DomainSearchDuplicate::new(SearchDuplicateCreationData {
            staging_entry_id: new_staging_entry.staging_entry_id.clone(),
            account_id: new_staging_entry.account_id.clone(),
            merchant_id: new_staging_entry.merchant_id.clone(),
            transformation_id: new_staging_entry.transformation_id.clone(),
            field_path: unique_field_name,
            encrypted_field_value: encrypted_unique_field,
            hashed_field_value: hashed_unique_field,
            primary_staging_entry_id: new_staging_entry.id.clone(),
            staging_entry_version: new_staging_entry.version,
        });

        let old_transaction = state
            .store
            .find_by_id_and_profile(
                &data.search_expectation.primary_transaction_id,
                &new_staging_entry.profile_id,
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

        // Build image from existing entries + staging entry (target side confirmation)
        let (source_entries, target_entry) = build_image(
            old_entries,
            &new_staging_entry,
            &self.source_account.account_id,
            &data.target_account.id,
        )?;

        // Determine status from image (includes expected entry and search group creation)
        let status_output = self
            .determine_status(
                state,
                ManyToOneStatusDeterminationInput {
                    source_entries,
                    target_entry,
                    source_account: data.source_account.clone(),
                    rule_id: old_domain_transaction.rule_id.clone(),
                    merchant_id: data.merchant_id.clone(),
                },
            )
            .await?;

        let entries_with_searchable_metadata = couple_entries_with_searchable_metadata(
            state,
            status_output.entries,
            &old_domain_transaction.profile_id,
        )
        .await?;

        let transaction_update_data = TransactionUpdateData {
            entries: entries_with_searchable_metadata,
            profile_id: old_domain_transaction.profile_id.clone(),
            rule_id: old_domain_transaction.rule_id.clone(),
            currency: old_domain_transaction.currency,
            data: status_output.transaction_data,
            transaction_id: old_domain_transaction.transaction_id.clone(),
            old_transaction_version: old_domain_transaction.version,
            search_group: status_output.search_group_creation,
            search_expectations: status_output.search_expectation.into_iter().collect(),
        };

        let transaction_data = transaction_update_data.get_new_transaction_data();

        let new_search_entry_keys = transaction_data
            .search_entry_keys
            .into_iter()
            .map(DomainSearchEntryKey::construct_new)
            .collect();

        let processed_staging_entry = new_staging_entry.construct_new();
        let new_search_duplicate = new_search_duplicate.construct_new();

        let new_search_group = transaction_data
            .search_group
            .map(|search_group| search_group.construct_new());

        let new_entries: Vec<EntryNew> = transaction_data
            .entries
            .into_iter()
            .map(|entry| entry.construct_new())
            .collect();

        let new_search_expectations: Vec<SearchExpectationNew> = transaction_data
            .search_expectations
            .into_iter()
            .map(|exp| exp.construct_new())
            .collect();

        let new_transaction = transaction_data.transaction.construct_new();

        Ok(ReconContextConfirmation::OneToOneManySingleV1 {
            engine: self,
            context: Box::new(ManyToOneConfirmationContext {
                staging_entry_archive,
                processed_staging_entry,
                new_search_duplicate,
                transaction_archive: old_domain_transaction,
                new_transaction,
                new_entries,
                new_search_expectations,
                new_search_entry_keys,
                new_search_group,
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

        match context {
            ManyToOneTransactionContext::ExpectedSourceAppend(context) => conn
                .transaction_async(|conn| async move {
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

                    context.new_source_entry.insert(&conn).await.map_err(|e| {
                        logging::error!("Failed to insert new source entry: {:?}", e);
                        *e.current_context()
                    })?;

                    SearchEntryKeyNew::batch_insert(context.new_search_entry_keys, &conn)
                        .await
                        .map_err(|e| {
                            logging::error!("Failed to batch insert searchable metadata: {:?}", e);
                            *e.current_context()
                        })?;

                    Entry::update_expected_for_source_append(
                        &conn,
                        &context.expected_entry_update.0.convert(),
                        context.expected_entry_update.1,
                    )
                    .await
                    .map_err(|e| {
                        logging::error!("Failed to update expected entry: {:?}", e);
                        *e.current_context()
                    })?;

                    let db_transaction = Transaction::append_expected_source_entry(
                        &conn,
                        &context.transaction_update.0.id,
                        context.transaction_update.1,
                    )
                    .await
                    .map_err(|e| {
                        logging::error!("Failed to append expected source entry: {:?}", e);
                        *e.current_context()
                    })?;

                    Result::<_, DatabaseError>::Ok(db_transaction)
                })
                .await
                .change_context(ProcessingError::DatabaseError),
            ManyToOneTransactionContext::ExistingSearchGroup(context) => {
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

                    if let Some(new_search_group) = context.new_search_group {
                        new_search_group.insert(&conn).await.map_err(|e| {
                            logging::error!("Failed to insert search group: {:?}", e);
                            *e.current_context()
                        })?;
                    }

                    let db_transaction =
                        context.new_transaction.insert(&conn).await.map_err(|e| {
                            logging::error!("Failed to insert transaction: {:?}", e);
                            *e.current_context()
                        })?;

                    Result::<_, DatabaseError>::Ok(db_transaction)
                })
                .await
                .change_context(ProcessingError::DatabaseError)
            }
            ManyToOneTransactionContext::NewSearchGroup(context) => {
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

                    context.new_search_group.insert(&conn).await.map_err(|e| {
                        logging::error!("Failed to insert search group: {:?}", e);
                        *e.current_context()
                    })?;

                    let db_transaction =
                        context.new_transaction.insert(&conn).await.map_err(|e| {
                            logging::error!("Failed to insert transaction: {:?}", e);
                            *e.current_context()
                        })?;

                    Result::<_, DatabaseError>::Ok(db_transaction)
                })
                .await
                .change_context(ProcessingError::DatabaseError)
            }
        }
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

            if let Some(new_search_group) = context.new_search_group {
                new_search_group.insert(&conn).await.map_err(|e| {
                    logging::error!("Failed to insert search group: {:?}", e);
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
        data: ManyToOneStatusDeterminationInput,
    ) -> ProcessingResult<Self::StatusDeterminationOutput> {
        let encryptor = state
            .encryption_client
            .get_encryptor_for_merchant(data.merchant_id.clone());

        let first_source_entry = data
            .source_entries
            .iter()
            .min_by_key(|e| e.effective_at)
            .ok_or(ProcessingError::InternalError(
                "No source entries found".to_string(),
            ))?
            .clone();

        // Validate entries — returns classified entries and match outcome.
        let (classified_entries, match_outcome) = validate_entries(self, &data, &encryptor).await?;
        classified_entries.validate_transaction_entry_count()?;

        // Determine transaction_data from match outcome
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

        let search_group_creation = match match_outcome {
            MatchOutcome::CurrencyMismatch => None,
            MatchOutcome::NoTarget
            | MatchOutcome::DataMismatch(
                BalanceState::Balanced
                | BalanceState::OverAmount
                | BalanceState::UnderAmount
                | BalanceState::BalancedWithTolerance,
            )
            | MatchOutcome::Good(
                BalanceState::Balanced
                | BalanceState::BalancedWithTolerance
                | BalanceState::OverAmount
                | BalanceState::UnderAmount,
            ) => {
                let grouping_value = self
                    .source_account
                    .grouping_field
                    .extract_value(&first_source_entry)?;

                let hashed_grouping_value = hash_string(&grouping_value);
                let encrypted_grouping_value = encryptor
                    .encrypt_string(grouping_value)
                    .await
                    .change_context(ProcessingError::InternalError(
                        "Failed to encrypt grouping value".to_string(),
                    ))?;

                Some(SearchGroupCreation {
                    encrypted_grouping_value,
                    hashed_grouping_value,
                    merchant_id: data.merchant_id.clone(),
                })
            }
        };

        // Build the search expectation when an Expected placeholder was
        // produced — for ManyToOne that is only the `NoTarget` outcome.
        let search_expectation = match match_outcome {
            MatchOutcome::NoTarget => {
                let search_identifier_value = extract_common_result(
                    data.source_entries
                        .iter()
                        .map(|e| self.search_identifier.extract_source_value(e)),
                    || ProcessingError::InternalError("Sources are empty".into()).into(),
                    || {
                        ProcessingError::InternalError(
                            "Source value mismatch across sources".into(),
                        )
                        .into()
                    },
                )?;

                let encrypted_source_value = encryptor
                    .encrypt_string(search_identifier_value.clone())
                    .await
                    .change_context(ProcessingError::InternalError(
                        "Failed to encrypt source value".into(),
                    ))?;

                Some(SearchExpectationCreation {
                    rule_id: data.rule_id.clone(),
                    target_account_id: self.target_account.account_id.clone(),
                    source_value: encrypted_source_value,
                    hashed_source_value: hash_string(&search_identifier_value),
                    target_key: self.search_identifier.get_target_field(),
                    merchant_id: data.merchant_id.clone(),
                })
            }
            MatchOutcome::CurrencyMismatch
            | MatchOutcome::DataMismatch(
                BalanceState::Balanced
                | BalanceState::BalancedWithTolerance
                | BalanceState::UnderAmount
                | BalanceState::OverAmount,
            )
            | MatchOutcome::Good(
                BalanceState::Balanced
                | BalanceState::BalancedWithTolerance
                | BalanceState::UnderAmount
                | BalanceState::OverAmount,
            ) => None,
        };

        let entries = classified_entries
            .into_iter()
            .map(|data| data.to_transaction_creation_entry(&match_outcome))
            .collect();

        Ok(ManyToOneStatusDeterminationOutput {
            transaction_data,
            entries,
            search_group_creation,
            search_expectation,
        })
    }
}

impl ReconEngineIncrementalTransactionInterface for OneToOneManySingleV1 {
    fn check_incremental_transaction_append_eligibility(
        &self,
        transaction: &DomainTransaction,
        staging_entry: &DomainStagingEntry,
    ) -> bool {
        if staging_entry.processing_mode != StagingEntryProcessingMode::Transaction
            || staging_entry.account_id != self.source_account.account_id
        {
            return false;
        }

        match transaction.status {
            DomainTransactionStatus::Expected => {}
            DomainTransactionStatus::Posted(_)
            | DomainTransactionStatus::OverAmount(_)
            | DomainTransactionStatus::UnderAmount(_)
            | DomainTransactionStatus::DataMismatch
            | DomainTransactionStatus::CurrencyMismatch
            | DomainTransactionStatus::SplitMismatch
            | DomainTransactionStatus::Archived
            | DomainTransactionStatus::Void
            | DomainTransactionStatus::PartiallyReconciled
            | DomainTransactionStatus::Matched(_) => return false,
        }

        self.source_accumulation_mode == SourceAccumulationMode::IncrementalWhileExpected
    }

    async fn create_incremental_transaction_context(
        &self,
        state: &AppState,
        data: Self::PrepTypeTransaction,
        old_domain_transaction: DomainTransaction,
    ) -> ProcessingResult<Self::ContextTypeTransaction> {
        let encryptor = state
            .encryption_client
            .get_encryptor_for_merchant(data.merchant_id.clone());

        let staging_entry_archive: (DomainStagingEntry, StagingEntryUpdateInternal) = (
            data.staging_entry.clone(),
            StagingEntryUpdate::DiscardUpdate {
                original_data: data.staging_entry.data.clone(),
            }
            .into(),
        );

        let new_staging_entry = data.staging_entry.clone().new_versioned(
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

        let hashed_field_value = hash_string(&unique_field_value);

        let encrypted_field_value = encryptor
            .encrypt_string(unique_field_value)
            .await
            .change_context(ProcessingError::InternalError(
                "Failed to encrypt unique field".to_string(),
            ))?;

        let new_search_duplicate = DomainSearchDuplicate::new(SearchDuplicateCreationData {
            staging_entry_id: new_staging_entry.staging_entry_id.clone(),
            account_id: new_staging_entry.account_id.clone(),
            merchant_id: new_staging_entry.merchant_id.clone(),
            transformation_id: new_staging_entry.transformation_id.clone(),
            field_path: unique_field_name,
            encrypted_field_value,
            hashed_field_value,
            primary_staging_entry_id: new_staging_entry.id.clone(),
            staging_entry_version: new_staging_entry.version,
        });

        let diesel_first_source_entry = state
            .store
            .find_first_pending_source_by_transaction_id_and_version(
                &old_domain_transaction.transaction_id,
                old_domain_transaction.version,
                &self.source_account.account_id,
            )
            .await
            .change_context(ProcessingError::DatabaseError)?;

        let first_source_entry = diesel_first_source_entry
            .convert_back(&encryptor)
            .await
            .change_context(ProcessingError::InternalError(
                "Failed to convert diesel entry to domain entry".to_string(),
            ))?;

        let (source_entries, target_entry) = build_image(
            vec![first_source_entry],
            &new_staging_entry,
            &self.source_account.account_id,
            &data.target_account.id,
        )?;

        let status_output = self
            .determine_status(
                state,
                ManyToOneStatusDeterminationInput {
                    source_entries,
                    target_entry,
                    source_account: data.source_account.clone(),
                    rule_id: old_domain_transaction.rule_id.clone(),
                    merchant_id: data.merchant_id.clone(),
                },
            )
            .await?;

        match status_output.transaction_data {
            TransactionData::V3(data) => match data {
                TransactionDataV3::Expected => {}
                TransactionDataV3::Matched(_)
                | TransactionDataV3::Posted(_)
                | TransactionDataV3::OverAmount(_)
                | TransactionDataV3::UnderAmount(_)
                | TransactionDataV3::DataMismatch(_)
                | TransactionDataV3::CurrencyMismatch
                | TransactionDataV3::SplitMismatch
                | TransactionDataV3::Archived
                | TransactionDataV3::Void(_)
                | TransactionDataV3::PartiallyReconciled(_) => {
                    return Err(ProcessingError::InternalError(
                        "Transaction result is not Expected before source append".to_string(),
                    )
                    .into());
                }
            },
        };

        let new_entry = status_output
            .entries
            .into_iter()
            .find(|entry| {
                entry
                    .primary_staging_entry_id()
                    .is_some_and(|id| id == new_staging_entry.id)
            })
            .ok_or(ProcessingError::InternalError(
                "No new entry found for processed staging entry".to_string(),
            ))?;

        let entry_with_searchable_metadata = couple_entries_with_searchable_metadata(
            state,
            vec![new_entry],
            &old_domain_transaction.profile_id,
        )
        .await?
        .pop()
        .ok_or(ProcessingError::InternalError(
            "Failed to couple new entry with searchable metadata".to_string(),
        ))?;

        let now = common_utils::date_time::now();
        let (domain_entry, domain_search_entry_keys) = entry_with_searchable_metadata
            .get_entry_and_search_entry_keys(
                &old_domain_transaction.id,
                &old_domain_transaction.transaction_id,
                old_domain_transaction.version,
                &old_domain_transaction.profile_id,
                now,
            );

        let new_entry = domain_entry.construct_new();
        let new_search_entry_keys = domain_search_entry_keys
            .into_iter()
            .map(DomainSearchEntryKey::construct_new)
            .collect();

        let processed_staging_entry = new_staging_entry.construct_new();
        let new_search_duplicate = new_search_duplicate.construct_new();

        let old_expected_entries = state
            .store
            .list_expected_entries_by_transaction_id_version_and_account(
                &old_domain_transaction.transaction_id,
                old_domain_transaction.version,
                &self.target_account.account_id,
            )
            .await
            .change_context(ProcessingError::DatabaseError)?;

        let [old_expected_entry] = old_expected_entries.as_slice() else {
            return Err(ProcessingError::InternalError(
                "Expected entry not found or multiple for transaction".to_string(),
            )
            .into());
        };

        let old_expected_entry: DomainEntry = old_expected_entry
            .clone()
            .convert_back(&encryptor)
            .await
            .change_context(ProcessingError::InternalError(
                "Failed to convert diesel entry to domain entry".to_string(),
            ))?;

        let mut actual_credit = old_domain_transaction.credit_amount.get_amount_as_i64();
        let mut actual_debit = old_domain_transaction.debit_amount.get_amount_as_i64();
        let old_expected_amount = old_expected_entry.amount.get_amount_as_i64();

        match old_expected_entry.entry_type {
            BalanceDirection::Credit => {
                actual_credit = actual_credit.checked_sub(old_expected_amount).ok_or(
                    ProcessingError::InternalError(
                        "Transaction aggregate overflow while removing expected entry".to_string(),
                    ),
                )?;
            }
            BalanceDirection::Debit => {
                actual_debit = actual_debit.checked_sub(old_expected_amount).ok_or(
                    ProcessingError::InternalError(
                        "Transaction aggregate overflow while removing expected entry".to_string(),
                    ),
                )?;
            }
        }

        let new_source_amount = new_entry.amount.get_amount_as_i64();
        match new_entry.balance_direction {
            BalanceDirection::Credit => {
                actual_credit = actual_credit.checked_add(new_source_amount).ok_or(
                    ProcessingError::InternalError(
                        "Transaction aggregate overflow while adding source entry".to_string(),
                    ),
                )?;
            }
            BalanceDirection::Debit => {
                actual_debit = actual_debit.checked_add(new_source_amount).ok_or(
                    ProcessingError::InternalError(
                        "Transaction aggregate overflow while adding source entry".to_string(),
                    ),
                )?;
            }
        }

        if actual_credit < 0 || actual_debit < 0 {
            return Err(ProcessingError::InternalError(
                "Expected entry exceeds transaction aggregate".to_string(),
            )
            .into());
        }

        let (balance_direction, expected_amount) = if actual_credit >= actual_debit {
            (BalanceDirection::Debit, actual_credit - actual_debit)
        } else {
            (BalanceDirection::Credit, actual_debit - actual_credit)
        };

        let (new_credit_amount, new_debit_amount) = match balance_direction {
            BalanceDirection::Credit => (
                actual_credit.checked_add(expected_amount),
                Some(actual_debit),
            ),
            BalanceDirection::Debit => (
                Some(actual_credit),
                actual_debit.checked_add(expected_amount),
            ),
        };

        let new_credit_amount = new_credit_amount.ok_or(ProcessingError::InternalError(
            "Transaction credit amount overflow".to_string(),
        ))?;
        let new_debit_amount = new_debit_amount.ok_or(ProcessingError::InternalError(
            "Transaction debit amount overflow".to_string(),
        ))?;

        let net_amount = MinorUnit::new(expected_amount);
        let new_credit_amount = MinorUnit::new(new_credit_amount);
        let new_debit_amount = MinorUnit::new(new_debit_amount);

        let expected_entry_update_data = ExpectedEntryUpdateInternal {
            balance_direction,
            amount: net_amount,
            effective_at: std::cmp::min(old_expected_entry.effective_at, new_entry.effective_at),
            last_modified_at: now,
        };

        let new_entry_ids = old_domain_transaction
            .entry_id
            .iter()
            .cloned()
            .chain(vec![new_entry.id.clone()])
            .collect();

        let transaction_update_data = ExpectedSourceAppendUpdateInternal {
            entry_ids: new_entry_ids,
            credit_amount: new_credit_amount,
            debit_amount: new_debit_amount,
            effective_at: std::cmp::min(
                old_domain_transaction.effective_at,
                new_entry.effective_at,
            ),
            last_modified_at: now,
        };

        Ok(ManyToOneTransactionContext::ExpectedSourceAppend(Box::new(
            ManyToOneExpectedSourceAppendContext {
                staging_entry_archive,
                transaction_update: (old_domain_transaction, transaction_update_data),
                processed_staging_entry,
                new_search_duplicate,
                new_source_entry: new_entry,
                new_search_entry_keys,
                expected_entry_update: (old_expected_entry, expected_entry_update_data),
            },
        )))
    }
}

#[async_trait::async_trait]
impl ReconEngineSearchGroupInterface for OneToOneManySingleV1 {
    async fn create_transaction_context_without_search_group(
        &self,
        state: &AppState,
        data: ManyToOneTransactionPrepType,
    ) -> ProcessingResult<ManyToOneTransactionContext> {
        let encryptor = state
            .encryption_client
            .get_encryptor_for_merchant(data.merchant_id.clone());

        let staging_entry_archive: (DomainStagingEntry, StagingEntryUpdateInternal) = (
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

        let new_search_duplicate = DomainSearchDuplicate::new(SearchDuplicateCreationData {
            staging_entry_id: new_staging_entry.staging_entry_id.clone(),
            account_id: new_staging_entry.account_id.clone(),
            merchant_id: new_staging_entry.merchant_id.clone(),
            transformation_id: new_staging_entry.transformation_id.clone(),
            field_path: unique_field_name,
            encrypted_field_value: encrypted_unique_field,
            hashed_field_value: hashed_unique_field,
            primary_staging_entry_id: new_staging_entry.id.clone(),
            staging_entry_version: new_staging_entry.version,
        });

        let (source_entries, target_entry) = build_image(
            vec![],
            &new_staging_entry,
            &self.source_account.account_id,
            &self.target_account.account_id,
        )?;

        let status_output = self
            .determine_status(
                state,
                ManyToOneStatusDeterminationInput {
                    source_entries,
                    target_entry,
                    rule_id: data.rule_id.clone(),
                    merchant_id: data.merchant_id.clone(),
                    source_account: data.source_account.clone(),
                },
            )
            .await?;

        let entries_with_searchable_metadata =
            couple_entries_with_searchable_metadata(state, status_output.entries, &data.profile_id)
                .await?;

        let transaction_creation_data = TransactionCreationData {
            entries: entries_with_searchable_metadata,
            profile_id: data.profile_id.clone(),
            rule_id: data.rule_id.clone(),
            currency: new_staging_entry.currency,
            data: status_output.transaction_data,
            search_group: status_output.search_group_creation,
            search_expectations: status_output.search_expectation.into_iter().collect(),
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

        let new_search_group = transaction_data
            .search_group
            .ok_or(ProcessingError::InternalError(
                "Failed to create search group".to_string(),
            ))?
            .construct_new();

        let new_entries: Vec<EntryNew> = transaction_data
            .entries
            .into_iter()
            .map(|entry| entry.construct_new())
            .collect();

        let new_search_expectations: Vec<SearchExpectationNew> = transaction_data
            .search_expectations
            .into_iter()
            .map(|exp| exp.construct_new())
            .collect();

        let new_transaction = transaction_data.transaction.construct_new();

        Ok(ManyToOneTransactionContext::NewSearchGroup(Box::new(
            ManyToOneTransactionContextWithNewSearchGroup {
                staging_entry_archive,
                processed_staging_entry,
                new_entries,
                new_search_expectations,
                new_search_entry_keys,
                new_transaction,
                new_search_duplicate,
                new_search_group,
            },
        )))
    }

    async fn create_transaction_context_with_search_group(
        &self,
        state: &AppState,
        data: ManyToOneTransactionPrepType,
        search_group: DomainSearchGroup,
    ) -> ProcessingResult<ManyToOneTransactionContext> {
        let encryptor = state
            .encryption_client
            .get_encryptor_for_merchant(data.merchant_id.clone());

        let staging_entry_archive: (DomainStagingEntry, StagingEntryUpdateInternal) = (
            data.staging_entry.clone(),
            StagingEntryUpdate::DiscardUpdate {
                original_data: data.staging_entry.data.clone(),
            }
            .into(),
        );

        let new_staging_entry = data.staging_entry.clone().new_versioned(
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

        let hashed_field_value = hash_string(&unique_field_value);

        let encrypted_field_value = encryptor
            .encrypt_string(unique_field_value)
            .await
            .change_context(ProcessingError::InternalError(
                "Failed to encrypt unique field".to_string(),
            ))?;

        let new_search_duplicate = DomainSearchDuplicate::new(SearchDuplicateCreationData {
            staging_entry_id: new_staging_entry.staging_entry_id.clone(),
            account_id: new_staging_entry.account_id.clone(),
            merchant_id: new_staging_entry.merchant_id.clone(),
            transformation_id: new_staging_entry.transformation_id.clone(),
            field_path: unique_field_name,
            encrypted_field_value,
            hashed_field_value,
            primary_staging_entry_id: new_staging_entry.id.clone(),
            staging_entry_version: new_staging_entry.version,
        });

        let old_transaction = state
            .store
            .find_by_id_and_profile(&search_group.primary_transaction_id, &data.profile_id)
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

        let old_entries: Vec<DomainEntry> = future::try_join_all(
            diesel_old_entries
                .into_iter()
                .map(|entry| entry.convert_back(&encryptor)),
        )
        .await
        .change_context(ProcessingError::InternalError(
            "Failed to convert diesel entries to domain entries".to_string(),
        ))?;

        let (source_entries, target_entry) = build_image(
            old_entries,
            &new_staging_entry,
            &self.source_account.account_id,
            &data.target_account.id,
        )?;

        let status_output = self
            .determine_status(
                state,
                ManyToOneStatusDeterminationInput {
                    source_entries,
                    target_entry,
                    source_account: data.source_account.clone(),
                    rule_id: old_domain_transaction.rule_id.clone(),
                    merchant_id: data.merchant_id.clone(),
                },
            )
            .await?;

        let entries_with_searchable_metadata =
            couple_entries_with_searchable_metadata(state, status_output.entries, &data.profile_id)
                .await?;

        let transaction_update_data = TransactionUpdateData {
            entries: entries_with_searchable_metadata,
            profile_id: old_domain_transaction.profile_id.clone(),
            rule_id: old_domain_transaction.rule_id.clone(),
            currency: old_domain_transaction.currency,
            data: status_output.transaction_data,
            transaction_id: old_domain_transaction.transaction_id.clone(),
            old_transaction_version: old_domain_transaction.version,
            search_group: status_output.search_group_creation,
            search_expectations: status_output.search_expectation.into_iter().collect(),
        };

        let transaction_data = transaction_update_data.get_new_transaction_data();

        let new_search_entry_keys = transaction_data
            .search_entry_keys
            .into_iter()
            .map(DomainSearchEntryKey::construct_new)
            .collect();

        let processed_staging_entry = new_staging_entry.construct_new();
        let new_search_duplicate = new_search_duplicate.construct_new();

        let new_search_group = transaction_data
            .search_group
            .map(|search_group| search_group.construct_new());

        let new_entries: Vec<EntryNew> = transaction_data
            .entries
            .into_iter()
            .map(|entry| entry.construct_new())
            .collect();

        let new_search_expectations: Vec<SearchExpectationNew> = transaction_data
            .search_expectations
            .into_iter()
            .map(|exp| exp.construct_new())
            .collect();

        let new_transaction = transaction_data.transaction.construct_new();

        Ok(ManyToOneTransactionContext::ExistingSearchGroup(Box::new(
            ManyToOneTransactionContextWithExistingSearchGroup {
                staging_entry_archive,
                processed_staging_entry,
                new_search_duplicate,
                new_entries,
                new_search_expectations,
                new_search_entry_keys,
                new_transaction,
                new_search_group,
                transaction_archive: old_domain_transaction,
            },
        )))
    }
}

/// Validates entries against the first source entry (by effective_at).
///
/// Returns classified entries (including the Expected placeholder when no
/// target is present) and the match outcome. The transaction-level search
/// expectation is constructed by `determine_status` rather than here — same
/// pattern as `search_group_creation`.
async fn validate_entries(
    engine: &OneToOneManySingleV1,
    data: &ManyToOneStatusDeterminationInput,
    encryptor: &EncryptionHandler<impl ApplicationEncryptionInterface>,
) -> ProcessingResult<(Vec<EntryDataWithClassification>, MatchOutcome)> {
    let classified_sources = data
        .source_entries
        .clone()
        .into_iter()
        .map(|entry| -> ProcessingResult<_> {
            if entry.currency != data.source_account.currency {
                return Err(Report::new(ProcessingError::InternalError(format!(
                    "Currency mismatch in source entry: expected currency {}, found {}",
                    data.source_account.currency, entry.currency
                ))));
            }

            Ok(EntryDataWithClassification {
                entry,
                classification: EntryClassification::Good,
            })
        })
        .collect::<ProcessingResult<Vec<_>>>()?;

    let expected_entry = compute_expected_entry(engine, data, encryptor).await?;

    let Some(target_entry) = &data.target_entry else {
        let entries: Vec<_> = classified_sources
            .into_iter()
            .chain(std::iter::once(expected_entry))
            .collect();

        return Ok((entries, MatchOutcome::NoTarget));
    };

    let (classified_target, has_currency_mismatch, has_data_mismatch) =
        if target_entry.currency != expected_entry.entry.currency {
            (
                EntryDataWithClassification {
                    entry: target_entry.clone(),
                    classification: EntryClassification::CurrencyMismatch(
                        CurrencyMismatchData::V1(CurrencyMismatchDataV1 {
                            expected_currency: expected_entry.entry.currency,
                            actual_currency: target_entry.currency,
                        }),
                    ),
                },
                true,
                false,
            )
        } else {
            match engine
                .match_rules
                .validate_metadata(target_entry, &expected_entry.entry)?
            {
                MatchRuleValidation::Passed => (
                    EntryDataWithClassification {
                        entry: target_entry.clone(),
                        classification: EntryClassification::Good,
                    },
                    false,
                    false,
                ),
                MatchRuleValidation::Failed(mismatch_data) => (
                    EntryDataWithClassification {
                        entry: target_entry.clone(),
                        classification: EntryClassification::MetadataMismatch(
                            MetadataMismatchData::V1(MetadataMismatchDataV1 {
                                mismatched_fields: mismatch_data.mismatched_fields,
                            }),
                        ),
                    },
                    false,
                    true,
                ),
            }
        };

    let match_outcome = if has_currency_mismatch {
        MatchOutcome::CurrencyMismatch
    } else {
        let balance_state = determine_balance_with_expected(&expected_entry, &classified_target);
        if has_data_mismatch {
            MatchOutcome::DataMismatch(balance_state)
        } else {
            MatchOutcome::Good(balance_state)
        }
    };

    let entries: Vec<_> = classified_sources
        .into_iter()
        .chain(std::iter::once(classified_target))
        .collect();

    Ok((entries, match_outcome))
}

fn determine_balance_with_expected(
    expected_entry: &EntryDataWithClassification,
    target_entry: &EntryDataWithClassification,
) -> BalanceState {
    let expected_amount = expected_entry.entry.amount();
    let expected_direction = expected_entry.entry.entry_type();

    let target_amount = target_entry.entry.amount();
    let target_direction = target_entry.entry.entry_type();

    if expected_direction != target_direction {
        BalanceState::UnderAmount
    } else if expected_amount == target_amount {
        BalanceState::Balanced
    } else if expected_amount > target_amount {
        BalanceState::UnderAmount
    } else {
        BalanceState::OverAmount
    }
}

async fn compute_expected_entry(
    engine: &OneToOneManySingleV1,
    data: &ManyToOneStatusDeterminationInput,
    encryptor: &EncryptionHandler<impl ApplicationEncryptionInterface>,
) -> ProcessingResult<EntryDataWithClassification> {
    let source_entries = &data.source_entries;

    let grouping_field = &engine.source_account.grouping_field;

    let grouping_value = extract_common_result(
        source_entries
            .iter()
            .map(|e| grouping_field.extract_value(e)),
        || ProcessingError::InternalError("Sources are empty".into()).into(),
        || {
            ProcessingError::InternalError("Grouping field value mismatch across sources".into())
                .into()
        },
    )?;

    let currency = extract_common(
        source_entries.iter().map(|e| e.currency),
        || ProcessingError::InternalError("Sources are empty".into()),
        || ProcessingError::InternalError("Currency mismatch across sources".into()),
    )?;

    let (balance_direction, amount) = expected_entry_balance(source_entries);

    let effective_at = source_entries
        .iter()
        .min_by_key(|e| e.effective_at)
        .ok_or(ProcessingError::InternalError("Sources are empty".into()))?
        .effective_at;

    let expected_metadata = engine
        .match_rules
        .extract_expected_entry_metadata_from_multiple_sources(source_entries)?;

    let encrypted_metadata = encryptor
        .encrypt_json_value(expected_metadata.into())
        .await
        .change_context(ProcessingError::InternalError(
            "Failed to encrypt metadata".into(),
        ))?;

    Ok(EntryDataWithClassification {
        entry: RawEntryData {
            account_id: engine.target_account.account_id.clone(),
            amount,
            entry_type: balance_direction,
            currency,
            effective_at,
            order_id: grouping_value,
            metadata: encrypted_metadata,
            merchant_id: data.merchant_id.clone(),
            primary_staging_entry_id: None,
            transformation_id: None,
            exposed_metadata: serde_json::json!({}),
        },
        classification: EntryClassification::Expected,
    })
}

fn build_image(
    existing_entries: Vec<DomainEntry>,
    staging_entry: &DomainStagingEntry,
    source_account_id: &str,
    target_account_id: &str,
) -> ProcessingResult<(Vec<RawEntryData>, Option<RawEntryData>)> {
    let staging_as_entry = RawEntryData::from_staging_entry(staging_entry);
    let is_staging_target = staging_entry.account_id == target_account_id;

    let mut source_entries = Vec::new();
    let mut target_entries = Vec::new();

    existing_entries
        .into_iter()
        .filter(|e| e.status != EntryStatus::Expected)
        .for_each(|entry| {
            if entry.account_id == source_account_id {
                source_entries.push(RawEntryData::from_domain_entry(entry));
            } else if entry.account_id == target_account_id {
                target_entries.push(RawEntryData::from_domain_entry(entry));
            }
        });

    if is_staging_target {
        target_entries.push(staging_as_entry);
    } else {
        source_entries.push(staging_as_entry);
    }

    if target_entries.len() > 1 {
        return Err(ProcessingError::InternalError(format!(
            "ManySingle requires exactly 0 or 1 target entry, found {}",
            target_entries.len()
        ))
        .into());
    }

    Ok((source_entries, target_entries.pop()))
}

fn expected_entry_balance(entries: &[RawEntryData]) -> (BalanceDirection, MinorUnit) {
    let credit_sum: MinorUnit = entries
        .iter()
        .filter(|data| data.entry_type() == BalanceDirection::Credit)
        .map(|data| data.amount())
        .sum();

    let debit_sum: MinorUnit = entries
        .iter()
        .filter(|data| data.entry_type() == BalanceDirection::Debit)
        .map(|data| data.amount())
        .sum();

    if credit_sum >= debit_sum {
        (BalanceDirection::Debit, credit_sum - debit_sum)
    } else {
        (BalanceDirection::Credit, debit_sum - credit_sum)
    }
}
