use std::collections::HashMap;

use storage_impl::{
    metadata_schema::MetadataSchemaInterface, recon_rule::ReconRuleInterface,
    transaction::TransactionInterface, transformation::TransformationConfigInterface,
};

use crate::{
    core::staging_entry::staging_entry_views_to_response,
    domain::{
        exception_management::{
            manual_reconciliation::{
                build_new_entries, load_existing_entries_with_search_keys, prepare_staging_links,
                resolve_transaction_data, MetadataProjection, MANUALLY_RECONCILABLE_STATUSES,
            },
            ResolutionProvider,
        },
        metadata_schema::{
            data_validator::RowValidator, types::RowValidationOutcome, ParsedRow, RowMapper,
        },
        recon_engine::process_duplicate::create_new_search_duplicate,
        staging_entry::{STAGING_ENTRY_LIST_DEFAULT_LIMIT, STAGING_ENTRY_LIST_MAX_LIMIT},
    },
    errors::{
        conversions::StorageErrorExt,
        staging_entry::{StagingEntryError, StagingEntryResponse},
        transaction::{TransactionError, TransactionResponse},
        transformation::{TransformationError, TransformationResponse},
    },
    routes::AppState,
    types::{transformers::ForeignFrom, ApplicationResponse},
    utils::{
        entry::EntryDataApiExt,
        exception_management::{EntryOpsVecExt, PreparedEntryVecExt},
        staging_entry::{
            ApiStagingEntryStatusExt, DomainStagingEntryStatusExt, StagingEntryDataExt,
            StagingEntryExt,
        },
        transaction::{TransactionEntryWithDataVecExt, TransactionExt},
    },
};
use api_models::{
    entry::EntryData as EntryDataApi,
    staging_entry::{
        ApiStagingEntryStatus, GetStagingEntryResponse, LinkableStagingEntryListV2Page,
        ListLinkableStagingEntriesV2Request,
    },
    transaction::{
        ForceReconcileRequest, GetTransactionResponse, ManualReconciliationRequest,
        ManualReconciliationV2Request,
    },
    transformation::{
        ApiTransformationConfigData, ValidateEntryDataRequest, ValidateEntryDataResponse,
    },
    user::User,
    AccountData,
};
use async_bb8_diesel::AsyncConnection;
use common_enums::{
    DieselStagingEntryStatus, DieselTransactionStatus, EntryStatus, ResolutionActionType,
};
use common_types::{
    amount::{AmountResponse, AmountType},
    entry::{EntryData, EntryDataV2},
    staging_entry::{
        status::{DomainStagingEntryStatus, StagingEntryManualReviewData},
        ListStagingEntriesFilters, StagingEntryData, StagingEntryDataV2, StagingEntryListFiltersV2,
    },
    transaction::{
        ForceMatchedData, ForceMatchedDataV1, ManuallyMatchedData, ManuallyMatchedDataV1,
        MatchedData, MatchedDataV1, PartiallyReconciledData, PartiallyReconciledDataV1,
        TransactionData, TransactionDataV3,
    },
};
use common_utils::{auth::AuthContext, logging};
use diesel_models::{
    entry::EntryNew,
    errors::DatabaseError,
    search_duplicate::SearchDuplicateNew,
    search_entry_key::SearchEntryKeyNew,
    staging_entry::{
        StagingEntry as DieselStagingEntry, StagingEntryNew, StagingEntryUpdate,
        StagingEntryVersionUpdate,
    },
    transaction::{TransactionNew, TransactionVersionUpdate},
};
use error_stack::{Report, ResultExt};
use external_services::user::interfaces::UserInterface;
use futures::future;
use masking::ExposeInterface;
use recon_domain_models::{
    behaviour::{Conversion, ReverseConversion},
    staging_entry::DomainStagingEntry,
    transaction::DomainTransaction,
};
use strum::IntoEnumIterator;

/// Get available resolution action types for a staging entry
pub async fn get_staging_entry_resolutions(
    state: AppState,
    id: String,
    auth_context: AuthContext,
) -> StagingEntryResponse<Vec<ResolutionActionType>> {
    // Get the existing staging entry to verify ownership and get data
    let existing_staging_entry = state
        .store
        .find_staging_entry_by_id_and_profile(id.clone(), auth_context.profile_id.clone())
        .await
        .to_not_found_response(|| StagingEntryError::StagingEntryNotFound)?;

    // Get encryptor for decryption
    let merchant_encryptor = state
        .encryption_client
        .get_encryptor_for_merchant(auth_context.merchant_id.clone());

    // Convert to domain model (with decryption)
    let domain_staging_entry: DomainStagingEntry = existing_staging_entry
        .convert_back(&merchant_encryptor)
        .await
        .change_context(StagingEntryError::InternalServerError)?;

    // Get available resolutions based on staging entry data
    let resolutions = domain_staging_entry.data.get_available_resolutions();
    Ok(ApplicationResponse::json(resolutions))
}

