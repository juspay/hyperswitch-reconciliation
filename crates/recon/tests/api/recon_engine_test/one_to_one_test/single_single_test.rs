use api_models::staging_entry::ApiStagingEntryCoarseStatus;
use api_models::transaction::ApiTransactionStatus;
use common_enums::{BalanceDirection, EntryStatus, StagingEntryProcessingMode};
use common_types::{
    recon_rule::{
        search_identifier::{
            CompositeDelimiter, CompositeSearchIdentifier, SearchIdentifier, SearchIdentifierKey,
            SearchIdentifierV2,
        },
        tolerance_config::{ToleranceConfig, ToleranceConfigV1},
    },
    transaction::status::{TransactionAmountMismatchData, TransactionMatchedData},
    units::MinorUnit,
};
use rstest::rstest;

use crate::api::recon_engine_test::types::{
    ExpectedSearchExpectation, ReconRuleCustomConfig, ReconRuleEngine, ReconTestEntry,
    ReconTestEntryExpectedResultBuilder, StagingEntryData,
};

#[rstest]
// Notes for this test
// 1. `transaction_id` is being used as unique identifier
// 2. `reference_id` is being used as search identifier against order_id
// 3. `match_rules_field_x` is being used as match identifier

// Case 1: One To One Single Single: Happy Case
#[case(
    ReconRuleEngine::OneToOneSingleSingleV1(("oms".to_string(), "psp".to_string())),
    ReconRuleCustomConfig::default(),
    vec![
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(10000, BalanceDirection::Credit, vec![("transaction_id", Some("test_value")), ("match_rules_field_1", Some("match_rules_value"))], StagingEntryProcessingMode::Transaction),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Expected, 100.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "order_123")])
                .build(),
                "oms",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(10000, BalanceDirection::Debit, vec![("transaction_id", Some("test_value_1")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("match_rules_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Matched, 100.00, BalanceDirection::Credit), (EntryStatus::Matched, 100.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Matched(TransactionMatchedData::Auto))
                .expected_search_expectations(vec![])
                .build(),
                "psp",
        ),
    ]
)]
// Case 2: One To One Single Single: Under Amount Case
#[case(
    ReconRuleEngine::OneToOneSingleSingleV1(("oms".to_string(), "psp".to_string())),
    ReconRuleCustomConfig::default(),
    vec![
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(10000, BalanceDirection::Credit, vec![("transaction_id", Some("test_value")), ("match_rules_field_1", Some("match_rules_value"))], StagingEntryProcessingMode::Transaction),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Expected, 100.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "order_123")])
                .build(),
                "oms",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(1000, BalanceDirection::Debit, vec![("transaction_id", Some("test_value_1")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("match_rules_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Mismatched, 10.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Mismatch))
                .expected_search_expectations(vec![])
                .build(),
                "psp",
        ),
    ]
)]
// Case 3: One To One Single Single: Over Amount Case
#[case(
    ReconRuleEngine::OneToOneSingleSingleV1(("oms".to_string(), "psp".to_string())),
    ReconRuleCustomConfig::default(),
    vec![
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(10000, BalanceDirection::Credit, vec![("transaction_id", Some("test_value")), ("match_rules_field_1", Some("match_rules_value"))], StagingEntryProcessingMode::Transaction),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Expected, 100.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "order_123")])
                .build(),
                "oms",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(15000, BalanceDirection::Debit, vec![("transaction_id", Some("test_value_1")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("match_rules_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Mismatched, 150.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::OverAmount(TransactionAmountMismatchData::Mismatch))
                .expected_search_expectations(vec![])
                .build(),
                "psp",
        ),
    ]
)]
// Case 4: One To One Single Single: Metadata Mismatch Case
#[case(
    ReconRuleEngine::OneToOneSingleSingleV1(("oms".to_string(), "psp".to_string())),
    ReconRuleCustomConfig::default(),
    vec![
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(10000, BalanceDirection::Credit, vec![("transaction_id", Some("test_value")), ("match_rules_field_1", Some("match_rules_value"))], StagingEntryProcessingMode::Transaction),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Expected, 100.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "order_123")])
                .build(),
                "oms",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(10000, BalanceDirection::Debit, vec![("transaction_id", Some("test_value_1")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("invalid_metadata_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Mismatched, 100.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::DataMismatch)
                .expected_search_expectations(vec![])
                .build(),
                "psp",
        ),
    ]
)]
// Case 5: One To One Single Single: Balance Direction mismatch Case
#[case(
    ReconRuleEngine::OneToOneSingleSingleV1(("oms".to_string(), "psp".to_string())),
    ReconRuleCustomConfig::default(),
    vec![
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(10000, BalanceDirection::Credit, vec![("transaction_id", Some("test_value")), ("match_rules_field_1", Some("match_rules_value"))], StagingEntryProcessingMode::Transaction),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Expected, 100.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "order_123")])
                .build(),
                "oms",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(10000, BalanceDirection::Credit, vec![("transaction_id", Some("test_value_1")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("match_rules_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Mismatched, 100.00, BalanceDirection::Credit)])
                .expected_transaction_status(ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Mismatch))
                .expected_search_expectations(vec![])
                .build(),
                "psp",
        ),
    ]
)]
// Case 6: One To One Single Single: Source with Debit Direction (Refund Flow)
#[case(
    ReconRuleEngine::OneToOneSingleSingleV1(("oms".to_string(), "psp".to_string())),
    ReconRuleCustomConfig::default(),
    vec![
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(5000, BalanceDirection::Debit, vec![("transaction_id", Some("test_value")), ("match_rules_field_1", Some("match_rules_value"))], StagingEntryProcessingMode::Transaction),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 50.00, BalanceDirection::Debit), (EntryStatus::Expected, 50.00, BalanceDirection::Credit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "order_123")])
                .build(),
                "oms",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(5000, BalanceDirection::Credit, vec![("transaction_id", Some("psp_refund_001")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("match_rules_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Matched, 50.00, BalanceDirection::Debit), (EntryStatus::Matched, 50.00, BalanceDirection::Credit)])
                .expected_transaction_status(ApiTransactionStatus::Matched(TransactionMatchedData::Auto))
                .expected_search_expectations(vec![])
                .build(),
                "psp",
        ),
    ]
)]
// Case 7: One To One Single Single: Combined Amount and Metadata Mismatch (Amount takes priority)
#[case(
    ReconRuleEngine::OneToOneSingleSingleV1(("oms".to_string(), "psp".to_string())),
    ReconRuleCustomConfig::default(),
    vec![
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(10000, BalanceDirection::Credit, vec![("transaction_id", Some("test_value")), ("match_rules_field_1", Some("match_rules_value"))], StagingEntryProcessingMode::Transaction),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Expected, 100.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "order_123")])
                .build(),
                "oms",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(8000, BalanceDirection::Debit, vec![("transaction_id", Some("test_value_1")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("wrong_metadata"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Mismatched, 80.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Mismatch))
                .expected_search_expectations(vec![])
                .build(),
                "psp",
        ),
    ]
)]
// Case 8: One To One Single Single: Small Amount (Edge Case)
#[case(
    ReconRuleEngine::OneToOneSingleSingleV1(("oms".to_string(), "psp".to_string())),
    ReconRuleCustomConfig::default(),
    vec![
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(1, BalanceDirection::Credit, vec![("transaction_id", Some("test_value")), ("match_rules_field_1", Some("match_rules_value"))], StagingEntryProcessingMode::Transaction),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 0.01, BalanceDirection::Credit), (EntryStatus::Expected, 0.01, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "order_123")])
                .build(),
                "oms",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(1, BalanceDirection::Debit, vec![("transaction_id", Some("micro_psp")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("match_rules_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Matched, 0.01, BalanceDirection::Credit), (EntryStatus::Matched, 0.01, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Matched(TransactionMatchedData::Auto))
                .expected_search_expectations(vec![])
                .build(),
                "psp",
        ),
    ]
)]
// Case 9: One To One Single Single: Large Amount
#[case(
    ReconRuleEngine::OneToOneSingleSingleV1(("oms".to_string(), "psp".to_string())),
    ReconRuleCustomConfig::default(),
    vec![
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(100000000, BalanceDirection::Credit, vec![("transaction_id", Some("test_value")), ("match_rules_field_1", Some("match_rules_value"))], StagingEntryProcessingMode::Transaction),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 1000000.00, BalanceDirection::Credit), (EntryStatus::Expected, 1000000.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "order_123")])
                .build(),
                "oms",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(100000000, BalanceDirection::Debit, vec![("transaction_id", Some("large_psp")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("match_rules_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Matched, 1000000.00, BalanceDirection::Credit), (EntryStatus::Matched, 1000000.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Matched(TransactionMatchedData::Auto))
                .expected_search_expectations(vec![])
                .build(),
                "psp",
        ),
    ]
)]
// Case 10: One To One Single Single: OverAmount with Metadata Mismatch (OverAmount takes priority)
#[case(
    ReconRuleEngine::OneToOneSingleSingleV1(("oms".to_string(), "psp".to_string())),
    ReconRuleCustomConfig::default(),
    vec![
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(10000, BalanceDirection::Credit, vec![("transaction_id", Some("test_value")), ("match_rules_field_1", Some("match_rules_value"))], StagingEntryProcessingMode::Transaction),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Expected, 100.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "order_123")])
                .build(),
                "oms",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(12000, BalanceDirection::Debit, vec![("transaction_id", Some("test_value_1")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("wrong_metadata"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Mismatched, 120.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::OverAmount(TransactionAmountMismatchData::Mismatch))
                .expected_search_expectations(vec![])
                .build(),
                "psp",
        ),
    ]
)]
// Case 11: Composite Search Identifier - Happy Case (matched on both composite keys)
#[case(
    ReconRuleEngine::OneToOneSingleSingleV1(("oms".to_string(), "psp".to_string())),
    ReconRuleCustomConfig::default().set_search_identifier("psp", composite_search_identifier()),
    vec![
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(10000, BalanceDirection::Credit, vec![("acc_id", Some("ACC-001")), ("region", Some("US")), ("match_rules_field_1", Some("match_rules_value")), ("transaction_id", Some("test_value"))], StagingEntryProcessingMode::Transaction),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Expected, 100.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "ACC-001|US")])
                .build(),
                "oms",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(10000, BalanceDirection::Debit, vec![("acc_ref", Some("ACC-001")), ("region_ref", Some("US")), ("match_rules_field_2", Some("match_rules_value")), ("transaction_id", Some("test_value_1"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Matched, 100.00, BalanceDirection::Credit), (EntryStatus::Matched, 100.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Matched(TransactionMatchedData::Auto))
                .build(),
                "psp",
        ),
    ]
)]
#[actix_web::test]
async fn test_one_to_one_single_single(
    #[case] rule_type: ReconRuleEngine,
    #[case] rule_custom_config: ReconRuleCustomConfig,
    #[case] test_entries: Vec<ReconTestEntry>,
) {
    use crate::api::recon_engine_test::run_recon_engine_test;
    run_recon_engine_test(rule_type, rule_custom_config, test_entries).await;
}

