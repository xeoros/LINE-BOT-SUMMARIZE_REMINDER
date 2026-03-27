# Design: Reach >90% coverage for core LINE + DB + AI + Scheduler scope

## Goal
Achieve >90% **aggregate line coverage** (reported by `cargo llvm-cov`) for a defined scope by expanding coverage inclusion and adding high-value tests. Scope includes:
- `handlers/summary.rs`, `handlers/reminder.rs`
- `admin/mod.rs`
- `ai/*`
- `line/*`
- `db/*`
- `config.rs`
- `scheduler/*`

Excluded for now:
- `slack/*`
- `teams/*`

## Non-Goals
- Achieve >90% across the entire repository.
- Add Slack or Microsoft Teams coverage in this phase.
- Change runtime behavior in production code unless needed for testability.

## Coverage Scope Change
Update `scripts/coverage.sh` ignore/include regex so coverage is calculated for the expanded scope listed above, while excluding Slack and Teams modules.

**Coverage definition:**\n
- Tool: `cargo llvm-cov` invoked by `scripts/coverage.sh`.\n
- Metric: line coverage **aggregate total** for included paths (TOTAL row in report).\n
- Threshold: `--fail-under-lines 90` remains and enforces the >90% requirement.

**Target include pattern (exact):**\n
Use this `--ignore-filename-regex` (negative lookahead) so only the scoped **`src/`** files are included (supports nested submodules):\n
```\n
^(?!.*src/(handlers/(summary|reminder)\\.rs|admin/mod\\.rs|ai/.*\\.rs|line/.*\\.rs|db/.*\\.rs|scheduler/.*\\.rs|config\\.rs)).*$\n
```\n
This matches paths containing the listed files and ignores everything else.\n
\n
**Examples**\n
Included:\n
- `src/handlers/summary.rs`\n
- `src/admin/mod.rs`\n
- `src/ai/mod.rs`\n
- `src/db/models.rs`\n
- `src/line/webhook.rs`\n
- `src/scheduler/mod.rs`\n
- `src/config.rs`\n
\n
Excluded:\n
- `src/slack/thread_identification.rs`\n
- `src/teams/command.rs`\n
- `src/scheduler/summaries.rs.bak`\n
- `tests/integration/line_webhook.rs`\n
\n
Include only these `src/` paths:\n
- `handlers/summary.rs`\n
- `handlers/reminder.rs`\n
- `admin/mod.rs`\n
- `ai/*.rs`\n
- `line/*.rs`\n
- `db/*.rs`\n
- `config.rs`\n
- `scheduler/*.rs`\n

This will be expressed via `--ignore-filename-regex` as a negative lookahead that excludes everything outside these paths.

## Test Strategy
- **Unit tests** for pure logic (parsers, formatters, data transformations).
- **Mocked HTTP** for LINE and AI provider clients to avoid external calls.
- **DB tests** using a local Postgres test database via `sqlx::test` (requires `DATABASE_URL`).\n
  - Migrations must be applied in tests via `sqlx::migrate!(\"./sql/migrations\")`.\n
  - Coverage runs will fail if `DATABASE_URL` is not set; this is required for DB coverage.\n
  - Expected local setup: a running Postgres instance and `DATABASE_URL` pointing to a test database (e.g. `postgres://user:pass@localhost:5432/line_bot_test`).\n
  - CI (if/when enabled) must provide a Postgres service and set `DATABASE_URL` similarly.
- **Scheduler tests** for schedule parsing/creation logic without real timers.

## Component Coverage Plan
- `handlers/summary.rs`:
  - Command parsing (count, time range, invalid inputs).
  - Time range handling and defaults.
- `handlers/reminder.rs`:
  - Checklist parsing, done/notify logic, scheduling parsing.
  - Edge cases (empty lists, malformed commands).
- `admin/mod.rs`:
  - Dashboard formatting, filters, date formatting.
- `ai/*`:
  - Provider selection logic, prompt assembly, error handling paths.
- `line/*`:
  - Webhook signature verification, request parsing, reply formatting.
- `db/*`:
  - Model serialization, helper formatting, error paths.
- `config.rs`:
  - Env parsing, defaults, invalid env cases.
- `scheduler/*`:
  - Schedule creation and parsing logic outcomes.

## Mocking & Test Tooling
- HTTP mocking: add a dev-dependency (e.g. `httpmock`) and use it to return deterministic responses for LINE + AI provider calls, including error cases.\n
- DB testing: use `sqlx::test` with migrations and per-test transactions; seed minimal rows for query tests.\n
- Fixtures: store JSON payloads inline in tests or in `tests/fixtures/` if reuse is needed.

**External-side-effect hooks:**\n
- LINE webhook tests should set required env (e.g. channel secret/token) to dummy values and validate signature generation with known fixtures.\n
- AI provider tests should set provider keys to dummy values and rely on HTTP mocks to avoid real network calls.\n

## Determinism (Time Handling)
For any tests that rely on “now”, introduce a testable time source:\n
- Prefer adding helper functions that accept a `DateTime<Utc>` parameter (or injecting a clock) so tests can use fixed timestamps.\n
- Avoid direct calls to `Utc::now()` inside logic under test unless the function already accepts a time input.

## Data Flow / Execution
1. Expand coverage scope in `scripts/coverage.sh`.
2. Add tests by module, prioritizing pure logic first.
3. Use mocks/doubles for HTTP and DB where possible.
4. Run `scripts/coverage.sh` and iterate until >90% for scope.

## Error Handling
Add tests for invalid inputs and error branches in all scoped modules to prevent untested error paths from dragging coverage down.

## Verification
- `scripts/coverage.sh` runs all library tests.
- Coverage for the defined scope is >= 90%.
- Tests are deterministic and do not require external network access.

## CI / Enforcement
- Coverage threshold is enforced by `--fail-under-lines 90` in `scripts/coverage.sh`.\n
- If coverage falls below 90%, the script exits non-zero to block regressions.

## Risks
- Expanded scope may include legacy code that is hard to cover without refactoring.
- Mocking external calls might require small testability hooks.
- Achieving >90% could require many tests; prioritize high-impact branches first.

## Rollout
- Single PR/set of changes: coverage scope update + test additions.
- Re-run coverage and confirm threshold before considering Slack/Teams.
