# AI Documentation Index

## How AI Agents Should Use This Documentation

### ALWAYS READ FIRST
1. `RULES.md` - Security and code standards that always apply
2. `CRITICAL-PATHS.md` - Cross-domain gotchas and critical user flows

### WHEN WORKING ON SPECIFIC FEATURES
Look in `domains/[feature]/` and read ALL files present:
- `AGENT-NOTES.md` - Primary gotchas and decisions
- Additional files if present (split when >300 lines)

### CURRENT DOMAINS
- `merchants/` - Merchant accounts and business profiles management
- `accounts/` - Account ledger system with balance tracking
- `ingestion/` - External file retrieval from PSPs, webhook processing, and file storage
- `transformation/` - File data transformation to staging entries, configuration and history management
- `reconciliation/` - Complete reconciliation system with strategy-based rule engine
  - `INDEX.md` - **START HERE** - Overview and navigation guide
  - `ARCHITECTURE.md` - System architecture and component design
  - `RECON-FLOW.md` - Detailed reconciliation lifecycle and processing flows
  - `STRATEGIES.md` - Strategy implementations (OneToOne variants)
  - `AGENT-NOTES.md` - Critical rules, gotchas, and operational procedures
- `recon-rules/` - Rule configuration and execution engine
  - `SCHEMA.md` - Complete recon_rule schema reference
  - `AGENT-NOTES.md` - Implementation details and critical gotchas
<!-- AI-UPDATED: 2026-10-23 - Link config updates, inline processing, and linked-entry lookup navigation -->
- `linking/` - Entry linking across transaction/confirmation flows on one account
  - [AGENT-NOTES.md](domains/linking/AGENT-NOTES.md) - Final/provisional links, config API/update constraints, matching keys, admin inline processing, linked-entry lookup, enqueue/retry limits, and migration/worker contracts
- `external-services/` - Secret management, AWS S3/OCI file storage, FaaS invocation, and external integrations
- `file-storage/` - File upload/download API validation, organization, and storage integration gotchas
- `encryption/` - Application encryption system with LocalEncryption and KeymanagerService strategies
- `audit-trail/` - Chronological activity tracking with event grouping and profile-scoped views
- `transactions/` - Transaction lifecycle, versioning, entries, and profile overview aggregation
  - `OVERVIEW-API.md` - Non-obvious aggregation, status, and time-range semantics
  - `MANUAL-RECONCILIATION.md` - Exception-management reconcile endpoints (v1 and the v2 delta API)
- `reporting/` - Triggers per-rule transaction/exception reports via the generic FaaS invocation client
  - `AGENT-NOTES.md` - Why validation lives in core, the Lambda `event` contract, and gotchas

### WHEN IMPLEMENTING FEATURES
Check `patterns/INDEX.md` for implementation patterns.
Start there when you need to know HOW to implement something.

### IMPORTANT
When working on any domain, you MUST read all .md files in that domain folder.
These files contain critical gotchas that will save hours of debugging.

### Purpose of This Documentation
This documentation focuses on:
- WHY decisions were made (not just what the code does)
- Non-obvious gotchas that waste developer time
- Business rules and constraints not enforced by types
- Critical relationships between system components

### File Size Rules
- Each file is kept under 300 lines for efficient context usage
- When files exceed this, they're split by topic
- Always read ALL files in a domain, not just AGENT-NOTES.md
