use std::collections::HashSet;

use api_models::{
    account::AccountData,
    staging_entry::{
        ApiListStagingEntriesFilters, GetStagingEntryResponse, ListStagingEntriesV2Request,
        StagingEntryCreateRequest, StagingEntryCreateRequestV2, StagingEntryListV2Page,
        StagingEntryUpdateRequest, VoidStagingEntryRequest,
    },
    transformation::ApiTransformationConfigData,
    user::User,
};
use async_bb8_diesel::AsyncConnection;
use common_types::{
    amount::{AmountResponse, AmountType},
    metadata_schema::{ExposedKeysProvider, ProcessingModeProvider},
    staging_entry::{
        status::DomainStagingEntryStatus, ListStagingEntriesFilters, StagingEntryData,
        StagingEntryDataV2, StagingEntryListFiltersV2, VoidData, VoidDataV1,
    },
};
use common_utils::{auth::AuthContext, errors::ErrorSwitchFrom, logging};
use diesel_models::{
    errors::DatabaseError,
    staging_entry::{
        StagingEntry as DieselStagingEntry, StagingEntryAccountView, StagingEntryFieldsUpdate,
        StagingEntryNew, StagingEntryUpdate, StagingEntryVersionUpdate,
    },
};
use error_stack::{report, ResultExt};
use external_services::user::interfaces::UserInterface;
use futures::future;
use masking::ExposeInterface;
use recon_domain_models::{behaviour::ReverseConversion, staging_entry::DomainStagingEntry};
use storage_impl::{
    account::AccountInterface, metadata_schema::MetadataSchemaInterface,
    transformation::TransformationConfigInterface,
};

use crate::{
    domain::{
        metadata_schema::{
            data_validator::RowValidator, types::RowValidationOutcome, ParsedRow, RowMapper,
        },
        staging_entry::{STAGING_ENTRY_LIST_DEFAULT_LIMIT, STAGING_ENTRY_LIST_MAX_LIMIT},
    },
    errors::{
        conversions::StorageErrorExt,
        staging_entry::{StagingEntryError, StagingEntryResponse, StagingEntryResult},
    },
    routes::AppState,
    types::{
        transformers::{ForeignFrom, ForeignInto},
        ApplicationResponse,
    },
    utils::{
        metadata_schema::project_metadata,
        staging_entry::{
            DomainStagingEntryStatusExt, StagingEntryDataExt, StagingEntryDataValidationExt,
            StagingEntryExt,
        },
    },
};