/// Get available resolution action types for a transaction
pub async fn get_transaction_resolutions(
    state: &AppState,
    id: String,
    auth_context: AuthContext,
) -> TransactionResponse<Vec<ResolutionActionType>> {
    let merchant_encryptor = state
        .encryption_client
        .get_encryptor_for_merchant(auth_context.merchant_id.clone());
    // Get the transaction with given ID and profile
    let existing_transaction = state
        .store
        .find_by_id_and_profile(&id, &auth_context.profile_id)
        .await
        .to_not_found_response(|| TransactionError::TransactionNotFound { id: id.clone() })?;

    // Convert to domain model (with decryption)
    let domain_transaction: DomainTransaction = existing_transaction
        .convert_back(&merchant_encryptor)
        .await
        .change_context(TransactionError::InternalServerError)?;

    let resolutions = domain_transaction.get_available_resolutions();
    Ok(ApplicationResponse::json(resolutions))
}

/// Force reconcile a mismatched transaction by ignoring specified non-critical field mismatches
pub async fn force_reconcile_transaction(
    state: &AppState,
    id: String,
    request: ForceReconcileRequest,
    auth: AuthContext,
) -> TransactionResponse<GetTransactionResponse> {
    let merchant_encryptor = state
        .encryption_client
        .get_encryptor_for_merchant(auth.merchant_id.clone());

    // 1. Fetch and validate transaction exists and belongs to profile
    let transaction = state
        .store
        .find_by_id_and_profile(&id, &auth.profile_id)
        .await
        .to_not_found_response(|| TransactionError::TransactionNotFound { id: id.clone() })?;

    let recon_rule = state
        .store
        .find_recon_rule_by_id(&transaction.rule_id)
        .await
        .to_not_found_response(|| TransactionError::ReconRuleNotFound {
            rule_id: transaction.rule_id.clone(),
        })?;

    // 2. Check that transaction is in Mismatched status
    match transaction.status {
        DieselTransactionStatus::DataMismatch | DieselTransactionStatus::SplitMismatch => {}
        DieselTransactionStatus::Archived
        | DieselTransactionStatus::Expected
        | DieselTransactionStatus::OverAmountExpected
        | DieselTransactionStatus::OverAmountMismatch
        | DieselTransactionStatus::UnderAmountExpected
        | DieselTransactionStatus::UnderAmountMismatch
        | DieselTransactionStatus::CurrencyMismatch
        | DieselTransactionStatus::PostedManual
        | DieselTransactionStatus::MatchedAuto
        | DieselTransactionStatus::MatchedManual
        | DieselTransactionStatus::MatchedForce
        | DieselTransactionStatus::MatchedWithTolerance
        | DieselTransactionStatus::Void
        | DieselTransactionStatus::PartiallyReconciled => {
            return Err(TransactionError::InvalidTransactionStatus {
                expected_statuses: vec![
                    DieselTransactionStatus::DataMismatch,
                    DieselTransactionStatus::SplitMismatch,
                ],
            }
            .into());
        }
    }

    let domain_transaction: DomainTransaction = transaction
        .convert_back(&merchant_encryptor)
        .await
        .change_context(TransactionError::InternalServerError)?;

    // 3. Validate that transaction can be force reconciled
    domain_transaction.validate_ledger_integrity()?;

    // 4. Execute all operations atomically in a database transaction
    let conn = state
        .store
        .get_connection()
        .await
        .change_context(TransactionError::InternalServerError)?;

    let reconciled_transaction = conn
        .transaction_async(|conn| async move {
            // Step 1: Archive old data using the centralized function
            let transaction = domain_transaction.convert();

            let (
                _archived_transaction,
                old_entries,
                _archived_expectations,
                _archived_search_group,
            ) = state
                .store
                .discard_transaction_with_entries_and_expectations(&conn, transaction.clone())
                .await
                .map_err(|e| *e.current_context())?;

            // Step 2: Create new entries with Matched status and incremented version
            let new_version = transaction.version + 1;
            let new_entries: Vec<EntryNew> = old_entries
                .iter()
                .map(|entry| {
                    EntryNew::new(
                        entry.account_id.clone(),
                        transaction.transaction_id.clone(),
                        entry.balance_direction,
                        entry.amount,
                        entry.currency,
                        EntryStatus::Matched, // Force reconciled entries are Matched
                        entry.effective_at,
                        entry.merchant_id.clone(),
                        entry.metadata.clone(),
                        new_version,
                        EntryData::V2(EntryDataV2::Matched),
                        entry.primary_staging_entry_id.clone(), // Preserve from original entry
                        entry.transformation_config_id.clone(), // Preserve from original entry
                        entry.entry_identifier.clone(),
                        entry.exposed_metadata.clone(),
                    )
                })
                .collect();

            // Step 3: Batch insert new entries
            let inserted_entries = EntryNew::batch_insert_chunked(&conn, new_entries)
                .await
                .map_err(|e| {
                    logging::error!("Failed to batch insert force reconciled entries: {:?}", e);
                    *e.current_context()
                })?;

            let update = TransactionVersionUpdate {
                new_entries: inserted_entries,
                data: TransactionData::V3(TransactionDataV3::Matched(MatchedData::V1(
                    MatchedDataV1::Force(ForceMatchedData::V1(ForceMatchedDataV1 {
                        reason: request.reason,
                    })),
                ))),
            };

            let force_reconciled_transaction_new =
                TransactionNew::new_versioned(transaction, update, Some(auth.user_id));

            // Step 4: Insert the force reconciled transaction
            let db_transaction = force_reconciled_transaction_new
                .insert(&conn)
                .await
                .map_err(|e| {
                    logging::error!("Failed to insert force reconciled transaction: {:?}", e);
                    *e.current_context()
                })?;

            Result::<_, DatabaseError>::Ok(db_transaction)
        })
        .await
        .change_context(TransactionError::InternalServerError)?;

    let reconciled_transaction: DomainTransaction = reconciled_transaction
        .convert_back(&merchant_encryptor)
        .await
        .change_context(TransactionError::InternalServerError)?;

    let transaction_modified_by = reconciled_transaction
        .get_modified_by(state, auth.merchant_id, auth.profile_id)
        .await?;

    let response =
        reconciled_transaction.to_response(&recon_rule.aging_config, transaction_modified_by)?;
    Ok(ApplicationResponse::json(response))
}

