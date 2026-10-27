use std::collections::{HashMap, HashSet};

use storage_impl::{recon_rule::ReconRuleInterface, transaction::TransactionInterface};

use crate::{
    errors::{
        conversions::StorageErrorExt,
        transaction::{TransactionError, TransactionResponse, TransactionResult},
    },
    routes::AppState,
    types::{
        transformers::{ForeignFrom, ForeignInto},
        ApplicationResponse,
    },
    utils::transaction::{
        group_transaction_views, hydrate_transaction_rule_views, ApiTransactionFilterStatusExt,
        TransactionEntryExt, TransactionEntryVecExt, TransactionExt,
    },
};
use api_models::{
    transaction::{
        ApiTransactionAccountFilters, ApiTransactionFilterStatus, GetTransactionResponse,
        ListTransactionsV2Request, TransactionListV2Page, TransactionRequest,
        TransactionWithEntriesResponse, VoidTransactionRequest,
    },
    user::User,
};
use async_bb8_diesel::AsyncConnection;
use common_enums::{DieselTransactionStatus, EntryStatus};
use common_types::{
    entry::{EntryData, EntryDataV2},
    transaction::{
        ManualPostedData, ManualPostedDataV1, PostedData, PostedDataV1, TransactionData,
        TransactionDataV3, VoidData, VoidDataV1,
    },
};
use common_utils::{auth::AuthContext, date_time, errors::ErrorSwitchFrom, logging};
use diesel_models::{
    entry::EntryNew,
    transaction::{Transaction, TransactionNew},
};
use diesel_models::{errors::DatabaseError, transaction::TransactionVersionUpdate};
use error_stack::ResultExt;
use external_services::user::interfaces::UserInterface;
use recon_domain_models::{
    behaviour::{Conversion, ReverseConversion},
    transaction::{DomainTransaction, TransactionCreationData},
};
use strum::IntoEnumIterator;

const TXN_LIST_DEFAULT_LIMIT: u16 = 10;
const TXN_LIST_MAX_LIMIT: u16 = 50;
const TXN_LIST_MAX_ENTRIES_PER_TRANSACTION: u16 = 5;

pub async fn create_transaction(
    state: &AppState,
    request: TransactionRequest,
    auth: AuthContext,
) -> TransactionResponse<GetTransactionResponse> {
    if request.entries.len() < 2 {
        return Err(TransactionError::InsufficientEntries.into());
    }
    if request.rule_id.trim().is_empty() {
        return Err(TransactionError::InvalidRuleId.into());
    }

    request.entries.validate_positive_amounts()?;
    request
        .entries
        .validate_accounts_and_currencies(&auth, state)
        .await?;
    request.entries.validate_credit_debit_balance()?;

    let merchant_encryptor = state
        .encryption_client
        .get_encryptor_for_merchant(auth.merchant_id.clone());
    let recon_rule = state
        .store
        .find_recon_rule_by_profile_id_and_rule_id(&auth.profile_id, &request.rule_id)
        .await
        .change_context(TransactionError::ReconRuleNotFound {
            rule_id: request.rule_id,
        })?;

    let currency = request
        .entries
        .first()
        .ok_or(TransactionError::InsufficientEntries)?
        .get_currency();

    let (creation_entries, transaction_data, search_expectations) = request
        .entries
        .to_creation_data(
            state,
            auth.merchant_id.clone(),
            &auth.profile_id,
            &merchant_encryptor,
        )
        .await?;

    let transaction_creation_data = TransactionCreationData {
        entries: creation_entries,
        profile_id: auth.profile_id.clone(),
        rule_id: recon_rule.id,
        currency,
        data: transaction_data,
        search_group: None,
        search_expectations,
        modified_by: Some(auth.user_id.clone()),
    };
    let transaction_with_entries = transaction_creation_data.get_new_transaction_data();

    let new_entries = transaction_with_entries
        .entries
        .into_iter()
        .map(|entry| entry.construct_new())
        .collect();

    let new_search_entry_keys = transaction_with_entries
        .search_entry_keys
        .into_iter()
        .map(|search_entry_key| search_entry_key.construct_new())
        .collect();

    let new_search_expectations = transaction_with_entries
        .search_expectations
        .into_iter()
        .map(|exp| exp.construct_new())
        .collect();

    let new_transaction = transaction_with_entries.transaction.construct_new();

    let (transaction, _created_entries, _created_expectations) = state
        .store
        .create_transaction_with_entries_and_expectations(
            new_transaction,
            new_entries,
            new_search_expectations,
            new_search_entry_keys,
        )
        .await
        .change_context(TransactionError::InternalServerError)?;

    let domain_transaction: DomainTransaction = transaction
        .convert_back(&merchant_encryptor)
        .await
        .change_context(TransactionError::InternalServerError)?;
    let transaction_modified_by = domain_transaction
        .get_modified_by(state, auth.merchant_id, auth.profile_id)
        .await?;
    let response =
        domain_transaction.to_response(&recon_rule.aging_config, transaction_modified_by)?;
    Ok(ApplicationResponse::json_created(response))
}

