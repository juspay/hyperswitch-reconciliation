use common_types::staging_entry::{ProcessingErrorType, ProcessingErrorTypeV1};
use masking::Secret;
use sha2::{Digest, Sha256};

use crate::domain::recon_engine::ProcessingError;

/// Hash string for privacy-preserving duplicate detection
pub fn hash_string(value: &str) -> Secret<String> {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    let hash_bytes = hasher.finalize();
    let hash_hex = hex::encode(hash_bytes);

    Secret::new(hash_hex)
}

impl From<ProcessingError> for ProcessingErrorType {
    fn from(error: ProcessingError) -> Self {
        Self::V1(match error {
            ProcessingError::NoRulesFound => ProcessingErrorTypeV1::NoRulesFound,
            ProcessingError::MissingSearchIdentifierValue(field) => {
                ProcessingErrorTypeV1::MissingSearchIdentifierValue {
                    missing_field: field.clone(),
                }
            }
            ProcessingError::MissingMatchField(field) => ProcessingErrorTypeV1::MissingMatchField {
                missing_field: field,
            },
            ProcessingError::StagingEntryCurrencyMismatch {
                staging_currency,
                account_currency,
            } => ProcessingErrorTypeV1::StagingEntryCurrencyMismatch {
                staging_currency: staging_currency.to_string(),
                account_currency: account_currency.to_string(),
            },
            ProcessingError::DuplicateEntry => ProcessingErrorTypeV1::DuplicateEntry,
            ProcessingError::NoExpectationEntryFound => {
                ProcessingErrorTypeV1::NoExpectationEntryFound
            }
            ProcessingError::MultipleExpectedEntriesFound { transaction_ids } => {
                ProcessingErrorTypeV1::MultipleExpectedEntriesFound { transaction_ids }
            }
            ProcessingError::MissingUniqueField(field) => {
                ProcessingErrorTypeV1::MissingUniqueField {
                    missing_field: field,
                }
            }
            ProcessingError::MissingGroupingField(field) => {
                ProcessingErrorTypeV1::MissingGroupingField {
                    missing_field: field,
                }
            }
            ProcessingError::DatabaseError => ProcessingErrorTypeV1::InternalError,
            ProcessingError::InternalError(_) => ProcessingErrorTypeV1::InternalError,
        })
    }
}

#[macro_export]
macro_rules! process_or_fail {
    ($operation:expr, $state:expr, $staging_entry:expr) => {
        match $operation.await {
            Ok(value) => value,
            Err(processing_error) => {
                let reason = $crate::domain::recon_engine::process_error_context(
                    $state,
                    processing_error,
                    $staging_entry,
                )
                .await?;
                return Ok(::api_models::recon_engine::ReconTriggerResponse::Failure(
                    reason,
                ));
            }
        }
    };
}

#[macro_export]
macro_rules! define_recon_flow {
    (
        $( $engine:ident ),* $(,)?
    ) => {
        pub enum ReconEngineType {
            $(
                $engine($engine),
            )*
        }

        impl ReconEngineType {
            pub async fn prep_transaction(
                self,
                state: &AppState,
                staging_entry: DomainStagingEntry,
                transaction_context: TransactionContext,
            ) -> ProcessingResult<ReconPrepTransaction> {
                match self {
                    $(
                        Self::$engine(engine) => {
                            engine.prep_transaction(state, staging_entry, transaction_context).await
                        }
                    )*
                }
            }

            pub async fn prep_confirmation(
                self,
                state: &AppState,
                staging_entry: DomainStagingEntry,
                confirmation_context: ConfirmationContext,
            ) -> ProcessingResult<ReconPrepConfirmation> {
                match self {
                    $(
                        Self::$engine(engine) => {
                            engine.prep_confirmation(state, staging_entry, confirmation_context).await
                        }
                    )*
                }
            }

            pub fn evaluate_trigger(&self, data: &impl ExtractFromEntryField) -> bool {
                match self {
                    $(
                        Self::$engine(engine) => engine.evaluate_trigger(data),
                    )*
                }
            }

            pub fn get_search_criteria(
                &self,
                data: &impl ExtractFromEntryField,
                rule_id: &str,
            ) -> ProcessingResult<Vec<SearchCriteria>> {
                match self {
                    $(
                        Self::$engine(engine) => engine.get_search_criteria(data, rule_id),
                    )*
                }
            }
        }

        pub enum ReconPrepTransaction {
            $(
                $engine {
                    engine: $engine,
                    prep: <$engine as ReconEngine>::PrepTypeTransaction,
                },
            )*
        }

        pub enum ReconPrepConfirmation {
            $(
                $engine {
                    engine: $engine,
                    prep: <$engine as ReconEngine>::PrepTypeConfirmation,
                },
            )*
        }

        pub enum ReconContextTransaction {
            $(
                $engine {
                    engine: $engine,
                    context: Box<<$engine as ReconEngine>::ContextTypeTransaction>,
                },
            )*
        }

        pub enum ReconContextConfirmation {
            $(
                $engine {
                    engine: $engine,
                    context: Box<<$engine as ReconEngine>::ContextTypeConfirmation>,
                },
            )*
        }

        impl ReconPrepTransaction {
            pub async fn create_transaction_context(
                self,
                state: &AppState,
            ) -> ProcessingResult<ReconContextTransaction> {
                match self {
                    $(
                        Self::$engine { engine, prep } => {
                            Box::pin(engine.create_transaction_context(state, prep)).await
                        }
                    )*
                }
            }
        }

        impl ReconPrepConfirmation {
            pub async fn create_confirmation_context(
                self,
                state: &AppState,
            ) -> ProcessingResult<ReconContextConfirmation> {
                match self {
                    $(
                        Self::$engine { engine, prep } => {
                            Box::pin(engine.create_confirmation_context(state, prep)).await
                        }
                    )*
                }
            }
        }

        impl ReconContextTransaction {
            pub async fn process_transaction_context(
                self,
                state: &AppState,
            ) -> ProcessingResult<Transaction> {
                match self {
                    $(
                        Self::$engine { engine, context } => {
                            engine.process_transaction_context(state, *context).await
                        }
                    )*
                }
            }
        }

        impl ReconContextConfirmation {
            pub async fn process_confirmation_context(
                self,
                state: &AppState,
            ) -> ProcessingResult<Transaction> {
                match self {
                    $(
                        Self::$engine { engine, context } => {
                            engine.process_confirmation_context(state, *context).await
                        }
                    )*
                }
            }
        }
    };
}