/// Get staging entries that can be linked to a transaction
pub async fn list_linkable_staging_entries_for_transaction(
    state: &AppState,
    id: String,
    auth_context: AuthContext,
) -> TransactionResponse<Vec<GetStagingEntryResponse>> {
    // Step 1: Fetch and validate transaction exists and belongs to profile
    let existing_transaction = state
        .store
        .find_by_id_and_profile(&id, &auth_context.profile_id)
        .await
        .to_not_found_response(|| TransactionError::TransactionNotFound { id: id.clone() })?;

    // Step 2: Create filters for staging entries query - filter by currency and status
    let filters = ListStagingEntriesFilters {
        status: Some(
            ApiStagingEntryStatus::NeedsManualReview(None)
                .to_domain_statuses()
                .into_iter()
                .map(DieselStagingEntryStatus::from)
                .collect(),
        ),
        currency: Some(vec![existing_transaction.currency]),
        entry_type: None,
        account_id: None,
        transformation_history_id: None,
        time_range: None,
        staging_entry_id: None,
    };

    // Step 3: Use existing list function to get staging entries
    let staging_entries = state
        .store
        .list_staging_entries_by_profile_id(&auth_context.profile_id, filters, None)
        .await
        .change_context(TransactionError::InternalServerError)?;

    // Step 4: Filter by is_linkable() before expensive domain conversion to avoid unnecessary decryption calls
    let linkable_staging_entries: Vec<_> = staging_entries
        .into_iter()
        .filter(|staging_entry_view| {
            DomainStagingEntryStatus::from(staging_entry_view.staging_entry.status).is_linkable()
        })
        .collect();

    // Step 5: Get encryptor for decryption operations
    let merchant_encryptor = state
        .encryption_client
        .get_encryptor_for_merchant(auth_context.merchant_id.clone());

    // Step 6: Convert only linkable domain models to response models concurrently, following staging_entry.rs pattern
    let response: Vec<GetStagingEntryResponse> =
        future::try_join_all(linkable_staging_entries.into_iter().map(
            |staging_entry_view| async {
                let domain_staging_entry: DomainStagingEntry = staging_entry_view
                    .staging_entry
                    .convert_back(&merchant_encryptor)
                    .await
                    .change_context(TransactionError::InternalServerError)?;

                let staging_entry_modified_by = domain_staging_entry
                    .get_modified_by(
                        state,
                        auth_context.merchant_id.clone(),
                        auth_context.profile_id.clone(),
                    )
                    .await
                    .change_context(TransactionError::InternalServerError)?;

                Ok::<GetStagingEntryResponse, Report<TransactionError>>(GetStagingEntryResponse {
                    id: domain_staging_entry.id,
                    staging_entry_id: domain_staging_entry.staging_entry_id,
                    account: AccountData {
                        account_id: staging_entry_view.account.id,
                        account_name: staging_entry_view.account.name,
                    },
                    entry_type: domain_staging_entry.entry_type,
                    amount: AmountResponse::from_minor_unit(
                        domain_staging_entry.amount,
                        domain_staging_entry.currency,
                    )
                    .change_context(TransactionError::InternalServerError)?,
                    status: domain_staging_entry.status.to_api_response().coarse(),
                    detailed_status: domain_staging_entry.status.to_api_response(),
                    processing_mode: domain_staging_entry.processing_mode,
                    effective_at: domain_staging_entry.effective_at,
                    metadata: domain_staging_entry.metadata,
                    exposed_metadata: domain_staging_entry.exposed_metadata,
                    searchable_metadata: domain_staging_entry.searchable_metadata,
                    transformation_id: domain_staging_entry.transformation_id.clone(),
                    transformation_config: ApiTransformationConfigData {
                        transformation_config_id: domain_staging_entry.transformation_id,
                        transformation_config_name: staging_entry_view.transformation_config.name,
                    },
                    transformation_history_id: domain_staging_entry.transformation_history_id,
                    data: domain_staging_entry.data.to_api_response(),
                    version: domain_staging_entry.version,
                    discarded_at: domain_staging_entry.discarded_at,
                    discarded_status: domain_staging_entry
                        .discarded_status
                        .map(|status| status.to_api_response().coarse()),
                    detailed_discarded_status: domain_staging_entry
                        .discarded_status
                        .map(DomainStagingEntryStatus::to_api_response),
                    discarded_data: domain_staging_entry
                        .discarded_data
                        .map(|d| d.to_api_response()),
                    order_id: domain_staging_entry.order_id,
                    modified_by: staging_entry_modified_by,
                })
            },
        ))
        .await?;

    Ok(ApplicationResponse::json(response))
}

