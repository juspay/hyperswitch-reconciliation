# Health - Agent Notes

## Critical Rules
<!-- AI-UPDATED: 2026-11-03 - Deep health check added -->
- **`/health` stays shallow; liveness/readiness probes must use it, never `/deep_health`**. A failing readiness probe pulls the instance from traffic, so one dependency (keymanager, Hyperswitch) being down would take every instance, and therefore every API, out of service at once
- **`/deep_health` is unauthenticated (`NoAuth`)**, so responses carry only the error enum's `Display` text; the full `error_stack` report (hosts, URLs) is logged, never returned
- **Every check reuses a client recon already uses for real work**. Don't add a check that needs a new connection or permission just for health

## What Is Checked
<!-- AI-UPDATED: 2026-11-03 - Deep health check added -->
- `database`: plain `SELECT 1` - read only, by choice. Hyperswitch's router also inserts/deletes a `configs` row; recon never writes from a health check
- `encryption`: keymanager `GET {url}/health` over recon's existing **mTLS client**, via `common_utils::keymanager::health_check_key_manager`. Don't switch to the keymanager's plain-HTTP management endpoint - the mTLS path is what recon actually depends on. No tenant header needed
- `user_client`: Hyperswitch's **shallow** `GET {base_url}/health`, not `/health/ready` - otherwise a Hyperswitch Redis/locker blip would mark recon unhealthy
- `recon_processor` / `linking_processor`: each task owns the only `Receiver` on its `watch` channel and loops forever, so `Sender::receiver_count() == 0` means it exited or panicked. An admin-stopped processor is still healthy; its run state is on the `recon_engine/status` API, not here
- Local encryption and the mock user client have nothing external to check: their `health_check()` returns `Ok(())`, so they report `healthy`

## Deliberately Not Checked
<!-- AI-UPDATED: 2026-11-03 - Deep health check added -->
- **File storage (S3/OCI)** - Hyperswitch doesn't check it either. A `file_exists` probe on a missing key returns 403, not 404, so it can't tell healthy from broken
- **Lambda (FaaS)** - Hyperswitch doesn't check it. Invoking runs real work (sends report emails)
- **Secrets management** - only used at startup; if the process booted, it worked

## Design Decisions
<!-- AI-UPDATED: 2026-11-03 - Deep health check added -->
- Checks run concurrently, each with a 5s timeout (`utils/health.rs`). The endpoint always returns the full per-component report: 200 when all are healthy, 503 otherwise. Unlike Hyperswitch's `/health/ready`, it doesn't stop at the first failure, so one outage can't hide another
- Backend-variant dispatch lives in `external_services` (`ApplicationEncryptionClient::health_check`, `UserClient::health_check`); `core/health.rs` only orchestrates
- Path is `/deep_health` (its own scope), not `/health/deep`
- One `status`-tagged `ComponentHealth` enum for every component (processors too, without `latency_ms`); `error` exists only on `unhealthy`. No overall `status` field: the HTTP code (200/503) is the overall result. Component names in logs come from the `HealthComponent` enum, not string literals

## Common Gotchas
<!-- AI-UPDATED: 2026-11-03 - Deep health check added -->
- **A processor that subscribes extra receivers** (`sender.subscribe()`) breaks the liveness signal - the count stays above zero after the task dies