pub async fn create_staging_entry(
    state: AppState,
    request: StagingEntryCreateRequest,
    auth_context: AuthContext,
) -> StagingEntryResponse<GetStagingEntryResponse> {
    // Validate that the account belongs to the authenticated user's profile
    let account = state
        .store
        .find_account_by_profile_id_and_account_id(&auth_context.profile_id, &request.account_id)
        .await
        .to_not_found_response(|| StagingEntryError::AccountNotFound {
            account_id: request.account_id.clone(),
        })?;

    // Validate amount is positive
    if request.amount.is_negative() {
        return Err(StagingEntryError::InvalidAmount {
            details: "Amount cannot be negative".to_string(),
        }
        .into());
    }

    // Convert FloatMajorUnit + currency to AmountType for validation and conversion to MinorUnit
    let amount_type = AmountType::from_major_unit(request.amount, request.currency)
        .change_context(StagingEntryError::InvalidAmount {
            details: "Invalid amount provided".to_string(),
        })?;

    // Extract MinorUnit for database storage
    let amount = amount_type.value();
    let currency = request.currency;

    let raw_metadata = request.metadata.clone().expose();

    let transformation_config = state
        .store
        .find_transformation_config_by_id_and_profile_id(
            &request.transformation_id,
            &auth_context.profile_id,
        )
        .await
        .change_context(StagingEntryError::TransformationConfigNotFound {
            transformation_id: request.transformation_id.clone(),
        })?;

    if transformation_config.account_id != request.account_id {
        return Err(report!(
            StagingEntryError::TransformationConfigAccountMismatch {
                transformation_id: request.transformation_id,
                account_id: request.account_id
            }
        ));
    }

    let metadata_schema = state
        .store
        .find_metadata_schema_by_id_and_profile_id(
            &transformation_config.metadata_schema_id,
            &auth_context.profile_id,
        )
        .await
        .change_context(StagingEntryError::MetadataSchemaNotFound {
            metadata_schema_id: transformation_config.metadata_schema_id,
        })?;

    let exposed_keys = metadata_schema.schema_data.exposed_keys();

    let exposed_metadata = project_metadata(&raw_metadata, &exposed_keys.exposed);
    let searchable_metadata = project_metadata(&raw_metadata, &exposed_keys.searchable);

    let merchant_encryptor = state
        .encryption_client
        .get_encryptor_for_merchant(auth_context.merchant_id.clone());

    // Encrypt metadata if present
    let encrypted_metadata = merchant_encryptor
        .encrypt_json_value(raw_metadata)
        .await
        .change_context(StagingEntryError::InternalServerError)?;

    let staging_entry_new = StagingEntryNew::new(
        request.account_id,
        auth_context.merchant_id.clone(),
        request.entry_type,
        amount,
        currency,
        request.effective_at,
        encrypted_metadata.into(),
        request.processing_mode,
        auth_context.profile_id.clone(),
        request.transformation_id,
        request.transformation_history_id,
        request.order_id,
        Some(auth_context.user_id),
        exposed_metadata,
        searchable_metadata,
    );

    let staging_entry = state
        .store
        .insert_staging_entry(staging_entry_new)
        .await
        .change_context(StagingEntryError::InternalServerError)?;

    // Convert back to domain model for response (with decryption)
    let domain_staging_entry: DomainStagingEntry = staging_entry
        .convert_back(&merchant_encryptor)
        .await
        .change_context(StagingEntryError::InternalServerError)?;

    let staging_entry_modified_by = domain_staging_entry
        .get_modified_by(&state, auth_context.merchant_id, auth_context.profile_id)
        .await?;

    let response = GetStagingEntryResponse {
        id: domain_staging_entry.id,
        staging_entry_id: domain_staging_entry.staging_entry_id,
        account: AccountData {
            account_id: account.id,
            account_name: account.name,
        },
        entry_type: domain_staging_entry.entry_type,
        amount: AmountResponse::from_minor_unit(
            domain_staging_entry.amount,
            domain_staging_entry.currency,
        )
        .change_context(StagingEntryError::InternalServerError)?,
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
            transformation_config_name: transformation_config.name,
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
    };

    Ok(ApplicationResponse::json_created(response))
}

pub async fn get_staging_entry_by_id(
    state: AppState,
    id: String,
    auth_context: AuthContext,
) -> StagingEntryResponse<GetStagingEntryResponse> {
    let staging_entry = state
        .store
        .find_staging_entry_by_id_and_profile(id, auth_context.profile_id.clone())
        .await
        .to_not_found_response(|| StagingEntryError::StagingEntryNotFound)?;

    // Get encryptor for decryption
    let merchant_encryptor = state
        .encryption_client
        .get_encryptor_for_merchant(auth_context.merchant_id.clone());

    // Convert to domain model (with decryption)
    let domain_staging_entry: DomainStagingEntry = staging_entry
        .convert_back(&merchant_encryptor)
        .await
        .change_context(StagingEntryError::InternalServerError)?;

    // Fetch account details
    let account = state
        .store
        .find_account_by_profile_id_and_account_id(
            &auth_context.profile_id,
            &domain_staging_entry.account_id,
        )
        .await
        .change_context(StagingEntryError::InternalServerError)?;

    let staging_entry_modified_by = domain_staging_entry
        .get_modified_by(&state, auth_context.merchant_id, auth_context.profile_id)
        .await?;

    let transformation_config = state
        .store
        .find_transformation_config_by_id_and_profile_id(
            &domain_staging_entry.transformation_id,
            &domain_staging_entry.profile_id,
        )
        .await
        .change_context(StagingEntryError::InternalServerError)?;

    let response = GetStagingEntryResponse {
        id: domain_staging_entry.id,
        staging_entry_id: domain_staging_entry.staging_entry_id,
        account: AccountData {
            account_id: account.id,
            account_name: account.name,
        },
        entry_type: domain_staging_entry.entry_type,
        amount: AmountResponse::from_minor_unit(
            domain_staging_entry.amount,
            domain_staging_entry.currency,
        )
        .change_context(StagingEntryError::InternalServerError)?,
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
            transformation_config_name: transformation_config.name,
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
    };

    Ok(ApplicationResponse::json(response))
}

