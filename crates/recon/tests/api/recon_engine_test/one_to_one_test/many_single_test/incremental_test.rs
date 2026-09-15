use crate::api::recon_engine_test::{
    run_recon_engine_test,
    types::{
        ExpectedSearchExpectation, ReconRuleCustomConfig, ReconRuleEngine, ReconTestEntry,
        ReconTestEntryExpectedResultBuilder, StagingEntryData,
    },
};
use api_models::{staging_entry::ApiStagingEntryCoarseStatus, transaction::ApiTransactionStatus};
use common_enums::{BalanceDirection, EntryStatus, SearchGroupStatus, StagingEntryProcessingMode};
use common_types::{
    recon_rule::{
        entry_field::EntryField,
        grouping_field::{CompositeGroupingField, GroupingField, GroupingFieldV1},
        search_identifier::{
            CompositeDelimiter, CompositeSearchIdentifier, SearchIdentifier, SearchIdentifierKey,
            SearchIdentifierV2,
        },
        strategies::one_to_one_strategy::SourceAccumulationMode,
    },
    transaction::status::{TransactionAmountMismatchData, TransactionMatchedData},
};
use rstest::rstest;

#[rstest]
// Notes for this test
// 1. `transaction_id` is being used a unique identifier for entries
// 2. `batch_id` is being used as grouping field for many side
// 3. Currency USD is the trigger
// 4. No match rules because of grouping fields

