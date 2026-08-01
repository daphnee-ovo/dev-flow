#!/usr/bin/env bash
set -euo pipefail

project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cargo test --manifest-path "$project_root/dow/Cargo.toml" dashboard:: -- --nocapture