pub async fn list_linkable_staging_entries_for_transaction_v2(
    state: &AppState,
    id: String,
    request: ListLinkableStagingEntriesV2Request,
    auth_context: AuthContext,
) -> TransactionResponse<LinkableStagingEntryListV2Page> {
    let ListLinkableStagingEntriesV2Request {
        filters,
        cursor_payload,
    } = request;

    filters
        .validate()
        .map_err(|_| TransactionError::TimeRangeValidationError)?;

    let existing_transaction = state
        .store
        .find_by_id_and_profile(&id, &auth_context.profile_id)
        .await
        .to_not_found_response(|| TransactionError::TransactionNotFound { id: id.clone() })?;

    let staging_entry_filters = StagingEntryListFiltersV2 {
        status: Some(
            StagingEntryManualReviewData::iter()
                .filter(|status| status.is_linkable())
                .map(|status| DomainStagingEntryStatus::NeedsManualReview(status).into())
                .collect(),
        ),
        entry_type: filters.entry_type,
        account_ids: filters.account_ids,
        transformation_config_ids: filters.transformation_config_ids,
        transformation_history_id: None,
        transformation_history_ids: filters.transformation_history_ids,
        currency: Some(vec![existing_transaction.currency]),
        time_range: filters.time_range,
        staging_entry_id: filters.staging_entry_id,
        order_id: filters.order_id,
    };

    let pagination_query = cursor_payload.clone().into_query(
        STAGING_ENTRY_LIST_DEFAULT_LIMIT,
        STAGING_ENTRY_LIST_MAX_LIMIT,
    );

    let page = state
        .store
        .list_staging_entries_v2(
            &auth_context.profile_id,
            staging_entry_filters,
            pagination_query,
        )
        .await
        .change_context(TransactionError::InternalServerError)?;

    let (next_cursor, prev_cursor) = cursor_payload.build_cursors(&page.views, page.has_more);
    let items = staging_entry_views_to_response(state, &auth_context, page.views)
        .await
        .change_context(TransactionError::InternalServerError)?;

    Ok(ApplicationResponse::json(LinkableStagingEntryListV2Page {
        items,
        next_cursor,
        prev_cursor,
    }))
}

