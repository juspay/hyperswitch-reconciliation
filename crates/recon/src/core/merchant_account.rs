use api_models::merchant_account as api;
use common_utils::{date_time, id_type::MerchantId, types::keymanager::Identifier, GenerateId};
use diesel_models::merchant_account::MerchantAccountNew as DieselMerchantAccountNew;
use error_stack::ResultExt;
use masking::{ExposeInterface, PeekInterface};
use recon_domain_models::{
    behaviour::ReverseConversion, merchant_account::DomainMerchantAccount,
    ApplicationEncryptionInterface,
};
use storage_impl::merchant_account::MerchantAccountInterface;

use crate::{
    errors::merchant_account::{MerchantAccountError, MerchantAccountResult},
    routes::AppState,
    types::ApplicationResponse,
};

pub async fn create_merchant_account(
    state: AppState,
    request: api::MerchantAccountCreateRequest,
) -> MerchantAccountResult<api::MerchantAccountResponse> {
    if request.merchant_name.peek().is_empty() {
        return Err(MerchantAccountError::MerchantNameEmpty.into());
    }

    let merchant_id = MerchantId::generate();
    let identifier = Identifier::ReconMerchant(merchant_id.clone());
    state
        .encryption_client
        .setup_key_for_identifier(&identifier)
        .await
        .change_context(MerchantAccountError::InternalServerError)?;

    let merchant_encryptor = state
        .encryption_client
        .get_encryptor_for_merchant(merchant_id.clone());

    let diesel_merchant_account_new = DieselMerchantAccountNew {
        merchant_id: merchant_id.clone(),
        merchant_name: merchant_encryptor
            .encrypt_string(request.merchant_name.expose())
            .await
            .change_context(MerchantAccountError::InternalServerError)?
            .into(),
        created_at: date_time::now(),
        last_modified_at: date_time::now(),
    };

    let diesel_merchant = state
        .store
        .insert_merchant_account(diesel_merchant_account_new)
        .await
        .change_context(MerchantAccountError::InternalServerError)?;

    let domain_merchant: DomainMerchantAccount = diesel_merchant
        .convert_back(&merchant_encryptor)
        .await
        .change_context(MerchantAccountError::InternalServerError)?;
    Ok(ApplicationResponse::json_created(
        api::MerchantAccountResponse {
            merchant_id: domain_merchant.merchant_id,
            merchant_name: domain_merchant.merchant_name,
        },
    ))
}

pub async fn get_merchant_account(
    state: AppState,
    merchant_id: &MerchantId,
) -> MerchantAccountResult<api::MerchantAccountResponse> {
    let merchant_account = state
        .store
        .find_merchant_account_by_id(merchant_id)
        .await
        .change_context(MerchantAccountError::InternalServerError)?;

    let merchant_encryptor = state
        .encryption_client
        .get_encryptor_for_merchant(merchant_id.clone());

    let domain_account: DomainMerchantAccount = merchant_account
        .convert_back(&merchant_encryptor)
        .await
        .change_context(MerchantAccountError::InternalServerError)?;

    Ok(ApplicationResponse::json(api::MerchantAccountResponse {
        merchant_id: domain_account.merchant_id.clone(),
        merchant_name: domain_account.merchant_name,
    }))
}
