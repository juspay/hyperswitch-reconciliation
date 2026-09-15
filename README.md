# Hyperswitch Recon

Hyperswitch Recon is a Rust workspace that powers reconciliation workflows for merchants and business profiles. It includes APIs for ingestion, transformation, ledger entries, reconciliation rules/engine, and exception handling, along with migrations, test tooling, and CI automation.

## What is in this repository

- Multi-crate Rust workspace (`crates/*`) with `recon` as the main API service.
- PostgreSQL schema and migrations under `migrations/`.
- Environment-specific config files under `config/`.
- Local orchestration via `docker-compose.yml`.
- GitHub Actions workflows for quality checks and tests under `.github/workflows/`.
- Domain and architecture notes for contributors under `ai-docs/`.

## Core API domains

The service exposes endpoints across these areas:

- Auth and bootstrap
- Merchant accounts and business profiles
- Accounts, entries, and transactions
- Ingestion configs/history and ingestion triggers
- Transformation configs/history and execution triggers
- Staging entries and bulk operations
- Reconciliation rules and reconciliation engine
- Exception management and audit trail
- Metadata schemas and search groups
- API docs and health endpoints

## Prerequisites

- Rust toolchain (stable) and Cargo
- PostgreSQL 15+
- `just` command runner

Example install for `just`:

```bash
cargo install just
```

## Quick start

1) Start PostgreSQL

```bash
docker compose up -d postgres
```

2) Create database and run migrations

```bash
just resurrect recon
```

3) Run the API service

```bash
cargo run -p recon --bin recon
```

4) Verify service is up

- Health: `http://localhost:8080/health`
- OpenAPI JSON: `http://localhost:8080/api_docs/openapi.json`
- Swagger UI: `http://localhost:8080/api_docs/ui/`

## Configuration

Configuration is loaded with this precedence (low to high):

1. Built-in defaults
2. Config file
3. Environment variables prefixed with `RECON__`

Defaults:

- `config/config.toml` is used by default.
- If `RUN_ENV=production`, `config/prod.toml` is used.

Override config file explicitly:

```bash
cargo run -p recon --bin recon -- --config-path config/ci.toml
```

Override any value via environment variable (example):

```bash
RECON__SERVER__PORT=9090 cargo run -p recon --bin recon
```

## Common developer commands

```bash
# Run all tests (recreates recon_test and runs nextest)
just test

# Rust quality checks
just fmt
just clippy
just check

# Coverage (llvm-cov + nextest)
just coverage --html

# Database helpers
just resurrect recon
just migrate recon
```

## CI workflows

Current CI is split by concern:

- `/.github/workflows/ci-quality.yml`
  - Rust format check
  - Clippy linting
- `/.github/workflows/ci-tests.yml`
  - PostgreSQL service setup
  - Test dependencies install
  - `just test`

## Repository layout

```text
.
├── Cargo.toml
├── justfile
├── config/
│   ├── config.toml
│   ├── ci.toml
│   ├── prod.toml
│   └── test.toml
├── crates/
│   ├── recon/
│   ├── api_models/
│   ├── common_enums/
│   ├── common_types/
│   ├── common_utils/
│   ├── diesel_models/
│   ├── external_services/
│   ├── recon_domain_models/
│   ├── recon_interfaces/
│   └── recon_derive/
├── migrations/
├── scripts/
├── ai-docs/
└── .github/
    └── workflows/
        ├── ci-quality.yml
        └── ci-tests.yml
```

## Contributor notes

- Start with `ai-docs/INDEX.md`.
- Read `ai-docs/RULES.md` and `ai-docs/CRITICAL-PATHS.md` before making changes.
- For domain-specific work, read all docs in `ai-docs/domains/<domain>/`.
- Keep changes aligned with existing patterns in the workspace.
