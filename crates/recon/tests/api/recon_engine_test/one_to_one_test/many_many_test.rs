use api_models::staging_entry::ApiStagingEntryCoarseStatus;
use api_models::transaction::ApiTransactionStatus;
use common_enums::{BalanceDirection, EntryStatus, SearchGroupStatus, StagingEntryProcessingMode};
use common_types::{
    recon_rule::{
        entry_field::EntryField,
        grouping_field::{CompositeGroupingField, GroupingField, GroupingFieldV1},
        search_identifier::{
            CompositeDelimiter, CompositeSearchIdentifier, SearchIdentifier, SearchIdentifierKey,
            SearchIdentifierV2,
        },
    },
    transaction::status::{TransactionAmountMismatchData, TransactionMatchedData},
};
use rstest::rstest;

use crate::api::recon_engine_test::types::{
    ExpectedSearchExpectation, ReconRuleCustomConfig, ReconRuleEngine, ReconTestEntry,
    ReconTestEntryExpectedResultBuilder, StagingEntryData,
};

#[rstest]
// Notes for this test
// 1. `transaction_id` is being used a unique identifier for entries
// 2. `batch_id` is being used as grouping field for many source side
// 3. `reference_id` is being used for search identifier against `batch_id`

// Case 1: One To One Many Many: Single Source Entry <-> Single Target Entry
#[case(
    ReconRuleEngine::OneToOneManyManyV1(("oms".to_string(), "psp".to_string())),
    ReconRuleCustomConfig::default(),
    vec![
        // First source entry in batch
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(
                10000,
                BalanceDirection::Credit,
                vec![
                    ("transaction_id", Some("ORD-001")),
                    ("batch_id", Some("BATCH-001")),
                ],
                StagingEntryProcessingMode::Transaction
            ),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![
                    (EntryStatus::Pending, 100.00, BalanceDirection::Credit),
                    (EntryStatus::Expected, 100.00, BalanceDirection::Debit)
                ])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-001")])
                .build(),
            "oms",
        ),
        // First target confirmation
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(
                10000,
                BalanceDirection::Debit,
                vec![
                    ("transaction_id", Some("PSP-001")),
                    ("reference_id", Some("BATCH-001")),
                ],
                StagingEntryProcessingMode::Confirmation
            ),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![
                    (EntryStatus::Matched, 100.00, BalanceDirection::Credit),
                    (EntryStatus::Matched, 100.00, BalanceDirection::Debit),
                ])
                .expected_transaction_status(ApiTransactionStatus::Matched(TransactionMatchedData::Auto))
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-001")])
                .build(),
            "psp",
        ),
    ]
)]
// Case 2: One To One Many Many: Multiple Source Entries -> Single Target Entry
#[case(
    ReconRuleEngine::OneToOneManyManyV1(("oms".to_string(), "psp".to_string())),
    ReconRuleCustomConfig::default(),
    vec![
        // First source entry
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(
                5000,
                BalanceDirection::Credit,
                vec![
                    ("transaction_id", Some("ORD-001")),
                    ("batch_id", Some("BATCH-002")),
                ],
                StagingEntryProcessingMode::Transaction
            ),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![
                    (EntryStatus::Pending, 50.00, BalanceDirection::Credit),
                    (EntryStatus::Expected, 50.00, BalanceDirection::Debit)
                ])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-002")])
                .build(),
            "oms",
        ),
        // Second source entry
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(
                3000,
                BalanceDirection::Credit,
                vec![
                    ("transaction_id", Some("ORD-002")),
                    ("batch_id", Some("BATCH-002")),
                ],
                StagingEntryProcessingMode::Transaction
            ),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![
                    (EntryStatus::Pending, 50.00, BalanceDirection::Credit),
                    (EntryStatus::Pending, 30.00, BalanceDirection::Credit),
                    (EntryStatus::Expected, 80.00, BalanceDirection::Debit)
                ])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-002")])
                .build(),
            "oms",
        ),
        // Third source entry
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(
                2000,
                BalanceDirection::Credit,
                vec![
                    ("transaction_id", Some("ORD-003")),
                    ("batch_id", Some("BATCH-002")),
                ],
                StagingEntryProcessingMode::Transaction
            ),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![
                    (EntryStatus::Pending, 50.00, BalanceDirection::Credit),
                    (EntryStatus::Pending, 30.00, BalanceDirection::Credit),
                    (EntryStatus::Pending, 20.00, BalanceDirection::Credit),
                    (EntryStatus::Expected, 100.00, BalanceDirection::Debit)
                ])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-002")])
                .build(),
            "oms",
        ),
        // Single target confirmation
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(
                10000,
                BalanceDirection::Debit,
                vec![
                    ("transaction_id", Some("PSP-BATCH-002")),
                    ("reference_id", Some("BATCH-002")),
                ],
                StagingEntryProcessingMode::Confirmation
            ),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![
                    (EntryStatus::Matched, 50.00, BalanceDirection::Credit),
                    (EntryStatus::Matched, 30.00, BalanceDirection::Credit),
                    (EntryStatus::Matched, 20.00, BalanceDirection::Credit),
                    (EntryStatus::Matched, 100.00, BalanceDirection::Debit),
                ])
                .expected_transaction_status(ApiTransactionStatus::Matched(TransactionMatchedData::Auto))
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-002")])
                .build(),
            "psp",
        ),
    ]
)]
// Case 3: One To One Many Many: Single Source Entry -> Multiple Target Entries
#[case(
    ReconRuleEngine::OneToOneManyManyV1(("oms".to_string(), "psp".to_string())),
    ReconRuleCustomConfig::default(),
    vec![
        // Single source entry
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(
                10000,
                BalanceDirection::Credit,
                vec![
                    ("transaction_id", Some("ORD-BULK-001")),
                    ("batch_id", Some("BATCH-003")),
                ],
                StagingEntryProcessingMode::Transaction
            ),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![
                    (EntryStatus::Pending, 100.00, BalanceDirection::Credit),
                    (EntryStatus::Expected, 100.00, BalanceDirection::Debit)
                ])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-003")])
                .build(),
            "oms",
        ),
        // First target entry
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(
                2500,
                BalanceDirection::Debit,
                vec![
                    ("transaction_id", Some("PSP-001")),
                    ("reference_id", Some("BATCH-003")),
                    ("batch_id", Some("PSP-BATCH-003")),
                ],
                StagingEntryProcessingMode::Confirmation
            ),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![
                    (EntryStatus::Pending, 100.00, BalanceDirection::Credit),
                    (EntryStatus::Pending, 25.00, BalanceDirection::Debit),
                    (EntryStatus::Expected, 75.00, BalanceDirection::Debit)
                ])
                .expected_transaction_status(ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Expected))
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-003")])
                .build(),
            "psp",
        ),
        // Second target entry
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(
                3500,
                BalanceDirection::Debit,
                vec![
                    ("transaction_id", Some("PSP-002")),
                    ("reference_id", Some("BATCH-003")),
                    ("batch_id", Some("PSP-BATCH-003")),
                ],
                StagingEntryProcessingMode::Confirmation
            ),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![
                    (EntryStatus::Pending, 100.00, BalanceDirection::Credit),
                    (EntryStatus::Pending, 25.00, BalanceDirection::Debit),
                    (EntryStatus::Pending, 35.00, BalanceDirection::Debit),
                    (EntryStatus::Expected, 40.00, BalanceDirection::Debit)
                ])
                .expected_transaction_status(ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Expected))
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-003")])
                .build(),
            "psp",
        ),
        // Third target entry
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(
                4000,
                BalanceDirection::Debit,
                vec![
                    ("transaction_id", Some("PSP-003")),
                    ("reference_id", Some("BATCH-003")),
                    ("batch_id", Some("PSP-BATCH-003")),
                ],
                StagingEntryProcessingMode::Confirmation
            ),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![
                    (EntryStatus::Matched, 100.00, BalanceDirection::Credit),
                    (EntryStatus::Matched, 25.00, BalanceDirection::Debit),
                    (EntryStatus::Matched, 35.00, BalanceDirection::Debit),
                    (EntryStatus::Matched, 40.00, BalanceDirection::Debit)
                ])
                .expected_transaction_status(ApiTransactionStatus::Matched(TransactionMatchedData::Auto))
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-003")])
                .build(),
            "psp",
        ),
    ]
)]
// Case 4: One To One Many Many: Multiple Sources -> Multiple Targets (Perfect Match)
#[case(
    ReconRuleEngine::OneToOneManyManyV1(("oms".to_string(), "psp".to_string())),
    ReconRuleCustomConfig::default(),
    vec![
        // First source
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(
                4000,
                BalanceDirection::Credit,
                vec![
                    ("transaction_id", Some("ORD-M1")),
                    ("batch_id", Some("BATCH-004")),
                ],
                StagingEntryProcessingMode::Transaction
            ),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![
                    (EntryStatus::Pending, 40.00, BalanceDirection::Credit),
                    (EntryStatus::Expected, 40.00, BalanceDirection::Debit)
                ])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-004")])
                .build(),
            "oms",
        ),
        // Second source
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(
                6000,
                BalanceDirection::Credit,
                vec![
                    ("transaction_id", Some("ORD-M2")),
                    ("batch_id", Some("BATCH-004")),
                ],
                StagingEntryProcessingMode::Transaction
            ),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![
                    (EntryStatus::Pending, 40.00, BalanceDirection::Credit),
                    (EntryStatus::Pending, 60.00, BalanceDirection::Credit),
                    (EntryStatus::Expected, 100.00, BalanceDirection::Debit)
                ])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-004")])
                .build(),
            "oms",
        ),
        // First target
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(
                3000,
                BalanceDirection::Debit,
                vec![
                    ("transaction_id", Some("PSP-M1")),
                    ("reference_id", Some("BATCH-004")),
                    ("batch_id", Some("PSP-BATCH-004")),
                ],
                StagingEntryProcessingMode::Confirmation
            ),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![
                    (EntryStatus::Pending, 40.00, BalanceDirection::Credit),
                    (EntryStatus::Pending, 60.00, BalanceDirection::Credit),
                    (EntryStatus::Pending, 30.00, BalanceDirection::Debit),
                    (EntryStatus::Expected, 70.00, BalanceDirection::Debit)
                ])
                .expected_transaction_status(ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Expected))
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-004")])
                .build(),
            "psp",
        ),
        // Second target
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(
                7000,
                BalanceDirection::Debit,
                vec![
                    ("transaction_id", Some("PSP-M2")),
                    ("reference_id", Some("BATCH-004")),
                    ("batch_id", Some("PSP-BATCH-004")),
                ],
                StagingEntryProcessingMode::Confirmation
            ),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![
                    (EntryStatus::Matched, 40.00, BalanceDirection::Credit),
                    (EntryStatus::Matched, 60.00, BalanceDirection::Credit),
                    (EntryStatus::Matched, 30.00, BalanceDirection::Debit),
                    (EntryStatus::Matched, 70.00, BalanceDirection::Debit)
                ])
                .expected_transaction_status(ApiTransactionStatus::Matched(TransactionMatchedData::Auto))
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-004")])
                .build(),
            "psp",
        ),
    ]
)]
// Case 5: One To One Many Many: Complex Sequence - Source, Target, Source Again
#[case(
    ReconRuleEngine::OneToOneManyManyV1(("oms".to_string(), "psp".to_string())),
    ReconRuleCustomConfig::default(),
    vec![
        // First source entry
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(
                3000,
                BalanceDirection::Credit,
                vec![
                    ("transaction_id", Some("ORD-SEQ-001")),
                    ("batch_id", Some("BATCH-005")),
                ],
                StagingEntryProcessingMode::Transaction
            ),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![
                    (EntryStatus::Pending, 30.00, BalanceDirection::Credit),
                    (EntryStatus::Expected, 30.00, BalanceDirection::Debit)
                ])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-005")])
                .build(),
            "oms",
        ),
        // First target entry
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(
                5000,
                BalanceDirection::Debit,
                vec![
                    ("transaction_id", Some("PSP-SEQ-001")),
                    ("reference_id", Some("BATCH-005")),
                    ("batch_id", Some("PSP-BATCH-005")),
                ],
                StagingEntryProcessingMode::Confirmation
            ),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![
                    (EntryStatus::Pending, 30.00, BalanceDirection::Credit),
                    (EntryStatus::Pending, 50.00, BalanceDirection::Debit),
                    (EntryStatus::Expected, 20.00, BalanceDirection::Credit)
                ])
                .expected_transaction_status(ApiTransactionStatus::OverAmount(TransactionAmountMismatchData::Expected))
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-005")])
                .build(),
            "psp",
        ),
        // Second source entry (arrives after first target)
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(
                2000,
                BalanceDirection::Credit,
                vec![
                    ("transaction_id", Some("ORD-SEQ-002")),
                    ("batch_id", Some("BATCH-005")),
                ],
                StagingEntryProcessingMode::Transaction
            ),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![
                    (EntryStatus::Matched, 30.00, BalanceDirection::Credit),
                    (EntryStatus::Matched, 20.00, BalanceDirection::Credit),
                    (EntryStatus::Matched, 50.00, BalanceDirection::Debit)
                ])
                .expected_transaction_status(ApiTransactionStatus::Matched(TransactionMatchedData::Auto))
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-005")])
                .build(),
            "oms",
        ),
    ]
)]
// Case 6: One To One Many Many: Amount Mismatch - UnderAmount
#[case(
    ReconRuleEngine::OneToOneManyManyV1(("oms".to_string(), "psp".to_string())),
    ReconRuleCustomConfig::default(),
    vec![
        // Multiple sources totaling 100
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(
                6000,
                BalanceDirection::Credit,
                vec![
                    ("transaction_id", Some("ORD-UNDER-001")),
                    ("batch_id", Some("BATCH-006")),
                ],
                StagingEntryProcessingMode::Transaction
            ),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![
                    (EntryStatus::Pending, 60.00, BalanceDirection::Credit),
                    (EntryStatus::Expected, 60.00, BalanceDirection::Debit)
                ])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-006")])
                .build(),
            "oms",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(
                4000,
                BalanceDirection::Credit,
                vec![
                    ("transaction_id", Some("ORD-UNDER-002")),
                    ("batch_id", Some("BATCH-006")),
                ],
                StagingEntryProcessingMode::Transaction
            ),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![
                    (EntryStatus::Pending, 60.00, BalanceDirection::Credit),
                    (EntryStatus::Pending, 40.00, BalanceDirection::Credit),
                    (EntryStatus::Expected, 100.00, BalanceDirection::Debit)
                ])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-006")])
                .build(),
            "oms",
        ),
        // Multiple targets totaling 80 (under by 20)
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(
                5000,
                BalanceDirection::Debit,
                vec![
                    ("transaction_id", Some("PSP-UNDER-001")),
                    ("reference_id", Some("BATCH-006")),
                    ("batch_id", Some("PSP-BATCH-006")),
                ],
                StagingEntryProcessingMode::Confirmation
            ),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![
                    (EntryStatus::Pending, 60.00, BalanceDirection::Credit),
                    (EntryStatus::Pending, 40.00, BalanceDirection::Credit),
                    (EntryStatus::Pending, 50.00, BalanceDirection::Debit),
                    (EntryStatus::Expected, 50.00, BalanceDirection::Debit)
                ])
                .expected_transaction_status(ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Expected))
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-006")])
                .build(),
            "psp",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(
                3000,
                BalanceDirection::Debit,
                vec![
                    ("transaction_id", Some("PSP-UNDER-002")),
                    ("reference_id", Some("BATCH-006")),
                    ("batch_id", Some("PSP-BATCH-006")),
                ],
                StagingEntryProcessingMode::Confirmation
            ),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![
                    (EntryStatus::Pending, 60.00, BalanceDirection::Credit),
                    (EntryStatus::Pending, 40.00, BalanceDirection::Credit),
                    (EntryStatus::Pending, 50.00, BalanceDirection::Debit),
                    (EntryStatus::Pending, 30.00, BalanceDirection::Debit),
                    (EntryStatus::Expected, 20.00, BalanceDirection::Debit)
                ])
                .expected_transaction_status(ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Expected))
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-006")])
                .build(),
            "psp",
        ),
    ]
)]
// Case 7: One To One Many Many: Amount Mismatch - OverAmount
#[case(
    ReconRuleEngine::OneToOneManyManyV1(("oms".to_string(), "psp".to_string())),
    ReconRuleCustomConfig::default(),
    vec![
        // Multiple sources totaling 80
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(
                5000,
                BalanceDirection::Credit,
                vec![
                    ("transaction_id", Some("ORD-OVER-001")),
                    ("batch_id", Some("BATCH-007")),
                ],
                StagingEntryProcessingMode::Transaction
            ),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![
                    (EntryStatus::Pending, 50.00, BalanceDirection::Credit),
                    (EntryStatus::Expected, 50.00, BalanceDirection::Debit)
                ])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-007")])
                .build(),
            "oms",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(
                3000,
                BalanceDirection::Credit,
                vec![
                    ("transaction_id", Some("ORD-OVER-002")),
                    ("batch_id", Some("BATCH-007")),
                ],
                StagingEntryProcessingMode::Transaction
            ),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![
                    (EntryStatus::Pending, 50.00, BalanceDirection::Credit),
                    (EntryStatus::Pending, 30.00, BalanceDirection::Credit),
                    (EntryStatus::Expected, 80.00, BalanceDirection::Debit)
                ])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-007")])
                .build(),
            "oms",
        ),
        // Multiple targets totaling 100 (over by 20)
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(
                6000,
                BalanceDirection::Debit,
                vec![
                    ("transaction_id", Some("PSP-OVER-001")),
                    ("reference_id", Some("BATCH-007")),
                    ("batch_id", Some("PSP-BATCH-007")),
                ],
                StagingEntryProcessingMode::Confirmation
            ),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![
                    (EntryStatus::Pending, 50.00, BalanceDirection::Credit),
                    (EntryStatus::Pending, 30.00, BalanceDirection::Credit),
                    (EntryStatus::Pending, 60.00, BalanceDirection::Debit),
                    (EntryStatus::Expected, 20.00, BalanceDirection::Debit)
                ])
                .expected_transaction_status(ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Expected))
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-007")])
                .build(),
            "psp",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(
                4000,
                BalanceDirection::Debit,
                vec![
                    ("transaction_id", Some("PSP-OVER-002")),
                    ("reference_id", Some("BATCH-007")),
                    ("batch_id", Some("PSP-BATCH-007")),
                ],
                StagingEntryProcessingMode::Confirmation
            ),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![
                    (EntryStatus::Pending, 50.00, BalanceDirection::Credit),
                    (EntryStatus::Pending, 30.00, BalanceDirection::Credit),
                    (EntryStatus::Pending, 60.00, BalanceDirection::Debit),
                    (EntryStatus::Pending, 40.00, BalanceDirection::Debit),
                    (EntryStatus::Expected, 20.00, BalanceDirection::Credit)
                ])
                .expected_transaction_status(ApiTransactionStatus::OverAmount(TransactionAmountMismatchData::Expected))
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-007")])
                .build(),
            "psp",
        ),
    ]
)]
// Case 8: One To One Many Many: Balance Direction Mismatch
#[case(
    ReconRuleEngine::OneToOneManyManyV1(("oms".to_string(), "psp".to_string())),
    ReconRuleCustomConfig::default(),
    vec![
        // Multiple sources with Credit direction
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(
                5000,
                BalanceDirection::Credit,
                vec![
                    ("transaction_id", Some("ORD-DIR-001")),
                    ("batch_id", Some("BATCH-008")),
                ],
                StagingEntryProcessingMode::Transaction
            ),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![
                    (EntryStatus::Pending, 50.00, BalanceDirection::Credit),
                    (EntryStatus::Expected, 50.00, BalanceDirection::Debit)
                ])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-008")])
                .build(),
            "oms",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(
                5000,
                BalanceDirection::Credit,
                vec![
                    ("transaction_id", Some("ORD-DIR-002")),
                    ("batch_id", Some("BATCH-008")),
                ],
                StagingEntryProcessingMode::Transaction
            ),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![
                    (EntryStatus::Pending, 50.00, BalanceDirection::Credit),
                    (EntryStatus::Pending, 50.00, BalanceDirection::Credit),
                    (EntryStatus::Expected, 100.00, BalanceDirection::Debit)
                ])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-008")])
                .build(),
            "oms",
        ),
        // Target with wrong direction (Credit instead of Debit)
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(
                10000,
                BalanceDirection::Credit, // Wrong direction
                vec![
                    ("transaction_id", Some("PSP-DIR-001")),
                    ("reference_id", Some("BATCH-008")),
                    ("batch_id", Some("PSP-BATCH-008")),
                ],
                StagingEntryProcessingMode::Confirmation
            ),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![
                    (EntryStatus::Pending, 50.00, BalanceDirection::Credit),
                    (EntryStatus::Pending, 50.00, BalanceDirection::Credit),
                    (EntryStatus::Pending, 100.00, BalanceDirection::Credit),
                    (EntryStatus::Expected, 200.00, BalanceDirection::Debit)
                ])
                .expected_transaction_status(ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Expected))
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-008")])
                .build(),
            "psp",
        ),
    ]
)]
// Case 9: One To One Many Many: Mixed Credits and Debits (Refund Scenario)
#[case(
    ReconRuleEngine::OneToOneManyManyV1(("oms".to_string(), "psp".to_string())),
    ReconRuleCustomConfig::default(),
    vec![
        // Initial source entry (sale)
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(
                10000,
                BalanceDirection::Credit,
                vec![
                    ("transaction_id", Some("ORD-MIXED-001")),
                    ("batch_id", Some("BATCH-009")),
                ],
                StagingEntryProcessingMode::Transaction
            ),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![
                    (EntryStatus::Pending, 100.00, BalanceDirection::Credit),
                    (EntryStatus::Expected, 100.00, BalanceDirection::Debit)
                ])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-009")])
                .build(),
            "oms",
        ),
        // Refund/reversal entry (debit on source)
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(
                3000,
                BalanceDirection::Debit,
                vec![
                    ("transaction_id", Some("ORD-MIXED-002")),
                    ("batch_id", Some("BATCH-009")),
                ],
                StagingEntryProcessingMode::Transaction
            ),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![
                    (EntryStatus::Pending, 100.00, BalanceDirection::Credit),
                    (EntryStatus::Pending, 30.00, BalanceDirection::Debit),
                    (EntryStatus::Expected, 70.00, BalanceDirection::Debit)
                ])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-009")])
                .build(),
            "oms",
        ),
        // Another sale
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(
                2000,
                BalanceDirection::Credit,
                vec![
                    ("transaction_id", Some("ORD-MIXED-003")),
                    ("batch_id", Some("BATCH-009")),
                ],
                StagingEntryProcessingMode::Transaction
            ),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![
                    (EntryStatus::Pending, 100.00, BalanceDirection::Credit),
                    (EntryStatus::Pending, 30.00, BalanceDirection::Debit),
                    (EntryStatus::Pending, 20.00, BalanceDirection::Credit),
                    (EntryStatus::Expected, 90.00, BalanceDirection::Debit)
                ])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-009")])
                .build(),
            "oms",
        ),
        // Target confirmation for net amount (90.00)
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(
                9000,
                BalanceDirection::Debit,
                vec![
                    ("transaction_id", Some("PSP-MIXED-001")),
                    ("reference_id", Some("BATCH-009")),
                    ("batch_id", Some("PSP-BATCH-009")),
                ],
                StagingEntryProcessingMode::Confirmation
            ),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![
                    (EntryStatus::Matched, 100.00, BalanceDirection::Credit),
                    (EntryStatus::Matched, 30.00, BalanceDirection::Debit),
                    (EntryStatus::Matched, 20.00, BalanceDirection::Credit),
                    (EntryStatus::Matched, 90.00, BalanceDirection::Debit)
                ])
                .expected_transaction_status(ApiTransactionStatus::Matched(TransactionMatchedData::Auto))
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-009")])
                .build(),
            "psp",
        ),
    ]
)]
// Case 10: One To One Many Many: Incremental Processing - Multiple Small Batches
#[case(
    ReconRuleEngine::OneToOneManyManyV1(("oms".to_string(), "psp".to_string())),
    ReconRuleCustomConfig::default(),
    vec![
        // First batch of sources
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(
                1000,
                BalanceDirection::Credit,
                vec![
                    ("transaction_id", Some("ORD-INC-001")),
                    ("batch_id", Some("BATCH-010")),
                ],
                StagingEntryProcessingMode::Transaction
            ),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![
                    (EntryStatus::Pending, 10.00, BalanceDirection::Credit),
                    (EntryStatus::Expected, 10.00, BalanceDirection::Debit)
                ])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-010")])
                .build(),
            "oms",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(
                2000,
                BalanceDirection::Credit,
                vec![
                    ("transaction_id", Some("ORD-INC-002")),
                    ("batch_id", Some("BATCH-010")),
                ],
                StagingEntryProcessingMode::Transaction
            ),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![
                    (EntryStatus::Pending, 10.00, BalanceDirection::Credit),
                    (EntryStatus::Pending, 20.00, BalanceDirection::Credit),
                    (EntryStatus::Expected, 30.00, BalanceDirection::Debit)
                ])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-010")])
                .build(),
            "oms",
        ),
        // First target partial payment
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(
                1500,
                BalanceDirection::Debit,
                vec![
                    ("transaction_id", Some("PSP-INC-001")),
                    ("reference_id", Some("BATCH-010")),
                    ("batch_id", Some("PSP-BATCH-010")),
                ],
                StagingEntryProcessingMode::Confirmation
            ),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![
                    (EntryStatus::Pending, 10.00, BalanceDirection::Credit),
                    (EntryStatus::Pending, 20.00, BalanceDirection::Credit),
                    (EntryStatus::Pending, 15.00, BalanceDirection::Debit),
                    (EntryStatus::Expected, 15.00, BalanceDirection::Debit)
                ])
                .expected_transaction_status(ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Expected))
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-010")])
                .build(),
            "psp",
        ),
        // More source entries
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(
                3000,
                BalanceDirection::Credit,
                vec![
                    ("transaction_id", Some("ORD-INC-003")),
                    ("batch_id", Some("BATCH-010")),
                ],
                StagingEntryProcessingMode::Transaction
            ),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![
                    (EntryStatus::Pending, 10.00, BalanceDirection::Credit),
                    (EntryStatus::Pending, 20.00, BalanceDirection::Credit),
                    (EntryStatus::Pending, 30.00, BalanceDirection::Credit),
                    (EntryStatus::Pending, 15.00, BalanceDirection::Debit),
                    (EntryStatus::Expected, 45.00, BalanceDirection::Debit)
                ])
                .expected_transaction_status(ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Expected))
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-010")])
                .build(),
            "oms",
        ),
        // Final settlement
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(
                4500,
                BalanceDirection::Debit,
                vec![
                    ("transaction_id", Some("PSP-INC-002")),
                    ("reference_id", Some("BATCH-010")),
                    ("batch_id", Some("PSP-BATCH-010")),
                ],
                StagingEntryProcessingMode::Confirmation
            ),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![
                    (EntryStatus::Matched, 10.00, BalanceDirection::Credit),
                    (EntryStatus::Matched, 20.00, BalanceDirection::Credit),
                    (EntryStatus::Matched, 30.00, BalanceDirection::Credit),
                    (EntryStatus::Matched, 15.00, BalanceDirection::Debit),
                    (EntryStatus::Matched, 45.00, BalanceDirection::Debit)
                ])
                .expected_transaction_status(ApiTransactionStatus::Matched(TransactionMatchedData::Auto))
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-010")])
                .build(),
            "psp",
        ),
    ]
)]
// Case 11: One To One Many Many: Late Entry After Matched
#[case(
    ReconRuleEngine::OneToOneManyManyV1(("oms".to_string(), "psp".to_string())),
    ReconRuleCustomConfig::default(),
    vec![
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(
                10000,
                BalanceDirection::Credit,
                vec![
                    ("transaction_id", Some("ORD-LATE-001")),
                    ("batch_id", Some("BATCH-LATE-001")),
                ],
                StagingEntryProcessingMode::Transaction
            ),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![
                    (EntryStatus::Pending, 100.00, BalanceDirection::Credit),
                    (EntryStatus::Expected, 100.00, BalanceDirection::Debit)
                ])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-LATE-001")])
                .build(),
            "oms",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(
                10000,
                BalanceDirection::Debit,
                vec![
                    ("transaction_id", Some("PSP-LATE-001")),
                    ("reference_id", Some("BATCH-LATE-001")),
                ],
                StagingEntryProcessingMode::Confirmation
            ),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![
                    (EntryStatus::Matched, 100.00, BalanceDirection::Credit),
                    (EntryStatus::Matched, 100.00, BalanceDirection::Debit),
                ])
                .expected_transaction_status(ApiTransactionStatus::Matched(TransactionMatchedData::Auto))
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-LATE-001")])
                .build(),
            "psp",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(
                2000,
                BalanceDirection::Debit,
                vec![
                    ("transaction_id", Some("PSP-LATE-002")),
                    ("reference_id", Some("BATCH-LATE-001")),
                ],
                StagingEntryProcessingMode::Confirmation
            ),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![
                    (EntryStatus::Pending, 100.00, BalanceDirection::Credit),
                    (EntryStatus::Pending, 100.00, BalanceDirection::Debit),
                    (EntryStatus::Pending, 20.00, BalanceDirection::Debit),
                    (EntryStatus::Expected, 20.00, BalanceDirection::Credit),
                ])
                .expected_transaction_status(ApiTransactionStatus::OverAmount(TransactionAmountMismatchData::Expected))
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-LATE-001")])
                .build(),
            "psp",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(
                2000,
                BalanceDirection::Credit,
                vec![
                    ("transaction_id", Some("ORD-LATE-002")),
                    ("batch_id", Some("BATCH-LATE-001")),
                ],
                StagingEntryProcessingMode::Transaction
            ),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![
                    (EntryStatus::Matched, 100.00, BalanceDirection::Credit),
                    (EntryStatus::Matched, 20.00, BalanceDirection::Credit),
                    (EntryStatus::Matched, 100.00, BalanceDirection::Debit),
                    (EntryStatus::Matched, 20.00, BalanceDirection::Debit),
                ])
                .expected_transaction_status(ApiTransactionStatus::Matched(TransactionMatchedData::Auto))
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-LATE-001")])
                .build(),
            "oms",
        ),
    ]
)]
// Case 12: Composite grouping + search — multiple source entries grouped by (batch_id, region), matched against multiple target entries whose composite target value matches. Happy Case.
#[case(
    ReconRuleEngine::OneToOneManyManyV1(("oms".to_string(), "psp".to_string())),
    ReconRuleCustomConfig::default()
        .set_grouping_field("oms", composite_grouping_field())
        .set_search_identifier("psp", composite_search_identifier()),
    vec![
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(
                4000,
                BalanceDirection::Credit,
                vec![("transaction_id", Some("ORD-001")), ("batch_id", Some("BATCH-001")), ("region", Some("US"))],
                StagingEntryProcessingMode::Transaction
            ),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 40.00, BalanceDirection::Credit), (EntryStatus::Expected, 40.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-001|US")])
                .build(),
            "oms",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(
                6000,
                BalanceDirection::Credit,
                vec![("transaction_id", Some("ORD-002")), ("batch_id", Some("BATCH-001")), ("region", Some("US"))],
                StagingEntryProcessingMode::Transaction
            ),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 40.00, BalanceDirection::Credit), (EntryStatus::Pending, 60.00, BalanceDirection::Credit), (EntryStatus::Expected, 100.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Expected)
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-001|US")])
                .build(),
            "oms",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(
                3000,
                BalanceDirection::Debit,
                vec![("transaction_id", Some("PSP-001")), ("reference_id", Some("BATCH-001")), ("region", Some("US"))],
                StagingEntryProcessingMode::Confirmation
            ),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Pending, 40.00, BalanceDirection::Credit), (EntryStatus::Pending, 60.00, BalanceDirection::Credit), (EntryStatus::Pending, 30.00, BalanceDirection::Debit), (EntryStatus::Expected, 70.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::UnderAmount(TransactionAmountMismatchData::Expected))
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-001|US")])
                .build(),
            "psp",
        ),
        ReconTestEntry::new(
            StagingEntryData::recon_staging_entry(
                7000,
                BalanceDirection::Debit,
                vec![("transaction_id", Some("PSP-002")), ("reference_id", Some("BATCH-001")), ("region", Some("US"))],
                StagingEntryProcessingMode::Confirmation
            ),
            ReconTestEntryExpectedResultBuilder::new()
                .expected_staging_entry_status(ApiStagingEntryCoarseStatus::Processed)
                .expected_entries_data(vec![(EntryStatus::Matched, 40.00, BalanceDirection::Credit), (EntryStatus::Matched, 60.00, BalanceDirection::Credit), (EntryStatus::Matched, 30.00, BalanceDirection::Debit), (EntryStatus::Matched, 70.00, BalanceDirection::Debit)])
                .expected_transaction_status(ApiTransactionStatus::Matched(TransactionMatchedData::Auto))
                .expected_search_group_status(SearchGroupStatus::Active)
                .expected_search_expectations(vec![ExpectedSearchExpectation::new("psp", "BATCH-001|US")])
                .build(),
            "psp",
        ),
    ]
)]
#[actix_web::test]
async fn test_one_to_one_many_many(
    #[case] rule_type: ReconRuleEngine,
    #[case] rule_custom_config: ReconRuleCustomConfig,
    #[case] test_entries: Vec<ReconTestEntry>,
) {
    use crate::api::recon_engine_test::run_recon_engine_test;
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
                target_field: "metadata.reference_id".parse().unwrap(),
            },
            SearchIdentifierKey {
                source_field: "metadata.region".parse().unwrap(),
                target_field: "metadata.region".parse().unwrap(),
            },
        ],
        delimiter: CompositeDelimiter::Pipe,
    }))
}
