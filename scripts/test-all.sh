#!/usr/bin/env bash
set -euo pipefail

# Simple unified test runner for Offline Intelligence Library
# Runs backend (Rust) tests

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo "==> Backend: cargo test --lib"
(
  cd "$ROOT_DIR/crates/offline-intelligence"
  cargo test --lib
)

echo "==> All tests passed"