pub async fn list_staging_entries_by_profile_id(
    state: AppState,
    filters: ApiListStagingEntriesFilters,
    auth_context: AuthContext,
) -> StagingEntryResponse<Vec<GetStagingEntryResponse>> {
    // Validate time range filters
    filters
        .validate()
        .map_err(|_| StagingEntryError::TimeRangeValidationError)?;

    let filters = ListStagingEntriesFilters::foreign_from(filters);

    let staging_entries = state
        .store
        .list_staging_entries_by_profile_id(&auth_context.profile_id, filters, None)
        .await
        .change_context(StagingEntryError::InternalServerError)?;

    let response = staging_entry_views_to_response(&state, &auth_context, staging_entries).await?;

    Ok(ApplicationResponse::json(response))
}

pub async fn list_staging_entries_v2(
    state: AppState,
    request: ListStagingEntriesV2Request,
    auth_context: AuthContext,
) -> StagingEntryResponse<StagingEntryListV2Page> {
    let ListStagingEntriesV2Request {
        filters,
        cursor_payload,
    } = request;

    filters
        .validate()
        .map_err(|err| StagingEntryError::switch_from(&err))?;

    let staging_entry_filters: StagingEntryListFiltersV2 = filters.foreign_into();
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
        .change_context(StagingEntryError::InternalServerError)?;

    let (next_cursor, prev_cursor) = cursor_payload.build_cursors(&page.views, page.has_more);
    let items = staging_entry_views_to_response(&state, &auth_context, page.views).await?;

    Ok(ApplicationResponse::json(StagingEntryListV2Page {
        items,
        next_cursor,
        prev_cursor,
    }))
}

pub(super) async fn staging_entry_views_to_response(
    state: &AppState,
    auth_context: &AuthContext,
    staging_entries: Vec<StagingEntryAccountView>,
) -> StagingEntryResult<Vec<GetStagingEntryResponse>> {
    // Collect unique user IDs from modified_by fields
    let user_ids: HashSet<String> = staging_entries
        .iter()
        .filter_map(|view| view.staging_entry.modified_by.clone())
        .collect();

    // Batch fetch all users in a single call
    let users_map = state
        .user_client
        .list_users_by_ids(
            auth_context.merchant_id.clone(),
            auth_context.profile_id.clone(),
            user_ids,
        )
        .await
        .change_context(StagingEntryError::InternalServerError)?;

    // Get encryptor for decryption
    let merchant_encryptor = state
        .encryption_client
        .get_encryptor_for_merchant(auth_context.merchant_id.clone());

    // Convert domain models to response models concurrently
    let response: Vec<GetStagingEntryResponse> =
        future::try_join_all(staging_entries.into_iter().map(|staging_entry_view| async {
            let domain_staging_entry: DomainStagingEntry = staging_entry_view
                .staging_entry
                .convert_back(&merchant_encryptor)
                .await
                .change_context(StagingEntryError::InternalServerError)?;

            let modified_by = domain_staging_entry
                .modified_by
                .as_ref()
                .map(|user_id| {
                    users_map
                        .get(user_id)
                        .map(|user| User::foreign_from(user.clone()))
                        .ok_or(StagingEntryError::InternalServerError)
                })
                .transpose()?;

            Ok::<GetStagingEntryResponse, error_stack::Report<StagingEntryError>>(
                GetStagingEntryResponse {
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
                    .change_context(StagingEntryError::InternalServerError)?,
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
                    modified_by,
                },
            )
        }))
        .await?;

    Ok(response)
}