pub async fn manual_transaction_reconciliation(
    state: &AppState,
    id: String,
    request: ManualReconciliationRequest,
    auth_context: AuthContext,
) -> TransactionResponse<GetTransactionResponse> {
    let existing_transaction = state
        .store
        .find_by_id_and_profile(&id, &auth_context.profile_id)
        .await
        .to_not_found_response(|| TransactionError::TransactionNotFound { id: id.clone() })?;

    let recon_rule = state
        .store
        .find_recon_rule_by_profile_id_and_rule_id(
            &auth_context.profile_id,
            &existing_transaction.rule_id,
        )
        .await
        .to_not_found_response(|| TransactionError::ReconRuleNotFound {
            rule_id: existing_transaction.rule_id.clone(),
        })?;

    match existing_transaction.status {
        DieselTransactionStatus::OverAmountExpected
        | DieselTransactionStatus::OverAmountMismatch
        | DieselTransactionStatus::UnderAmountExpected
        | DieselTransactionStatus::UnderAmountMismatch
        | DieselTransactionStatus::DataMismatch
        | DieselTransactionStatus::CurrencyMismatch
        | DieselTransactionStatus::SplitMismatch
        | DieselTransactionStatus::Expected
        | DieselTransactionStatus::PartiallyReconciled => {}
        DieselTransactionStatus::Archived
        | DieselTransactionStatus::PostedManual
        | DieselTransactionStatus::MatchedAuto
        | DieselTransactionStatus::MatchedManual
        | DieselTransactionStatus::MatchedForce
        | DieselTransactionStatus::MatchedWithTolerance
        | DieselTransactionStatus::Void => {
            return Err(TransactionError::InvalidTransactionStatus {
                expected_statuses: vec![
                    DieselTransactionStatus::OverAmountExpected,
                    DieselTransactionStatus::OverAmountMismatch,
                    DieselTransactionStatus::UnderAmountExpected,
                    DieselTransactionStatus::UnderAmountMismatch,
                    DieselTransactionStatus::DataMismatch,
                    DieselTransactionStatus::CurrencyMismatch,
                    DieselTransactionStatus::SplitMismatch,
                    DieselTransactionStatus::Expected,
                    DieselTransactionStatus::PartiallyReconciled,
                ],
            }
            .into());
        }
    }

    let staging_entries_to_process: Vec<DieselStagingEntry> = future::try_join_all(
        request
            .transaction_entries
            .iter()
            .filter_map(|entry| entry.staging_entry_id.as_ref())
            .map(|staging_entry_id| async {
                state
                    .store
                    .find_staging_entry_by_id_and_profile(
                        staging_entry_id.clone(),
                        auth_context.profile_id.clone(),
                    )
                    .await
                    .to_not_found_response(|| TransactionError::StagingEntryNotFound {
                        staging_entry_id: staging_entry_id.clone(),
                    })
            }),
    )
    .await?
    .into_iter()
    .filter(|staging_entry| staging_entry.status != DieselStagingEntryStatus::Processed)
    .collect();

    // Validate that the staging entry can be linked
    if let Some(staging_entry) = staging_entries_to_process
        .iter()
        .find(|staging_entry| !DomainStagingEntryStatus::from(staging_entry.status).is_linkable())
    {
        return Err(TransactionError::StagingEntryNotLinkable {
            staging_entry_id: staging_entry.id.clone(),
        }
        .into());
    }

    // Validate currency consistency - staging entries must match transaction currency
    if let Some(staging_entry) = staging_entries_to_process
        .iter()
        .find(|staging_entry| staging_entry.currency != existing_transaction.currency)
    {
        return Err(TransactionError::StagingEntryCurrencyMismatch {
            staging_entry_id: staging_entry.id.clone(),
            staging_currency: staging_entry.currency,
            transaction_currency: existing_transaction.currency,
        }
        .into());
    }

    let merchant_encryptor = state
        .encryption_client
        .get_encryptor_for_merchant(auth_context.merchant_id.clone());

    let domain_staging_entries_to_process: Vec<DomainStagingEntry> = future::try_join_all(
        staging_entries_to_process
            .iter()
            .map(|staging_entry| async {
                staging_entry
                    .clone()
                    .convert_back(&merchant_encryptor)
                    .await
                    .change_context(TransactionError::InternalServerError)
            }),
    )
    .await?;

    let staging_preparation_data =
        future::try_join_all(domain_staging_entries_to_process.iter().map(
            |domain_staging_entry| async {
                // Fetch transformation config
                let transformation_config = state
                    .store
                    .find_transformation_config_by_id_and_profile_id(
                        &domain_staging_entry.transformation_id,
                        &domain_staging_entry.profile_id,
                    )
                    .await
                    .change_context(TransactionError::InternalServerError)?;

                // Create processed staging entry
                let processed_staging_entry = StagingEntryNew::new_versioned(
                    domain_staging_entry.clone().convert(),
                    StagingEntryVersionUpdate::ProcessedUpdate {
                        data: StagingEntryData::V2(StagingEntryDataV2::Processed),
                    },
                    Some(auth_context.user_id.clone()),
                );

                // Create search duplicate
                let search_duplicate = create_new_search_duplicate(
                    state,
                    domain_staging_entry,
                    &processed_staging_entry,
                    &transformation_config,
                    &merchant_encryptor,
                )
                .await
                .change_context(TransactionError::InternalServerError)?;

                Ok::<
                    (DomainStagingEntry, StagingEntryNew, SearchDuplicateNew),
                    Report<TransactionError>,
                >((
                    domain_staging_entry.clone(),
                    processed_staging_entry,
                    search_duplicate,
                ))
            },
        ))
        .await?;

    // Create a mapping from staging_entry_id to transformation_id for easy lookup
    let staging_entry_id_transformation_id_mapper: HashMap<String, String> =
        domain_staging_entries_to_process
            .iter()
            .map(|staging_entry| {
                (
                    staging_entry.id.clone(),
                    staging_entry.transformation_id.clone(),
                )
            })
            .collect();

    request.transaction_entries.validate_positive_amounts()?;
    request
        .transaction_entries
        .validate_accounts_and_currencies(&auth_context, state)
        .await?;

    let is_balanced = &request.transaction_entries.validate_credit_debit_balance();
    let new_version = existing_transaction.version + 1;

    // Check if there are any Expected entries
    let has_expected_entries = request
        .transaction_entries
        .iter()
        .any(|entry| matches!(entry.data, EntryDataApi::Expected));

    let transaction_data = if has_expected_entries {
        // If has Expected entries, always PartiallyReconciled (regardless of balance)
        TransactionData::V3(TransactionDataV3::PartiallyReconciled(
            PartiallyReconciledData::V1(PartiallyReconciledDataV1 {
                reason: request.reason,
            }),
        ))
    } else {
        // No Expected entries, check balance
        match is_balanced {
            Ok(_) => {
                // Balanced + no Expected = Matched
                TransactionData::V3(TransactionDataV3::Matched(MatchedData::V1(
                    MatchedDataV1::Manual(ManuallyMatchedData::V1(ManuallyMatchedDataV1 {
                        reason: request.reason,
                    })),
                )))
            }
            Err(_) => {
                // Unbalanced = PartiallyReconciled
                TransactionData::V3(TransactionDataV3::PartiallyReconciled(
                    PartiallyReconciledData::V1(PartiallyReconciledDataV1 {
                        reason: request.reason,
                    }),
                ))
            }
        }
    };

    // Validate no entries are in Matched, Archived, or Void status
    if let Some(invalid_entry) = request.transaction_entries.iter().find(|entry| {
        matches!(
            entry.data,
            EntryDataApi::Matched
                | EntryDataApi::Posted
                | EntryDataApi::Void
                | EntryDataApi::Archived
        )
    }) {
        return Err(TransactionError::InvalidEntryStatus {
            // TODO: This api is to be deprecated
            entry_id: "".to_string(),
            status: EntryStatus::from(
                &invalid_entry
                    .data
                    .clone()
                    .to_diesel()
                    .change_context(TransactionError::InternalServerError)?,
            ),
            expected_statuses: vec![
                EntryStatus::Pending,
                EntryStatus::Expected,
                EntryStatus::Mismatched,
            ],
        }
        .into());
    }

    let entries_to_create: Vec<EntryNew> =
        future::try_join_all(request.transaction_entries.iter().map(
            |transaction_entry_with_data| async {
                let entry_data = if has_expected_entries {
                    // If transaction has Expected entries, preserve all original statuses
                    transaction_entry_with_data
                        .data
                        .clone()
                        .to_diesel()
                        .change_context(TransactionError::InternalServerError)?
                } else {
                    // No Expected entries, check if balanced
                    match is_balanced {
                        Ok(_) => EntryData::V2(EntryDataV2::Matched), // Balanced and no Expected entries = Matched
                        Err(_) => transaction_entry_with_data
                            .data
                            .clone()
                            .to_diesel()
                            .change_context(TransactionError::InternalServerError)?, // Unbalanced = keep original
                    }
                };
                let encrypted_metadata = merchant_encryptor
                    .encrypt_json_value(transaction_entry_with_data.metadata.clone().expose())
                    .await
                    .change_context(TransactionError::InternalServerError)?;

                // Get transformation_id from staging entry if available
                let transformation_id = transaction_entry_with_data
                    .staging_entry_id
                    .as_ref()
                    .and_then(|staging_id| {
                        staging_entry_id_transformation_id_mapper.get(staging_id)
                    })
                    .cloned();

                let processed_staging_entry_data =
                    match transaction_entry_with_data.staging_entry_id.clone() {
                        Some(primary_staging_entry_id) => staging_preparation_data
                            .iter()
                            .find(|data| data.0.id == primary_staging_entry_id),
                        None => None,
                    };

                let processed_staging_entry_id =
                    processed_staging_entry_data.map(|data| data.1.id.clone());

                let entry_new = EntryNew::new(
                    transaction_entry_with_data.account_id.clone(),
                    existing_transaction.transaction_id.clone(),
                    transaction_entry_with_data.entry_type,
                    AmountType::from_major_unit(
                        transaction_entry_with_data.amount,
                        transaction_entry_with_data.currency,
                    )
                    .change_context(TransactionError::InvalidAmount {
                        details: "Invalid amount provided".to_string(),
                    })?
                    .value(),
                    transaction_entry_with_data.currency,
                    (&entry_data).into(),
                    transaction_entry_with_data.effective_at,
                    auth_context.merchant_id.clone(),
                    encrypted_metadata.into(),
                    new_version,
                    entry_data,
                    processed_staging_entry_id,
                    transformation_id,
                    transaction_entry_with_data.order_id.clone(),
                    serde_json::json!({}),
                );

                Ok::<EntryNew, Report<TransactionError>>(entry_new)
            },
        ))
        .await?;

    let conn = state
        .store
        .get_connection()
        .await
        .change_context(TransactionError::InternalServerError)?;

    let final_transaction = conn
        .transaction_async(|conn| async move {
            // Archive old transaction, entries, and expectations
            let (
                _archived_transaction,
                _old_entries,
                _archived_expectations,
                _archived_search_group,
            ) = state
                .store
                .discard_transaction_with_entries_and_expectations(
                    &conn,
                    existing_transaction.clone(),
                )
                .await
                .map_err(|e| *e.current_context())?;

            // Process staging entries using pre-calculated data
            for (staging_entry, processed_staging_entry, search_duplicate) in
                staging_preparation_data.iter()
            {
                // Archive the original staging entry
                let archive_update = StagingEntryUpdate::DiscardUpdate {
                    original_data: staging_entry.data.clone(),
                };

                DieselStagingEntry::update_by_id(&conn, &staging_entry.id, archive_update.into())
                    .await
                    .map_err(|e| {
                        logging::error!("Failed to archive staging entry: {e}");
                        *e.current_context()
                    })?;

                // Insert processed staging entry
                processed_staging_entry
                    .clone()
                    .insert(&conn)
                    .await
                    .map_err(|e| {
                        logging::error!("Failed to insert processed staging entry: {e}");
                        *e.current_context()
                    })?;

                // Insert search duplicate
                search_duplicate.clone().insert(&conn).await.map_err(|e| {
                    logging::error!("Failed to insert search duplicate: {e}");
                    *e.current_context()
                })?;
            }

            // Batch insert new entries
            let inserted_entries = EntryNew::batch_insert_chunked(&conn, entries_to_create)
                .await
                .map_err(|e| {
                    logging::error!("Failed to batch insert new entries: {e}");
                    *e.current_context()
                })?;

            // Create the new transaction with determined data
            let db_transaction = TransactionNew::new_versioned(
                existing_transaction.clone(),
                TransactionVersionUpdate {
                    new_entries: inserted_entries,
                    data: transaction_data,
                },
                Some(auth_context.user_id),
            )
            .insert(&conn)
            .await
            .map_err(|e| {
                logging::error!("Failed to insert new transaction: {e}");
                *e.current_context()
            })?;

            Result::<_, DatabaseError>::Ok(db_transaction)
        })
        .await
        .change_context(TransactionError::InternalServerError)?;

    let final_transaction: DomainTransaction = final_transaction
        .convert_back(&merchant_encryptor)
        .await
        .change_context(TransactionError::InternalServerError)?;

    let transaction_modified_by = final_transaction
        .get_modified_by(state, auth_context.merchant_id, auth_context.profile_id)
        .await?;

    Ok(ApplicationResponse::json(final_transaction.to_response(
        &recon_rule.aging_config,
        transaction_modified_by,
    )?))
}

