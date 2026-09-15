use api_models::entry::{
    ApiEntryListFilters, EntryCreateRequest, EntryListPage, GetEntryResponse, ListEntriesRequest,
};
use common_enums::EntryStatus;
use common_types::amount::{AmountResponse, AmountType};
use common_types::entry::{EntryData, EntryDataV2, EntryListFilters, ListEntriesFilters};
use common_utils::auth::AuthContext;
use common_utils::errors::ErrorSwitchFrom;
use diesel_models::entry::EntryNew;
use error_stack::ResultExt;
use futures::future;
use masking::ExposeInterface;
use recon_domain_models::behaviour::ReverseConversion;
use recon_domain_models::entry::DomainEntry;

use crate::utils::entry::EntryDataExt;
use crate::{
    db::{account::AccountInterface, entry::EntryInterface, transaction::TransactionInterface},
    domain::entry::{ENTRY_LIST_DEFAULT_LIMIT, ENTRY_LIST_MAX_LIMIT},
    errors::{
        conversions::StorageErrorExt,
        entry::{EntryError, EntryResponse, EntryResult},
    },
    routes::AppState,
    types::ApplicationResponse,
};

pub async fn create_entry(
    state: AppState,
    request: EntryCreateRequest,
    auth_context: AuthContext,
) -> EntryResponse<GetEntryResponse> {
    // Validate that the account belongs to the authenticated user's profile
    state
        .store
        .find_account_by_profile_id_and_account_id(&auth_context.profile_id, &request.account_id)
        .await
        .to_not_found_response(|| EntryError::AccountNotFound {
            account_id: request.account_id.clone(),
        })?;

    // Validate amount is positive
    if request.amount.is_negative() {
        return Err(EntryError::InvalidAmount {
            details: "Amount cannot be negative".to_string(),
        }
        .into());
    }

    // Convert FloatMajorUnit + currency to AmountType for validation and conversion to MinorUnit
    let amount_type = AmountType::from_major_unit(request.amount, request.currency)
        .change_context(EntryError::InvalidAmount {
            details: "Invalid amount provided".to_string(),
        })?;

    // Extract MinorUnit for database storage
    let amount = amount_type.value();
    let currency = request.currency;

    let merchant_encryptor = state
        .encryption_client
        .get_encryptor_for_merchant(auth_context.merchant_id.clone());

    // Encrypt metadata if present
    let encrypted_metadata = merchant_encryptor
        .encrypt_json_value(request.metadata.clone().expose())
        .await
        .change_context(EntryError::InternalServerError)?;

    let entry_new = EntryNew::new(
        request.account_id,
        request.transaction_id,
        request.entry_type,
        amount,
        currency,
        EntryStatus::Pending,
        request.effective_at,
        auth_context.merchant_id.clone(),
        encrypted_metadata.into(),
        1,
        EntryData::V2(EntryDataV2::Pending),
        None, // No staging_entry_id for manual entries
        None, // No transformation_id for manual entries
        request.order_id,
        serde_json::json!({}),
    );

    let entry = state
        .store
        .insert_entry(entry_new)
        .await
        .change_context(EntryError::InternalServerError)?;

    // Convert back to domain model for response (with decryption)
    let domain_entry: DomainEntry = entry
        .convert_back(&merchant_encryptor)
        .await
        .change_context(EntryError::InternalServerError)?;

    let response = GetEntryResponse {
        entry_id: domain_entry.entry_id,
        account_id: domain_entry.account_id,
        transaction_id: domain_entry.transaction_id,
        entry_type: domain_entry.entry_type,
        amount: AmountResponse::from_minor_unit(domain_entry.amount, domain_entry.currency)
            .change_context(EntryError::InternalServerError)?,
        status: domain_entry.status,
        effective_at: domain_entry.effective_at,
        metadata: domain_entry.metadata,
        exposed_metadata: domain_entry.exposed_metadata,
        discarded_at: domain_entry.discarded_at,
        created_at: domain_entry.created_at,
        discarded_status: domain_entry.discarded_status,
        version: domain_entry.version,
        data: domain_entry.data.to_api_response()?,
        discarded_data: domain_entry.discarded_data.to_api_response()?,
        staging_entry_id: domain_entry.staging_entry_id,
        transformation_id: domain_entry.transformation_id,
        order_id: domain_entry.order_id,
    };

    Ok(ApplicationResponse::json_created(response))
}

pub async fn get_entry_by_id(
    state: AppState,
    entry_id: String,
    auth_context: AuthContext,
) -> EntryResponse<GetEntryResponse> {
    let entry = state
        .store
        .find_entry_by_id(&entry_id)
        .await
        .to_not_found_response(|| EntryError::EntryNotFound)?;

    // Validate that the account belongs to the authenticated user's profile
    state
        .store
        .find_account_by_profile_id_and_account_id(&auth_context.profile_id, &entry.account_id)
        .await
        .to_not_found_response(|| EntryError::EntryNotFound)?;

    let merchant_encryptor = state
        .encryption_client
        .get_encryptor_for_merchant(auth_context.merchant_id.clone());

    // Convert back to domain model for response (with decryption)
    let domain_entry: DomainEntry = entry
        .convert_back(&merchant_encryptor)
        .await
        .change_context(EntryError::InternalServerError)?;

    let response = GetEntryResponse {
        entry_id: domain_entry.entry_id,
        account_id: domain_entry.account_id,
        transaction_id: domain_entry.transaction_id,
        entry_type: domain_entry.entry_type,
        amount: AmountResponse::from_minor_unit(domain_entry.amount, domain_entry.currency)
            .change_context(EntryError::InternalServerError)?,
        status: domain_entry.status,
        effective_at: domain_entry.effective_at,
        metadata: domain_entry.metadata,
        exposed_metadata: domain_entry.exposed_metadata,
        discarded_at: domain_entry.discarded_at,
        created_at: domain_entry.created_at,
        discarded_status: domain_entry.discarded_status,
        version: domain_entry.version,
        data: domain_entry.data.to_api_response()?,
        discarded_data: domain_entry.discarded_data.to_api_response()?,
        staging_entry_id: domain_entry.staging_entry_id,
        transformation_id: domain_entry.transformation_id,
        order_id: domain_entry.order_id,
    };

    Ok(ApplicationResponse::json(response))
}

