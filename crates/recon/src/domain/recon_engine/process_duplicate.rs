use common_enums::SearchDuplicateStatus;
use diesel_models::{
    metadata_schema::MetadataSchema, search_duplicate::SearchDuplicateNew,
    staging_entry::StagingEntryNew, transformation_config::TransformationConfig,
};
use error_stack::ResultExt;
use masking::ExposeInterface;
use recon_domain_models::{
    staging_entry::DomainStagingEntry, ApplicationEncryptionInterface, EncryptionHandler,
};

use common_types::metadata_schema::UniqueConstraintProvider;
use storage_impl::{
    metadata_schema::MetadataSchemaInterface, search_duplicate::SearchDuplicateInterface,
};

use crate::{
    domain::metadata_schema::implementations::unique_constraint::UniqueConstraintExt,
    domain::recon_engine::{utils::extract_unique_field, ProcessingError, ProcessingResult},
    errors::conversions::StorageErrorExt,
    routes::AppState,
    utils::recon_engine::{self},
};

pub async fn check_for_duplicate(
    state: &AppState,
    staging_entry: &DomainStagingEntry,
    metadata_schema: &MetadataSchema,
) -> ProcessingResult<()> {
    let unique_constraint = metadata_schema.schema_data.get_unique_constraint();
    let (field_name, field_value) = extract_unique_field(
        unique_constraint.extract_unique_value(staging_entry),
        unique_constraint.get_unique_field_delimiter(),
    )?;

    let hashed_field_value = recon_engine::hash_string(&field_value);

    // Check if duplicates already exist
    let existing_duplicates = state
        .store
        .list_by_transformation_id_field_path_hashed_field_value(
            &staging_entry.transformation_id,
            field_name,
            &hashed_field_value.expose(),
            Some(SearchDuplicateStatus::Active),
        )
        .await
        .change_context(ProcessingError::DatabaseError)?;

    if !existing_duplicates.is_empty() {
        return Err(ProcessingError::DuplicateEntry.into());
    }

    // No duplicates found, processing can continue
    Ok(())
}

pub async fn create_new_search_duplicate<T: ApplicationEncryptionInterface>(
    state: &AppState,
    staging_entry: &DomainStagingEntry,
    processed_staging_entry: &StagingEntryNew,
    transformation_config: &TransformationConfig,
    encryptor: &EncryptionHandler<T>,
) -> ProcessingResult<SearchDuplicateNew> {
    // Get field path and extract value from staging entry
    let metadata_schema = state
        .store
        .find_metadata_schema_by_id_and_profile_id(
            &transformation_config.metadata_schema_id,
            &transformation_config.profile_id,
        )
        .await
        .to_not_found_response(|| ProcessingError::DatabaseError)?;

    build_search_duplicate(
        staging_entry,
        processed_staging_entry,
        transformation_config,
        &metadata_schema,
        encryptor,
    )
    .await
}

pub async fn build_search_duplicate<T: ApplicationEncryptionInterface>(
    staging_entry: &DomainStagingEntry,
    processed_staging_entry: &StagingEntryNew,
    transformation_config: &TransformationConfig,
    metadata_schema: &MetadataSchema,
    encryptor: &EncryptionHandler<T>,
) -> ProcessingResult<SearchDuplicateNew> {
    let unique_constraint = metadata_schema.schema_data.get_unique_constraint();
    let (field_name, field_value) = extract_unique_field(
        unique_constraint.extract_unique_value(staging_entry),
        unique_constraint.get_unique_field_delimiter(),
    )?;

    let encrypted_field_value = encryptor
        .encrypt_string(field_value.to_string())
        .await
        .change_context(ProcessingError::DatabaseError)?;
    let hashed_field_value = recon_engine::hash_string(&field_value);

    let new_search_duplicate = SearchDuplicateNew::new(
        processed_staging_entry.staging_entry_id.clone(),
        processed_staging_entry.account_id.clone(),
        processed_staging_entry.merchant_id.clone(),
        transformation_config.id.clone(),
        field_name,
        encrypted_field_value.into(),
        hashed_field_value,
        processed_staging_entry.id.clone(),
        processed_staging_entry.version,
    );

    Ok(new_search_duplicate)
}