// Case 1: One To One Many Single: Happy Case
#[case(
    ReconRuleEngine::OneToOneManySingleV1((
        "oms".to_string(),
        "psp".to_string(),
    )),
    ReconRuleCustomConfig::default().set_source_accumulation_mode(SourceAccumulationMode::IncrementalWhileExpected),
    vec![
       ReconTestEntry::new(StagingEntryData::recon_staging_entry(1000, BalanceDirection::Credit, vec![("transaction_id", Some("ORD-001")), ("batch_id", Some("BATCH-001"))], StagingEntryProcessingMode::Transaction),  ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 10.00, BalanceDirection::Credit), (EntryStatus::Expected, 10.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_transaction_version(1)
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-001")])
                .build(),
                "oms"),
         ReconTestEntry::new(StagingEntryData::recon_staging_entry(1000, BalanceDirection::Debit, vec![("transaction_id", Some("REF-001")), ("batch_id", Some("BATCH-001"))], StagingEntryProcessingMode::Transaction),  ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 10.00, BalanceDirection::Credit), (EntryStatus::Pending, 10.00, BalanceDirection::Debit), (EntryStatus::Expected, 0.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_transaction_version(1)
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-001")])
                .build(),
                "oms"),
        ReconTestEntry::new(StagingEntryData::recon_staging_entry(1000, BalanceDirection::Credit, vec![("transaction_id", Some("ORD-002")), ("batch_id", Some("BATCH-001"))], StagingEntryProcessingMode::Transaction),  ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 10.00, BalanceDirection::Credit), (EntryStatus::Pending, 10.00, BalanceDirection::Debit), (EntryStatus::Pending, 10.00, BalanceDirection::Credit), (EntryStatus::Expected, 10.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_transaction_version(1)
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-001")])
                .build(),
                "oms"),
         ReconTestEntry::new(StagingEntryData::recon_staging_entry(1000, BalanceDirection::Debit, vec![("transaction_id", Some("BATCH-001")), ("batch_id", Some("BATCH-001"))], StagingEntryProcessingMode::Confirmation),  ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Matched, 10.00, BalanceDirection::Credit), (EntryStatus::Matched, 10.00, BalanceDirection::Debit), (EntryStatus::Matched, 10.00, BalanceDirection::Credit), (EntryStatus::Matched, 10.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Matched(TransactionMatchedData::Auto))
                .expected_transaction_version(2)
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![])
                .build(),
                "psp"),
    ],
)]
// Case 2: One To One Many Single: Balance Direction Mismatch
#[case(
    ReconRuleEngine::OneToOneManySingleV1((
        "oms".to_string(),
        "psp".to_string(),
    )),
    ReconRuleCustomConfig::default().set_source_accumulation_mode(SourceAccumulationMode::IncrementalWhileExpected),
    vec![
       ReconTestEntry::new(StagingEntryData::recon_staging_entry(1000, BalanceDirection::Credit, vec![("transaction_id", Some("ORD-001")), ("batch_id", Some("BATCH-001"))], StagingEntryProcessingMode::Transaction),  ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 10.00, BalanceDirection::Credit), (EntryStatus::Expected, 10.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-001")])
                .build(),
                "oms"),
        ReconTestEntry::new(StagingEntryData::recon_staging_entry(1000, BalanceDirection::Credit, vec![("transaction_id", Some("ORD-002")), ("batch_id", Some("BATCH-001"))], StagingEntryProcessingMode::Transaction),  ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 10.00, BalanceDirection::Credit), (EntryStatus::Pending, 10.00, BalanceDirection::Credit), (EntryStatus::Expected, 20.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-001")])
                .build(),
                "oms"),
         ReconTestEntry::new(StagingEntryData::recon_staging_entry(2000, BalanceDirection::Credit, vec![("transaction_id", Some("BATCH-001")), ("batch_id", Some("BATCH-001"))], StagingEntryProcessingMode::Confirmation),  ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 10.00, BalanceDirection::Credit), (EntryStatus::Pending, 10.00, BalanceDirection::Credit), (EntryStatus::Pending, 20.00, BalanceDirection::Credit)])
                .expected_transaction_status(ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Expected))
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![])
                .build(),
                "psp"),
    ],
)]
// Case 3: One To One Many Single: UnderAmount Expected
#[case(
    ReconRuleEngine::OneToOneManySingleV1((
        "oms".to_string(),
        "psp".to_string(),
    )),
    ReconRuleCustomConfig::default().set_source_accumulation_mode(SourceAccumulationMode::IncrementalWhileExpected),
    vec![
       ReconTestEntry::new(StagingEntryData::recon_staging_entry(1000, BalanceDirection::Credit, vec![("transaction_id", Some("ORD-001")), ("batch_id", Some("BATCH-001"))], StagingEntryProcessingMode::Transaction),  ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 10.00, BalanceDirection::Credit), (EntryStatus::Expected, 10.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-001")])
                .build(),
                "oms"),
        ReconTestEntry::new(StagingEntryData::recon_staging_entry(1000, BalanceDirection::Credit, vec![("transaction_id", Some("ORD-002")), ("batch_id", Some("BATCH-001"))], StagingEntryProcessingMode::Transaction),  ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 10.00, BalanceDirection::Credit), (EntryStatus::Pending, 10.00, BalanceDirection::Credit), (EntryStatus::Expected, 20.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-001")])
                .build(),
                "oms"),
         ReconTestEntry::new(StagingEntryData::recon_staging_entry(1500, BalanceDirection::Debit, vec![("transaction_id", Some("BATCH-001")), ("batch_id", Some("BATCH-001"))], StagingEntryProcessingMode::Confirmation),  ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 10.00, BalanceDirection::Credit), (EntryStatus::Pending, 10.00, BalanceDirection::Credit), (EntryStatus::Pending, 15.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Expected))
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![])
                .build(),
                "psp"),
    ],
)]
// Case 4: One To One Many Single: OverAmount Expected
#[case(
    ReconRuleEngine::OneToOneManySingleV1((
        "oms".to_string(),
        "psp".to_string(),
    )),
    ReconRuleCustomConfig::default().set_source_accumulation_mode(SourceAccumulationMode::IncrementalWhileExpected),
    vec![
       ReconTestEntry::new(StagingEntryData::recon_staging_entry(1000, BalanceDirection::Credit, vec![("transaction_id", Some("ORD-001")), ("batch_id", Some("BATCH-001"))], StagingEntryProcessingMode::Transaction),  ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 10.00, BalanceDirection::Credit), (EntryStatus::Expected, 10.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-001")])
                .build(),
                "oms"),
        ReconTestEntry::new(StagingEntryData::recon_staging_entry(1000, BalanceDirection::Credit, vec![("transaction_id", Some("ORD-002")), ("batch_id", Some("BATCH-001"))], StagingEntryProcessingMode::Transaction),  ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 10.00, BalanceDirection::Credit), (EntryStatus::Pending, 10.00, BalanceDirection::Credit), (EntryStatus::Expected, 20.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-001")])
                .build(),
                "oms"),
         ReconTestEntry::new(StagingEntryData::recon_staging_entry(2500, BalanceDirection::Debit, vec![("transaction_id", Some("BATCH-001")), ("batch_id", Some("BATCH-001"))], StagingEntryProcessingMode::Confirmation),  ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 10.00, BalanceDirection::Credit), (EntryStatus::Pending, 10.00, BalanceDirection::Credit), (EntryStatus::Pending, 25.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::OverAmount(TransactionAmountMismatchData::Expected))
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![])
                .build(),
                "psp"),
    ],
)]
// Case 5: One To One Many Single: Target Arrives First
#[case(
    ReconRuleEngine::OneToOneManySingleV1((
        "oms".to_string(),
        "psp".to_string(),
    )),
    ReconRuleCustomConfig::default().set_source_accumulation_mode(SourceAccumulationMode::IncrementalWhileExpected),
    vec![
       ReconTestEntry::new(StagingEntryData::recon_staging_entry(1000, BalanceDirection::Credit, vec![("transaction_id", Some("ORD-001")), ("batch_id", Some("BATCH-001"))], StagingEntryProcessingMode::Transaction),  ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 10.00, BalanceDirection::Credit), (EntryStatus::Expected, 10.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-001")])
                .build(),
                "oms"),
        ReconTestEntry::new(StagingEntryData::recon_staging_entry(2500, BalanceDirection::Debit, vec![("transaction_id", Some("BATCH-001")), ("batch_id", Some("BATCH-001"))], StagingEntryProcessingMode::Confirmation),  ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 10.00, BalanceDirection::Credit), (EntryStatus::Pending, 25.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::OverAmount(TransactionAmountMismatchData::Expected))
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![])
                .build(),
                "psp"),
        ReconTestEntry::new(StagingEntryData::recon_staging_entry(1500, BalanceDirection::Credit, vec![("transaction_id", Some("ORD-002")), ("batch_id", Some("BATCH-001"))], StagingEntryProcessingMode::Transaction),  ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Matched, 10.00, BalanceDirection::Credit), (EntryStatus::Matched, 15.00, BalanceDirection::Credit), (EntryStatus::Matched, 25.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Matched(TransactionMatchedData::Auto))
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![])
                .build(),
                "oms"),
    ],
)]
// Case 6: One To One Many Single: Multiple Sources with Exact Match Target
#[case(
    ReconRuleEngine::OneToOneManySingleV1((
        "oms".to_string(),
        "psp".to_string(),
    )),
    ReconRuleCustomConfig::default().set_source_accumulation_mode(SourceAccumulationMode::IncrementalWhileExpected),
    vec![
       ReconTestEntry::new(StagingEntryData::recon_staging_entry(2500, BalanceDirection::Credit, vec![("transaction_id", Some("ORD-001")), ("batch_id", Some("BATCH-001"))], StagingEntryProcessingMode::Transaction),  ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 25.00, BalanceDirection::Credit), (EntryStatus::Expected, 25.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-001")])
                .build(),
                "oms"),
        ReconTestEntry::new(StagingEntryData::recon_staging_entry(2500, BalanceDirection::Credit, vec![("transaction_id", Some("ORD-002")), ("batch_id", Some("BATCH-001"))], StagingEntryProcessingMode::Transaction),  ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 25.00, BalanceDirection::Credit), (EntryStatus::Pending, 25.00, BalanceDirection::Credit), (EntryStatus::Expected, 50.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-001")])
                .build(),
                "oms"),
        ReconTestEntry::new(StagingEntryData::recon_staging_entry(2500, BalanceDirection::Credit, vec![("transaction_id", Some("ORD-003")), ("batch_id", Some("BATCH-001"))], StagingEntryProcessingMode::Transaction),  ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 25.00, BalanceDirection::Credit), (EntryStatus::Pending, 25.00, BalanceDirection::Credit), (EntryStatus::Pending, 25.00, BalanceDirection::Credit), (EntryStatus::Expected, 75.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-001")])
                .build(),
                "oms"),
        ReconTestEntry::new(StagingEntryData::recon_staging_entry(2500, BalanceDirection::Credit, vec![("transaction_id", Some("ORD-004")), ("batch_id", Some("BATCH-001"))], StagingEntryProcessingMode::Transaction),  ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 25.00, BalanceDirection::Credit), (EntryStatus::Pending, 25.00, BalanceDirection::Credit), (EntryStatus::Pending, 25.00, BalanceDirection::Credit), (EntryStatus::Pending, 25.00, BalanceDirection::Credit), (EntryStatus::Expected, 100.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-001")])
                .build(),
                "oms"),
         ReconTestEntry::new(StagingEntryData::recon_staging_entry(10000, BalanceDirection::Debit, vec![("transaction_id", Some("BATCH-001")), ("batch_id", Some("BATCH-001"))], StagingEntryProcessingMode::Confirmation),  ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Matched, 25.00, BalanceDirection::Credit), (EntryStatus::Matched, 25.00, BalanceDirection::Credit), (EntryStatus::Matched, 25.00, BalanceDirection::Credit), (EntryStatus::Matched, 25.00, BalanceDirection::Credit), (EntryStatus::Matched, 100.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Matched(TransactionMatchedData::Auto))
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![])
                .build(),
                "psp"),
    ],
)]
// Case 7: One To One Many Single: Mixed Credits and Debits (Refunds in batch)
#[case(
    ReconRuleEngine::OneToOneManySingleV1((
        "oms".to_string(),
        "psp".to_string(),
    )),
    ReconRuleCustomConfig::default().set_source_accumulation_mode(SourceAccumulationMode::IncrementalWhileExpected),
    vec![
       ReconTestEntry::new(StagingEntryData::recon_staging_entry(5000, BalanceDirection::Credit, vec![("transaction_id", Some("ORD-001")), ("batch_id", Some("BATCH-001"))], StagingEntryProcessingMode::Transaction),  ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 50.00, BalanceDirection::Credit), (EntryStatus::Expected, 50.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-001")])
                .build(),
                "oms"),
        ReconTestEntry::new(StagingEntryData::recon_staging_entry(3000, BalanceDirection::Credit, vec![("transaction_id", Some("ORD-002")), ("batch_id", Some("BATCH-001"))], StagingEntryProcessingMode::Transaction),  ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 50.00, BalanceDirection::Credit), (EntryStatus::Pending, 30.00, BalanceDirection::Credit), (EntryStatus::Expected, 80.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-001")])
                .build(),
                "oms"),
        // Refund entry (Debit on source side)
        ReconTestEntry::new(StagingEntryData::recon_staging_entry(2000, BalanceDirection::Debit, vec![("transaction_id", Some("REF-001")), ("batch_id", Some("BATCH-001"))], StagingEntryProcessingMode::Transaction),  ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 50.00, BalanceDirection::Credit), (EntryStatus::Pending, 30.00, BalanceDirection::Credit), (EntryStatus::Pending, 20.00, BalanceDirection::Debit), (EntryStatus::Expected, 60.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-001")])
                .build(),
                "oms"),
         // Target confirms net amount
         ReconTestEntry::new(StagingEntryData::recon_staging_entry(6000, BalanceDirection::Debit, vec![("transaction_id", Some("BATCH-001")), ("batch_id", Some("BATCH-001"))], StagingEntryProcessingMode::Confirmation),  ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Matched, 50.00, BalanceDirection::Credit), (EntryStatus::Matched, 30.00, BalanceDirection::Credit), (EntryStatus::Matched, 20.00, BalanceDirection::Debit), (EntryStatus::Matched, 60.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Matched(TransactionMatchedData::Auto))
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![])
                .build(),
                "psp"),
    ],
)]
// Case 8: One To One Many Single: Small Amounts
#[case(
    ReconRuleEngine::OneToOneManySingleV1((
        "oms".to_string(),
        "psp".to_string(),
    )),
    ReconRuleCustomConfig::default().set_source_accumulation_mode(SourceAccumulationMode::IncrementalWhileExpected),
    vec![
       ReconTestEntry::new(StagingEntryData::recon_staging_entry(1, BalanceDirection::Credit, vec![("transaction_id", Some("MICRO-001")), ("batch_id", Some("MICRO-BATCH"))], StagingEntryProcessingMode::Transaction),  ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 0.01, BalanceDirection::Credit), (EntryStatus::Expected, 0.01, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "MICRO-BATCH")])
                .build(),
                "oms"),
        ReconTestEntry::new(StagingEntryData::recon_staging_entry(2, BalanceDirection::Credit, vec![("transaction_id", Some("MICRO-002")), ("batch_id", Some("MICRO-BATCH"))], StagingEntryProcessingMode::Transaction),  ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 0.01, BalanceDirection::Credit), (EntryStatus::Pending, 0.02, BalanceDirection::Credit), (EntryStatus::Expected, 0.03, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "MICRO-BATCH")])
                .build(),
                "oms"),
         ReconTestEntry::new(StagingEntryData::recon_staging_entry(3, BalanceDirection::Debit, vec![("transaction_id", Some("MICRO-BATCH")), ("batch_id", Some("MICRO-BATCH"))], StagingEntryProcessingMode::Confirmation),  ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Matched, 0.01, BalanceDirection::Credit), (EntryStatus::Matched, 0.02, BalanceDirection::Credit), (EntryStatus::Matched, 0.03, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Matched(TransactionMatchedData::Auto))
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![])
                .build(),
                "psp"),
    ],
)]
// Case 9: One To One Many Single: Multiple Sources Then Partial Target (UnderAmount)
#[case(
    ReconRuleEngine::OneToOneManySingleV1((
        "oms".to_string(),
        "psp".to_string(),
    )),
    ReconRuleCustomConfig::default().set_source_accumulation_mode(SourceAccumulationMode::IncrementalWhileExpected),
    vec![
       ReconTestEntry::new(StagingEntryData::recon_staging_entry(5000, BalanceDirection::Credit, vec![("transaction_id", Some("ORD-001")), ("batch_id", Some("BATCH-001"))], StagingEntryProcessingMode::Transaction),  ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 50.00, BalanceDirection::Credit), (EntryStatus::Expected, 50.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-001")])
                .build(),
                "oms"),
        ReconTestEntry::new(StagingEntryData::recon_staging_entry(5000, BalanceDirection::Credit, vec![("transaction_id", Some("ORD-002")), ("batch_id", Some("BATCH-001"))], StagingEntryProcessingMode::Transaction),  ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 50.00, BalanceDirection::Credit), (EntryStatus::Pending, 50.00, BalanceDirection::Credit), (EntryStatus::Expected, 100.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-001")])
                .build(),
                "oms"),
        ReconTestEntry::new(StagingEntryData::recon_staging_entry(5000, BalanceDirection::Credit, vec![("transaction_id", Some("ORD-003")), ("batch_id", Some("BATCH-001"))], StagingEntryProcessingMode::Transaction),  ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 50.00, BalanceDirection::Credit), (EntryStatus::Pending, 50.00, BalanceDirection::Credit), (EntryStatus::Pending, 50.00, BalanceDirection::Credit), (EntryStatus::Expected, 150.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-001")])
                .build(),
                "oms"),
         // Target is less than expected
         ReconTestEntry::new(StagingEntryData::recon_staging_entry(12000, BalanceDirection::Debit, vec![("transaction_id", Some("BATCH-001")), ("batch_id", Some("BATCH-001"))], StagingEntryProcessingMode::Confirmation),  ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 50.00, BalanceDirection::Credit), (EntryStatus::Pending, 50.00, BalanceDirection::Credit), (EntryStatus::Pending, 50.00, BalanceDirection::Credit), (EntryStatus::Pending, 120.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Expected))
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![])
                .build(),
                "psp"),
    ],
)]
// Case 10: One To One Many Single: Source with Debit Direction (Full Refund Batch)
#[case(
    ReconRuleEngine::OneToOneManySingleV1((
        "oms".to_string(),
        "psp".to_string(),
    )),
    ReconRuleCustomConfig::default().set_source_accumulation_mode(SourceAccumulationMode::IncrementalWhileExpected),
    vec![
       ReconTestEntry::new(StagingEntryData::recon_staging_entry(3000, BalanceDirection::Debit, vec![("transaction_id", Some("REF-001")), ("batch_id", Some("REFUND-BATCH"))], StagingEntryProcessingMode::Transaction),  ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 30.00, BalanceDirection::Debit), (EntryStatus::Expected, 30.00, BalanceDirection::Credit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "REFUND-BATCH")])
                .build(),
                "oms"),
        ReconTestEntry::new(StagingEntryData::recon_staging_entry(2000, BalanceDirection::Debit, vec![("transaction_id", Some("REF-002")), ("batch_id", Some("REFUND-BATCH"))], StagingEntryProcessingMode::Transaction),  ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 30.00, BalanceDirection::Debit), (EntryStatus::Pending, 20.00, BalanceDirection::Debit), (EntryStatus::Expected, 50.00, BalanceDirection::Credit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "REFUND-BATCH")])
                .build(),
                "oms"),
         ReconTestEntry::new(StagingEntryData::recon_staging_entry(5000, BalanceDirection::Credit, vec![("transaction_id", Some("REFUND-BATCH")), ("batch_id", Some("REFUND-BATCH"))], StagingEntryProcessingMode::Confirmation),  ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Matched, 30.00, BalanceDirection::Debit), (EntryStatus::Matched, 20.00, BalanceDirection::Debit), (EntryStatus::Matched, 50.00, BalanceDirection::Credit)])
                .expected_transaction_status(ApiTransactionStatus::Matched(TransactionMatchedData::Auto))
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![])
                .build(),
                "psp"),
    ],
)]
// Case 11: One To One Many Single: Late Source After Matched
#[case(
    ReconRuleEngine::OneToOneManySingleV1((
        "oms".to_string(),
        "psp".to_string(),
    )),
    ReconRuleCustomConfig::default().set_source_accumulation_mode(SourceAccumulationMode::IncrementalWhileExpected),
    vec![
       ReconTestEntry::new(StagingEntryData::recon_staging_entry(1000, BalanceDirection::Credit, vec![("transaction_id", Some("ORD-LATE-1")), ("batch_id", Some("BATCH-LATE"))], StagingEntryProcessingMode::Transaction),  ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 10.00, BalanceDirection::Credit), (EntryStatus::Expected, 10.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-LATE")])
                .build(),
                "oms"),
        ReconTestEntry::new(StagingEntryData::recon_staging_entry(1000, BalanceDirection::Debit, vec![("transaction_id", Some("BATCH-LATE"))], StagingEntryProcessingMode::Confirmation),  ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Matched, 10.00, BalanceDirection::Credit), (EntryStatus::Matched, 10.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Matched(TransactionMatchedData::Auto))
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![])
                .build(),
                "psp"),
        ReconTestEntry::new(StagingEntryData::recon_staging_entry(500, BalanceDirection::Credit, vec![("transaction_id", Some("ORD-LATE-2")), ("batch_id", Some("BATCH-LATE"))], StagingEntryProcessingMode::Transaction),  ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 10.00, BalanceDirection::Credit), (EntryStatus::Pending, 5.00, BalanceDirection::Credit), (EntryStatus::Pending, 10.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Expected))
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![])
                .build(),
                "oms"),
    ],
)]
// Case 12: Composite grouping + search — sources grouped by (batch_id, region), settled by a single target whose composite target value matches. Happy Case.
#[case(
    ReconRuleEngine::OneToOneManySingleV1((
        "oms".to_string(),
        "psp".to_string(),
    )),
    ReconRuleCustomConfig::default().set_source_accumulation_mode(SourceAccumulationMode::IncrementalWhileExpected)
        .set_grouping_field("oms", composite_grouping_field())
        .set_search_identifier("psp", composite_search_identifier()),
    vec![
        ReconTestEntry::new(StagingEntryData::recon_staging_entry(1000, BalanceDirection::Credit, vec![("transaction_id", Some("ORD-001")), ("batch_id", Some("BATCH-001")), ("region", Some("US"))], StagingEntryProcessingMode::Transaction),  ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 10.00, BalanceDirection::Credit), (EntryStatus::Expected, 10.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-001|US")])
                .build(),
                "oms"),
        ReconTestEntry::new(StagingEntryData::recon_staging_entry(1000, BalanceDirection::Credit, vec![("transaction_id", Some("ORD-002")), ("batch_id", Some("BATCH-001")), ("region", Some("US"))], StagingEntryProcessingMode::Transaction),  ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 10.00, BalanceDirection::Credit), (EntryStatus::Pending, 10.00, BalanceDirection::Credit), (EntryStatus::Expected, 20.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-001|US")])
                .build(),
                "oms"),
        // Single target settles both, composite target value: "BATCH-001|US"
        ReconTestEntry::new(StagingEntryData::recon_staging_entry(2000, BalanceDirection::Debit, vec![("transaction_id", Some("BATCH-001")), ("region", Some("US"))], StagingEntryProcessingMode::Confirmation),  ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Matched, 10.00, BalanceDirection::Credit), (EntryStatus::Matched, 10.00, BalanceDirection::Credit), (EntryStatus::Matched, 20.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Matched(TransactionMatchedData::Auto))
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![])
                .build(),
                "psp"),
    ],
)]
// Case 13: Incremental append flips the residual direction and a target settles it.
#[case(
    ReconRuleEngine::OneToOneManySingleV1((
        "oms".to_string(),
        "psp".to_string(),
    )),
    ReconRuleCustomConfig::default().set_source_accumulation_mode(SourceAccumulationMode::IncrementalWhileExpected),
    vec![
        ReconTestEntry::new(StagingEntryData::recon_staging_entry(1000, BalanceDirection::Credit, vec![("transaction_id", Some("ORD-FLIP-1")), ("batch_id", Some("BATCH-FLIP"))], StagingEntryProcessingMode::Transaction), ReconTestEntryExpectedResultBuilder::new()
            .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
            .expected_entries_data(vec![(EntryStatus::Pending, 10.00, BalanceDirection::Credit), (EntryStatus::Expected, 10.00, BalanceDirection::Debit)])
            .expected_transaction_status(ApiTransactionStatus::Expected)
            .expected_transaction_version(1)
            .expected_transaction_amounts(10.00, 10.00)
            .expected_search_group_status(SearchGroupStatus::Active)
            .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-FLIP")])
            .build(),
            "oms"),
        ReconTestEntry::new(StagingEntryData::recon_staging_entry(2500, BalanceDirection::Debit, vec![("transaction_id", Some("ORD-FLIP-2")), ("batch_id", Some("BATCH-FLIP"))], StagingEntryProcessingMode::Transaction), ReconTestEntryExpectedResultBuilder::new()
            .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
            .expected_entries_data(vec![(EntryStatus::Pending, 10.00, BalanceDirection::Credit), (EntryStatus::Pending, 25.00, BalanceDirection::Debit), (EntryStatus::Expected, 15.00, BalanceDirection::Credit)])
            .expected_transaction_status(ApiTransactionStatus::Expected)
            .expected_transaction_version(1)
            .expected_transaction_amounts(25.00, 25.00)
            .expected_search_group_status(SearchGroupStatus::Active)
            .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-FLIP")])
            .build(),
            "oms"),
        ReconTestEntry::new(StagingEntryData::recon_staging_entry(1500, BalanceDirection::Credit, vec![("transaction_id", Some("BATCH-FLIP")), ("batch_id", Some("BATCH-FLIP"))], StagingEntryProcessingMode::Confirmation), ReconTestEntryExpectedResultBuilder::new()
            .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
            .expected_entries_data(vec![(EntryStatus::Matched, 10.00, BalanceDirection::Credit), (EntryStatus::Matched, 25.00, BalanceDirection::Debit), (EntryStatus::Matched, 15.00, BalanceDirection::Credit)])
            .expected_transaction_status(ApiTransactionStatus::Matched(TransactionMatchedData::Auto))
            .expected_transaction_version(2)
            .expected_transaction_amounts(25.00, 25.00)
            .expected_search_group_status(SearchGroupStatus::Active)
            .expected_search_expectations(vec![])
            .build(),
            "psp"),
    ],
)]
// Case 14: Incremental appends grow and then reduce the residual to zero.
#[case(
    ReconRuleEngine::OneToOneManySingleV1((
        "oms".to_string(),
        "psp".to_string(),
    )),
    ReconRuleCustomConfig::default().set_source_accumulation_mode(SourceAccumulationMode::IncrementalWhileExpected),
    vec![
        ReconTestEntry::new(StagingEntryData::recon_staging_entry(1000, BalanceDirection::Credit, vec![("transaction_id", Some("ORD-ZERO-1")), ("batch_id", Some("BATCH-ZERO"))], StagingEntryProcessingMode::Transaction), ReconTestEntryExpectedResultBuilder::new()
            .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
            .expected_entries_data(vec![(EntryStatus::Pending, 10.00, BalanceDirection::Credit), (EntryStatus::Expected, 10.00, BalanceDirection::Debit)])
            .expected_transaction_status(ApiTransactionStatus::Expected)
            .expected_transaction_version(1)
            .expected_transaction_amounts(10.00, 10.00)
            .expected_search_group_status(SearchGroupStatus::Active)
            .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-ZERO")])
            .build(),
            "oms"),
        ReconTestEntry::new(StagingEntryData::recon_staging_entry(1000, BalanceDirection::Credit, vec![("transaction_id", Some("ORD-ZERO-2")), ("batch_id", Some("BATCH-ZERO"))], StagingEntryProcessingMode::Transaction), ReconTestEntryExpectedResultBuilder::new()
            .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
            .expected_entries_data(vec![(EntryStatus::Pending, 10.00, BalanceDirection::Credit), (EntryStatus::Pending, 10.00, BalanceDirection::Credit), (EntryStatus::Expected, 20.00, BalanceDirection::Debit)])
            .expected_transaction_status(ApiTransactionStatus::Expected)
            .expected_transaction_version(1)
            .expected_transaction_amounts(20.00, 20.00)
            .expected_search_group_status(SearchGroupStatus::Active)
            .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-ZERO")])
            .build(),
            "oms"),
        ReconTestEntry::new(StagingEntryData::recon_staging_entry(2000, BalanceDirection::Debit, vec![("transaction_id", Some("ORD-ZERO-3")), ("batch_id", Some("BATCH-ZERO"))], StagingEntryProcessingMode::Transaction), ReconTestEntryExpectedResultBuilder::new()
            .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
            .expected_entries_data(vec![(EntryStatus::Pending, 10.00, BalanceDirection::Credit), (EntryStatus::Pending, 10.00, BalanceDirection::Credit), (EntryStatus::Pending, 20.00, BalanceDirection::Debit), (EntryStatus::Expected, 0.00, BalanceDirection::Debit)])
            .expected_transaction_status(ApiTransactionStatus::Expected)
            .expected_transaction_version(1)
            .expected_transaction_amounts(20.00, 20.00)
            .expected_search_group_status(SearchGroupStatus::Active)
            .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-ZERO")])
            .build(),
            "oms"),
    ],
)]
#[actix_web::test]
async fn test_one_to_one_many_single_incremental(
    #[case] rule_type: ReconRuleEngine,
    #[case] rule_custom_config: ReconRuleCustomConfig,
    #[case] test_entries: Vec<ReconTestEntry>,
) {
    run_recon_engine_test(rule_type, rule_custom_config, test_entries).await;
}

fn composite_grouping_field() -> GroupingField {
    GroupingField::V1(GroupingFieldV1::Composite(CompositeGroupingField {
        fields: vec![
            EntryField::Metadata("batch_id".to_string()),
            EntryField::Metadata("region".to_string()),
        ],
        delimiter: CompositeDelimiter::Pipe,
    }))
}

fn composite_search_identifier() -> SearchIdentifier {
    SearchIdentifier::V2(SearchIdentifierV2::Composite(CompositeSearchIdentifier {
        keys: vec![
            SearchIdentifierKey {
                source_field: "metadata.batch_id".parse().unwrap(),
                target_field: "metadata.transaction_id".parse().unwrap(),
            },
            SearchIdentifierKey {
                source_field: "metadata.region".parse().unwrap(),
                target_field: "metadata.region".parse().unwrap(),
            },
        ],
        delimiter: CompositeDelimiter::Pipe,
    }))
}
