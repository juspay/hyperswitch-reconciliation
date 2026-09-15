# Reporting - Agent Notes

## How It Works
<!-- AI-UPDATED: 2026-08-24 - Report generation feature added -->
`POST /report/transactions` and `POST /report/exceptions` (`routes/reporting.rs`) each deserialize a filter body and call `core::reporting::generate_transaction_report`/`generate_exception_report`. Both core functions follow the same steps independently:
1. Look up `rule_id` scoped to the caller's profile (`ReconRuleInterface::find_recon_rule_by_profile_id_and_rule_id`) - 404s if it doesn't exist or belongs to a different profile.
2. Compute the aging cutoff for that rule as of now (`rule.aging_config.missing_before(date_time::now())`).
3. Resolve the caller's own email (`UserInterface::get_user_by_id`) and add it to the request's `emails` list.
4. Build a `TransactionReportLambdaPayload`/`ExceptionReportLambdaPayload` (`domain/reporting.rs`) and invoke the configured Lambda function (`ReportConfig::function_name`, `config.rs`) via `state.faas.invoke_event`.

The Lambda queries the DB itself and produces the report - `recon` only ever hands it a filter/context payload, never rows.

## Design Decisions
<!-- AI-UPDATED: 2026-08-24 - Report generation feature added -->
<!-- AI-UPDATED: 2026-09-09 - Report permissions changed Write -> Read -->
- Report logic lives entirely in the `recon` crate (`api_models::reporting`, `core/reporting.rs`, `domain/reporting.rs`, `errors/reporting.rs`, `routes/reporting.rs`) - `external_services::faas` stays a generic transport with no report-specific knowledge
- `generate_transaction_report`/`generate_exception_report` are plain functions, not a trait - each does its own rule lookup, email resolution, aging computation, payload construction, and Lambda invoke independently; nothing is factored into a shared helper, including the invoke call itself
- `domain/reporting.rs` holds only the two Lambda payload structs (plain data, no logic, no I/O) - all DB/user-client/FaaS calls stay in `core/reporting.rs`
- `TransactionReportFilters`/`ExceptionReportFilters` and their Lambda payload counterparts are separate types per report kind, not one shared type - they may need different fields later
- Reports are gated by the existing `ProfileReconTransactionRead`/`ProfileReconExceptionRead` permissions rather than a new `ReconReport` permission - a brand new permission would need to be registered and granted to roles in the separate user-authorization system before it could work in production, so reusing an already-granted permission avoids that dependency entirely. `Read` (not `Write`) because triggering a report mutates nothing in recon - it reads existing rows and mails them out; the `POST` is only there to carry a filter body

## Common Gotchas
<!-- AI-UPDATED: 2026-08-24 - Report generation feature added -->
- `rule_id` is required in both filter types - it's the only scoping the report query has, and one request always maps to exactly one rule (no fan-out across a profile's rules)
- The caller's own email is always added to the outbound Lambda payload's `emails`, even if the request sent none - the request field is optional, the payload's is never empty
- `aging_less_than` in the request is an "as of" reference time, not the aging cutoff itself - `AgingConfig::missing_before` computes the actual value sent to the Lambda, and it can be `None` for a rule with no aging config configured
- `end_time` in the request is optional and defaults to now when omitted; the Lambda payload always carries both `start_time` and `end_time` explicitly
- A failed Lambda invoke fails the request (500, `ReportError::LambdaInvocationFailed`) rather than being logged and ignored
- `TransactionReportLambdaPayload`/`ExceptionReportLambdaPayload`'s `PrimitiveDateTime` fields must carry `#[serde(with = "common_utils::custom_serde::iso8601")]` (`::option` for `aging_less_than`) - `time`'s bare `Serialize` for `PrimitiveDateTime` serializes as a `(year, ordinal_day, hour, minute, second, nanosecond)` tuple, not an ISO string, unless the `serde-human-readable` feature is on (it isn't anywhere in this workspace); the real Lambda's `parse_time()` expects a string and will fail on the tuple