pub async fn validate_raw_entry_data(
    state: &AppState,
    req: ValidateEntryDataRequest,
    auth: AuthContext,
) -> TransformationResponse<ValidateEntryDataResponse> {
    let transformation_config = state
        .store
        .find_transformation_config_by_id_and_profile_id(&req.transformation_id, &auth.profile_id)
        .await
        .to_not_found_response(|| TransformationError::TransformationConfigNotFound {
            id: req.transformation_id.clone(),
        })?;
    let metadata_schema = state
        .store
        .find_metadata_schema_by_id_and_profile_id(
            &transformation_config.metadata_schema_id,
            &auth.profile_id,
        )
        .await
        .to_not_found_response(
            || TransformationError::MetadataSchemaNotFoundForTransformation {
                transformation_id: req.transformation_id,
            },
        )?;

    let parsed_row = ParsedRow::from_api_fields(req.raw_entry_data, None);
    let mapped_row = metadata_schema.schema_data.map_row(parsed_row);
    let result = match metadata_schema.schema_data.validate_row(&mapped_row) {
        RowValidationOutcome::Validated(validated) => {
            ValidateEntryDataResponse::Success(validated.to_api_response())
        }
        RowValidationOutcome::Rejected(rejection) => {
            ValidateEntryDataResponse::Failure(rejection.to_api_response())
        }
    };

    Ok(ApplicationResponse::json(result))
}

