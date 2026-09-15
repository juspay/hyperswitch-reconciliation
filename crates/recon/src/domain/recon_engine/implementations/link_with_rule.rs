use common_types::recon_rule::{
    entry_field::EntryField,
    link_with_rule::{LinkWithRule, LinkWithRuleTargetData, LinkWithRuleV1},
};

pub trait LinkWithRuleExt {
    fn extract_current_rule_account_id(&self) -> String;

    fn extract_current_rule_field(&self) -> EntryField;

    fn extract_target_data(&self) -> Vec<LinkWithRuleTargetData>;
}

impl LinkWithRuleExt for LinkWithRule {
    fn extract_current_rule_account_id(&self) -> String {
        match self {
            Self::V1(link_with_rule_v1) => link_with_rule_v1.extract_current_rule_account_id(),
        }
    }

    fn extract_current_rule_field(&self) -> EntryField {
        match self {
            Self::V1(link_with_rule_v1) => link_with_rule_v1.extract_current_rule_field(),
        }
    }

    fn extract_target_data(&self) -> Vec<LinkWithRuleTargetData> {
        match self {
            Self::V1(link_with_rule_v1) => link_with_rule_v1.extract_target_data(),
        }
    }
}

impl LinkWithRuleExt for LinkWithRuleV1 {
    fn extract_current_rule_account_id(&self) -> String {
        self.current_rule_account_id.clone()
    }
    fn extract_current_rule_field(&self) -> EntryField {
        self.current_rule_field.clone()
    }

    fn extract_target_data(&self) -> Vec<LinkWithRuleTargetData> {
        self.target_data.clone()
    }
}
