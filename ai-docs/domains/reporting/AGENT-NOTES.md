# Reporting - Agent Notes

## How It Works
<!-- AI-UPDATED: 2026-11-02 - Transaction-flow report flow and validation -->
`POST /report/transactions`, `POST /report/exceptions`, and `POST /report/transaction_flows` (`routes/reporting.rs`) deserialize separate filter bodies and call the corresponding `core::reporting::generate_*_report` function. Each core function independently:
1. Validates the time range before any DB/user/FaaS calls; invalid ranges return 400.
2. Looks up `rule_id` scoped to the caller's profile (`ReconRuleInterface::find_recon_rule_by_profile_id_and_rule_id`) - 404s if it doesn't exist or belongs to a different profile.
3. For transaction/exception reports only, computes the aging cutoff as of now (`rule.aging_config.missing_before(date_time::now())`). Transaction-flow reports do not compute or send an aging cutoff.
4. Resolves the caller's own email (`UserInterface::get_user_by_id`) and merges it with the request's `emails` using a `HashSet` to deduplicate recipients.
5. Builds the corresponding Lambda payload (`domain/reporting.rs`) and invokes the configured function (`ReportConfig::function_name`, `config.rs`) via `state.faas.invoke_event`.

The Lambda queries the DB itself and produces the report - `recon` only ever hands it a filter/context payload, never rows.
HTTP 200 means the asynchronous invocation was accepted, not that report generation or email delivery completed.

## Design Decisions
<!-- AI-UPDATED: 2026-08-24 - Report generation feature added -->
<!-- AI-UPDATED: 2026-09-09 - Report permissions changed Write -> Read -->
<!-- AI-UPDATED: 2026-11-02 - Extend report contracts and permissions to transaction flows -->
- Report logic lives entirely in the `recon` crate (`api_models::reporting`, `core/reporting.rs`, `domain/reporting.rs`, `errors/reporting.rs`, `routes/reporting.rs`) - `external_services::faas` stays a generic transport with no report-specific knowledge
- The three `generate_*_report` functions are plain functions, not a trait - each owns its rule lookup, email resolution, payload construction, and Lambda invoke; aging computation applies only to transaction/exception reports
- `domain/reporting.rs` holds the three Lambda payload structs (plain data, no logic, no I/O) - all DB/user-client/FaaS calls stay in `core/reporting.rs`
- `TransactionReportFilters`/`ExceptionReportFilters`/`TransactionFlowReportFilters` and their Lambda payload counterparts are separate types per report kind - the transaction-flow payload has only `profile_id`, `rule_id`, `start_time`, `end_time`, and `emails`
- Reports are gated by the existing `ProfileReconTransactionRead`/`ProfileReconExceptionRead` permissions rather than a new `ReconReport` permission - a brand new permission would need to be registered and granted to roles in the separate user-authorization system before it could work in production, so reusing an already-granted permission avoids that dependency entirely. `Read` (not `Write`) because triggering a report mutates nothing in recon - it reads existing rows and mails them out; the `POST` is only there to carry a filter body
- Transaction-flow reports use JWT auth with `ProfileReconTransactionRead`, the same permission as transaction reports

## Common Gotchas
<!-- AI-UPDATED: 2026-11-02 - Transaction-flow request/config contract and aging clarification -->
- `rule_id` is required in all three filter types; one request maps to exactly one rule within the authenticated profile (no fan-out across a profile's rules)
- The transaction-flow URL is `/report/transaction_flows` (plural, snake_case); its request uses top-level `start_time`/`end_time` because `TimeRange` is flattened, not a nested `time_range` object
- The caller's own email is always added to the outbound Lambda payload's `emails`, even if the request sent none - the request field is optional, the payload's is never empty
- `aging_less_than` is an outbound transaction/exception payload field, not a request filter - `AgingConfig::missing_before(now)` computes it, and it can be `None` for a rule with no aging config; transaction-flow payloads omit the field entirely
- `end_time` in the request is optional and defaults to now when omitted; the Lambda payload always carries both `start_time` and `end_time` explicitly
- A failed Lambda invoke fails the request (500, `ReportError::LambdaInvocationFailed`) rather than being logged and ignored
- All three Lambda payloads' `PrimitiveDateTime` fields must carry `#[serde(with = "common_utils::custom_serde::iso8601")]` (`::option` for `aging_less_than`) - `time`'s bare `Serialize` for `PrimitiveDateTime` serializes as a `(year, ordinal_day, hour, minute, second, nanosecond)` tuple, not an ISO string, unless the `serde-human-readable` feature is on (it isn't anywhere in this workspace); the real Lambda's `parse_time()` expects a string and will fail on the tuple
- `[reporting_service].transaction_flow_report_function` is required alongside the transaction/exception function names; `ReportConfig::validate()` rejects whitespace-only or empty values at startup even if transaction-flow reports are unused. Shipped configs use `noop-transaction-flow-report` locally/CI/tests/docker-compose and `recon-linked-entry-report` in sandbox/production - that is the deployed Lambda's pre-rename name, so the value intentionally doesn't match the key until the Lambda itself is renamed
