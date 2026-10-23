use common_types::{
    link_config::link_trigger::{
        LinkTrigger, LinkTriggerCondition, LinkTriggerGroup, LinkTriggerOperator, LinkTriggerV1,
    },
    recon_rule::entry_field::ExtractFromEntryField,
};

pub trait LinkTriggerExt {
    fn validate(&self, data: &impl ExtractFromEntryField) -> bool;
}

impl LinkTriggerExt for LinkTrigger {
    fn validate(&self, data: &impl ExtractFromEntryField) -> bool {
        match self {
            Self::V1(trigger) => trigger.validate(data),
        }
    }
}

impl LinkTriggerExt for LinkTriggerV1 {
    fn validate(&self, data: &impl ExtractFromEntryField) -> bool {
        self.any.iter().any(|group| group.validate(data))
    }
}

impl LinkTriggerExt for LinkTriggerGroup {
    fn validate(&self, data: &impl ExtractFromEntryField) -> bool {
        self.all.iter().all(|condition| condition.validate(data))
    }
}

impl LinkTriggerExt for LinkTriggerCondition {
    fn validate(&self, data: &impl ExtractFromEntryField) -> bool {
        let Some(field_value) = data.extract_field(&self.field) else {
            return false;
        };

        self.operator.check(&field_value, &self.value)
    }
}

trait LinkTriggerOperatorExt {
    fn check(&self, field_value: &str, expected_value: &str) -> bool;
}

impl LinkTriggerOperatorExt for LinkTriggerOperator {
    fn check(&self, field_value: &str, expected_value: &str) -> bool {
        match self {
            Self::Equals => field_value == expected_value,
            Self::NotEquals => field_value != expected_value,
        }
    }
}
