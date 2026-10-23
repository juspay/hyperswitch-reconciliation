use api_models::linking_processor::{LinkingProcessorStatusResponse, LinkingTriggerResponse};
use common_enums::{EntryLinkingStatus, LinkingProcessorStatus};
use common_types::entry_linking::{EntryLinkingData, EntryLinkingDataV1};
use common_utils::{auth::AuthContext, logging};
use diesel_models::entry::EntryUpdate;
use tokio::sync::watch;

use crate::{
    db::{account::AccountInterface, entry::EntryInterface},
    domain::linking_processor::process_entry_linking_core,
    errors::{
        conversions::StorageErrorExt,
        linking_processor::{LinkingProcessorError, LinkingProcessorResponse},
    },
    routes::AppState,
    types::ApplicationResponse,
};

pub async fn set_processor_status(
    state: AppState,
    status: LinkingProcessorStatus,
) -> LinkingProcessorResponse<()> {
    state
        .linking_processor_status
        .send(status)
        .map_err(|_| LinkingProcessorError::InternalServerError)?;

    Ok(ApplicationResponse::StatusOk)
}

pub async fn get_processor_status(
    state: &AppState,
) -> LinkingProcessorResponse<LinkingProcessorStatusResponse> {
    let current_status = *state.linking_processor_status.borrow();

    Ok(ApplicationResponse::json(LinkingProcessorStatusResponse {
        status: current_status,
    }))
}

pub async fn enqueue_entry(
    state: AppState,
    entry_id: String,
    auth_context: AuthContext,
) -> LinkingProcessorResponse<()> {
    let entry = state
        .store
        .find_entry_by_id(&entry_id)
        .await
        .to_not_found_response(|| LinkingProcessorError::EntryNotFound)?;

    state
        .store
        .find_account_by_profile_id_and_account_id(&auth_context.profile_id, &entry.account_id)
        .await
        .to_not_found_response(|| LinkingProcessorError::EntryNotFound)?;

    let expected_statuses = EntryLinkingStatus::enqueuable_statuses();
    if !expected_statuses.contains(&entry.linking_status) {
        return Err(LinkingProcessorError::InvalidEntryLinkingStatus {
            current_status: entry.linking_status,
            expected_statuses,
        }
        .into());
    }

    state
        .store
        .update_entry_by_id(
            &entry_id,
            EntryUpdate::LinkingUpdate {
                data: EntryLinkingData::V1(EntryLinkingDataV1::Pending),
            },
        )
        .await
        .to_not_found_response(|| LinkingProcessorError::EntryNotFound)?;

    Ok(ApplicationResponse::StatusOk)
}

pub async fn process_entry(
    state: AppState,
    entry_id: String,
) -> LinkingProcessorResponse<LinkingTriggerResponse> {
    let entry = state
        .store
        .find_entry_by_id(&entry_id)
        .await
        .to_not_found_response(|| LinkingProcessorError::EntryNotFound)?;

    if entry.linking_status != EntryLinkingStatus::Pending {
        return Err(LinkingProcessorError::InvalidEntryLinkingStatus {
            current_status: entry.linking_status,
            expected_statuses: vec![EntryLinkingStatus::Pending],
        }
        .into());
    }

    let response = process_entry_linking_core(&state, entry).await?;
    Ok(ApplicationResponse::json(response))
}

pub async fn linking_processor(
    state: AppState,
    mut status: watch::Receiver<LinkingProcessorStatus>,
) {
    logging::info!("Linking processor task started");

    const BATCH_SIZE: u16 = 1;
    loop {
        let current_status = *status.borrow_and_update();
        match current_status {
            LinkingProcessorStatus::Running => {
                logging::info!("Linking processor is running...");

                match state.store.list_linking_pending_entries(BATCH_SIZE).await {
                    Ok(entries) if entries.is_empty() => {
                        logging::info!("No pending entries to link, sleeping...");
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    }
                    Ok(entries) => {
                        logging::info!("Found {} linking pending entries", entries.len());

                        let futures: Vec<_> = entries
                            .into_iter()
                            .map(|entry| {
                                let state_clone = state.clone();
                                async move {
                                    Box::pin(async move {
                                        process_entry_linking_core(&state_clone, entry).await
                                    })
                                    .await
                                }
                            })
                            .collect();

                        futures::future::join_all(futures).await;
                    }
                    Err(e) => {
                        logging::error!("Failed to list entries pending to be linked: {}", e);
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    }
                }
            }
            LinkingProcessorStatus::Stopped => {
                logging::info!("Linking processor is stopped, waiting for status change...");

                if status.changed().await.is_err() {
                    logging::error!("Linking processor channel closed, exiting");
                    break;
                }
            }
        }
    }
}
