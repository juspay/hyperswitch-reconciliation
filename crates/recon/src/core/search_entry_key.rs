use api_models::search_entry_key::{SearchEntryKeyListRequest, SearchEntryKeyResponse};
use common_utils::auth::AuthContext;
use error_stack::ResultExt;
use storage_impl::search_entry_key::SearchEntryKeyInterface;

use crate::{
    errors::search_entry_key::{SearchEntryError, SearchEntryResult},
    routes::AppState,
    types::ApplicationResponse,
    utils::search_entry_key::SearchEntryKeyExt,
};

pub async fn list_search_entry_keys_by_entry_ids(
    state: &AppState,
    auth: AuthContext,
    request: SearchEntryKeyListRequest,
) -> SearchEntryResult<Vec<SearchEntryKeyResponse>> {
    let search_entry_keys = state
        .store
        .list_search_entry_keys_by_entry_ids_and_profile_id(
            request.entry_ids,
            &auth.profile_id,
            None,
        )
        .await
        .change_context(SearchEntryError::InternalServerError)?;

    let response = search_entry_keys
        .into_iter()
        .map(SearchEntryKeyExt::to_api_response)
        .collect();

    Ok(ApplicationResponse::json(response))
}
