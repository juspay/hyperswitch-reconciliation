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

// Case 1: One To One Single Many: Happy Case
#[case(
    ReconRuleEngine::OneToOneSingleManyV1(("oms".to_string(), "psp".to_string())),
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
            StagingEntryData::recon_staging_entry(3000, BalanceDirection::Debit, vec![("transaction_id", Some("test_value_1")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("match_rules_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Pending, 30.00, BalanceDirection::Debit), (EntryStatus::Expected, 70.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Expected))
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "order_123")])
                .build(),
                "psp",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(7000, BalanceDirection::Debit, vec![("transaction_id", Some("test_value_2")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("match_rules_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Matched, 100.00, BalanceDirection::Credit), (EntryStatus::Matched, 30.00, BalanceDirection::Debit), (EntryStatus::Matched, 70.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Matched(TransactionMatchedData::Auto))
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "order_123")])
                .build(),
                "psp",
        ),
    ]
)]
// Case 2: One To One Single Many: Metadata Mismatch
#[case(
    ReconRuleEngine::OneToOneSingleManyV1(("oms".to_string(), "psp".to_string())),
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
            StagingEntryData::recon_staging_entry(3000, BalanceDirection::Debit, vec![("transaction_id", Some("test_value_1")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("match_rules_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Pending, 30.00, BalanceDirection::Debit), (EntryStatus::Expected, 70.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Expected))
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "order_123")])
                .build(),
                "psp",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(7000, BalanceDirection::Debit, vec![("transaction_id", Some("test_value_2")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("invalid_metadata_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Pending, 30.00, BalanceDirection::Debit), (EntryStatus::Mismatched, 70.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::DataMismatch)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "order_123")])
                .build(),
                "psp",
        ),
    ]
)]
// Case 3: One To One Single Many: Same-direction target
#[case(
    ReconRuleEngine::OneToOneSingleManyV1(("oms".to_string(), "psp".to_string())),
    ReconRuleCustomConfig::default(),
    vec![
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(10000, BalanceDirection::Debit, vec![("transaction_id", Some("test_value")), ("match_rules_field_1", Some("match_rules_value"))], StagingEntryProcessingMode::Transaction),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Debit), (EntryStatus::Expected, 100.00, BalanceDirection::Credit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "order_123")])
                .build(),
                "oms",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(3000, BalanceDirection::Debit, vec![("transaction_id", Some("test_value_1")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("match_rules_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Debit), (EntryStatus::Pending, 30.00, BalanceDirection::Debit), (EntryStatus::Expected, 130.00, BalanceDirection::Credit)])
                .expected_transaction_status(ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Expected))
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "order_123")])
                .build(),
                "psp",
        ),
    ]
)]
// Case 4: One To One Single Many: Underamount Expected
#[case(
    ReconRuleEngine::OneToOneSingleManyV1(("oms".to_string(), "psp".to_string())),
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
            StagingEntryData::recon_staging_entry(3000, BalanceDirection::Debit, vec![("transaction_id", Some("test_value_1")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("match_rules_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Pending, 30.00, BalanceDirection::Debit), (EntryStatus::Expected, 70.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Expected))
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "order_123")])
                .build(),
                "psp",
        ),
    ]
)]
// Case 5: One To One Single Many: Overamount
#[case(
    ReconRuleEngine::OneToOneSingleManyV1(("oms".to_string(), "psp".to_string())),
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
            StagingEntryData::recon_staging_entry(3000, BalanceDirection::Debit, vec![("transaction_id", Some("test_value_1")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("match_rules_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Pending, 30.00, BalanceDirection::Debit), (EntryStatus::Expected, 70.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Expected))
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "order_123")])
                .build(),
                "psp",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(8000, BalanceDirection::Debit, vec![("transaction_id", Some("test_value_2")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("match_rules_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Pending, 30.00, BalanceDirection::Debit), (EntryStatus::Pending, 80.00, BalanceDirection::Debit), (EntryStatus::Expected, 10.00, BalanceDirection::Credit)])
                .expected_transaction_status(ApiTransactionStatus::OverAmount(TransactionAmountMismatchData::Expected))
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "order_123")])
                .build(),
                "psp",
        ),
    ]
)]
// Case 6: One To One Single Many: Three Targets Perfect Match
#[case(
    ReconRuleEngine::OneToOneSingleManyV1(("oms".to_string(), "psp".to_string())),
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
            StagingEntryData::recon_staging_entry(2500, BalanceDirection::Debit, vec![("transaction_id", Some("test_value_1")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("match_rules_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Pending, 25.00, BalanceDirection::Debit), (EntryStatus::Expected, 75.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Expected))
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "order_123")])
                .build(),
                "psp",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(5000, BalanceDirection::Debit, vec![("transaction_id", Some("test_value_2")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("match_rules_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Pending, 25.00, BalanceDirection::Debit), (EntryStatus::Pending, 50.00, BalanceDirection::Debit), (EntryStatus::Expected, 25.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Expected))
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "order_123")])
                .build(),
                "psp",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(2500, BalanceDirection::Debit, vec![("transaction_id", Some("test_value_3")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("match_rules_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Matched, 100.00, BalanceDirection::Credit), (EntryStatus::Matched, 25.00, BalanceDirection::Debit), (EntryStatus::Matched, 50.00, BalanceDirection::Debit), (EntryStatus::Matched, 25.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Matched(TransactionMatchedData::Auto))
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "order_123")])
                .build(),
                "psp",
        ),
    ]
)]
// Case 7: One To One Single Many: Source with Debit Direction (Refund Split)
#[case(
    ReconRuleEngine::OneToOneSingleManyV1(("oms".to_string(), "psp".to_string())),
    ReconRuleCustomConfig::default(),
    vec![
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(8000, BalanceDirection::Debit, vec![("transaction_id", Some("test_value")), ("match_rules_field_1", Some("match_rules_value"))], StagingEntryProcessingMode::Transaction),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 80.00, BalanceDirection::Debit), (EntryStatus::Expected, 80.00, BalanceDirection::Credit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "order_123")])
                .build(),
                "oms",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(5000, BalanceDirection::Credit, vec![("transaction_id", Some("psp_refund_1")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("match_rules_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 80.00, BalanceDirection::Debit), (EntryStatus::Pending, 50.00, BalanceDirection::Credit), (EntryStatus::Expected, 30.00, BalanceDirection::Credit)])
                .expected_transaction_status(ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Expected))
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "order_123")])
                .build(),
                "psp",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(3000, BalanceDirection::Credit, vec![("transaction_id", Some("psp_refund_2")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("match_rules_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Matched, 80.00, BalanceDirection::Debit), (EntryStatus::Matched, 50.00, BalanceDirection::Credit), (EntryStatus::Matched, 30.00, BalanceDirection::Credit)])
                .expected_transaction_status(ApiTransactionStatus::Matched(TransactionMatchedData::Auto))
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "order_123")])
                .build(),
                "psp",
        ),
    ]
)]
// Case 8: One To One Single Many: Combined UnderAmount and Metadata Mismatch
#[case(
    ReconRuleEngine::OneToOneSingleManyV1(("oms".to_string(), "psp".to_string())),
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
            StagingEntryData::recon_staging_entry(4000, BalanceDirection::Debit, vec![("transaction_id", Some("test_value_1")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("match_rules_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Pending, 40.00, BalanceDirection::Debit), (EntryStatus::Expected, 60.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Expected))
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "order_123")])
                .build(),
                "psp",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(5000, BalanceDirection::Debit, vec![("transaction_id", Some("test_value_2")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("wrong_metadata"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Pending, 40.00, BalanceDirection::Debit), (EntryStatus::Mismatched, 50.00, BalanceDirection::Debit), (EntryStatus::Expected, 10.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Mismatch))
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "order_123")])
                .build(),
                "psp",
        ),
    ]
)]
// Case 9: One To One Single Many: Small Amounts Split
#[case(
    ReconRuleEngine::OneToOneSingleManyV1(("oms".to_string(), "psp".to_string())),
    ReconRuleCustomConfig::default(),
    vec![
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(100, BalanceDirection::Credit, vec![("transaction_id", Some("test_value")), ("match_rules_field_1", Some("match_rules_value"))], StagingEntryProcessingMode::Transaction),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 1.00, BalanceDirection::Credit), (EntryStatus::Expected, 1.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "order_123")])
                .build(),
                "oms",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(50, BalanceDirection::Debit, vec![("transaction_id", Some("micro_psp_1")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("match_rules_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 1.00, BalanceDirection::Credit), (EntryStatus::Pending, 0.50, BalanceDirection::Debit), (EntryStatus::Expected, 0.50, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Expected))
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "order_123")])
                .build(),
                "psp",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(50, BalanceDirection::Debit, vec![("transaction_id", Some("micro_psp_2")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("match_rules_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Matched, 1.00, BalanceDirection::Credit), (EntryStatus::Matched, 0.50, BalanceDirection::Debit), (EntryStatus::Matched, 0.50, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Matched(TransactionMatchedData::Auto))
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "order_123")])
                .build(),
                "psp",
        ),
    ]
)]
// Case 10: One To One Single Many: Late target after full Matched
#[case(
    ReconRuleEngine::OneToOneSingleManyV1(("oms".to_string(), "psp".to_string())),
    ReconRuleCustomConfig::default(),
    vec![
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(1000, BalanceDirection::Credit, vec![("transaction_id", Some("test_value")), ("match_rules_field_1", Some("match_rules_value"))], StagingEntryProcessingMode::Transaction),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![
                    (EntryStatus::Pending, 10.00, BalanceDirection::Credit),
                    (EntryStatus::Expected, 10.00, BalanceDirection::Debit),
                ])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "order_123")])
                .build(),
                "oms",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(1000, BalanceDirection::Debit, vec![("transaction_id", Some("test_value_1")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("match_rules_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![
                    (EntryStatus::Matched, 10.00, BalanceDirection::Credit),
                    (EntryStatus::Matched, 10.00, BalanceDirection::Debit),
                ])
                .expected_transaction_status(ApiTransactionStatus::Matched(TransactionMatchedData::Auto))
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "order_123")])
                .build(),
                "psp",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(1000, BalanceDirection::Credit, vec![("transaction_id", Some("test_value_2")), ("reference_id", Some("order_123")), ("match_rules_field_2", Some("match_rules_value"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![
                    (EntryStatus::Pending, 10.00, BalanceDirection::Credit),
                    (EntryStatus::Pending, 10.00, BalanceDirection::Debit),
                    (EntryStatus::Pending, 10.00, BalanceDirection::Credit),
                    (EntryStatus::Expected, 10.00, BalanceDirection::Debit),
                ])
                .expected_transaction_status(ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Expected))
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "order_123")])
                .build(),
                "psp",
        ),
    ]
)]
// Case 11: Composite Search Identifier - Multiple target splits matched against composite key
#[case(
    ReconRuleEngine::OneToOneSingleManyV1(("oms".to_string(), "psp".to_string())),
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
            StagingEntryData::recon_staging_entry(4000, BalanceDirection::Debit, vec![("acc_ref", Some("ACC-001")), ("region_ref", Some("US")), ("match_rules_field_2", Some("match_rules_value")), ("transaction_id", Some("test_value_2"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 100.00, BalanceDirection::Credit), (EntryStatus::Pending, 40.00, BalanceDirection::Debit), (EntryStatus::Expected, 60.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Expected))
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "ACC-001|US")])
                .build(),
                "psp",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(6000, BalanceDirection::Debit, vec![("acc_ref", Some("ACC-001")), ("region_ref", Some("US")), ("match_rules_field_2", Some("match_rules_value")), ("transaction_id", Some("test_value_3"))], StagingEntryProcessingMode::Confirmation),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Matched, 100.00, BalanceDirection::Credit), (EntryStatus::Matched, 40.00, BalanceDirection::Debit), (EntryStatus::Matched, 60.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Matched(TransactionMatchedData::Auto))
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "ACC-001|US")])
                .build(),
                "psp",
        ),
    ]
)]
#[actix_web::test]
async fn test_one_to_one_single_many(
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