fn valid_200_tolerance_config() -> ToleranceConfig {
    ToleranceConfig::V1(ToleranceConfigV1 {
        lower_tolerance: MinorUnit::new(200),
        upper_tolerance: MinorUnit::new(200),
        tolerance_account_id: "psp".to_string(),
    })
}

#[rstest]
// Notes for this test
// 1. `transaction_id` is being used as unique identifier
// 2. `reference_id` is being used as search identifier against order_id
// 3. `match_rules_field_x` is being used as match identifier

// Case 1: Tolerance - Underpayment within tolerance => Matched(WithTolerance)
// Source: 10000, Target: 9900, Tolerance: 200 => Delta: 100 (within)
#[case(
    ReconRuleEngine::OneToOneSingleSingleV1(("oms".to_string(), "psp".to_string())),
    ReconRuleCustomConfig::default().set_tolerance_config("psp", Some(valid_200_tolerance_config())),
    vec![
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(10000, BalanceDirection::Credit, vec![("transaction_id", Some("test_value")), ("match_rules_field_1", Some("match_rules_value"))], StagingEntryProcessingMode::Transaction),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Expected, 100.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "order_123")])
                .build(),
                "oms",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(9900, BalanceDirection::Debit, vec![("transaction_id", Some("test_value_1")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("match_rules_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![
                    (EntryStatus::Matched, 100.00, BalanceDirection::Credit),
                    (EntryStatus::Matched, 99.00, BalanceDirection::Debit),
                    (EntryStatus::Matched, 1.00, BalanceDirection::Debit),
                ])
                .expected_transaction_status(ApiTransactionStatus::Matched(TransactionMatchedData::WithTolerance))
                .expected_search_expectations(vec![])
                .build(),
                "psp",
        ),
    ]
)]
// Case 2: Tolerance - Overpayment within tolerance => Matched(WithTolerance)
// Source: 10000, Target: 10100, Tolerance: 200 => Delta: 100 (within)
#[case(
    ReconRuleEngine::OneToOneSingleSingleV1(("oms".to_string(), "psp".to_string())),
    ReconRuleCustomConfig::default().set_tolerance_config("psp", Some(valid_200_tolerance_config())),
    vec![
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(10000, BalanceDirection::Credit, vec![("transaction_id", Some("test_value")), ("match_rules_field_1", Some("match_rules_value"))], StagingEntryProcessingMode::Transaction),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Expected, 100.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "order_123")])
                .build(),
                "oms",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(10100, BalanceDirection::Debit, vec![("transaction_id", Some("test_value_1")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("match_rules_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![
                    (EntryStatus::Matched, 100.00, BalanceDirection::Credit),
                    (EntryStatus::Matched, 101.00, BalanceDirection::Debit),
                    (EntryStatus::Matched, 1.00, BalanceDirection::Credit),
                ])
                .expected_transaction_status(ApiTransactionStatus::Matched(TransactionMatchedData::WithTolerance))
                .expected_search_expectations(vec![])
                .build(),
                "psp",
        ),
    ]
)]
// Case 3: Tolerance - Exact underpayment boundary (delta == tolerance) => within tolerance
// Source: 10000, Target: 9800, Tolerance: 200 => Delta: 200 (exactly at boundary)
#[case(
    ReconRuleEngine::OneToOneSingleSingleV1(("oms".to_string(), "psp".to_string())),
    ReconRuleCustomConfig::default().set_tolerance_config("psp", Some(valid_200_tolerance_config())),
    vec![
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(10000, BalanceDirection::Credit, vec![("transaction_id", Some("test_value")), ("match_rules_field_1", Some("match_rules_value"))], StagingEntryProcessingMode::Transaction),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Expected, 100.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "order_123")])
                .build(),
                "oms",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(9800, BalanceDirection::Debit, vec![("transaction_id", Some("test_value_1")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("match_rules_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![
                    (EntryStatus::Matched, 100.00, BalanceDirection::Credit),
                    (EntryStatus::Matched, 98.00, BalanceDirection::Debit),
                    (EntryStatus::Matched, 2.00, BalanceDirection::Debit),
                ])
                .expected_transaction_status(ApiTransactionStatus::Matched(TransactionMatchedData::WithTolerance))
                .expected_search_expectations(vec![])
                .build(),
                "psp",
        ),
    ]
)]
// Case 4: Tolerance - Exact overpayment boundary (delta == tolerance) => within tolerance
// Source: 10000, Target: 10200, Tolerance: 200 => Delta: 200 (exactly at boundary)
#[case(
    ReconRuleEngine::OneToOneSingleSingleV1(("oms".to_string(), "psp".to_string())),
    ReconRuleCustomConfig::default().set_tolerance_config("psp", Some(valid_200_tolerance_config())),
    vec![
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(10000, BalanceDirection::Credit, vec![("transaction_id", Some("test_value")), ("match_rules_field_1", Some("match_rules_value"))], StagingEntryProcessingMode::Transaction),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Expected, 100.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "order_123")])
                .build(),
                "oms",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(10200, BalanceDirection::Debit, vec![("transaction_id", Some("test_value_1")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("match_rules_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![
                    (EntryStatus::Matched, 100.00, BalanceDirection::Credit),
                    (EntryStatus::Matched, 102.00, BalanceDirection::Debit),
                    (EntryStatus::Matched, 2.00, BalanceDirection::Credit),
                ])
                .expected_transaction_status(ApiTransactionStatus::Matched(TransactionMatchedData::WithTolerance))
                .expected_search_expectations(vec![])
                .build(),
                "psp",
        ),
    ]
)]
// Case 5: Tolerance - Underpayment outside tolerance => UnderAmount
// Source: 10000, Target: 9500, Tolerance: 200 => Delta: 500 (outside)
#[case(
    ReconRuleEngine::OneToOneSingleSingleV1(("oms".to_string(), "psp".to_string())),
    ReconRuleCustomConfig::default().set_tolerance_config("psp", Some(valid_200_tolerance_config())),
    vec![
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(10000, BalanceDirection::Credit, vec![("transaction_id", Some("test_value")), ("match_rules_field_1", Some("match_rules_value"))], StagingEntryProcessingMode::Transaction),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Expected, 100.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "order_123")])
                .build(),
                "oms",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(9500, BalanceDirection::Debit, vec![("transaction_id", Some("test_value_1")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("match_rules_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Mismatched, 95.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Mismatch))
                .expected_search_expectations(vec![])
                .build(),
                "psp",
        ),
    ]
)]
// Case 6: Tolerance - Overpayment outside tolerance => OverAmount
// Source: 10000, Target: 10500, Tolerance: 200 => Delta: 500 (outside)
#[case(
    ReconRuleEngine::OneToOneSingleSingleV1(("oms".to_string(), "psp".to_string())),
    ReconRuleCustomConfig::default().set_tolerance_config("psp", Some(valid_200_tolerance_config())),
    vec![
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(10000, BalanceDirection::Credit, vec![("transaction_id", Some("test_value")), ("match_rules_field_1", Some("match_rules_value"))], StagingEntryProcessingMode::Transaction),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Expected, 100.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "order_123")])
                .build(),
                "oms",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(10500, BalanceDirection::Debit, vec![("transaction_id", Some("test_value_1")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("match_rules_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Mismatched, 105.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::OverAmount(TransactionAmountMismatchData::Mismatch))
                .expected_search_expectations(vec![])
                .build(),
                "psp",
        ),
    ]
)]
// Case 7: Tolerance + metadata mismatch (overpayment within tolerance) => DataMismatch
// Source: 10000, Target: 10100, Tolerance: 200 => Delta: 100 (within)
#[case(
    ReconRuleEngine::OneToOneSingleSingleV1(("oms".to_string(), "psp".to_string())),
    ReconRuleCustomConfig::default().set_tolerance_config("psp", Some(valid_200_tolerance_config())),
    vec![
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(10000, BalanceDirection::Credit, vec![("transaction_id", Some("test_value")), ("match_rules_field_1", Some("match_rules_value"))], StagingEntryProcessingMode::Transaction),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Expected, 100.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "order_123")])
                .build(),
                "oms",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(10100, BalanceDirection::Debit, vec![("transaction_id", Some("test_value_1")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("invalid_metadata_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Mismatched, 101.00, BalanceDirection::Debit), (EntryStatus::Pending, 1.00, BalanceDirection::Credit)])
                .expected_transaction_status(ApiTransactionStatus::DataMismatch)
                .expected_search_expectations(vec![])
                .build(),
                "psp",
        ),
    ]
)]
// Case 8: Tolerance + metadata mismatch (underpayment within tolerance) => DataMismatch
// Source: 10000, Target: 9900, Tolerance: 200 => Delta: 100 (within)
#[case(
    ReconRuleEngine::OneToOneSingleSingleV1(("oms".to_string(), "psp".to_string())),
    ReconRuleCustomConfig::default().set_tolerance_config("psp", Some(valid_200_tolerance_config())),
    vec![
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(10000, BalanceDirection::Credit, vec![("transaction_id", Some("test_value")), ("match_rules_field_1", Some("match_rules_value"))], StagingEntryProcessingMode::Transaction),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Expected, 100.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "order_123")])
                .build(),
                "oms",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(9900, BalanceDirection::Debit, vec![("transaction_id", Some("test_value_1")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("invalid_metadata_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Mismatched, 99.00, BalanceDirection::Debit), (EntryStatus::Pending, 1.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::DataMismatch)
                .expected_search_expectations(vec![])
                .build(),
                "psp",
        ),
    ]
)]
#[actix_web::test]
async fn test_one_to_one_single_single_tolerance(
    #[case] rule_type: ReconRuleEngine,
    #[case] rule_custom_config: ReconRuleCustomConfig,
    #[case] test_entries: Vec<ReconTestEntry>,
) {
    use crate::api::recon_engine_test::run_recon_engine_test;
    run_recon_engine_test(rule_type, rule_custom_config, test_entries).await;
}

fn composite_search_identifier() -> SearchIdentifier {
    SearchIdentifier::V2(SearchIdentifierV2::Composite(CompositeSearchIdentifier {
        keys: vec![
            SearchIdentifierKey {
                source_field: "metadata.acc_id".parse().unwrap(),
                target_field: "metadata.acc_ref".parse().unwrap(),
            },
            SearchIdentifierKey {
                source_field: "metadata.region".parse().unwrap(),
                target_field: "metadata.region_ref".parse().unwrap(),
            },
        ],
        delimiter: CompositeDelimiter::Pipe,
    }))
}
