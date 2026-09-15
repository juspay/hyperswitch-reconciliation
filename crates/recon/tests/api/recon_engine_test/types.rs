use api_models::staging_entry::ApiStagingEntryCoarseStatus;
use std::collections::HashMap;

use crate::api::recon_engine_test::{
    one_to_many_test::create_accounts_for_one_to_many,
    one_to_one_test::create_accounts_for_one_to_one,
};
use crate::common::TestClient;
use api_models::{
    metadata_schema::{FieldType, MetadataExposeConfig},
    transaction::ApiTransactionStatus,
    GetAccountResponse,
};
use async_trait::async_trait;
use common_enums::{
    BalanceDirection, Currency, EntryStatus, SearchGroupStatus, StagingEntryProcessingMode,
};
use common_types::recon_rule::grouping_field::GroupingField;
use common_types::recon_rule::match_rules::MatchRules;
use common_types::recon_rule::search_identifier::SearchIdentifier;
use common_types::recon_rule::strategies::one_to_one_strategy::SourceAccumulationMode;
use common_types::recon_rule::tolerance_config::ToleranceConfig;
use common_types::recon_rule::trigger::Trigger;
use common_types::{recon_rule::entry_field::EntryField, units::FloatMajorUnit};

#[derive(Debug, Clone)]
pub enum ReconRuleEngine {
    OneToOneSingleSingleV1((String, String)),
    OneToOneSingleManyV1((String, String)),
    OneToOneManySingleV1((String, String)),
    OneToOneManyManyV1((String, String)),
    OneToManySingleSingleV1((String, Vec<(String, f64)>)), // on target side, we can set split values as well
}

#[async_trait]
pub trait ReconRuleEngineExt {
    async fn create_accounts(
        &self,
        client: &TestClient,
        token: &str,
    ) -> (Vec<GetAccountResponse>, Vec<GetAccountResponse>);
}

#[async_trait]
impl ReconRuleEngineExt for ReconRuleEngine {
    async fn create_accounts(
        &self,
        client: &TestClient,
        token: &str,
    ) -> (Vec<GetAccountResponse>, Vec<GetAccountResponse>) {
        match self {
            Self::OneToOneSingleSingleV1((source_account_name, target_account_name))
            | Self::OneToOneSingleManyV1((source_account_name, target_account_name))
            | Self::OneToOneManySingleV1((source_account_name, target_account_name))
            | Self::OneToOneManyManyV1((source_account_name, target_account_name)) => {
                create_accounts_for_one_to_one(
                    client,
                    token,
                    source_account_name,
                    target_account_name,
                )
                .await
            }
            Self::OneToManySingleSingleV1(account_names) => {
                let (source_account_name, target_account_names) = account_names;
                create_accounts_for_one_to_many(
                    client,
                    token,
                    source_account_name,
                    &target_account_names
                        .iter()
                        .map(|acc| acc.0.clone())
                        .collect::<Vec<_>>(),
                )
                .await
            }
        }
    }
}

pub struct StagingEntryData {
    pub amount: i64,
    pub currency: Currency,
    pub effective_at: String,
    pub entry_type: BalanceDirection,
    pub order_id: String,
    pub processing_mode: StagingEntryProcessingMode,
    pub metadata: HashMap<String, Option<String>>,
}

impl StagingEntryData {
    pub fn recon_staging_entry(
        amount: i64,
        entry_type: BalanceDirection,
        metadata: Vec<(&str, Option<&str>)>,
        processing_mode: StagingEntryProcessingMode,
    ) -> Self {
        Self {
            amount,
            currency: Currency::USD,
            effective_at: "12-12-2025".to_string(),
            entry_type,
            order_id: "order_123".to_string(),
            processing_mode,
            metadata: metadata
                .into_iter()
                .map(|(key, value)| (key.to_owned(), value.map(str::to_string)))
                .collect::<HashMap<_, _>>(),
        }
    }

    pub fn get_raw_data(&self) -> HashMap<String, Option<String>> {
        let mut metadata = self.metadata.clone();
        metadata.insert(
            EntryField::Amount.to_string(),
            Some(self.amount.to_string()),
        );
        metadata.insert(
            EntryField::Currency.to_string(),
            Some(self.currency.to_string()),
        );
        metadata.insert(
            EntryField::EffectiveAt.to_string(),
            Some(self.effective_at.clone()),
        );
        metadata.insert(
            EntryField::EntryType.to_string(),
            Some(self.entry_type.to_string()),
        );
        metadata.insert(EntryField::OrderId.to_string(), Some(self.order_id.clone()));
        metadata
    }

