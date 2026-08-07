#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
helper="$repo_root/scripts/official_client_benchmark.py"
python_runner="$repo_root/scripts/python"

if [[ $# -eq 0 ]]; then
  echo "usage: scripts/official-client-benchmark.sh <audit|prepare-vanilla|write-default-replay|replay-hash|generate-datapack|prepare-world|run-official-smoke|run-bbb-smoke|summarize-mangohud|capture-preflight|summarize-blocked|summarize-workload|summarize-renderdoc|summarize-formal> [args...]" >&2
  exit 2
fi

exec "$python_runner" python "$helper" "$@"
