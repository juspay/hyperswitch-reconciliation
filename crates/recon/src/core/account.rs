use api_models::account::{self as api, GetAccountResponse};
use common_types::units::MinorUnit;
use common_utils::{date_time, id_generation::generate_id_with_default_len};
use diesel_models::account::{AccountNew, AccountUpdate};
use error_stack::ResultExt;

use crate::{
    db::account::AccountInterface,
    errors::{
        account::{AccountError, AccountResponse},
        conversions::StorageErrorExt,
    },
    routes::AppState,
    types::ApplicationResponse,
    utils::account::AccountExt,
};

pub async fn list_accounts_by_profile_id(
    state: &AppState,
    auth_profile_id: &str,
) -> AccountResponse<Vec<GetAccountResponse>> {
    let accounts = state
        .store
        .list_accounts_by_profile_id(auth_profile_id)
        .await
        .change_context(AccountError::InternalServerError)?;

    let response = accounts
        .into_iter()
        .map(|account| account.to_account_response())
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ApplicationResponse::json(response))
}

pub async fn get_account(
    state: &AppState,
    account_id: String,
    auth_profile_id: &str,
) -> AccountResponse<GetAccountResponse> {
    let account = state
        .store
        .find_account_by_profile_id_and_account_id(auth_profile_id, &account_id)
        .await
        .to_not_found_response(|| AccountError::AccountNotFound {
            account_id: account_id.clone(),
        })?;

    Ok(ApplicationResponse::json(account.to_account_response()?))
}

pub async fn create_account(
    state: &AppState,
    request: api::AccountCreateRequest,
    auth_profile_id: &str,
) -> AccountResponse<GetAccountResponse> {
    // Validation
    if request.account_name.trim().is_empty() {
        return Err(AccountError::AccountNameEmpty.into());
    }

    let account_id = generate_id_with_default_len("account");

    if request.initial_balance.is_negative() {
        return Err(AccountError::InvalidAmount.into());
    }
    let initial_balance = request
        .initial_balance
        .to_minor_unit_as_i64(request.currency)
        .change_context(AccountError::InvalidAmount)?;

    // Create new account
    let account_new = AccountNew {
        id: account_id.clone(),
        balance_direction: request.account_type,
        name: request.account_name.clone(),
        created_at: date_time::now(),
        last_modified_at: date_time::now(),
        initial_balance,
        profile_id: auth_profile_id.to_string(),
        currency: request.currency,
        matched_debits: MinorUnit::zero(),
        matched_credits: MinorUnit::zero(),
        pending_debits: MinorUnit::zero(),
        pending_credits: MinorUnit::zero(),
        expected_debits: MinorUnit::zero(),
        expected_credits: MinorUnit::zero(),
        mismatched_debits: MinorUnit::zero(),
        mismatched_credits: MinorUnit::zero(),
        posted_debits: MinorUnit::zero(),
        posted_credits: MinorUnit::zero(),
    };
    let account = state
        .store
        .insert_account(account_new)
        .await
        .change_context(AccountError::InternalServerError)?;

    Ok(ApplicationResponse::json_created(
        account.to_account_response()?,
    ))
}

pub async fn update_account(
    state: &AppState,
    account_id: String,
    request: api::AccountUpdateRequest,
    auth_profile_id: &str,
) -> AccountResponse<GetAccountResponse> {
    let existing_account = state
        .store
        .find_account_by_profile_id_and_account_id(auth_profile_id, &account_id)
        .await
        .to_not_found_response(|| AccountError::AccountNotFound {
            account_id: account_id.clone(),
        })?;

    let account = match request.account_name {
        Some(name) => {
            if name.trim().is_empty() {
                return Err(AccountError::AccountNameEmpty.into());
            }
            let account_update = AccountUpdate::NameUpdate { name };

            state
                .store
                .update_account_by_id(&account_id, account_update)
                .await
                .change_context(AccountError::InternalServerError)?
        }
        None => existing_account,
    };

    Ok(ApplicationResponse::json(account.to_account_response()?))
}
