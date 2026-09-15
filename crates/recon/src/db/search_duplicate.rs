use async_trait::async_trait;
use common_enums::SearchDuplicateStatus;
use common_types::field_path_key::FieldPathKey;
use diesel_models::{
    search_duplicate::{SearchDuplicate, SearchDuplicateNew},
    StorageResult,
};

use super::Store;

#[async_trait]
pub trait SearchDuplicateInterface {
    async fn insert_search_duplicate(
        &self,
        search_duplicate: SearchDuplicateNew,
    ) -> StorageResult<SearchDuplicate>;

    async fn find_by_id(&self, search_duplicate_id: String) -> StorageResult<SearchDuplicate>;

    async fn list_by_staging_entry_id(
        &self,
        staging_entry_id: &str,
        status: Option<SearchDuplicateStatus>,
    ) -> StorageResult<Vec<SearchDuplicate>>;

    async fn list_by_transformation_id_field_path_hashed_field_value(
        &self,
        transformation_id: &str,
        field_path: FieldPathKey,
        hashed_field_value: &str,
        status: Option<SearchDuplicateStatus>,
    ) -> StorageResult<Vec<SearchDuplicate>>;
}

#[async_trait]
impl SearchDuplicateInterface for Store {
    async fn insert_search_duplicate(
        &self,
        search_duplicate: SearchDuplicateNew,
    ) -> StorageResult<SearchDuplicate> {
        let conn = self.get_connection().await?;
        search_duplicate.insert(&conn).await
    }

    async fn find_by_id(&self, search_duplicate_id: String) -> StorageResult<SearchDuplicate> {
        let conn = self.get_connection().await?;
        SearchDuplicate::find_by_id(&conn, search_duplicate_id).await
    }

    async fn list_by_staging_entry_id(
        &self,
        staging_entry_id: &str,
        status: Option<SearchDuplicateStatus>,
    ) -> StorageResult<Vec<SearchDuplicate>> {
        let conn = self.get_connection().await?;
        SearchDuplicate::list_by_staging_entry_id(&conn, staging_entry_id, status).await
    }

    async fn list_by_transformation_id_field_path_hashed_field_value(
        &self,
        transformation_id: &str,
        field_path: FieldPathKey,
        hashed_field_value: &str,
        status: Option<SearchDuplicateStatus>,
    ) -> StorageResult<Vec<SearchDuplicate>> {
        let conn = self.get_connection().await?;
        SearchDuplicate::list_by_transformation_id_field_path_hashed_field_value(
            &conn,
            transformation_id,
            field_path.as_ref(),
            hashed_field_value,
            status,
        )
        .await
    }
}
