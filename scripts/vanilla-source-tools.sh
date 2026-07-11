#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
project="$repo_root/scripts/vanilla-source-tools"

if [[ $# -eq 0 ]]; then
  echo "usage: scripts/vanilla-source-tools.sh <python-command-or-script> [args...]" >&2
  exit 2
fi

exec uv run --offline --frozen --project "$project" --python 3.12 -- "$@"