pub async fn get_transaction(
    state: &AppState,
    auth: AuthContext,
    id: String,
) -> TransactionResponse<GetTransactionResponse> {
    let merchant_encryptor = state
        .encryption_client
        .get_encryptor_for_merchant(auth.merchant_id.clone());

    let transaction = state
        .store
        .find_by_id_and_profile(&id, &auth.profile_id)
        .await
        .change_context(TransactionError::TransactionNotFound { id: id.clone() })?;

    let domain_transaction: DomainTransaction = transaction
        .convert_back(&merchant_encryptor)
        .await
        .change_context(TransactionError::InternalServerError)?;

    // Fetch the recon rule to get aging config
    let recon_rule = state
        .store
        .find_recon_rule_by_profile_id_and_rule_id(&auth.profile_id, &domain_transaction.rule_id)
        .await
        .change_context(TransactionError::ReconRuleNotFound {
            rule_id: domain_transaction.rule_id.clone(),
        })?;

    let transaction_modified_by = domain_transaction
        .get_modified_by(state, auth.merchant_id, auth.profile_id)
        .await?;

    let response =
        domain_transaction.to_response(&recon_rule.aging_config, transaction_modified_by)?;

    Ok(ApplicationResponse::json(response))
}

/// List transactions with detailed account/entry information
pub async fn list_transactions(
    state: &AppState,
    auth: AuthContext,
    filters: ApiTransactionAccountFilters,
) -> TransactionResponse<Vec<TransactionWithEntriesResponse>> {
    filters
        .validate()
        .map_err(|_| TransactionError::TimeRangeValidationError)?;

    let filter_status = filters.status.clone();

    let transaction_account_views = state
        .store
        .list_with_accounts_and_entries(&auth.profile_id, filters.foreign_into(), None)
        .await
        .change_context(TransactionError::InternalServerError)?;

    let encryptor = state
        .encryption_client
        .get_encryptor_for_merchant(auth.merchant_id.clone());

    let mut grouped_views = group_transaction_views(&encryptor, transaction_account_views).await?;

    if let Some(statuses) = filter_status {
        grouped_views.retain(|view| {
            let api_transaction_filter_status =
                ApiTransactionFilterStatus::from_domain_transaction_status(
                    &view.transaction.status,
                    &view.rule.aging_config,
                    view.transaction.effective_at,
                    view.transaction.discarded_at.unwrap_or(date_time::now()),
                );

            statuses.contains(&api_transaction_filter_status)
        });
    }

    // Collect unique user_ids from all transactions for batch lookup
    let user_ids: HashSet<String> = grouped_views
        .iter()
        .filter_map(|view| view.transaction.modified_by.clone())
        .collect();

    // Batch fetch all users in a single call
    let users_map = state
        .user_client
        .list_users_by_ids(auth.merchant_id.clone(), auth.profile_id, user_ids)
        .await
        .change_context(TransactionError::InternalServerError)?;

    let response = grouped_views
        .into_iter()
        .map(|grouped_view| {
            let modified_by = grouped_view
                .transaction
                .modified_by
                .as_ref()
                .map(|user_id| {
                    users_map
                        .get(user_id)
                        .map(|user| User::foreign_from(user.clone()))
                        .ok_or(TransactionError::InternalServerError)
                })
                .transpose()?;
            grouped_view.to_response(modified_by)
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ApplicationResponse::json(response))
}

pub async fn list_transactions_v2(
    state: &AppState,
    auth: AuthContext,
    request: ListTransactionsV2Request,
) -> TransactionResponse<TransactionListV2Page> {
    let ListTransactionsV2Request {
        filters,
        cursor_payload,
    } = request;

    filters
        .validate()
        .map_err(|err| TransactionError::switch_from(&err))?;

    let rule_missing_threshold_map = if filters.contains_exactly_one_of_missing_or_expected() {
        let rules = state
            .store
            .list_recon_rules_by_profile_id(&auth.profile_id, None)
            .await
            .change_context(TransactionError::InternalServerError)?;

        rules
            .into_iter()
            .filter(|rule| {
                filters
                    .rule_id
                    .clone()
                    .is_none_or(|rule_id| rule_id == rule.id)
            })
            .map(|rule| (rule.id, rule.aging_config.missing_before(date_time::now())))
            .collect()
    } else {
        HashMap::default()
    };
    let transaction_filters = filters.foreign_into();

    let pagination_query = cursor_payload
        .clone()
        .into_query(TXN_LIST_DEFAULT_LIMIT, TXN_LIST_MAX_LIMIT);

    let page = state
        .store
        .list_transactions_v2(
            &auth.profile_id,
            transaction_filters,
            rule_missing_threshold_map,
            pagination_query,
        )
        .await
        .change_context(TransactionError::InternalServerError)?;

    let encryptor = state
        .encryption_client
        .get_encryptor_for_merchant(auth.merchant_id.clone());

    let ordered_grouped_views = hydrate_transaction_rule_views(
        state,
        &encryptor,
        &auth.profile_id,
        page.views,
        TXN_LIST_MAX_ENTRIES_PER_TRANSACTION,
    )
    .await?;

    let (next_cursor, prev_cursor) =
        cursor_payload.build_cursors(&ordered_grouped_views, page.has_more);

    let user_ids: HashSet<String> = ordered_grouped_views
        .iter()
        .filter_map(|view| view.transaction.modified_by.clone())
        .collect();

    let users_map = state
        .user_client
        .list_users_by_ids(auth.merchant_id.clone(), auth.profile_id, user_ids)
        .await
        .change_context(TransactionError::InternalServerError)?;

    let items = ordered_grouped_views
        .into_iter()
        .map(|grouped_view| {
            let modified_by = grouped_view
                .transaction
                .modified_by
                .as_ref()
                .map(|user_id| {
                    users_map
                        .get(user_id)
                        .map(|user| User::foreign_from(user.clone()))
                        .ok_or(TransactionError::InternalServerError)
                })
                .transpose()?;
            grouped_view.to_response(modified_by)
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ApplicationResponse::json(TransactionListV2Page {
        items,
        next_cursor,
        prev_cursor,
    }))
}

pub async fn void_transaction(
    state: &AppState,
    id: String,
    request: VoidTransactionRequest,
    auth_context: AuthContext,
) -> TransactionResponse<GetTransactionResponse> {
    let merchant_encryptor = state
        .encryption_client
        .get_encryptor_for_merchant(auth_context.merchant_id.clone());

    // Fetch the existing transaction to be voided by ID and profile
    let existing_transaction = state
        .store
        .find_by_id_and_profile(&id, &auth_context.profile_id)
        .await
        .to_not_found_response(|| TransactionError::TransactionNotFound { id })?;

    let recon_rule = state
        .store
        .find_recon_rule_by_profile_id_and_rule_id(
            &auth_context.profile_id,
            &existing_transaction.rule_id,
        )
        .await
        .change_context(TransactionError::ReconRuleNotFound {
            rule_id: existing_transaction.rule_id.clone(),
        })?;

    let voided_transaction = void_transaction_core(
        state,
        existing_transaction,
        request.reason,
        Some(auth_context.user_id),
    )
    .await?
    .ok_or(TransactionError::InvalidTransactionStatus {
        expected_statuses: DieselTransactionStatus::iter()
            .filter(|status| status.is_voidable())
            .collect::<Vec<_>>(),
    })?;

    let voided_domain_transaction: DomainTransaction = voided_transaction
        .convert_back(&merchant_encryptor)
        .await
        .change_context(TransactionError::InternalServerError)?;

    let transaction_modified_by = voided_domain_transaction
        .get_modified_by(state, auth_context.merchant_id, auth_context.profile_id)
        .await?;

    let response =
        voided_domain_transaction.to_response(&recon_rule.aging_config, transaction_modified_by)?;
    Ok(ApplicationResponse::json(response))
}

pub async fn void_transaction_core(
    state: &AppState,
    existing_transaction: Transaction,
    void_reason: Option<String>,
    modified_by: Option<String>,
) -> TransactionResult<Option<Transaction>> {
    // Validate that transaction can be voided
    if !existing_transaction.status.is_voidable() {
        return Ok(None);
    }

    // Execute all operations atomically in a database transaction
    let conn = state
        .store
        .get_connection()
        .await
        .change_context(TransactionError::InternalServerError)?;

    let void_transaction = conn
        .transaction_async(|conn| async move {
            // Step 1: Archive old data using the centralized function
            let (
                _archived_transaction,
                old_entries,
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

            // Step 2: Create new void entries
            let new_version = existing_transaction.version + 1;
            let mut new_entries = Vec::new();
            for entry in &old_entries {
                let void_entry_new = EntryNew::new(
                    entry.account_id.clone(),
                    existing_transaction.transaction_id.clone(),
                    entry.balance_direction,
                    entry.amount,
                    entry.currency,
                    EntryStatus::Void,
                    entry.effective_at,
                    entry.merchant_id.clone(),
                    entry.metadata.clone(),
                    new_version,
                    EntryData::V2(EntryDataV2::Void),
                    entry.primary_staging_entry_id.clone(), // Preserve from original entry
                    entry.transformation_config_id.clone(), // Preserve from original entry
                    entry.entry_identifier.clone(),
                    entry.exposed_metadata.clone(),
                );
                new_entries.push(void_entry_new);
            }

            // Batch insert new entries
            let inserted_entries = EntryNew::batch_insert_chunked(&conn, new_entries)
                .await
                .map_err(|e| {
                    logging::error!("Failed to batch insert new entries: {:?}", e);
                    *e.current_context()
                })?;

            // Create the void transaction
            let update = TransactionVersionUpdate {
                new_entries: inserted_entries,
                data: TransactionData::V3(TransactionDataV3::Void(VoidData::V1(VoidDataV1 {
                    reason: void_reason,
                }))),
            };
            let void_transaction_new =
                TransactionNew::new_versioned(existing_transaction, update, modified_by);

            // Insert the void transaction
            let db_transaction = void_transaction_new.insert(&conn).await.map_err(|e| {
                logging::error!("Failed to insert void transaction: {:?}", e);
                *e.current_context()
            })?;

            Result::<_, DatabaseError>::Ok(db_transaction)
        })
        .await
        .change_context(TransactionError::InternalServerError)?;

    Ok(Some(void_transaction))
}

pub async fn manual_post_transaction_core(
    state: &AppState,
    existing_transaction: Transaction,
    reason: Option<String>,
    modified_by: Option<String>,
) -> TransactionResult<Option<Transaction>> {
    // Validate that transaction can be manually posted
    if !existing_transaction.status.is_manually_postable() {
        return Ok(None);
    }

    // Execute all operations atomically in a database transaction
    let conn = state
        .store
        .get_connection()
        .await
        .change_context(TransactionError::InternalServerError)?;

    let posted_transaction = conn
        .transaction_async(|conn| async move {
            // Step 1: Archive old data using the centralized function
            let (
                _archived_transaction,
                old_entries,
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

            // Step 2: Create new posted entries
            let new_version = existing_transaction.version + 1;
            let mut new_entries = Vec::new();
            for entry in &old_entries {
                let posted_entry_new = EntryNew::new(
                    entry.account_id.clone(),
                    existing_transaction.transaction_id.clone(),
                    entry.balance_direction,
                    entry.amount,
                    entry.currency,
                    EntryStatus::Posted,
                    entry.effective_at,
                    entry.merchant_id.clone(),
                    entry.metadata.clone(),
                    new_version,
                    EntryData::V2(EntryDataV2::Posted),
                    entry.primary_staging_entry_id.clone(),
                    entry.transformation_config_id.clone(),
                    entry.entry_identifier.clone(),
                    entry.exposed_metadata.clone(),
                );
                new_entries.push(posted_entry_new);
            }

            // Batch insert new entries
            let inserted_entries = EntryNew::batch_insert_chunked(&conn, new_entries)
                .await
                .map_err(|e| {
                    logging::error!("Failed to batch insert posted entries: {:?}", e);
                    *e.current_context()
                })?;

            // Step 3: Create the posted transaction
            let update = TransactionVersionUpdate {
                new_entries: inserted_entries,
                data: TransactionData::V3(TransactionDataV3::Posted(PostedData::V1(
                    PostedDataV1::Manual(ManualPostedData::V1(ManualPostedDataV1 { reason })),
                ))),
            };
            let posted_transaction_new =
                TransactionNew::new_versioned(existing_transaction, update, modified_by);

            // Insert the posted transaction
            let db_transaction = posted_transaction_new.insert(&conn).await.map_err(|e| {
                logging::error!("Failed to insert posted transaction: {:?}", e);
                *e.current_context()
            })?;

            Result::<_, DatabaseError>::Ok(db_transaction)
        })
        .await
        .change_context(TransactionError::InternalServerError)?;

    Ok(Some(posted_transaction))
}
