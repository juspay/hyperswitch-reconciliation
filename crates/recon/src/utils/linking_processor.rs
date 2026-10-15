use common_types::entry_linking::{LinkingProcessingErrorType, LinkingProcessingErrorTypeV1};

use crate::domain::linking_processor::LinkingProcessingError;

impl From<LinkingProcessingError> for LinkingProcessingErrorType {
    fn from(error: LinkingProcessingError) -> Self {
        Self::V1(match error {
            LinkingProcessingError::InternalError(message) => {
                LinkingProcessingErrorTypeV1::InternalError { message }
            }
            LinkingProcessingError::DatabaseError => LinkingProcessingErrorTypeV1::InternalError {
                message: error.to_string(),
            },
            LinkingProcessingError::TransformationConfigMissing => {
                LinkingProcessingErrorTypeV1::TransformationConfigMissing
            }
            LinkingProcessingError::NoLinkConfigFound => {
                LinkingProcessingErrorTypeV1::NoLinkConfigFound
            }
            LinkingProcessingError::MissingSearchIdentifierValue(missing_field) => {
                LinkingProcessingErrorTypeV1::MissingSearchIdentifierValue { missing_field }
            }
            LinkingProcessingError::MultiplePostedLinksFound => {
                LinkingProcessingErrorTypeV1::MultiplePostedLinksFound
            }
        })
    }
}

#[macro_export]
macro_rules! process_linking_or_fail {
    ($operation:expr, $state:expr, $entry:expr) => {
        match $operation.await {
            Ok(value) => value,
            Err(processing_error) => {
                let reason = $crate::domain::linking_processor::process_linking_error_context(
                    $state,
                    processing_error,
                    $entry,
                )
                .await?;
                return Ok(
                    ::api_models::linking_processor::LinkingTriggerResponse::Failure(reason),
                );
            }
        }
    };
}
