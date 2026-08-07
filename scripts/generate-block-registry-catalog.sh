#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
export CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-/tmp/bbb-target-main}

"$repo_root/scripts/python" \
  "$repo_root/scripts/generate-block-registry-catalog.py" "$@"

cargo test --quiet --manifest-path "$repo_root/Cargo.toml" \
  -p bbb-pack block_destroy_profiles::tests