pub async fn void_staging_entry(
    state: AppState,
    id: String,
    request: VoidStagingEntryRequest,
    auth_context: AuthContext,
) -> StagingEntryResponse<GetStagingEntryResponse> {
    // First, get the existing staging entry to verify ownership
    let existing_staging_entry = state
        .store
        .find_staging_entry_by_id_and_profile(id.clone(), auth_context.profile_id.clone())
        .await
        .to_not_found_response(|| StagingEntryError::StagingEntryNotFound)?;

    // Validate that the staging entry can be voided
    if !DomainStagingEntryStatus::from(existing_staging_entry.status).is_voidable() {
        return Err(StagingEntryError::StagingEntryNotVoidable {
            id: existing_staging_entry.id.clone(),
        }
        .into());
    }

    // Call the core void function
    let void_staging_entry = void_staging_entry_core(
        &state,
        existing_staging_entry.clone(),
        request.reason,
        Some(auth_context.user_id),
    )
    .await?
    .ok_or(StagingEntryError::StagingEntryNotVoidable {
        id: existing_staging_entry.id.clone(),
    })?;

    // Get encryptor for decryption
    let merchant_encryptor = state
        .encryption_client
        .get_encryptor_for_merchant(auth_context.merchant_id.clone());

    // Convert to domain model (with decryption)
    let domain_staging_entry: DomainStagingEntry = void_staging_entry
        .convert_back(&merchant_encryptor)
        .await
        .change_context(StagingEntryError::InternalServerError)?;

    // Fetch account details
    let account = state
        .store
        .find_account_by_profile_id_and_account_id(
            &auth_context.profile_id,
            &domain_staging_entry.account_id,
        )
        .await
        .change_context(StagingEntryError::InternalServerError)?;

    let staging_entry_modified_by = domain_staging_entry
        .get_modified_by(&state, auth_context.merchant_id, auth_context.profile_id)
        .await?;

    let transformation_config = state
        .store
        .find_transformation_config_by_id_and_profile_id(
            &domain_staging_entry.transformation_id,
            &domain_staging_entry.profile_id,
        )
        .await
        .change_context(StagingEntryError::InternalServerError)?;

    let response = GetStagingEntryResponse {
        id: domain_staging_entry.id,
        staging_entry_id: domain_staging_entry.staging_entry_id,
        account: AccountData {
            account_id: account.id,
            account_name: account.name,
        },
        entry_type: domain_staging_entry.entry_type,
        amount: AmountResponse::from_minor_unit(
            domain_staging_entry.amount,
            domain_staging_entry.currency,
        )
        .change_context(StagingEntryError::InternalServerError)?,
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
            transformation_config_name: transformation_config.name,
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
    };

    Ok(ApplicationResponse::json(response))
}

pub async fn void_staging_entry_core(
    state: &AppState,
    existing_staging_entry: DieselStagingEntry,
    reason: Option<String>,
    modified_by: Option<String>,
) -> StagingEntryResult<Option<DieselStagingEntry>> {
    // Validate that the staging entry can be voided
    if !DomainStagingEntryStatus::from(existing_staging_entry.status).is_voidable() {
        return Ok(None);
    }

    // Execute versioning logic atomically
    let conn = state
        .store
        .get_connection()
        .await
        .change_context(StagingEntryError::InternalServerError)?;

    let void_staging_entry = conn
        .transaction_async(|conn| async move {
            // Step 1: Archive the existing staging entry
            let archive_update = StagingEntryUpdate::DiscardUpdate {
                original_data: existing_staging_entry.data.clone(),
            };

            DieselStagingEntry::update_by_id(
                &conn,
                &existing_staging_entry.id,
                archive_update.into(),
            )
            .await
            .map_err(|e| {
                logging::error!("Failed to archive old staging entry: {:?}", e);
                *e.current_context()
            })?;

            // Step 2: Create new void staging entry
            let void_data =
                StagingEntryData::V2(StagingEntryDataV2::Void(VoidData::V1(VoidDataV1 {
                    reason,
                })));
            let void_staging_entry_new = StagingEntryNew::new_versioned(
                existing_staging_entry.clone(),
                StagingEntryVersionUpdate::VoidUpdate { data: void_data },
                modified_by,
            );

            // Insert the void staging entry
            let db_staging_entry = void_staging_entry_new.insert(&conn).await.map_err(|e| {
                logging::error!("Failed to insert void staging entry: {:?}", e);
                *e.current_context()
            })?;

            Result::<_, DatabaseError>::Ok(db_staging_entry)
        })
        .await
        .change_context(StagingEntryError::InternalServerError)?;

    Ok(Some(void_staging_entry))
}

