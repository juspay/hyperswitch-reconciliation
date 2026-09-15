use api_models::search_expectation::SearchExpectationResponse;
use common_utils::auth::AuthContext;
use error_stack::ResultExt;
use futures::future;
use recon_domain_models::{
    behaviour::ReverseConversion, search_expectation::DomainSearchExpectation,
};

use crate::{
    db::{search_expectation::SearchExpectationInterface, transaction::TransactionInterface},
    errors::search_expectation::{SearchExpectationError, SearchExpectationResult},
    routes::AppState,
    types::ApplicationResponse,
};

pub async fn list_search_expectations_by_primary_transaction_id(
    state: AppState,
    primary_transaction_id: &str,
    auth_context: AuthContext,
) -> SearchExpectationResult<Vec<SearchExpectationResponse>> {
    state
        .store
        .find_by_id_and_profile(primary_transaction_id, &auth_context.profile_id)
        .await
        .change_context(SearchExpectationError::TransactionNotFound)?;

    let search_expectations = state
        .store
        .list_expected_expectations_by_primary_transaction_id(primary_transaction_id)
        .await
        .change_context(SearchExpectationError::InternalServerError)?;

    let domain_search_expectations: Vec<DomainSearchExpectation> =
        future::try_join_all(search_expectations.into_iter().map(|search_expectation| {
            let merchant_encryptor = state
                .encryption_client
                .get_encryptor_for_merchant(auth_context.merchant_id.clone());
            async move {
                search_expectation
                    .convert_back(&merchant_encryptor)
                    .await
                    .change_context(SearchExpectationError::InternalServerError)
            }
        }))
        .await?;

    Ok(ApplicationResponse::json(
        domain_search_expectations
            .into_iter()
            .map(|search_expectation| search_expectation.to_api_response())
            .collect(),
    ))
}
