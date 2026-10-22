// @generated automatically by Diesel CLI.

diesel::table! {
    use diesel::sql_types::*;
    use common_enums::diesel_exports::*;

    account (id) {
        #[max_length = 64]
        id -> Varchar,
        balance_direction -> BalanceDirection,
        name -> Text,
        created_at -> Timestamp,
        last_modified_at -> Timestamp,
        initial_balance -> Int8,
        #[max_length = 64]
        profile_id -> Varchar,
        currency -> Currency,
        matched_debits -> Int8,
        matched_credits -> Int8,
        pending_debits -> Int8,
        pending_credits -> Int8,
        expected_debits -> Int8,
        expected_credits -> Int8,
        mismatched_debits -> Int8,
        mismatched_credits -> Int8,
        posted_debits -> Int8,
        posted_credits -> Int8,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use common_enums::diesel_exports::*;

    api_key (id) {
        #[max_length = 64]
        id -> Varchar,
        #[max_length = 64]
        merchant_id -> Varchar,
        #[max_length = 64]
        name -> Varchar,
        #[max_length = 256]
        description -> Nullable<Varchar>,
        #[max_length = 128]
        hashed_api_key -> Varchar,
        #[max_length = 24]
        prefix -> Varchar,
        #[max_length = 64]
        created_by -> Varchar,
        created_at -> Timestamp,
        expires_at -> Nullable<Timestamp>,
        last_modified_at -> Timestamp,
        status -> ApiKeyStatus,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use common_enums::diesel_exports::*;

    business_profile (profile_id) {
        #[max_length = 64]
        profile_id -> Varchar,
        profile_name -> Bytea,
        #[max_length = 64]
        merchant_id -> Varchar,
        created_at -> Timestamp,
        last_modified_at -> Timestamp,
        #[max_length = 64]
        timezone -> Varchar,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use common_enums::diesel_exports::*;

    entry (id) {
        #[max_length = 64]
        id -> Varchar,
        #[max_length = 64]
        account_id -> Varchar,
        #[max_length = 64]
        logical_transaction_id -> Varchar,
        balance_direction -> BalanceDirection,
        amount -> Int8,
        currency -> Currency,
        status -> EntryStatus,
        effective_at -> Timestamp,
        metadata -> Bytea,
        discarded_at -> Nullable<Timestamp>,
        discarded_status -> Nullable<EntryStatus>,
        created_at -> Timestamp,
        last_modified_at -> Timestamp,
        transaction_version -> Int4,
        #[max_length = 64]
        merchant_id -> Varchar,
        data -> Jsonb,
        discarded_data -> Nullable<Jsonb>,
        #[max_length = 64]
        primary_staging_entry_id -> Nullable<Varchar>,
        #[max_length = 64]
        transformation_config_id -> Nullable<Varchar>,
        entry_identifier -> Text,
        exposed_metadata -> Jsonb,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use common_enums::diesel_exports::*;

    ingestion_config (id) {
        #[max_length = 64]
        id -> Varchar,
        #[max_length = 64]
        profile_id -> Varchar,
        #[max_length = 64]
        account_id -> Varchar,
        #[max_length = 255]
        name -> Varchar,
        description -> Nullable<Text>,
        is_active -> Bool,
        config -> Jsonb,
        created_at -> Timestamp,
        last_modified_at -> Timestamp,
        #[max_length = 64]
        merchant_id -> Varchar,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use common_enums::diesel_exports::*;

    ingestion_history (id) {
        #[max_length = 64]
        ingestion_history_id -> Varchar,
        #[max_length = 64]
        ingestion_config_id -> Varchar,
        #[max_length = 64]
        profile_id -> Varchar,
        status -> IngestionStatus,
        data -> Nullable<Jsonb>,
        processed_at -> Nullable<Timestamp>,
        created_at -> Timestamp,
        last_modified_at -> Timestamp,
        trigger_data -> Nullable<Jsonb>,
        raw_payload -> Jsonb,
        #[max_length = 64]
        account_id -> Varchar,
        #[max_length = 64]
        id -> Varchar,
        version -> Int4,
        discarded_at -> Nullable<Timestamp>,
        discarded_status -> Nullable<IngestionStatus>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use common_enums::diesel_exports::*;

    merchant_account (merchant_id) {
        #[max_length = 64]
        merchant_id -> Varchar,
        merchant_name -> Bytea,
        created_at -> Timestamp,
        last_modified_at -> Timestamp,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use common_enums::diesel_exports::*;

    metadata_schema (id) {
        #[max_length = 64]
        id -> Varchar,
        #[max_length = 64]
        schema_id -> Varchar,
        #[max_length = 64]
        profile_id -> Varchar,
        #[max_length = 64]
        account_id -> Varchar,
        version -> Int4,
        schema_data -> Jsonb,
        created_at -> Timestamp,
        last_modified_at -> Timestamp,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use common_enums::diesel_exports::*;

    recon_rule (id) {
        #[max_length = 64]
        id -> Varchar,
        #[max_length = 255]
        name -> Varchar,
        description -> Text,
        created_at -> Timestamp,
        last_modified_at -> Timestamp,
        is_active -> Bool,
        priority -> Int2,
        #[max_length = 64]
        profile_id -> Varchar,
        strategy -> Jsonb,
        aging_config -> Jsonb,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use common_enums::diesel_exports::*;

    recon_rule_account (id) {
        #[max_length = 64]
        id -> Varchar,
        #[max_length = 64]
        account_id -> Varchar,
        #[max_length = 64]
        rule_id -> Varchar,
        account_type -> ReconRuleAccountType,
        created_at -> Timestamp,
        last_modified_at -> Timestamp,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use common_enums::diesel_exports::*;

    search_duplicate (id) {
        #[max_length = 64]
        id -> Varchar,
        #[max_length = 64]
        logical_staging_entry_id -> Varchar,
        #[max_length = 64]
        account_id -> Varchar,
        #[max_length = 64]
        transformation_config_id -> Varchar,
        field_path -> Text,
        encrypted_field_value -> Bytea,
        #[max_length = 64]
        hashed_field_value -> Varchar,
        created_at -> Timestamp,
        last_modified_at -> Timestamp,
        #[max_length = 64]
        merchant_id -> Varchar,
        #[max_length = 64]
        primary_staging_entry_id -> Varchar,
        staging_entry_version -> Int4,
        status -> SearchDuplicateStatus,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use common_enums::diesel_exports::*;

    search_entry_key (id) {
        #[max_length = 64]
        id -> Varchar,
        #[max_length = 64]
        profile_id -> Varchar,
        #[max_length = 64]
        entry_id -> Varchar,
        #[max_length = 64]
        primary_transaction_id -> Varchar,
        metadata_key -> Text,
        metadata_value -> Nullable<Text>,
        created_at -> Timestamp,
        last_modified_at -> Timestamp,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use common_enums::diesel_exports::*;

    search_expectation (id) {
        #[max_length = 64]
        id -> Varchar,
        #[max_length = 64]
        rule_id -> Varchar,
        #[max_length = 64]
        target_account_id -> Varchar,
        target_key -> Text,
        status -> ExpectationStatus,
        created_at -> Timestamp,
        last_modified_at -> Timestamp,
        encrypted_source_value -> Bytea,
        #[max_length = 64]
        hashed_source_value -> Varchar,
        #[max_length = 64]
        merchant_id -> Varchar,
        #[max_length = 64]
        primary_transaction_id -> Varchar,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use common_enums::diesel_exports::*;

    search_group (id) {
        #[max_length = 64]
        id -> Varchar,
        #[max_length = 64]
        rule_id -> Varchar,
        #[max_length = 64]
        primary_transaction_id -> Varchar,
        encrypted_grouping_value -> Bytea,
        #[max_length = 64]
        hashed_grouping_value -> Varchar,
        #[max_length = 64]
        merchant_id -> Varchar,
        #[max_length = 64]
        profile_id -> Varchar,
        status -> SearchGroupStatus,
        created_at -> Timestamp,
        last_modified_at -> Timestamp,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use common_enums::diesel_exports::*;

    staging_entry (id) {
        #[max_length = 64]
        staging_entry_id -> Varchar,
        #[max_length = 64]
        account_id -> Varchar,
        balance_direction -> BalanceDirection,
        amount -> Int8,
        currency -> Currency,
        #[max_length = 64]
        status -> Varchar,
        effective_at -> Timestamp,
        metadata -> Bytea,
        created_at -> Timestamp,
        last_modified_at -> Timestamp,
        processing_mode -> StagingEntryProcessingMode,
        #[max_length = 64]
        profile_id -> Varchar,
        #[max_length = 64]
        transformation_config_id -> Varchar,
        #[max_length = 64]
        transformation_history_id -> Nullable<Varchar>,
        #[max_length = 64]
        merchant_id -> Varchar,
        data -> Jsonb,
        #[max_length = 64]
        id -> Varchar,
        version -> Int4,
        discarded_at -> Nullable<Timestamp>,
        #[max_length = 64]
        discarded_status -> Nullable<Varchar>,
        discarded_data -> Nullable<Jsonb>,
        entry_identifier -> Text,
        #[max_length = 64]
        modified_by -> Nullable<Varchar>,
        exposed_metadata -> Jsonb,
        searchable_metadata -> Jsonb,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use common_enums::diesel_exports::*;

    transaction (id) {
        #[max_length = 64]
        id -> Varchar,
        #[max_length = 64]
        transaction_id -> Varchar,
        entry_ids -> Array<Nullable<Text>>,
        created_at -> Timestamp,
        discarded_at -> Nullable<Timestamp>,
        last_modified_at -> Timestamp,
        #[max_length = 64]
        discarded_status -> Nullable<Varchar>,
        version -> Int4,
        #[max_length = 64]
        profile_id -> Varchar,
        #[max_length = 64]
        rule_id -> Varchar,
        credit_amount -> Int8,
        debit_amount -> Int8,
        currency -> Currency,
        data -> Jsonb,
        discarded_data -> Nullable<Jsonb>,
        effective_at -> Timestamp,
        #[max_length = 64]
        status -> Varchar,
        #[max_length = 64]
        modified_by -> Nullable<Varchar>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use common_enums::diesel_exports::*;

    transformation_config (id) {
        #[max_length = 64]
        id -> Varchar,
        #[max_length = 64]
        profile_id -> Varchar,
        #[max_length = 64]
        ingestion_config_id -> Varchar,
        #[max_length = 64]
        account_id -> Varchar,
        #[max_length = 255]
        name -> Varchar,
        config -> Jsonb,
        is_active -> Bool,
        created_at -> Timestamp,
        last_modified_at -> Timestamp,
        #[max_length = 64]
        metadata_schema_id -> Varchar,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use common_enums::diesel_exports::*;

    transformation_history (id) {
        #[max_length = 64]
        id -> Varchar,
        #[max_length = 64]
        transformation_config_id -> Varchar,
        #[max_length = 64]
        ingestion_history_id -> Varchar,
        #[max_length = 64]
        profile_id -> Varchar,
        #[max_length = 64]
        account_id -> Varchar,
        status -> TransformationStatus,
        data -> Nullable<Jsonb>,
        processed_at -> Nullable<Timestamp>,
        created_at -> Timestamp,
        last_modified_at -> Timestamp,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    account,
    api_key,
    business_profile,
    entry,
    ingestion_config,
    ingestion_history,
    merchant_account,
    metadata_schema,
    recon_rule,
    recon_rule_account,
    search_duplicate,
    search_entry_key,
    search_expectation,
    search_group,
    staging_entry,
    transaction,
    transformation_config,
    transformation_history,
);
