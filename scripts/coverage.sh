#!/usr/bin/env bash
set -euo pipefail

cargo llvm-cov --lib \
  --ignore-filename-regex 'src/(slack|teams)/|src/handlers/mod\.rs|src/main\.rs' \
  --fail-under-lines 90 \
  -- --test-threads=1
