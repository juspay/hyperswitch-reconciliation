use crate::api::recon_engine_test::{
    run_recon_engine_test,
    types::{
        ExpectedSearchExpectation, ReconRuleCustomConfig, ReconRuleEngine, ReconTestEntry,
        ReconTestEntryExpectedResult,
        ReconTestEntryExpectedResultBuilder, StagingEntryData,
    },
};
use api_models::{staging_entry::ApiStagingEntryCoarseStatus, transaction::ApiTransactionStatus};
use common_enums::{BalanceDirection, EntryStatus, SearchGroupStatus, StagingEntryProcessingMode};
use common_types::{
    recon_rule::{
        match_rules::{MatchOperator, MatchRuleV1, MatchRules, MatchRulesV1},
        strategies::one_to_one_strategy::{ManySingleCompletionPolicy, SourceAccumulationMode},
    },
    transaction::status::{TransactionAmountMismatchData, TransactionMatchedData},
};
use rstest::rstest;

#[rstest]
// Case No. 1: KeepOpen accepts a late source into the matched transaction.
#[case::keep_open(
    ManySingleCompletionPolicy::KeepOpen,
    SearchGroupStatus::Active,
    ReconTestEntryExpectedResultBuilder::new()
        .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
        .expected_entries_data(vec![(EntryStatus::Pending, 6.00, BalanceDirection::Credit), (EntryStatus::Pending, 4.00, BalanceDirection::Credit), (EntryStatus::Pending, 5.00, BalanceDirection::Credit), (EntryStatus::Pending, 10.00, BalanceDirection::Debit)])
        .expected_transaction_status(ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Expected))
        .expected_search_group_status(SearchGroupStatus::Active)
        .expected_search_expectations(vec![])
        .build(),
)]
// Case No. 2: CloseOnTarget routes a late source into a new transaction.
#[case::close_on_target(
    ManySingleCompletionPolicy::CloseOnTarget,
    SearchGroupStatus::Archived,
    ReconTestEntryExpectedResultBuilder::new()
        .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
        .expected_entries_data(vec![(EntryStatus::Pending, 5.00, BalanceDirection::Credit), (EntryStatus::Expected, 5.00, BalanceDirection::Debit)])
        .expected_transaction_status(ApiTransactionStatus::Expected)
        .expected_transaction_version(1)
        .expected_search_group_status(SearchGroupStatus::Active)
        .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-LATE")])
        .build(),
)]
#[actix_web::test]
async fn test_one_to_one_many_single_completion_policy_late_source(
    #[case] policy: ManySingleCompletionPolicy,
    #[case] group_after_target: SearchGroupStatus,
    #[case] late_source_result: ReconTestEntryExpectedResult,
    #[values(SourceAccumulationMode::Versioned, SourceAccumulationMode::IncrementalWhileExpected)]
    accumulation_mode: SourceAccumulationMode,
) {
    let source_version = match accumulation_mode {
        SourceAccumulationMode::Versioned => 2,
        SourceAccumulationMode::IncrementalWhileExpected => 1,
    };
    run_recon_engine_test(
        ReconRuleEngine::OneToOneManySingleV1(("oms".to_string(), "psp".to_string())),
        ReconRuleCustomConfig::default()
            .set_completion_policy(policy)
            .set_source_accumulation_mode(accumulation_mode),
        vec![
            ReconTestEntry::new(
                StagingEntryData::recon_staging_entry(600, BalanceDirection::Credit, vec![("transaction_id", Some("ORD-1")), ("batch_id", Some("BATCH-LATE"))], StagingEntryProcessingMode::Transaction),
                ReconTestEntryExpectedResultBuilder::new()
                    .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                    .expected_entries_data(vec![(EntryStatus::Pending, 6.00, BalanceDirection::Credit), (EntryStatus::Expected, 6.00, BalanceDirection::Debit)])
                    .expected_transaction_status(ApiTransactionStatus::Expected)
                    .expected_transaction_version(1)
                    .expected_search_group_status(SearchGroupStatus::Active)
                    .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-LATE")])
                    .build(),
                "oms",
            ),
            ReconTestEntry::new(
                StagingEntryData::recon_staging_entry(400, BalanceDirection::Credit, vec![("transaction_id", Some("ORD-2")), ("batch_id", Some("BATCH-LATE"))], StagingEntryProcessingMode::Transaction),
                ReconTestEntryExpectedResultBuilder::new()
                    .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                    .expected_entries_data(vec![(EntryStatus::Pending, 6.00, BalanceDirection::Credit), (EntryStatus::Pending, 4.00, BalanceDirection::Credit), (EntryStatus::Expected, 10.00, BalanceDirection::Debit)])
                    .expected_transaction_status(ApiTransactionStatus::Expected)
                    .expected_transaction_version(source_version)
                    .expected_search_group_status(SearchGroupStatus::Active)
                    .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-LATE")])
                    .build(),
                "oms",
            ),
            ReconTestEntry::new(
                StagingEntryData::recon_staging_entry(1000, BalanceDirection::Debit, vec![("transaction_id", Some("BATCH-LATE"))], StagingEntryProcessingMode::Confirmation),
                ReconTestEntryExpectedResultBuilder::new()
                    .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                    .expected_entries_data(vec![(EntryStatus::Matched, 6.00, BalanceDirection::Credit), (EntryStatus::Matched, 4.00, BalanceDirection::Credit), (EntryStatus::Matched, 10.00, BalanceDirection::Debit)])
                    .expected_transaction_status(ApiTransactionStatus::Matched(TransactionMatchedData::Auto))
                    .expected_transaction_version(source_version + 1)
                    .expected_search_group_status(group_after_target)
                    .expected_search_expectations(vec![])
                    .build(),
                "psp",
            ),
            ReconTestEntry::new(
                StagingEntryData::recon_staging_entry(500, BalanceDirection::Credit, vec![("transaction_id", Some("ORD-LATE")), ("batch_id", Some("BATCH-LATE"))], StagingEntryProcessingMode::Transaction),
                late_source_result,
                "oms",
            ),
        ],
    ).await;
}

