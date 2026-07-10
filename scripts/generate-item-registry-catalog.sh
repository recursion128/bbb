#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
export CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-/tmp/bbb-target-main}

exec cargo run --quiet --manifest-path "$repo_root/Cargo.toml" \
  -p bbb-pack --example generate_item_registry_catalog -- "$@"
