use api_models::reporting::{ExceptionReportFilters, TransactionReportFilters};
use common_utils::{auth::AuthContext, date_time};
use error_stack::ResultExt;
use external_services::user::interfaces::UserInterface;
use std::collections::HashSet;

use crate::{
    config::ReportType,
    db::recon_rule::ReconRuleInterface,
    domain::reporting::{ExceptionReportLambdaPayload, TransactionReportLambdaPayload},
    errors::{
        conversions::StorageErrorExt,
        reporting::{ReportError, ReportResponse},
    },
    routes::AppState,
    types::ApplicationResponse,
};

pub async fn generate_transaction_report(
    state: AppState,
    auth: AuthContext,
    filters: TransactionReportFilters,
) -> ReportResponse<()> {
    filters
        .validate()
        .map_err(|_| ReportError::TimeRangeValidationError)?;

    let rule = state
        .store
        .find_recon_rule_by_profile_id_and_rule_id(&auth.profile_id, &filters.rule_id)
        .await
        .to_not_found_response(|| ReportError::RuleNotFound {
            rule_id: filters.rule_id.clone(),
        })?;
    let missing_before = rule.aging_config.missing_before(date_time::now());

    let primary_email = state
        .user_client
        .get_user_by_id(
            auth.merchant_id.clone(),
            auth.profile_id.clone(),
            auth.user_id.clone(),
        )
        .await
        .change_context(ReportError::InternalServerError)?
        .email;
    let emails: HashSet<String> = filters
        .emails
        .into_iter()
        .chain(vec![primary_email])
        .collect();

    let payload = TransactionReportLambdaPayload {
        profile_id: auth.profile_id,
        rule_id: filters.rule_id,
        start_time: filters.time_range.start_time,
        end_time: filters.time_range.end_time.unwrap_or_else(date_time::now),
        emails: emails.into_iter().collect(),
        aging_less_than: missing_before,
    };
    let payload_bytes =
        serde_json::to_vec(&payload).change_context(ReportError::PayloadSerializationFailed)?;

    state
        .faas
        .invoke_event(
            state
                .config
                .reporting_service
                .function_name(ReportType::Transaction),
            payload_bytes,
        )
        .await
        .change_context(ReportError::LambdaInvocationFailed)?;

    Ok(ApplicationResponse::StatusOk)
}

pub async fn generate_exception_report(
    state: AppState,
    auth: AuthContext,
    filters: ExceptionReportFilters,
) -> ReportResponse<()> {
    filters
        .validate()
        .map_err(|_| ReportError::TimeRangeValidationError)?;

    let rule = state
        .store
        .find_recon_rule_by_profile_id_and_rule_id(&auth.profile_id, &filters.rule_id)
        .await
        .to_not_found_response(|| ReportError::RuleNotFound {
            rule_id: filters.rule_id.clone(),
        })?;
    let missing_before = rule.aging_config.missing_before(date_time::now());

    let primary_email = state
        .user_client
        .get_user_by_id(
            auth.merchant_id.clone(),
            auth.profile_id.clone(),
            auth.user_id.clone(),
        )
        .await
        .change_context(ReportError::InternalServerError)?
        .email;
    let emails: HashSet<String> = filters
        .emails
        .into_iter()
        .chain(vec![primary_email])
        .collect();

    let payload = ExceptionReportLambdaPayload {
        profile_id: auth.profile_id,
        rule_id: filters.rule_id,
        start_time: filters.time_range.start_time,
        end_time: filters.time_range.end_time.unwrap_or_else(date_time::now),
        emails: emails.into_iter().collect(),
        aging_less_than: missing_before,
    };
    let payload_bytes =
        serde_json::to_vec(&payload).change_context(ReportError::PayloadSerializationFailed)?;

    state
        .faas
        .invoke_event(
            state
                .config
                .reporting_service
                .function_name(ReportType::Exception),
            payload_bytes,
        )
        .await
        .change_context(ReportError::LambdaInvocationFailed)?;

    Ok(ApplicationResponse::StatusOk)
}
