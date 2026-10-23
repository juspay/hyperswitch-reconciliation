use diesel_models::errors::DatabaseError;
use error_stack::Report;

use super::{
    account::AccountError, business_profile::BusinessProfileError, entry::EntryError,
    ingestion::IngestionError, link_config::LinkConfigError,
    linking_processor::LinkingProcessorError, merchant_account::MerchantAccountError,
    metadata_schema::MetadataSchemaError, recon_engine::ReconEngineError,
    recon_rule::ReconRuleError, reporting::ReportError, staging_entry::StagingEntryError,
    transaction::TransactionError, transformation::TransformationError, ReconAppError,
};
use crate::{
    domain::recon_engine::ProcessingError,
    errors::{search_expectation::SearchExpectationError, search_group::SearchGroupError},
};

pub trait StorageErrorExt<T, E> {
    /// Converts a database error to entity-specific NotFound error if applicable,
    /// otherwise returns InternalServerError
    fn to_not_found_response(self, context: impl FnOnce() -> E) -> Result<T, Report<E>>;

    /// Converts a database error to entity-specific duplicate/unique violation error if applicable,
    /// otherwise returns InternalServerError
    fn to_duplicate_response(self, context: impl FnOnce() -> E) -> Result<T, Report<E>>;
}

impl<T> StorageErrorExt<T, LinkingProcessorError> for Result<T, Report<DatabaseError>> {
    fn to_not_found_response(
        self,
        context: impl FnOnce() -> LinkingProcessorError,
    ) -> Result<T, Report<LinkingProcessorError>> {
        self.map_err(|error| match error.current_context() {
            DatabaseError::NotFound => error.change_context(context()),
            _ => error.change_context(LinkingProcessorError::InternalServerError),
        })
    }

    fn to_duplicate_response(
        self,
        context: impl FnOnce() -> LinkingProcessorError,
    ) -> Result<T, Report<LinkingProcessorError>> {
        self.map_err(|error| match error.current_context() {
            DatabaseError::UniqueViolation => error.change_context(context()),
            _ => error.change_context(LinkingProcessorError::InternalServerError),
        })
    }
}

impl<T> StorageErrorExt<T, ReconAppError> for Result<T, Report<DatabaseError>> {
    fn to_not_found_response(
        self,
        context: impl FnOnce() -> ReconAppError,
    ) -> Result<T, Report<ReconAppError>> {
        self.map_err(|error| match error.current_context() {
            DatabaseError::NotFound => error.change_context(context()),
            _ => error.change_context(ReconAppError::InternalServerError),
        })
    }

    fn to_duplicate_response(
        self,
        context: impl FnOnce() -> ReconAppError,
    ) -> Result<T, Report<ReconAppError>> {
        self.map_err(|error| match error.current_context() {
            DatabaseError::UniqueViolation => error.change_context(context()),
            _ => error.change_context(ReconAppError::InternalServerError),
        })
    }
}

impl<T> StorageErrorExt<T, MerchantAccountError> for Result<T, Report<DatabaseError>> {
    fn to_not_found_response(
        self,
        context: impl FnOnce() -> MerchantAccountError,
    ) -> Result<T, Report<MerchantAccountError>> {
        self.map_err(|error| match error.current_context() {
            DatabaseError::NotFound => error.change_context(context()),
            _ => error.change_context(MerchantAccountError::InternalServerError),
        })
    }

    fn to_duplicate_response(
        self,
        context: impl FnOnce() -> MerchantAccountError,
    ) -> Result<T, Report<MerchantAccountError>> {
        self.map_err(|error| match error.current_context() {
            DatabaseError::UniqueViolation => error.change_context(context()),
            _ => error.change_context(MerchantAccountError::InternalServerError),
        })
    }
}

