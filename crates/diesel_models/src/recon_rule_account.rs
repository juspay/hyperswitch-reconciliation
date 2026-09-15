use crate::schema::recon_rule_account;
use common_enums::ReconRuleAccountType;
use diesel::prelude::{Insertable, Queryable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

#[derive(Clone, Debug, Deserialize, Serialize, Queryable)]
#[diesel(table_name = recon_rule_account)]
pub struct ReconRuleAccount {
    pub id: String,
    pub account_id: String,
    pub rule_id: String,
    pub account_type: ReconRuleAccountType,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
}

#[derive(Clone, Debug, Deserialize, Serialize, Insertable)]
#[diesel(table_name = recon_rule_account)]
pub struct ReconRuleAccountNew {
    pub id: String,
    pub account_id: String,
    pub rule_id: String,
    pub account_type: ReconRuleAccountType,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
}