pub async fn update_staging_entry(
    state: AppState,
    staging_entry_id: String,
    request: StagingEntryUpdateRequest,
    auth_context: AuthContext,
) -> StagingEntryResponse<GetStagingEntryResponse> {
    // 1. Amount must be positive
    if request.amount.is_negative() {
        return Err(StagingEntryError::InvalidAmount {
            details: "Amount cannot be negative".to_string(),
        }
        .into());
    }

    // 2. Data Processing: Convert amount
    let amount_type = AmountType::from_major_unit(request.amount, request.currency)
        .change_context(StagingEntryError::InvalidAmount {
            details: "Invalid amount provided".to_string(),
        })?;
    let amount = amount_type.value();

    // 3. Authorization: Find existing staging entry by id & profile
    let existing_staging_entry = state
        .store
        .find_staging_entry_by_id_and_profile(
            staging_entry_id.clone(),
            auth_context.profile_id.clone(),
        )
        .await
        .to_not_found_response(|| StagingEntryError::StagingEntryNotFound)?;

    // 4. Validate that editing is allowed
    existing_staging_entry.data.validate_editing_allowed()?;

    // 5. Validate new account belongs to same profile
    let new_account = state
        .store
        .find_account_by_profile_id_and_account_id(&auth_context.profile_id, &request.account_id)
        .await
        .to_not_found_response(|| StagingEntryError::AccountNotFound {
            account_id: request.account_id.clone(),
        })?;

    // 6. Validate that the new transformation config belongs to the profile
    let transformation_config = state
        .store
        .find_transformation_config_by_id_and_profile_id(
            &request.transformation_id,
            &auth_context.profile_id,
        )
        .await
        .to_not_found_response(|| StagingEntryError::TransformationConfigNotFound {
            transformation_id: request.transformation_id.clone(),
        })?;

    // 7. Validate transformation belongs to the account
    if transformation_config.account_id != request.account_id {
        return Err(StagingEntryError::TransformationConfigAccountMismatch {
            transformation_id: request.transformation_id.clone(),
            account_id: request.account_id.clone(),
        }
        .into());
    }

    // 8. Encrypt new metadata
    let merchant_encryptor = state
        .encryption_client
        .get_encryptor_for_merchant(auth_context.merchant_id.clone());
    let encrypted_metadata = merchant_encryptor
        .encrypt_json_value(request.metadata.clone().expose())
        .await
        .change_context(StagingEntryError::InternalServerError)?;

    // 8b. Derive exposed/searchable subsets from the schema linked to this transformation config
    let schema = state
        .store
        .find_metadata_schema_by_id_and_profile_id(
            &transformation_config.metadata_schema_id,
            &auth_context.profile_id,
        )
        .await
        .to_not_found_response(|| StagingEntryError::MetadataSchemaNotFound {
            metadata_schema_id: transformation_config.metadata_schema_id.clone(),
        })?;
    let exposed_keys = schema.schema_data.exposed_keys();
    let raw_metadata = request.metadata.clone().expose();
    let exposed_metadata = project_metadata(&raw_metadata, &exposed_keys.exposed);
    let searchable_metadata = project_metadata(&raw_metadata, &exposed_keys.searchable);

    // 9. Atomic Transaction: Archive old + Create new version
    let conn = state
        .store
        .get_connection()
        .await
        .change_context(StagingEntryError::InternalServerError)?;

    let updated_staging_entry = conn
        .transaction_async(|txn_conn| async move {
            // Archive current version
            let archive_update = StagingEntryUpdate::DiscardUpdate {
                original_data: existing_staging_entry.data.clone(),
            };
            DieselStagingEntry::update_by_id(
                &txn_conn,
                &existing_staging_entry.id,
                archive_update.into(),
            )
            .await
            .map_err(|e| {
                logging::error!("Failed to archive current staging entry: {:?}", e);
                *e.current_context()
            })?;

            // Create new version with updated fields
            let fields_update = StagingEntryFieldsUpdate {
                reason: request.reason.clone(),
                account_id: request.account_id.clone(),
                entry_type: request.entry_type,
                amount,
                currency: request.currency,
                effective_at: request.effective_at,
                metadata: encrypted_metadata.into(),
                transformation_id: request.transformation_id.clone(),
                order_id: request.order_id.clone(),
                exposed_metadata,
                searchable_metadata,
            };
            let new_staging_entry = StagingEntryNew::new_updated(
                existing_staging_entry,
                fields_update,
                Some(auth_context.user_id),
            );
            let updated_staging_entry = new_staging_entry.insert(&txn_conn).await.map_err(|e| {
                logging::error!("Failed to insert updated staging entry: {:?}", e);
                *e.current_context()
            })?;

            Result::<_, DatabaseError>::Ok(updated_staging_entry)
        })
        .await
        .change_context(StagingEntryError::InternalServerError)?;

    // 10. Convert to domain model and construct response
    let domain_staging_entry: DomainStagingEntry = updated_staging_entry
        .convert_back(&merchant_encryptor)
        .await
        .change_context(StagingEntryError::InternalServerError)?;

    let staging_entry_modified_by = domain_staging_entry
        .get_modified_by(&state, auth_context.merchant_id, auth_context.profile_id)
        .await?;

    let response = GetStagingEntryResponse {
        id: domain_staging_entry.id,
        staging_entry_id: domain_staging_entry.staging_entry_id,
        account: AccountData {
            account_id: new_account.id,
            account_name: new_account.name,
        },
        entry_type: domain_staging_entry.entry_type,
        amount: AmountResponse::from_minor_unit(
            domain_staging_entry.amount,
            domain_staging_entry.currency,
        )
        .change_context(StagingEntryError::InternalServerError)?,
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
            transformation_config_name: transformation_config.name,
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
    };

    Ok(ApplicationResponse::json(response))
}