impl<T> StorageErrorExt<T, BusinessProfileError> for Result<T, Report<DatabaseError>> {
    fn to_not_found_response(
        self,
        context: impl FnOnce() -> BusinessProfileError,
    ) -> Result<T, Report<BusinessProfileError>> {
        self.map_err(|error| match error.current_context() {
            DatabaseError::NotFound => error.change_context(context()),
            _ => error.change_context(BusinessProfileError::InternalServerError),
        })
    }

    fn to_duplicate_response(
        self,
        context: impl FnOnce() -> BusinessProfileError,
    ) -> Result<T, Report<BusinessProfileError>> {
        self.map_err(|error| match error.current_context() {
            DatabaseError::UniqueViolation => error.change_context(context()),
            _ => error.change_context(BusinessProfileError::InternalServerError),
        })
    }
}

impl<T> StorageErrorExt<T, StagingEntryError> for Result<T, Report<DatabaseError>> {
    fn to_not_found_response(
        self,
        context: impl FnOnce() -> StagingEntryError,
    ) -> Result<T, Report<StagingEntryError>> {
        self.map_err(|error| match error.current_context() {
            DatabaseError::NotFound => error.change_context(context()),
            _ => error.change_context(StagingEntryError::InternalServerError),
        })
    }

    fn to_duplicate_response(
        self,
        context: impl FnOnce() -> StagingEntryError,
    ) -> Result<T, Report<StagingEntryError>> {
        self.map_err(|error| match error.current_context() {
            DatabaseError::UniqueViolation => error.change_context(context()),
            _ => error.change_context(StagingEntryError::InternalServerError),
        })
    }
}

impl<T> StorageErrorExt<T, AccountError> for Result<T, Report<DatabaseError>> {
    fn to_not_found_response(
        self,
        context: impl FnOnce() -> AccountError,
    ) -> Result<T, Report<AccountError>> {
        self.map_err(|error| match error.current_context() {
            DatabaseError::NotFound => error.change_context(context()),
            _ => error.change_context(AccountError::InternalServerError),
        })
    }

    fn to_duplicate_response(
        self,
        context: impl FnOnce() -> AccountError,
    ) -> Result<T, Report<AccountError>> {
        self.map_err(|error| match error.current_context() {
            DatabaseError::UniqueViolation => error.change_context(context()),
            _ => error.change_context(AccountError::InternalServerError),
        })
    }
}

impl<T> StorageErrorExt<T, EntryError> for Result<T, Report<DatabaseError>> {
    fn to_not_found_response(
        self,
        context: impl FnOnce() -> EntryError,
    ) -> Result<T, Report<EntryError>> {
        self.map_err(|error| match error.current_context() {
            DatabaseError::NotFound => error.change_context(context()),
            _ => error.change_context(EntryError::InternalServerError),
        })
    }

    fn to_duplicate_response(
        self,
        context: impl FnOnce() -> EntryError,
    ) -> Result<T, Report<EntryError>> {
        self.map_err(|error| match error.current_context() {
            DatabaseError::UniqueViolation => error.change_context(context()),
            _ => error.change_context(EntryError::InternalServerError),
        })
    }
}

impl<T> StorageErrorExt<T, ReconRuleError> for Result<T, Report<DatabaseError>> {
    fn to_not_found_response(
        self,
        context: impl FnOnce() -> ReconRuleError,
    ) -> Result<T, Report<ReconRuleError>> {
        self.map_err(|error| match error.current_context() {
            DatabaseError::NotFound => error.change_context(context()),
            _ => error.change_context(ReconRuleError::InternalServerError),
        })
    }

    fn to_duplicate_response(
        self,
        context: impl FnOnce() -> ReconRuleError,
    ) -> Result<T, Report<ReconRuleError>> {
        self.map_err(|error| match error.current_context() {
            DatabaseError::UniqueViolation => error.change_context(context()),
            _ => error.change_context(ReconRuleError::InternalServerError),
        })
    }
}