pub async fn list_entries_by_account_id(
    state: AppState,
    account_id: String,
    auth_context: AuthContext,
    filters: ListEntriesFilters,
) -> EntryResponse<Vec<GetEntryResponse>> {
    // Validate time range filters
    filters
        .validate()
        .map_err(|_| EntryError::TimeRangeValidationError)?;

    // Validate that the account belongs to the authenticated user's profile
    state
        .store
        .find_account_by_profile_id_and_account_id(&auth_context.profile_id, &account_id)
        .await
        .to_not_found_response(|| EntryError::AccountNotFound {
            account_id: account_id.clone(),
        })?;

    let entries = state
        .store
        .list_entries_by_account_id(&account_id, filters)
        .await
        .change_context(EntryError::InternalServerError)?;

    let response = entries_to_response(&state, &auth_context, entries).await?;

    Ok(ApplicationResponse::json(response))
}

pub async fn list_entries(
    state: AppState,
    request: ListEntriesRequest,
    auth_context: AuthContext,
) -> EntryResponse<EntryListPage> {
    let ListEntriesRequest {
        filters,
        cursor_payload,
    } = request;

    filters
        .validate()
        .map_err(|err| EntryError::switch_from(&err))?;

    let ApiEntryListFilters {
        primary_transaction_id,
        account_ids,
        entry_type,
        currency,
        status,
        time_range,
        staging_entry_ids,
        transformation_config_ids,
        order_ids,
    } = filters;

    let transaction = state
        .store
        .find_by_id_and_profile(&primary_transaction_id, &auth_context.profile_id)
        .await
        .to_not_found_response(|| EntryError::TransactionNotFound {
            id: primary_transaction_id.clone(),
        })?;

    let entry_filters = EntryListFilters {
        logical_transaction_id: transaction.transaction_id,
        version: transaction.version,
        account_ids,
        entry_type,
        currency,
        status,
        time_range,
        staging_entry_ids,
        transformation_config_ids,
        order_ids,
    };

    let pagination_query = cursor_payload
        .clone()
        .into_query(ENTRY_LIST_DEFAULT_LIMIT, ENTRY_LIST_MAX_LIMIT);

    let page = state
        .store
        .list_entries(entry_filters, pagination_query)
        .await
        .change_context(EntryError::InternalServerError)?;

    let (next_cursor, prev_cursor) = cursor_payload.build_cursors(&page.entries, page.has_more);
    let items = entries_to_response(&state, &auth_context, page.entries).await?;

    Ok(ApplicationResponse::json(EntryListPage {
        items,
        next_cursor,
        prev_cursor,
    }))
}

pub async fn list_entries_by_transaction_id(
    state: AppState,
    transaction_id: String,
    auth_context: AuthContext,
) -> EntryResponse<Vec<GetEntryResponse>> {
    let entries = state
        .store
        .list_entries_by_transaction_id_and_version(&transaction_id, None)
        .await
        .change_context(EntryError::InternalServerError)?;

    // TODO: Validate that all accounts in this transaction belong to the authenticated user's profile
    let response = entries_to_response(&state, &auth_context, entries).await?;

    Ok(ApplicationResponse::json(response))
}

async fn entries_to_response(
    state: &AppState,
    auth_context: &AuthContext,
    entries: Vec<diesel_models::entry::Entry>,
) -> EntryResult<Vec<GetEntryResponse>> {
    let merchant_encryptor = state
        .encryption_client
        .get_encryptor_for_merchant(auth_context.merchant_id.clone());

    future::try_join_all(entries.into_iter().map(|entry| async {
        let domain_entry: DomainEntry = entry
            .convert_back(&merchant_encryptor)
            .await
            .change_context(EntryError::InternalServerError)?;

        Ok::<GetEntryResponse, error_stack::Report<EntryError>>(GetEntryResponse {
            entry_id: domain_entry.entry_id,
            account_id: domain_entry.account_id,
            transaction_id: domain_entry.transaction_id,
            entry_type: domain_entry.entry_type,
            amount: AmountResponse::from_minor_unit(domain_entry.amount, domain_entry.currency)
                .change_context(EntryError::InternalServerError)?,
            status: domain_entry.status,
            effective_at: domain_entry.effective_at,
            metadata: domain_entry.metadata,
            exposed_metadata: domain_entry.exposed_metadata,
            discarded_at: domain_entry.discarded_at,
            created_at: domain_entry.created_at,
            discarded_status: domain_entry.discarded_status,
            version: domain_entry.version,
            data: domain_entry.data.to_api_response()?,
            discarded_data: domain_entry.discarded_data.to_api_response()?,
            staging_entry_id: domain_entry.staging_entry_id,
            transformation_id: domain_entry.transformation_id,
            order_id: domain_entry.order_id,
        })
    }))
    .await
}
