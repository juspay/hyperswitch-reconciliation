use api_models::recon_engine::{ProcessorStatusResponse, ReconTriggerResponse};
use common_enums::{DieselStagingEntryStatus, ReconProcessorStatus};
use common_types::staging_entry::status::DomainStagingEntryStatus;
use common_utils::{auth::AuthContext, logging};
use tokio::sync::watch;

use crate::{
    domain::recon_engine::process_staging_entry_core_v2,
    errors::{conversions::StorageErrorExt, recon_engine::ReconEngineResponse, ReconEngineError},
    routes::AppState,
    types::ApplicationResponse,
};

pub async fn process_staging_entry(
    state: AppState,
    id: String,
    auth_context: AuthContext,
) -> ReconEngineResponse<ReconTriggerResponse> {
    // Load staging entry
    let staging_entry = state
        .store
        .find_staging_entry_by_id_and_profile(id.clone(), auth_context.profile_id.clone())
        .await
        .to_not_found_response(|| ReconEngineError::StagingEntryNotFound { id: id.clone() })?;

    // Check if the staging entry is in a processable state
    match DomainStagingEntryStatus::from(staging_entry.status) {
        DomainStagingEntryStatus::Pending | DomainStagingEntryStatus::NeedsManualReview(_) => {}
        DomainStagingEntryStatus::Processed => {
            return Err(ReconEngineError::StagingEntryAlreadyProcessed { id }.into());
        }
        DomainStagingEntryStatus::Void => {
            return Err(ReconEngineError::StagingEntryVoided { id }.into());
        }
        DomainStagingEntryStatus::Archived => {
            return Err(ReconEngineError::StagingEntryArchived { id }.into());
        }
    }

    let response = Box::pin(process_staging_entry_core_v2(&state, staging_entry, true)).await?;

    Ok(ApplicationResponse::json(response))
}

/// Force processes a staging entry, bypassing duplicate detection
/// Used for manual approval of entries in NeedsManualReview status due to duplicates
pub async fn process_duplicate_staging_entry(
    state: AppState,
    id: String,
    auth_context: AuthContext,
) -> ReconEngineResponse<ReconTriggerResponse> {
    // Load staging entry with profile verification
    let staging_entry = state
        .store
        .find_staging_entry_by_id_and_profile(id.clone(), auth_context.profile_id.clone())
        .await
        .to_not_found_response(|| ReconEngineError::StagingEntryNotFound { id: id.clone() })?;

    // Validate that staging entry represents a duplicate condition that can be force processed
    if !staging_entry.data.is_duplicate() {
        return Err(ReconEngineError::InvalidForDuplicateProcessing { id }.into());
    }

    // Process the staging entry using core logic with duplicate check bypass
    let response = Box::pin(process_staging_entry_core_v2(&state, staging_entry, false)).await?;

    Ok(ApplicationResponse::json(response))
}

pub async fn set_processor_status(
    state: AppState,
    status: ReconProcessorStatus,
) -> ReconEngineResponse<()> {
    state
        .recon_processor_status
        .send(status)
        .map_err(|_| ReconEngineError::InternalServerError)?;

    Ok(ApplicationResponse::StatusOk)
}

pub async fn get_processor_status(
    state: &AppState,
) -> ReconEngineResponse<ProcessorStatusResponse> {
    let current_status = *state.recon_processor_status.borrow();

    Ok(ApplicationResponse::json(ProcessorStatusResponse {
        status: current_status,
    }))
}

pub async fn recon_processor_v2(
    state: AppState,
    mut status: watch::Receiver<ReconProcessorStatus>,
) {
    logging::info!("Recon processor task started");

    const BATCH_SIZE: u16 = 1; // Keep 1 for now due to transaction/confirmation race safety

    loop {
        let current_status = *status.borrow_and_update();

        match current_status {
            ReconProcessorStatus::Running => {
                logging::info!("Recon processor is running...");
                // Reduced batch size to 1 to prevent race conditions between transaction and confirmation processing
                // When both are processed concurrently, confirmations may not find expected entries created by transactions
                match state
                    .store
                    .list_all_staging_entries(Some(DieselStagingEntryStatus::Pending), BATCH_SIZE)
                    .await
                {
                    Ok(staging_entries) if staging_entries.is_empty() => {
                        logging::info!("No pending staging entries to process, sleeping...");
                        // Wait a bit before next iteration if empty
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    }
                    Ok(staging_entries) => {
                        logging::info!("Found {} pending staging entries", staging_entries.len());

                        // Process each staging entry individually
                        let futures: Vec<_> = staging_entries
                            .into_iter()
                            .map(|staging_entry| {
                                let state_clone = state.clone();
                                async move {
                                    Box::pin(async move {
                                        process_staging_entry_core_v2(
                                            &state_clone,
                                            staging_entry,
                                            true,
                                        )
                                        .await
                                    })
                                    .await
                                }
                            })
                            .collect();

                        // Wait for all entries to complete
                        futures::future::join_all(futures).await;
                    }
                    Err(e) => {
                        logging::error!("Failed to list staging entries: {}", e);
                        // Wait a bit before next iteration if error
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    }
                }
            }
            ReconProcessorStatus::Stopped => {
                logging::info!("Recon processor is stopped, waiting for status change...");

                // Wait for the status to change
                if status.changed().await.is_err() {
                    logging::error!("Recon processor channel closed, exiting");
                    break;
                }
            }
        }
    }

    logging::info!("Recon processor task exiting");
}
