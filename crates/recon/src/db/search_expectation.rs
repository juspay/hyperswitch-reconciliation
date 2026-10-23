use async_trait::async_trait;
use common_types::field_path_key::FieldPathKey;
use diesel_models::{
    search_expectation::{SearchExpectation, SearchExpectationNew, SearchExpectationUpdate},
    StorageResult,
};

use super::Store;

pub struct SearchCriteria {
    pub rule_id: String,
    pub target_key: FieldPathKey,
    pub hashed_source_value: String,
}

#[async_trait]
pub trait SearchExpectationInterface {
    async fn insert_search_expectation(
        &self,
        search_expectation: SearchExpectationNew,
    ) -> StorageResult<SearchExpectation>;

    async fn find_by_id(&self, id: &str) -> StorageResult<SearchExpectation>;

    async fn list_by_target_account_id_rule_id_target_key_hashed_source_value(
        &self,
        target_account_id: &str,
        rule_id: &str,
        target_key: &FieldPathKey,
        hashed_source_value: &str,
    ) -> StorageResult<Vec<SearchExpectation>>;

    async fn find_for_multiple_targets(
        &self,
        target_account_id: &str,
        search_criteria: Vec<SearchCriteria>,
    ) -> StorageResult<Vec<SearchExpectation>>;

    async fn batch_update_status_by_primary_transaction_ids(
        &self,
        primary_transaction_ids: Vec<String>,
        search_expectation_update: SearchExpectationUpdate,
    ) -> StorageResult<Vec<SearchExpectation>>;

    async fn find_expected_expectations_by_primary_transaction_id_and_target_account_id(
        &self,
        primary_transaction_id: &str,
        target_account_id: &str,
    ) -> StorageResult<Vec<SearchExpectation>>;

    async fn list_expected_expectations_by_primary_transaction_id(
        &self,
        primary_transaction_id: &str,
    ) -> StorageResult<Vec<SearchExpectation>>;
}

#[async_trait]
impl SearchExpectationInterface for Store {
    async fn insert_search_expectation(
        &self,
        search_expectation: SearchExpectationNew,
    ) -> StorageResult<SearchExpectation> {
        let conn = self.get_connection().await?;
        search_expectation.insert(&conn).await
    }

    async fn find_by_id(&self, id: &str) -> StorageResult<SearchExpectation> {
        let conn = self.get_connection().await?;
        SearchExpectation::find_by_id(&conn, id.to_string()).await
    }

    async fn list_by_target_account_id_rule_id_target_key_hashed_source_value(
        &self,
        target_account_id: &str,
        rule_id: &str,
        target_key: &FieldPathKey,
        hashed_source_value: &str,
    ) -> StorageResult<Vec<SearchExpectation>> {
        let conn = self.get_connection().await?;
        SearchExpectation::list_by_target_account_id_rule_id_target_key_hashed_source_value(
            &conn,
            target_account_id,
            rule_id,
            target_key,
            hashed_source_value,
        )
        .await
    }

    async fn find_for_multiple_targets(
        &self,
        target_account_id: &str,
        search_criteria: Vec<SearchCriteria>,
    ) -> StorageResult<Vec<SearchExpectation>> {
        let conn = self.get_connection().await?;
        let query_search_criteria = search_criteria
            .into_iter()
            .map(|criteria| {
                (
                    criteria.rule_id,
                    criteria.target_key,
                    criteria.hashed_source_value,
                )
            })
            .collect::<Vec<_>>();

        SearchExpectation::find_for_multiple_targets(
            &conn,
            target_account_id,
            &query_search_criteria,
        )
        .await
    }

    async fn batch_update_status_by_primary_transaction_ids(
        &self,
        primary_transaction_ids: Vec<String>,
        search_expectation_update: SearchExpectationUpdate,
    ) -> StorageResult<Vec<SearchExpectation>> {
        let conn = self.get_connection().await?;
        let search_expectation_update_internal = search_expectation_update.into();
        SearchExpectation::batch_update_status_by_primary_transaction_ids(
            &conn,
            primary_transaction_ids,
            search_expectation_update_internal,
        )
        .await
    }

    async fn find_expected_expectations_by_primary_transaction_id_and_target_account_id(
        &self,
        primary_transaction_id: &str,
        target_account_id: &str,
    ) -> StorageResult<Vec<SearchExpectation>> {
        let conn = self.get_connection().await?;
        SearchExpectation::find_expected_expectations_by_primary_transaction_id_and_target_account_id(
            &conn,
            primary_transaction_id,
            target_account_id,
        )
        .await
    }

    async fn list_expected_expectations_by_primary_transaction_id(
        &self,
        primary_transaction_id: &str,
    ) -> StorageResult<Vec<SearchExpectation>> {
        let conn = self.get_connection().await?;
        SearchExpectation::list_expected_by_primary_transaction_id(&conn, primary_transaction_id)
            .await
    }
}
