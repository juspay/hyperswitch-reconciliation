use common_enums::ReconRuleAccountType;
use common_utils::{
    date_time,
    errors::{CustomResult, ValidationError},
    id_generation::generate_id_with_default_len,
};
use diesel_models::recon_rule_account::{
    ReconRuleAccount as DieselReconRuleAccount, ReconRuleAccountNew,
};
use time::PrimitiveDateTime;

use crate::{behaviour::Conversion, ApplicationEncryptionInterface, EncryptionHandler};

#[derive(Clone, Debug, serde::Serialize)]
pub struct DomainReconRuleAccount {
    pub id: String,
    pub account_id: String,
    pub rule_id: String,
    pub account_type: ReconRuleAccountType,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
}

impl DomainReconRuleAccount {
    pub fn new(account_id: String, rule_id: String, account_type: ReconRuleAccountType) -> Self {
        let id = generate_id_with_default_len("rec_acc");
        let now = date_time::now();
        Self {
            id,
            account_id,
            rule_id,
            account_type,
            created_at: now,
            last_modified_at: now,
        }
    }
}

#[async_trait::async_trait]
impl Conversion for DomainReconRuleAccount {
    type DstType = DieselReconRuleAccount;
    type NewDstType = ReconRuleAccountNew;

    fn construct_new(self) -> Self::NewDstType {
        Self::NewDstType {
            id: self.id,
            account_id: self.account_id,
            rule_id: self.rule_id,
            account_type: self.account_type,
            created_at: self.created_at,
            last_modified_at: self.last_modified_at,
        }
    }

    fn convert(self) -> Self::DstType {
        Self::DstType {
            id: self.id,
            account_id: self.account_id,
            rule_id: self.rule_id,
            account_type: self.account_type,
            created_at: self.created_at,
            last_modified_at: self.last_modified_at,
        }
    }

    async fn convert_back(
        _encryptor: &EncryptionHandler<impl ApplicationEncryptionInterface>,
        item: Self::DstType,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized,
    {
        Ok(Self {
            id: item.id,
            account_id: item.account_id,
            rule_id: item.rule_id,
            account_type: item.account_type,
            created_at: item.created_at,
            last_modified_at: item.last_modified_at,
        })
    }
}