    pub fn get_metadata_fields(
        &self,
    ) -> Vec<(EntryField, bool, FieldType, Option<MetadataExposeConfig>)> {
        self.metadata
            .clone()
            .into_keys()
            .map(|key| {
                (
                    EntryField::Metadata(key.clone()),
                    false,
                    FieldType::String {
                        validation_rules: vec![],
                        transformation_rules: vec![],
                    },
                    Some(MetadataExposeConfig {
                        is_searchable: true,
                    }),
                )
            })
            .collect()
    }
}

pub struct ReconTestEntry {
    pub staging_entry_data: StagingEntryData,
    pub expected_result: ReconTestEntryExpectedResult,
    pub account_name: String,
}

impl ReconTestEntry {
    pub fn new(
        staging_entry_data: StagingEntryData,
        expected_result: ReconTestEntryExpectedResult,
        account_name: &str,
    ) -> Self {
        Self {
            staging_entry_data,
            expected_result,
            account_name: account_name.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExpectedSearchExpectation {
    pub account_name: String,
    pub source_value: String,
}

impl ExpectedSearchExpectation {
    pub fn new(account_name: &str, source_value: &str) -> Self {
        Self {
            account_name: account_name.to_string(),
            source_value: source_value.to_string(),
        }
    }
}

#[derive(Debug)]
pub struct ReconTestEntryExpectedResult {
    pub expected_staging_entry_status: ApiStagingEntryCoarseStatus,
    pub expected_entries_data: Vec<ReconTestEntryExpectedData>,
    pub expected_transaction_status: ApiTransactionStatus,
    pub expected_transaction_version: Option<i32>,
    pub expected_transaction_amounts: Option<(FloatMajorUnit, FloatMajorUnit)>,
    pub expected_search_group_status: Option<SearchGroupStatus>,
    pub expected_search_expectations: Option<Vec<ExpectedSearchExpectation>>,
}

pub struct ReconTestEntryExpectedResultBuilder {
    expected_staging_entry_status: Option<ApiStagingEntryCoarseStatus>,
    expected_entries_data: Option<Vec<ReconTestEntryExpectedData>>,
    expected_transaction_status: Option<ApiTransactionStatus>,
    expected_transaction_version: Option<i32>,
    expected_transaction_amounts: Option<(FloatMajorUnit, FloatMajorUnit)>,
    expected_search_group_status: Option<SearchGroupStatus>,
    expected_search_expectations: Option<Vec<ExpectedSearchExpectation>>,
}

impl Default for ReconTestEntryExpectedResultBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ReconTestEntryExpectedResultBuilder {
    pub fn new() -> Self {
        Self {
            expected_staging_entry_status: None,
            expected_entries_data: None,
            expected_transaction_status: None,
            expected_transaction_version: None,
            expected_transaction_amounts: None,
            expected_search_group_status: None,
            expected_search_expectations: None,
        }
    }

    pub fn expected_staging_entry_status(mut self, status: ApiStagingEntryCoarseStatus) -> Self {
        self.expected_staging_entry_status = Some(status);
        self
    }

    pub fn expected_entries_data(
        mut self,
        data: Vec<(EntryStatus, f64, BalanceDirection)>,
    ) -> Self {
        self.expected_entries_data = Some(
            data.into_iter()
                .map(|(status, amount, entry_type)| {
                    ReconTestEntryExpectedData::new(status, amount, entry_type)
                })
                .collect::<Vec<_>>(),
        );
        self
    }

    pub fn expected_transaction_status(mut self, status: ApiTransactionStatus) -> Self {
        self.expected_transaction_status = Some(status);
        self
    }

    pub fn expected_transaction_version(mut self, version: i32) -> Self {
        self.expected_transaction_version = Some(version);
        self
    }

    pub fn expected_transaction_amounts(mut self, credit: f64, debit: f64) -> Self {
        self.expected_transaction_amounts =
            Some((FloatMajorUnit::new(credit), FloatMajorUnit::new(debit)));
        self
    }

    pub fn expected_search_group_status(mut self, status: SearchGroupStatus) -> Self {
        self.expected_search_group_status = Some(status);
        self
    }

    pub fn expected_search_expectations(
        mut self,
        expectations: Vec<ExpectedSearchExpectation>,
    ) -> Self {
        self.expected_search_expectations = Some(expectations);
        self
    }

    pub fn build(self) -> ReconTestEntryExpectedResult {
        ReconTestEntryExpectedResult {
            expected_staging_entry_status: self
                .expected_staging_entry_status
                .expect("expected_staging_entry_status is required"),
            expected_entries_data: self
                .expected_entries_data
                .expect("expected_entries_data is required"),
            expected_transaction_status: self
                .expected_transaction_status
                .expect("expected_transaction_status is required"),
            expected_transaction_version: self.expected_transaction_version,
            expected_transaction_amounts: self.expected_transaction_amounts,
            expected_search_group_status: self.expected_search_group_status,
            expected_search_expectations: self.expected_search_expectations,
        }
    }
}

#[derive(Debug)]
pub struct ReconTestEntryExpectedData {
    pub status: EntryStatus,
    pub amount: FloatMajorUnit,
    pub entry_type: BalanceDirection,
}

impl ReconTestEntryExpectedData {
    pub fn new(status: EntryStatus, amount: f64, entry_type: BalanceDirection) -> Self {
        Self {
            status,
            amount: FloatMajorUnit::new(amount),
            entry_type,
        }
    }
}

#[derive(Debug, Default)]
pub struct ReconRuleCustomConfig {
    // (target_account_name, ...)
    pub search_identifier: HashMap<String, SearchIdentifier>,
    pub match_rules: HashMap<String, MatchRules>,
    pub tolerance_config: HashMap<String, Option<ToleranceConfig>>,

    // (source_account_name, ...)
    pub grouping_field: HashMap<String, GroupingField>,
    pub trigger: HashMap<String, Trigger>,
    pub source_accumulation_mode: SourceAccumulationMode,
}

impl ReconRuleCustomConfig {
    pub fn set_search_identifier(
        mut self,
        target_account_name: &str,
        search_identifier: SearchIdentifier,
    ) -> Self {
        self.search_identifier
            .insert(target_account_name.to_string(), search_identifier);
        self
    }

    pub fn set_match_rules(mut self, target_account_name: &str, match_rules: MatchRules) -> Self {
        self.match_rules
            .insert(target_account_name.to_string(), match_rules);
        self
    }

    pub fn set_tolerance_config(
        mut self,
        target_account_name: &str,
        tolerance_config: Option<ToleranceConfig>,
    ) -> Self {
        self.tolerance_config
            .insert(target_account_name.to_string(), tolerance_config);
        self
    }

    pub fn set_grouping_field(
        mut self,
        source_account_name: &str,
        grouping_field: GroupingField,
    ) -> Self {
        self.grouping_field
            .insert(source_account_name.to_string(), grouping_field);
        self
    }

    pub fn set_trigger(mut self, source_account_name: &str, trigger: Trigger) -> Self {
        self.trigger
            .insert(source_account_name.to_string(), trigger);
        self
    }

    pub fn set_source_accumulation_mode(mut self, mode: SourceAccumulationMode) -> Self {
        self.source_accumulation_mode = mode;
        self
    }

    pub fn populate_account_ids(self, accounts: HashMap<String, String>) -> Self {
        fn remap_keys<V>(
            map: HashMap<String, V>,
            accounts: &HashMap<String, String>,
        ) -> HashMap<String, V> {
            map.into_iter()
                .map(|(k, v)| (accounts.get(&k).cloned().unwrap_or(k), v))
                .collect()
        }

        let tolerance_config = remap_keys(self.tolerance_config, &accounts)
            .into_iter()
            .map(|(key, value)| {
                (
                    key.clone(),
                    value.map(|config| match config {
                        ToleranceConfig::V1(mut config_v1) => {
                            config_v1.tolerance_account_id = key.clone();
                            ToleranceConfig::V1(config_v1)
                        }
                    }),
                )
            })
            .collect();

        Self {
            search_identifier: remap_keys(self.search_identifier, &accounts),
            match_rules: remap_keys(self.match_rules, &accounts),
            tolerance_config,
            grouping_field: remap_keys(self.grouping_field, &accounts),
            trigger: remap_keys(self.trigger, &accounts),
            source_accumulation_mode: self.source_accumulation_mode,
        }
    }
}
