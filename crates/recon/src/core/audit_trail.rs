use api_models::{audit_trail::AuditTrailFilters, AccountData};
use common_utils::auth::AuthContext;
use error_stack::{report, ResultExt};
use std::collections::{HashMap, HashSet};
use storage_impl::account::AccountInterface;

use crate::{
    errors::audit_trail::{AuditTrailError, AuditTrailResponse},
    routes::AppState,
    types::ApplicationResponse,
    utils::audit_trail::{self, AuditEventExt},
};

pub async fn get_audit_trail(
    state: &AppState,
    auth_context: AuthContext,
    filters: AuditTrailFilters,
) -> AuditTrailResponse<Vec<api_models::audit_trail::AuditEvent>> {
    // Validate filters
    filters
        .validate()
        .map_err(|_| report!(AuditTrailError::InvalidTimeRange))?;

    // Fetch all event occurrences from domain
    let ingestion_events =
        audit_trail::get_ingestion_events(state, &auth_context.profile_id, &filters.time_range)
            .await?;
    let staging_events =
        audit_trail::get_staging_entry_events(state, &auth_context.profile_id, &filters.time_range)
            .await?;
    let transaction_events =
        audit_trail::get_transaction_events(state, &auth_context, &filters.time_range).await?;

    // Collect all events into a single vec
    let mut all_occurrences = ingestion_events
        .into_iter()
        .chain(staging_events)
        .chain(transaction_events)
        .collect::<Vec<_>>();

    // Collect unique account_ids from all events
    let unique_account_ids: Vec<String> = all_occurrences
        .iter()
        .flat_map(|occurrence| occurrence.account_ids())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    // Fetch accounts for these account_ids
    let accounts = state
        .store
        .list_accounts_by_profile_id_and_account_ids(&auth_context.profile_id, &unique_account_ids)
        .await
        .change_context(AuditTrailError::InternalServerError)?;

    // Create account_id -> AccountData mapping
    let account_map: HashMap<String, AccountData> = accounts
        .into_iter()
        .map(|account| {
            (
                account.id.clone(),
                AccountData {
                    account_id: account.id,
                    account_name: account.name,
                },
            )
        })
        .collect();

    // Sort all occurrences by timestamp
    all_occurrences.sort_by_key(|occurrence| *occurrence.timestamp());

    // Group occurrences into final events
    let grouped_events = audit_trail::group_occurrences_into_grouped_events(all_occurrences);

    let mut api_response = grouped_events
        .into_iter()
        .map(|grouped| grouped.to_api_response(&account_map))
        .collect::<Result<Vec<_>, _>>()?;

    api_response.sort_by_key(|event| *event.timestamp());

    Ok(ApplicationResponse::json(api_response))
}