/// Create staging entry with metadata schema validation (V2)
pub async fn create_staging_entry_with_validation(
    state: AppState,
    request: StagingEntryCreateRequestV2,
    auth_context: AuthContext,
) -> StagingEntryResponse<GetStagingEntryResponse> {
    // 1. Validate that the account belongs to the authenticated user's profile
    let account = state
        .store
        .find_account_by_profile_id_and_account_id(&auth_context.profile_id, &request.account_id)
        .await
        .to_not_found_response(|| StagingEntryError::AccountNotFound {
            account_id: request.account_id.clone(),
        })?;

    // 2. Fetch metadata schema by ID and validate it belongs to the profile
    let schema = state
        .store
        .find_metadata_schema_by_id_and_profile_id(
            &request.metadata_schema_id,
            &auth_context.profile_id,
        )
        .await
        .to_not_found_response(|| StagingEntryError::MetadataSchemaNotFound {
            metadata_schema_id: request.metadata_schema_id.clone(),
        })?;

    let parsed_row = ParsedRow::from_api_fields(request.raw_data, None);
    let mapped_row = schema.schema_data.map_row(parsed_row);
    let validated_entry = match schema.schema_data.validate_row(&mapped_row) {
        RowValidationOutcome::Validated(validated) => validated,
        RowValidationOutcome::Rejected(rejection) => {
            Err(StagingEntryError::MetadataSchemaValidationFailed {
                details: rejection
                    .field_errors
                    .iter()
                    .map(|located| {
                        format!(
                            "{}: {}",
                            located.value.get_identifier(),
                            located.value.get_error()
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", "),
            })?
        }
    };

    // 7. Get merchant encryptor for metadata encryption
    let merchant_encryptor = state
        .encryption_client
        .get_encryptor_for_merchant(auth_context.merchant_id.clone());

    // 8. Encrypt validated metadata
    let encrypted_metadata = merchant_encryptor
        .encrypt_json_value(validated_entry.row_data.metadata)
        .await
        .change_context(StagingEntryError::InternalServerError)?;

    // 9. Create staging entry with validated data
    let staging_entry_new = StagingEntryNew::new(
        request.account_id,
        auth_context.merchant_id.clone(),
        validated_entry.row_data.balance_direction,
        validated_entry.row_data.amount,
        validated_entry.row_data.currency,
        validated_entry.row_data.effective_at,
        encrypted_metadata.into(),
        schema.schema_data.get_processing_mode(),
        auth_context.profile_id.clone(),
        request.transformation_id,
        request.transformation_history_id,
        validated_entry.row_data.order_id,
        Some(auth_context.user_id),
        validated_entry.row_data.exposed_metadata,
        validated_entry.row_data.searchable_metadata,
    );

    let staging_entry = state
        .store
        .insert_staging_entry(staging_entry_new)
        .await
        .change_context(StagingEntryError::InternalServerError)?;

    // 10. Convert back to domain model for response (with decryption)
    let domain_staging_entry: DomainStagingEntry = staging_entry
        .convert_back(&merchant_encryptor)
        .await
        .change_context(StagingEntryError::InternalServerError)?;

    let staging_entry_modified_by = domain_staging_entry
        .get_modified_by(&state, auth_context.merchant_id, auth_context.profile_id)
        .await?;

    let transformation_config = state
        .store
        .find_transformation_config_by_id_and_profile_id(
            &domain_staging_entry.transformation_id,
            &domain_staging_entry.profile_id,
        )
        .await
        .change_context(StagingEntryError::InternalServerError)?;

    let response = GetStagingEntryResponse {
        id: domain_staging_entry.id,
        staging_entry_id: domain_staging_entry.staging_entry_id,
        account: AccountData {
            account_id: account.id,
            account_name: account.name,
        },
        entry_type: domain_staging_entry.entry_type,
        amount: AmountResponse::from_minor_unit(
            domain_staging_entry.amount,
            domain_staging_entry.currency,
        )
        .change_context(StagingEntryError::InternalServerError)?,
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
            transformation_config_name: transformation_config.name,
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
    };

    Ok(ApplicationResponse::json_created(response))
}
