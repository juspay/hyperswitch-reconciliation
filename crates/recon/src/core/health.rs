use api_models::health::DeepHealthResponse;
use common_enums::{LinkingProcessorStatus, ReconProcessorStatus};

use crate::{
    routes::AppState,
    types::ReconResponse,
    utils::health::{
        health_response, overall_status, run_check, run_optional_check, ProcessorStatusExt,
    },
};

pub async fn deep_health_check(state: AppState) -> ReconResponse<DeepHealthResponse> {
    let (database, encryption, user_client) = futures::join!(
        run_check("database", state.store.health_check()),
        run_optional_check("encryption", state.encryption_client.health_check()),
        run_optional_check("user_client", state.user_client.health_check()),
    );

    let recon_processor = state
        .recon_processor_status
        .processor_health(ReconProcessorStatus::Running);
    let linking_processor = state
        .linking_processor_status
        .processor_health(LinkingProcessorStatus::Running);

    let status = overall_status(&[
        database.status,
        encryption.status,
        user_client.status,
        recon_processor.status,
        linking_processor.status,
    ]);

    Ok(health_response(
        status,
        DeepHealthResponse {
            status,
            database,
            encryption,
            user_client,
            recon_processor,
            linking_processor,
        },
    ))
}
