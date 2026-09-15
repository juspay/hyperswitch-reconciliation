# Universal Rules for AI Agents

## Security (ALWAYS APPLY)
- Never log sensitive data (passwords, tokens, PII, API keys)
- All user input must be validated before use
- Use parameterized queries, never string concatenation for SQL
- Never store monetary values as floating point - use MinorUnit (i64)
- All IDs must be generated server-side using nanoid format

## Code Standards
- Use existing patterns found in codebase
- Always run tests before marking task complete
- ALWAYS use `just test` instead of `cargo test` for running tests
- Follow Rust conventions: snake_case for functions, PascalCase for types
- Use error-stack for error handling with proper context
- Routes → Core → DB architecture must be followed
<!-- AI-UPDATED: 2025-07-18 - Clippy format string standard -->
- Use inline format strings: `format!("value_{variable}")` not `format!("value_{}", variable)`
<!-- AI-UPDATED: 2026-08-26 - PR title / commit message convention wired into CI -->
- PR titles and merge-queue commit messages must be `type(scope): Message` — type lowercase (cocogitto default set: feat/fix/chore/refactor/docs/style/test/perf/build/ci/revert), scope mandatory, message starts with a capital letter
- Enforced by its own workflow, `.github/workflows/pr-title-check.yml` (kept separate from `ci-quality.yml` so a title-only edit doesn't retrigger fmt/clippy/typos), NOT `cog.toml` — cocogitto's own config has no option for mandatory scope or message casing, so those two rules are a hand-written regex layered on top of `cog verify`
- **Gotcha**: `cog verify` panics (`user.name was not found`) on a fresh CI runner because it unconditionally reads git identity config even though it never commits anything — `pr-title-check.yml` must run `git config user.name`/`user.email` before calling `cog verify`

## API Conventions
<!-- AI-UPDATED: 2025-06-24 - Route naming convention -->
- All API routes MUST use snake_case (e.g., `/merchant_accounts`, `/recon_engine`, NOT `/recon-engine`)
- Path parameters use snake_case (e.g., `/{profile_id}`, `/{staging_entry_id}`)
- Query parameters use snake_case
- JSON field names use snake_case

## Monetary Values
- ALWAYS use MinorUnit (i64) for amounts in database
- ALWAYS use AmountType for API/business logic layer
- Currency must always be specified alongside amounts
- Initial balance = pending balance = posted balance on account creation

## Entity Relationships
- Every business profile MUST belong to a merchant
- Every account MUST belong to a business profile
- Merchant → Business Profile → Account hierarchy is strict
- Profile IDs in auth context must match resource profile IDs

## Status Management
- Soft deletes only - use discarded_at timestamp
- When discarding, always capture current status in discarded_status
- Status transitions may be one-way (check business rules)
- Never delete records, only mark as discarded

## Import Style (ALWAYS FOLLOW)
- Follow ai-docs/patterns/IMPORT-STYLE.md for all Rust code
- Import enums at top, use short names in code
- Never use fully qualified paths in match statements
- Group imports: std → external → internal → local
- Separate import groups with blank lines

## When You're Unsure
- Check domain-specific AGENT-NOTES.md first
- Look at existing similar code in the codebase
- Verify with integration tests, not just unit tests
- Ask for clarification rather than guess business rules
