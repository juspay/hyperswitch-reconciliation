# Health - Agent Notes

## Critical Rules
<!-- AI-UPDATED: 2026-11-03 - Deep health check added -->
- **`/health` stays shallow; Kubernetes probes must use it, never `/deep_health`**. A failing readiness probe pulls the pod from traffic, so one dependency (keymanager, Hyperswitch) being down would take every pod, and therefore every API, out of service at once. Probes live in `hyperswitch-infra/infra-configurations/recon/*-values.yaml`
- **`/deep_health` is unauthenticated (`NoAuth`) and exposed at the edge in sandbox only** (`hyperswitch-infra/envoy-whitelist-paths/sandbox/recon-engine/allowedlist.txt`); prod is cluster-internal until it's added to the prod allowlist. Responses carry only the error enum's `Display` text; the full `error_stack` report (hosts, URLs) is logged, never returned
- **Never add a check that needs an infra change** (new IAM permission, network path, or port). Every check reuses a connection recon already makes for real work

## What Is Checked
<!-- AI-UPDATED: 2026-11-03 - Deep health check added -->
- `database`: plain `SELECT 1` - read only, by choice. Hyperswitch's router also inserts/deletes a `configs` row; recon never writes from a health check
- `encryption`: keymanager `GET {url}/health` over the **main mTLS port** (443 -> 5000) with recon's client cert, via `common_utils::keymanager::health_check_key_manager`. The keymanager also serves `/health` on its plain-HTTP management port 6128 (its own k8s probes use that) - don't switch to it, the mTLS path is what recon actually depends on. No tenant header needed
- `user_client`: Hyperswitch's **shallow** `GET {base_url}/health`, not `/health/ready` - otherwise a Hyperswitch Redis/locker blip would mark recon unhealthy
- `recon_processor` / `linking_processor`: each task owns the only `Receiver` on its `watch` channel and loops forever, so `Sender::receiver_count() == 0` means it exited or panicked. `running: false` (admin-stopped) is still healthy
- Local encryption and the mock user client have nothing to check: their `health_check()` returns `None`, rendered as `not_applicable` (distinct from `healthy`)

## Deliberately Not Checked
<!-- AI-UPDATED: 2026-11-03 - Deep health check added -->
- **File storage (S3/OCI)** - Hyperswitch doesn't check it either. A `file_exists` probe on a missing key returns 403, not 404, without `s3:ListBucket`, which recon's role doesn't need (it only uses Put/GetObject)
- **Lambda (FaaS)** - Hyperswitch doesn't check it. Invoking runs real work (sends report emails), and a metadata read needs `lambda:GetFunctionConfiguration`, which the role isn't granted
- **Secrets management** - only used at startup; if the process booted, it worked

## Design Decisions
<!-- AI-UPDATED: 2026-11-03 - Deep health check added -->
- Checks run concurrently, each with a 5s timeout (`utils/health.rs`). The endpoint always returns the full per-component report: 200 when all are healthy or `not_applicable`, 503 otherwise. Unlike Hyperswitch's `/health/ready`, it doesn't stop at the first failure, so one outage can't hide another
- Backend-variant dispatch lives in `external_services` (`ApplicationEncryptionClient::health_check`, `UserClient::health_check`); `core/health.rs` only orchestrates
- Path is `/deep_health` (its own scope), not `/health/deep`

## Common Gotchas
<!-- AI-UPDATED: 2026-11-03 - Deep health check added -->
- **A processor that subscribes extra receivers** (`sender.subscribe()`) breaks the liveness signal - the count stays above zero after the task dies
- New routes must also be added to the Envoy allowlist in `hyperswitch-infra` to be reachable from outside the cluster