#[rstest]
// Case No. 1: Close on target with a final under-amount mismatch.
#[case::under_amount(500, "BATCH-CLOSE", EntryStatus::Pending, ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Mismatch))]
// Case No. 2: Close on target with a final over-amount mismatch.
#[case::over_amount(1500, "BATCH-CLOSE", EntryStatus::Pending, ApiTransactionStatus::OverAmount(TransactionAmountMismatchData::Mismatch))]
// Case No. 3: Close on target with matching amounts but mismatched data.
#[case::data_mismatch(1000, "OTHER-BATCH", EntryStatus::Mismatched, ApiTransactionStatus::DataMismatch)]
// Case No. 4: Close on target with both data and under-amount mismatches.
#[case::data_and_under_amount(500, "OTHER-BATCH", EntryStatus::Mismatched, ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Mismatch))]
// Case No. 5: Close on target with both data and over-amount mismatches.
#[case::data_and_over_amount(1500, "OTHER-BATCH", EntryStatus::Mismatched, ApiTransactionStatus::OverAmount(TransactionAmountMismatchData::Mismatch))]
#[actix_web::test]
async fn test_one_to_one_many_single_close_on_target_mismatch(
    #[case] target_amount: i64,
    #[case] target_batch: &str,
    #[case] target_entry_status: EntryStatus,
    #[case] transaction_status: ApiTransactionStatus,
    #[values(SourceAccumulationMode::Versioned, SourceAccumulationMode::IncrementalWhileExpected)]
    accumulation_mode: SourceAccumulationMode,
) {
    run_recon_engine_test(
        ReconRuleEngine::OneToOneManySingleV1(("oms".to_string(), "psp".to_string())),
        ReconRuleCustomConfig::default()
            .set_completion_policy(ManySingleCompletionPolicy::CloseOnTarget)
            .set_source_accumulation_mode(accumulation_mode)
            .set_match_rules("psp", MatchRules::V1(MatchRulesV1 {
                rules: vec![MatchRuleV1 {
                    source_field: "metadata.batch_id".parse().unwrap(),
                    target_field: "metadata.batch_id".parse().unwrap(),
                    operator: MatchOperator::Equals,
                    label: None,
                }],
            })),
        vec![
            ReconTestEntry::new(
                StagingEntryData::recon_staging_entry(1000, BalanceDirection::Credit, vec![("transaction_id", Some("ORD-1")), ("batch_id", Some("BATCH-CLOSE"))], StagingEntryProcessingMode::Transaction),
                ReconTestEntryExpectedResultBuilder::new()
                    .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                    .expected_entries_data(vec![(EntryStatus::Pending, 10.00, BalanceDirection::Credit), (EntryStatus::Expected, 10.00, BalanceDirection::Debit)])
                    .expected_transaction_status(ApiTransactionStatus::Expected)
                    .expected_search_group_status(SearchGroupStatus::Active)
                    .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-CLOSE")])
                    .build(),
                "oms",
            ),
            ReconTestEntry::new(
                StagingEntryData::recon_staging_entry(target_amount, BalanceDirection::Debit, vec![("transaction_id", Some("BATCH-CLOSE")), ("batch_id", Some(target_batch))], StagingEntryProcessingMode::Confirmation),
                ReconTestEntryExpectedResultBuilder::new()
                    .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                    .expected_entries_data(vec![(EntryStatus::Pending, 10.00, BalanceDirection::Credit), (target_entry_status, target_amount as f64 / 100.0, BalanceDirection::Debit)])
                    .expected_transaction_status(transaction_status)
                    .expected_transaction_version(2)
                    .expected_search_group_status(SearchGroupStatus::Archived)
                    .expected_search_expectations(vec![])
                    .build(),
                "psp",
            ),
        ],
    ).await;
}