/// List users who can be assigned to exception management tasks
pub async fn list_assignees(
    state: &AppState,
    auth_context: AuthContext,
) -> TransactionResponse<Vec<User>> {
    let assignees = state
        .user_client
        .list_assignees(auth_context.merchant_id, auth_context.profile_id)
        .await
        .change_context(TransactionError::InternalServerError)?;

    let users = assignees.into_iter().map(User::foreign_from).collect();

    Ok(ApplicationResponse::json(users))
}

pub async fn manual_transaction_reconciliation_v2(
    state: &AppState,
    id: String,
    request: ManualReconciliationV2Request,
    auth_context: AuthContext,
) -> TransactionResponse<GetTransactionResponse> {
    let ManualReconciliationV2Request { entry_ops, reason } = request;

    let existing_transaction = state
        .store
        .find_by_id_and_profile(&id, &auth_context.profile_id)
        .await
        .to_not_found_response(|| TransactionError::TransactionNotFound { id: id.clone() })?;

    if !MANUALLY_RECONCILABLE_STATUSES.contains(&existing_transaction.status) {
        return Err(TransactionError::InvalidTransactionStatus {
            expected_statuses: MANUALLY_RECONCILABLE_STATUSES.to_vec(),
        }
        .into());
    }

    entry_ops.validate_payload(&existing_transaction)?;

    let transaction_currency = existing_transaction.currency;
    let merchant_encryptor = state
        .encryption_client
        .get_encryptor_for_merchant(auth_context.merchant_id.clone());

    let recon_rule = state
        .store
        .find_recon_rule_by_profile_id_and_rule_id(
            &existing_transaction.profile_id,
            &existing_transaction.rule_id,
        )
        .await
        .to_not_found_response(|| TransactionError::ReconRuleNotFound {
            rule_id: existing_transaction.rule_id.clone(),
        })?;

    let existing_entries = load_existing_entries_with_search_keys(
        state,
        &auth_context.profile_id,
        &existing_transaction,
    )
    .await?;

    entry_ops.validate_with_old_entries(&existing_entries)?;

    entry_ops
        .validate_referenced_accounts_from_db(&auth_context, state, transaction_currency)
        .await?;

    let staging_links = prepare_staging_links(
        state,
        &auth_context,
        &entry_ops,
        transaction_currency,
        &merchant_encryptor,
    )
    .await?;

    let config_ids = entry_ops
        .referenced_config_ids(&existing_entries)
        .into_iter()
        .chain(
            staging_links
                .values()
                .map(|link| link.snapshot.transformation_id.clone()),
        )
        .collect();

    let projection =
        MetadataProjection::resolve(state, &auth_context.profile_id, config_ids).await?;

    for (config_id, account_id) in entry_ops.transformations_to_validate(&existing_entries)? {
        let config = projection.config(config_id)?;

        if config.account_id != account_id {
            return Err(TransactionError::TransformationConfigAccountMismatch {
                config_id: config_id.to_string(),
                account_id: account_id.to_string(),
            }
            .into());
        }
    }

    let mut prepared_entries = build_new_entries(
        existing_entries,
        entry_ops,
        &existing_transaction,
        &auth_context,
        &staging_links,
        &projection,
        &merchant_encryptor,
    )
    .await?;

    prepared_entries.validate_rule_account_coverage(&recon_rule)?;

    let transaction_data = resolve_transaction_data(&mut prepared_entries, reason);

    let entry_count = prepared_entries.len();
    let (new_entries, entry_search_keys) = prepared_entries.into_iter().fold(
        (Vec::with_capacity(entry_count), Vec::new()),
        |(mut entries, mut search_keys), prepared| {
            search_keys.extend(
                prepared
                    .search_keys
                    .into_iter()
                    .map(|key| (prepared.entry.id.clone(), key)),
            );
            entries.push(prepared.entry);
            (entries, search_keys)
        },
    );

    let conn = state
        .store
        .get_connection()
        .await
        .change_context(TransactionError::InternalServerError)?;

    let user_id = auth_context.user_id.clone();

    let transaction = conn
        .transaction_async(|conn| async move {
            let (transaction, _, _, _) = state
                .store
                .discard_transaction_with_entries_and_expectations(&conn, existing_transaction)
                .await
                .map_err(|e| *e.current_context())?;

            for link in staging_links.into_values() {
                DieselStagingEntry::update_by_id(
                    &conn,
                    &link.snapshot.id,
                    StagingEntryUpdate::DiscardUpdate {
                        original_data: link.snapshot.data,
                    }
                    .into(),
                )
                .await
                .map_err(|e| *e.current_context())?;
                link.processed
                    .insert(&conn)
                    .await
                    .map_err(|e| *e.current_context())?;
                link.search_duplicate
                    .insert(&conn)
                    .await
                    .map_err(|e| *e.current_context())?;
            }

            let inserted_entries = EntryNew::batch_insert_chunked(&conn, new_entries)
                .await
                .map_err(|e| *e.current_context())?;

            let transaction = TransactionNew::new_versioned(
                transaction,
                TransactionVersionUpdate {
                    new_entries: inserted_entries,
                    data: transaction_data,
                },
                Some(user_id),
            )
            .insert(&conn)
            .await
            .map_err(|e| *e.current_context())?;

            let search_entry_keys = entry_search_keys
                .into_iter()
                .map(|(entry_id, key)| {
                    key.get_search_entry_key(
                        transaction.profile_id.clone(),
                        entry_id,
                        transaction.id.clone(),
                    )
                    .construct_new()
                })
                .collect();

            SearchEntryKeyNew::batch_insert(search_entry_keys, &conn)
                .await
                .map_err(|e| *e.current_context())?;

            Result::<_, DatabaseError>::Ok(transaction)
        })
        .await
        .change_context(TransactionError::InternalServerError)?;

    let transaction: DomainTransaction = transaction
        .convert_back(&merchant_encryptor)
        .await
        .change_context(TransactionError::InternalServerError)?;

    let modified_by = transaction
        .get_modified_by(state, auth_context.merchant_id, auth_context.profile_id)
        .await?;

    Ok(ApplicationResponse::json(
        transaction.to_response(&recon_rule.aging_config, modified_by)?,
    ))
}