impl<T> StorageErrorExt<T, ReconEngineError> for Result<T, Report<DatabaseError>> {
    fn to_not_found_response(
        self,
        context: impl FnOnce() -> ReconEngineError,
    ) -> Result<T, Report<ReconEngineError>> {
        self.map_err(|error| match error.current_context() {
            DatabaseError::NotFound => error.change_context(context()),
            _ => error.change_context(ReconEngineError::InternalServerError),
        })
    }

    fn to_duplicate_response(
        self,
        context: impl FnOnce() -> ReconEngineError,
    ) -> Result<T, Report<ReconEngineError>> {
        self.map_err(|error| match error.current_context() {
            DatabaseError::UniqueViolation => error.change_context(context()),
            _ => error.change_context(ReconEngineError::InternalServerError),
        })
    }
}

impl<T> StorageErrorExt<T, TransactionError> for Result<T, Report<DatabaseError>> {
    fn to_not_found_response(
        self,
        context: impl FnOnce() -> TransactionError,
    ) -> Result<T, Report<TransactionError>> {
        self.map_err(|error| match error.current_context() {
            DatabaseError::NotFound => error.change_context(context()),
            _ => error.change_context(TransactionError::InternalServerError),
        })
    }

    fn to_duplicate_response(
        self,
        context: impl FnOnce() -> TransactionError,
    ) -> Result<T, Report<TransactionError>> {
        self.map_err(|error| match error.current_context() {
            DatabaseError::UniqueViolation => error.change_context(context()),
            _ => error.change_context(TransactionError::InternalServerError),
        })
    }
}

impl<T> StorageErrorExt<T, TransformationError> for Result<T, Report<DatabaseError>> {
    fn to_not_found_response(
        self,
        context: impl FnOnce() -> TransformationError,
    ) -> Result<T, Report<TransformationError>> {
        self.map_err(|error| match error.current_context() {
            DatabaseError::NotFound => error.change_context(context()),
            _ => error.change_context(TransformationError::InternalServerError),
        })
    }

    fn to_duplicate_response(
        self,
        context: impl FnOnce() -> TransformationError,
    ) -> Result<T, Report<TransformationError>> {
        self.map_err(|error| match error.current_context() {
            DatabaseError::UniqueViolation => error.change_context(context()),
            _ => error.change_context(TransformationError::InternalServerError),
        })
    }
}

impl<T> StorageErrorExt<T, IngestionError> for Result<T, Report<DatabaseError>> {
    fn to_not_found_response(
        self,
        context: impl FnOnce() -> IngestionError,
    ) -> Result<T, Report<IngestionError>> {
        self.map_err(|error| match error.current_context() {
            DatabaseError::NotFound => error.change_context(context()),
            _ => error.change_context(IngestionError::InternalServerError),
        })
    }

    fn to_duplicate_response(
        self,
        context: impl FnOnce() -> IngestionError,
    ) -> Result<T, Report<IngestionError>> {
        self.map_err(|error| match error.current_context() {
            DatabaseError::UniqueViolation => error.change_context(context()),
            _ => error.change_context(IngestionError::InternalServerError),
        })
    }
}

impl<T> StorageErrorExt<T, ProcessingError> for Result<T, Report<DatabaseError>> {
    fn to_not_found_response(
        self,
        context: impl FnOnce() -> ProcessingError,
    ) -> Result<T, Report<ProcessingError>> {
        self.map_err(|error| match error.current_context() {
            DatabaseError::NotFound => error.change_context(context()),
            _ => error.change_context(ProcessingError::DatabaseError),
        })
    }

    fn to_duplicate_response(
        self,
        context: impl FnOnce() -> ProcessingError,
    ) -> Result<T, Report<ProcessingError>> {
        self.map_err(|error| match error.current_context() {
            DatabaseError::UniqueViolation => error.change_context(context()),
            _ => error.change_context(ProcessingError::DatabaseError),
        })
    }
}

