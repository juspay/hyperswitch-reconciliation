use api_models::linking_engine::LinkingProcessorStatusResponse;
use common_enums::LinkingProcessorStatus;
use common_utils::logging;
use tokio::sync::watch;

use crate::{
    db::entry::EntryInterface,
    domain::linking_engine::process_entry_linking_core,
    errors::linking_engine::{LinkingEngineError, LinkingEngineResponse},
    routes::AppState,
    types::ApplicationResponse,
};

pub async fn set_processor_status(
    state: AppState,
    status: LinkingProcessorStatus,
) -> LinkingEngineResponse<()> {
    state
        .linking_processor_status
        .send(status)
        .map_err(|_| LinkingEngineError::InternalServerError)?;

    Ok(ApplicationResponse::StatusOk)
}

pub async fn get_processor_status(
    state: &AppState,
) -> LinkingEngineResponse<LinkingProcessorStatusResponse> {
    let current_status = *state.linking_processor_status.borrow();

    Ok(ApplicationResponse::json(LinkingProcessorStatusResponse {
        status: current_status,
    }))
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
