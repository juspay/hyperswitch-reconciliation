use crate::api::recon_engine_test::types::{
    ExpectedSearchExpectation, ReconRuleCustomConfig, ReconRuleEngine, ReconTestEntry,
    ReconTestEntryExpectedResultBuilder, StagingEntryData,
};
use api_models::staging_entry::ApiStagingEntryCoarseStatus;
use api_models::transaction::ApiTransactionStatus;
use common_enums::{BalanceDirection, EntryStatus, StagingEntryProcessingMode};
use common_types::{
    recon_rule::search_identifier::{
        CompositeDelimiter, CompositeSearchIdentifier, SearchIdentifier, SearchIdentifierKey,
        SearchIdentifierV2,
    },
    transaction::status::{TransactionAmountMismatchData, TransactionMatchedData},
};
use rstest::rstest;

#[rstest]
// Notes for this test
// 1. `transaction_id` is being used a unique identifier
// 2. `reference_id` is being used a search identifier against order_id
// 3. `match_rules_field_x` is being used a match identifier

// Case 1: One To Many Single Single: Happy Case
#[case(
    ReconRuleEngine::OneToManySingleSingleV1(("oms".to_string(), vec![("psp".to_string(), 100.0)])),
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
// Case 2: One To Many Single Single: Multiple Targets Case (UnderAmount Expected)
#[case(
    ReconRuleEngine::OneToManySingleSingleV1(("oms".to_string(), vec![("psp1".to_string(), 30.0), ("psp2".to_string(), 70.0)])),
    ReconRuleCustomConfig::default(),
    vec![
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(10000, BalanceDirection::Credit, vec![("transaction_id", Some("test_value")), ("match_rules_field_1", Some("match_rules_value"))], StagingEntryProcessingMode::Transaction),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Expected, 30.00, BalanceDirection::Debit), (EntryStatus::Expected, 70.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp1", "order_123"), ExpectedSearchExpectation::new("psp2", "order_123")])
                .build(),
                "oms",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(3000, BalanceDirection::Debit, vec![("transaction_id", Some("test_value_1")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("match_rules_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Pending, 30.00, BalanceDirection::Debit), (EntryStatus::Expected, 70.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Expected))
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp2", "order_123")])
                .build(),
                "psp1",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(7000, BalanceDirection::Debit, vec![("transaction_id", Some("test_value_2")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("match_rules_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Matched, 100.00, BalanceDirection::Credit), (EntryStatus::Matched, 30.00, BalanceDirection::Debit), (EntryStatus::Matched, 70.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Matched(TransactionMatchedData::Auto))
                .expected_search_expectations(vec![])
                .build(),
                "psp2",
        ),
    ]
)]
// Case 3: One To Many Single Single: UnderAmount Mismatch
#[case(
    ReconRuleEngine::OneToManySingleSingleV1(("oms".to_string(), vec![("psp1".to_string(), 30.0), ("psp2".to_string(), 70.0)])),
    ReconRuleCustomConfig::default(),
    vec![
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(10000, BalanceDirection::Credit, vec![("transaction_id", Some("test_value")), ("match_rules_field_1", Some("match_rules_value"))], StagingEntryProcessingMode::Transaction),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Expected, 30.00, BalanceDirection::Debit), (EntryStatus::Expected, 70.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp1", "order_123"), ExpectedSearchExpectation::new("psp2", "order_123")])
                .build(),
                "oms",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(4000, BalanceDirection::Debit, vec![("transaction_id", Some("test_value_1")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("match_rules_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Mismatched, 40.00, BalanceDirection::Debit), (EntryStatus::Expected, 70.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Mismatch))
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp2", "order_123")])
                .build(),
                "psp1",
        ),
    ]
)]
// Case 4: One To Many Single Single: OverAmount Mismatch
#[case(
    ReconRuleEngine::OneToManySingleSingleV1(("oms".to_string(), vec![("psp1".to_string(), 30.0), ("psp2".to_string(), 70.0)])),
    ReconRuleCustomConfig::default(),
    vec![
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(10000, BalanceDirection::Credit, vec![("transaction_id", Some("test_value")), ("match_rules_field_1", Some("match_rules_value"))], StagingEntryProcessingMode::Transaction),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Expected, 30.00, BalanceDirection::Debit), (EntryStatus::Expected, 70.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp1", "order_123"), ExpectedSearchExpectation::new("psp2", "order_123")])
                .build(),
                "oms",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(11000, BalanceDirection::Debit, vec![("transaction_id", Some("test_value_1")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("match_rules_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Mismatched, 110.00, BalanceDirection::Debit), (EntryStatus::Expected, 70.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::OverAmount(TransactionAmountMismatchData::Mismatch))
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp2", "order_123")])
                .build(),
                "psp1",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(7000, BalanceDirection::Debit, vec![("transaction_id", Some("test_value_2")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("match_rules_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Mismatched, 110.00, BalanceDirection::Debit), (EntryStatus::Pending, 70.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::OverAmount(TransactionAmountMismatchData::Mismatch))
                .expected_search_expectations(vec![])
                .build(),
                "psp2",
        ),
    ]
)]
// Case 5: One To Many Single Single: Balance Direction Mismatch
#[case(
    ReconRuleEngine::OneToManySingleSingleV1(("oms".to_string(), vec![("psp1".to_string(), 30.0), ("psp2".to_string(), 70.0)])),
    ReconRuleCustomConfig::default(),
    vec![
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(10000, BalanceDirection::Credit, vec![("transaction_id", Some("test_value")), ("match_rules_field_1", Some("match_rules_value"))], StagingEntryProcessingMode::Transaction),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Expected, 30.00, BalanceDirection::Debit), (EntryStatus::Expected, 70.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp1", "order_123"), ExpectedSearchExpectation::new("psp2", "order_123")])
                .build(),
                "oms",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(3000, BalanceDirection::Credit, vec![("transaction_id", Some("test_value_1")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("match_rules_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Mismatched, 30.00, BalanceDirection::Credit), (EntryStatus::Expected, 70.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Mismatch))
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp2", "order_123")])
                .build(),
                "psp1",
        ),
    ]
)]
// Case 6: One To Many Single Single: SplitMismatch
#[case(
    ReconRuleEngine::OneToManySingleSingleV1(("oms".to_string(), vec![("psp1".to_string(), 40.0), ("psp2".to_string(), 60.0)])),
    ReconRuleCustomConfig::default(),
    vec![
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(10000, BalanceDirection::Credit, vec![("transaction_id", Some("test_value")), ("match_rules_field_1", Some("match_rules_value"))], StagingEntryProcessingMode::Transaction),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Expected, 40.00, BalanceDirection::Debit), (EntryStatus::Expected, 60.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp1", "order_123"), ExpectedSearchExpectation::new("psp2", "order_123")])
                .build(),
                "oms",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(5000, BalanceDirection::Debit, vec![("transaction_id", Some("test_value_1")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("match_rules_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Mismatched, 50.00, BalanceDirection::Debit), (EntryStatus::Expected, 60.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Mismatch))
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp2", "order_123")])
                .build(),
                "psp1",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(5000, BalanceDirection::Debit, vec![("transaction_id", Some("test_value_2")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("match_rules_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Mismatched, 50.00, BalanceDirection::Debit), (EntryStatus::Mismatched, 50.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::SplitMismatch)
                .expected_search_expectations(vec![])
                .build(),
                "psp2",
        ),
    ]
)]
// Case 7: One To Many Single Single: DataMismatch
#[case(
    ReconRuleEngine::OneToManySingleSingleV1(("oms".to_string(), vec![("psp1".to_string(), 30.0), ("psp2".to_string(), 70.0)])),
    ReconRuleCustomConfig::default(),
    vec![
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(10000, BalanceDirection::Credit, vec![("transaction_id", Some("test_value")), ("match_rules_field_1", Some("match_rules_value"))], StagingEntryProcessingMode::Transaction),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Expected, 30.00, BalanceDirection::Debit), (EntryStatus::Expected, 70.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp1", "order_123"), ExpectedSearchExpectation::new("psp2", "order_123")])
                .build(),
                "oms",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(3000, BalanceDirection::Debit, vec![("transaction_id", Some("test_value_1")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("invalid_match_rules_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Mismatched, 30.00, BalanceDirection::Debit), (EntryStatus::Expected, 70.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::DataMismatch)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp2", "order_123")])
                .build(),
                "psp1",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(7000, BalanceDirection::Debit, vec![("transaction_id", Some("test_value_2")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("match_rules_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Mismatched, 30.00, BalanceDirection::Debit), (EntryStatus::Pending, 70.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::DataMismatch)
                .expected_search_expectations(vec![])
                .build(),
                "psp2",
        ),
    ]
)]
// Case 8: One To Many Single Single: Status Priority Check (OverAmount > DataMismatch)
#[case(
    ReconRuleEngine::OneToManySingleSingleV1(("oms".to_string(), vec![("psp1".to_string(), 30.0), ("psp2".to_string(), 70.0)])),
    ReconRuleCustomConfig::default(),
    vec![
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(10000, BalanceDirection::Credit, vec![("transaction_id", Some("test_value")), ("match_rules_field_1", Some("match_rules_value"))], StagingEntryProcessingMode::Transaction),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Expected, 30.00, BalanceDirection::Debit), (EntryStatus::Expected, 70.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp1", "order_123"), ExpectedSearchExpectation::new("psp2", "order_123")])
                .build(),
                "oms",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(3000, BalanceDirection::Debit, vec![("transaction_id", Some("test_value_1")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("invalid_match_rules_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Mismatched, 30.00, BalanceDirection::Debit), (EntryStatus::Expected, 70.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::DataMismatch)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp2", "order_123")])
                .build(),
                "psp1",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(8000, BalanceDirection::Debit, vec![("transaction_id", Some("test_value_2")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("match_rules_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Mismatched, 30.00, BalanceDirection::Debit), (EntryStatus::Mismatched, 80.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::OverAmount(TransactionAmountMismatchData::Mismatch))
                .expected_search_expectations(vec![])
                .build(),
                "psp2",
        ),
    ]
)]
// Case 9: One To Many Single Single: Three Targets All Match
#[case(
    ReconRuleEngine::OneToManySingleSingleV1(("oms".to_string(), vec![("psp1".to_string(), 20.0), ("psp2".to_string(), 30.0), ("psp3".to_string(), 50.0)])),
    ReconRuleCustomConfig::default(),
    vec![
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(10000, BalanceDirection::Credit, vec![("transaction_id", Some("test_value")), ("match_rules_field_1", Some("match_rules_value"))], StagingEntryProcessingMode::Transaction),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Expected, 20.00, BalanceDirection::Debit), (EntryStatus::Expected, 30.00, BalanceDirection::Debit), (EntryStatus::Expected, 50.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp1", "order_123"), ExpectedSearchExpectation::new("psp2", "order_123"), ExpectedSearchExpectation::new("psp3", "order_123")])
                .build(),
                "oms",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(2000, BalanceDirection::Debit, vec![("transaction_id", Some("test_value_1")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("match_rules_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Pending, 20.00, BalanceDirection::Debit), (EntryStatus::Expected, 30.00, BalanceDirection::Debit), (EntryStatus::Expected, 50.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Expected))
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp2", "order_123"), ExpectedSearchExpectation::new("psp3", "order_123")])
                .build(),
                "psp1",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(3000, BalanceDirection::Debit, vec![("transaction_id", Some("test_value_2")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("match_rules_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Pending, 20.00, BalanceDirection::Debit), (EntryStatus::Pending, 30.00, BalanceDirection::Debit), (EntryStatus::Expected, 50.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Expected))
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp3", "order_123")])
                .build(),
                "psp2",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(5000, BalanceDirection::Debit, vec![("transaction_id", Some("test_value_3")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("match_rules_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Matched, 100.00, BalanceDirection::Credit), (EntryStatus::Matched, 20.00, BalanceDirection::Debit), (EntryStatus::Matched, 30.00, BalanceDirection::Debit), (EntryStatus::Matched, 50.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Matched(TransactionMatchedData::Auto))
                .expected_search_expectations(vec![])
                .build(),
                "psp3",
        ),
    ]
)]
// Case 10: One To Many Single Single: Debit Source (Refund Split)
#[case(
    ReconRuleEngine::OneToManySingleSingleV1(("oms".to_string(), vec![("psp1".to_string(), 40.0), ("psp2".to_string(), 60.0)])),
    ReconRuleCustomConfig::default(),
    vec![
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(5000, BalanceDirection::Debit, vec![("transaction_id", Some("test_value")), ("match_rules_field_1", Some("match_rules_value"))], StagingEntryProcessingMode::Transaction),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 50.00, BalanceDirection::Debit), (EntryStatus::Expected, 20.00, BalanceDirection::Credit), (EntryStatus::Expected, 30.00, BalanceDirection::Credit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp1", "order_123"), ExpectedSearchExpectation::new("psp2", "order_123")])
                .build(),
                "oms",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(2000, BalanceDirection::Credit, vec![("transaction_id", Some("psp_refund_1")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("match_rules_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 50.00, BalanceDirection::Debit), (EntryStatus::Pending, 20.00, BalanceDirection::Credit), (EntryStatus::Expected, 30.00, BalanceDirection::Credit)])
                .expected_transaction_status(ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Expected))
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp2", "order_123")])
                .build(),
                "psp1",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(3000, BalanceDirection::Credit, vec![("transaction_id", Some("psp_refund_2")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("match_rules_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Matched, 50.00, BalanceDirection::Debit), (EntryStatus::Matched, 20.00, BalanceDirection::Credit), (EntryStatus::Matched, 30.00, BalanceDirection::Credit)])
                .expected_transaction_status(ApiTransactionStatus::Matched(TransactionMatchedData::Auto))
                .expected_search_expectations(vec![])
                .build(),
                "psp2",
        ),
    ]
)]
// Case 11: One To Many Single Single: Small Amounts Split
#[case(
    ReconRuleEngine::OneToManySingleSingleV1(("oms".to_string(), vec![("psp1".to_string(), 50.0), ("psp2".to_string(), 50.0)])),
    ReconRuleCustomConfig::default(),
    vec![
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(100, BalanceDirection::Credit, vec![("transaction_id", Some("test_value")), ("match_rules_field_1", Some("match_rules_value"))], StagingEntryProcessingMode::Transaction),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 1.00, BalanceDirection::Credit), (EntryStatus::Expected, 0.50, BalanceDirection::Debit), (EntryStatus::Expected, 0.50, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp1", "order_123"), ExpectedSearchExpectation::new("psp2", "order_123")])
                .build(),
                "oms",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(50, BalanceDirection::Debit, vec![("transaction_id", Some("micro_psp_1")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("match_rules_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 1.00, BalanceDirection::Credit), (EntryStatus::Pending, 0.50, BalanceDirection::Debit), (EntryStatus::Expected, 0.50, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Expected))
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp2", "order_123")])
                .build(),
                "psp1",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(50, BalanceDirection::Debit, vec![("transaction_id", Some("micro_psp_2")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("match_rules_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Matched, 1.00, BalanceDirection::Credit), (EntryStatus::Matched, 0.50, BalanceDirection::Debit), (EntryStatus::Matched, 0.50, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Matched(TransactionMatchedData::Auto))
                .expected_search_expectations(vec![])
                .build(),
                "psp2",
        ),
    ]
)]
// Case 12: One To Many Single Single: All Targets with Data Mismatch
#[case(
    ReconRuleEngine::OneToManySingleSingleV1(("oms".to_string(), vec![("psp1".to_string(), 50.0), ("psp2".to_string(), 50.0)])),
    ReconRuleCustomConfig::default(),
    vec![
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(10000, BalanceDirection::Credit, vec![("transaction_id", Some("test_value")), ("match_rules_field_1", Some("match_rules_value"))], StagingEntryProcessingMode::Transaction),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Expected, 50.00, BalanceDirection::Debit), (EntryStatus::Expected, 50.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp1", "order_123"), ExpectedSearchExpectation::new("psp2", "order_123")])
                .build(),
                "oms",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(5000, BalanceDirection::Debit, vec![("transaction_id", Some("test_value_1")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("wrong_value_1"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Mismatched, 50.00, BalanceDirection::Debit), (EntryStatus::Expected, 50.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::DataMismatch)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp2", "order_123")])
                .build(),
                "psp1",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(5000, BalanceDirection::Debit, vec![("transaction_id", Some("test_value_2")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("wrong_value_2"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Mismatched, 50.00, BalanceDirection::Debit), (EntryStatus::Mismatched, 50.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::DataMismatch)
                .expected_search_expectations(vec![])
                .build(),
                "psp2",
        ),
    ]
)]
// Case 13: One To Many Single Single: Large Amount with Multiple Targets
#[case(
    ReconRuleEngine::OneToManySingleSingleV1(("oms".to_string(), vec![("psp1".to_string(), 25.0), ("psp2".to_string(), 75.0)])),
    ReconRuleCustomConfig::default(),
    vec![
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(100000000, BalanceDirection::Credit, vec![("transaction_id", Some("test_value")), ("match_rules_field_1", Some("match_rules_value"))], StagingEntryProcessingMode::Transaction),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 1000000.00, BalanceDirection::Credit), (EntryStatus::Expected, 250000.00, BalanceDirection::Debit), (EntryStatus::Expected, 750000.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp1", "order_123"), ExpectedSearchExpectation::new("psp2", "order_123")])
                .build(),
                "oms",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(25000000, BalanceDirection::Debit, vec![("transaction_id", Some("large_psp_1")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("match_rules_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 1000000.00, BalanceDirection::Credit), (EntryStatus::Pending, 250000.00, BalanceDirection::Debit), (EntryStatus::Expected, 750000.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Expected))
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp2", "order_123")])
                .build(),
                "psp1",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(75000000, BalanceDirection::Debit, vec![("transaction_id", Some("large_psp_2")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("match_rules_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Matched, 1000000.00, BalanceDirection::Credit), (EntryStatus::Matched, 250000.00, BalanceDirection::Debit), (EntryStatus::Matched, 750000.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Matched(TransactionMatchedData::Auto))
                .expected_search_expectations(vec![])
                .build(),
                "psp2",
        ),
    ]
)]
// Case 14: One To Many Single Single: UnderAmount Expected then Recovers
#[case(
    ReconRuleEngine::OneToManySingleSingleV1(("oms".to_string(), vec![("psp".to_string(), 100.0)])),
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
        // Under amount confirmation
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(8000, BalanceDirection::Debit, vec![("transaction_id", Some("test_value_1")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("match_rules_value"))], StagingEntryProcessingMode::Confirmation),
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
// Case 15: Composite search identifier across multiple targets (30/70 split) - Happy Case
#[case(
    ReconRuleEngine::OneToManySingleSingleV1(("oms".to_string(), vec![("psp1".to_string(), 30.0), ("psp2".to_string(), 70.0)])),
    ReconRuleCustomConfig::default()
        .set_search_identifier("psp1", composite_search_identifier())
        .set_search_identifier("psp2", composite_search_identifier()),
    vec![
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(10000, BalanceDirection::Credit, vec![("acc_id", Some("ACC-001")), ("region", Some("US")), ("match_rules_field_1", Some("match_rules_value")), ("transaction_id", Some("test_value"))], StagingEntryProcessingMode::Transaction),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Expected, 30.00, BalanceDirection::Debit), (EntryStatus::Expected, 70.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp1", "ACC-001|US"), ExpectedSearchExpectation::new("psp2", "ACC-001|US")])
                .build(),
                "oms",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(3000, BalanceDirection::Debit, vec![("acc_ref", Some("ACC-001")), ("region_ref", Some("US")), ("match_rules_field_2", Some("match_rules_value")), ("transaction_id", Some("test_value_2"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Pending, 30.00, BalanceDirection::Debit), (EntryStatus::Expected, 70.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Expected))
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp2", "ACC-001|US")])
                .build(),
                "psp1",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(7000, BalanceDirection::Debit, vec![("acc_ref", Some("ACC-001")), ("region_ref", Some("US")), ("match_rules_field_2", Some("match_rules_value")), ("transaction_id", Some("test_value_3"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Matched, 100.00, BalanceDirection::Credit), (EntryStatus::Matched, 30.00, BalanceDirection::Debit), (EntryStatus::Matched, 70.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Matched(TransactionMatchedData::Auto))
                .expected_search_expectations(vec![])
                .build(),
                "psp2",
        ),
    ]
)]
#[actix_web::test]
async fn test_one_to_many_single_single(
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
