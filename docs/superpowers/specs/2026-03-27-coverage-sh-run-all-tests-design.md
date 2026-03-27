# Design: coverage.sh runs all tests

## Goal
Update `scripts/coverage.sh` to run all **library tests** under coverage (`cargo llvm-cov --lib`) while keeping the existing filename filter and coverage threshold. This avoids the current behavior where the test filter prevents tests from running and yields 0% coverage.

## Non-Goals
- Changing which files are counted for coverage (the existing `--ignore-filename-regex` stays as-is).
- Changing the coverage threshold (`--fail-under-lines 90`) or test threading options.
- Adding or modifying any tests.

## Current Behavior and Root Cause
`coverage.sh` passes a single test-name filter with `|` characters. Cargo’s test filter is a literal substring match (not regex), so the filter matches no tests, and the run reports `running 0 tests` with 0% coverage.

## Proposed Change
Remove the test-name filter from the `cargo llvm-cov` invocation so all tests execute. Retain:
- `--lib`
- `--ignore-filename-regex '^(?!.*(handlers/(summary|reminder)\.rs|ai/(mod|prompt)\.rs)).*$'`
- `--fail-under-lines 90`
- `-- --test-threads=1`

**Before (current command):**
```bash
cargo llvm-cov --lib \
  --ignore-filename-regex '^(?!.*(handlers/(summary|reminder)\.rs|ai/(mod|prompt)\.rs)).*$' \
  --fail-under-lines 90 \
  -- --test-threads=1 'handlers::summary::tests|handlers::reminder::tests|ai::tests|ai::prompt::tests'
```

**After (proposed command):**
```bash
cargo llvm-cov --lib \
  --ignore-filename-regex '^(?!.*(handlers/(summary|reminder)\.rs|ai/(mod|prompt)\.rs)).*$' \
  --fail-under-lines 90 \
  -- --test-threads=1
```

## Package / Workspace Scope
This repo appears to be a single crate (`line_bot_summarize`). The change keeps `cargo llvm-cov --lib` without a `-p` flag; expected scope is the default package only. If this becomes a workspace with multiple packages, consider adding `-p line_bot_summarize` to lock scope.

## Data Flow / Execution Flow
1. `scripts/coverage.sh` runs `cargo llvm-cov --lib` with coverage settings.
2. Cargo executes all library tests (no name filter).
3. Coverage is reported for the allowed files per the regex.

## Error Handling
No changes. The script keeps `set -euo pipefail` and `--fail-under-lines 90` to fail on errors or low coverage.

## Risks
- Running all tests may increase runtime compared to filtered runs.
- Previously skipped tests may surface flakes; if so, address flakiness separately rather than re-adding a broad filter.

## Testing / Verification
- Run `scripts/coverage.sh`.
- Expect `running N tests` where `N` matches `cargo test --lib` (or the crate’s known test count).
- Verify coverage percent is reported for targeted files (`handlers/summary.rs`, `handlers/reminder.rs`, `ai/mod.rs`, `ai/prompt.rs`).

## Rollout
Single-file change to `scripts/coverage.sh`.