impl<T> StorageErrorExt<T, MetadataSchemaError> for Result<T, Report<DatabaseError>> {
    fn to_not_found_response(
        self,
        context: impl FnOnce() -> MetadataSchemaError,
    ) -> Result<T, Report<MetadataSchemaError>> {
        self.map_err(|error| match error.current_context() {
            DatabaseError::NotFound => error.change_context(context()),
            _ => error.change_context(MetadataSchemaError::InternalServerError),
        })
    }

    fn to_duplicate_response(
        self,
        context: impl FnOnce() -> MetadataSchemaError,
    ) -> Result<T, Report<MetadataSchemaError>> {
        self.map_err(|error| match error.current_context() {
            DatabaseError::UniqueViolation => error.change_context(context()),
            _ => error.change_context(MetadataSchemaError::InternalServerError),
        })
    }
}

impl<T> StorageErrorExt<T, SearchGroupError> for Result<T, Report<DatabaseError>> {
    fn to_not_found_response(
        self,
        context: impl FnOnce() -> SearchGroupError,
    ) -> Result<T, Report<SearchGroupError>> {
        self.map_err(|error| match error.current_context() {
            DatabaseError::NotFound => error.change_context(context()),
            _ => error.change_context(SearchGroupError::InternalServerError),
        })
    }

    fn to_duplicate_response(
        self,
        context: impl FnOnce() -> SearchGroupError,
    ) -> Result<T, Report<SearchGroupError>> {
        self.map_err(|error| match error.current_context() {
            DatabaseError::UniqueViolation => error.change_context(context()),
            _ => error.change_context(SearchGroupError::InternalServerError),
        })
    }
}

impl<T> StorageErrorExt<T, SearchExpectationError> for Result<T, Report<DatabaseError>> {
    fn to_not_found_response(
        self,
        context: impl FnOnce() -> SearchExpectationError,
    ) -> Result<T, Report<SearchExpectationError>> {
        self.map_err(|error| match error.current_context() {
            DatabaseError::NotFound => error.change_context(context()),
            _ => error.change_context(SearchExpectationError::InternalServerError),
        })
    }

    fn to_duplicate_response(
        self,
        context: impl FnOnce() -> SearchExpectationError,
    ) -> Result<T, Report<SearchExpectationError>> {
        self.map_err(|error| match error.current_context() {
            DatabaseError::UniqueViolation => error.change_context(context()),
            _ => error.change_context(SearchExpectationError::InternalServerError),
        })
    }
}

impl<T> StorageErrorExt<T, ReportError> for Result<T, Report<DatabaseError>> {
    fn to_not_found_response(
        self,
        context: impl FnOnce() -> ReportError,
    ) -> Result<T, Report<ReportError>> {
        self.map_err(|error| match error.current_context() {
            DatabaseError::NotFound => error.change_context(context()),
            _ => error.change_context(ReportError::InternalServerError),
        })
    }

    fn to_duplicate_response(
        self,
        context: impl FnOnce() -> ReportError,
    ) -> Result<T, Report<ReportError>> {
        self.map_err(|error| match error.current_context() {
            DatabaseError::UniqueViolation => error.change_context(context()),
            _ => error.change_context(ReportError::InternalServerError),
        })
    }
}

impl<T> StorageErrorExt<T, LinkConfigError> for Result<T, Report<DatabaseError>> {
    fn to_not_found_response(
        self,
        context: impl FnOnce() -> LinkConfigError,
    ) -> Result<T, Report<LinkConfigError>> {
        self.map_err(|error| match error.current_context() {
            DatabaseError::NotFound => error.change_context(context()),
            _ => error.change_context(LinkConfigError::InternalServerError),
        })
    }

    fn to_duplicate_response(
        self,
        context: impl FnOnce() -> LinkConfigError,
    ) -> Result<T, Report<LinkConfigError>> {
        self.map_err(|error| match error.current_context() {
            DatabaseError::UniqueViolation => error.change_context(context()),
            _ => error.change_context(LinkConfigError::InternalServerError),
        })
    }
}
