#!/bin/sh
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$repo_root/scripts/gate-stamp-lib.sh"
python_runner="$repo_root/scripts/python"
helper="$repo_root/scripts/lifecycle_soak.py"
manifest="$repo_root/scripts/fixtures/lifecycle-soak/manifest-26.1.json"
target_dir=/tmp/bbb-target-soak
client_bin="$target_dir/debug/bbb-native"

usage() {
  cat <<'EOF'
Usage: scripts/lifecycle-soak-gate.sh

Environment:
  BBB_PLAYABLE_ACCEPT_EULA=1    Required EULA acknowledgement
  BBB_PLAYABLE_JAVA             Explicit Java executable
  BBB_VANILLA_ROOT              Optional managed Vanilla cache root
  BBB_LIFECYCLE_SOAK_ARTIFACT_DIR  Result directory; default is fresh /tmp

The caller must also provide DISPLAY or WAYLAND_DISPLAY. The gate never
downloads Java, assets, credentials, or a server jar.
EOF
}

case "${1:-}" in
  "")
    ;;
  -h|--help)
    usage
    exit 0
    ;;
  *)
    usage >&2
    exit 2
    ;;
esac

if ! vanilla_root=$("$repo_root/scripts/vanilla" path 26.1 --kind root); then
  echo "lifecycle soak failed: prepare the managed Vanilla 26.1 workspace" >&2
  exit 2
fi
export BBB_VANILLA_ROOT="$vanilla_root"
if [ -n "$(git -C "$repo_root" status --porcelain)" ]; then
  echo "lifecycle soak failed: worktree must be clean" >&2
  exit 2
fi
start_head=$(git -C "$repo_root" rev-parse HEAD)
artifact_dir=${BBB_LIFECYCLE_SOAK_ARTIFACT_DIR:-}
if [ -z "$artifact_dir" ]; then
  artifact_dir=$(mktemp -d "${TMPDIR:-/tmp}/bbb-lifecycle-soak.XXXXXX")
else
  mkdir -p "$artifact_dir"
  artifact_dir=$(CDPATH= cd -- "$artifact_dir" && pwd)
fi

echo "lifecycle soak: building current HEAD with validation-only fault receipts"
CARGO_TARGET_DIR="$target_dir" \
  "$repo_root/scripts/cargo-dev.sh" cargo build \
  -p bbb-native --features lifecycle-soak

"$python_runner" python "$helper" run \
  --manifest "$manifest" \
  --repo-root "$repo_root" \
  --artifact-dir "$artifact_dir" \
  --client-bin "$client_bin" \
  --playable-gate "$repo_root/scripts/playable-gate.sh"

if [ "$(git -C "$repo_root" rev-parse HEAD)" != "$start_head" ] ||
  [ -n "$(git -C "$repo_root" status --porcelain)" ]
then
  echo "lifecycle soak failed: repository changed during the gate" >&2
  exit 1
fi
report="$artifact_dir/report.json"
git_dir=$(git -C "$repo_root" rev-parse --absolute-git-dir)
mkdir -p "$git_dir/bbb"
cp "$report" "$git_dir/bbb/last-lifecycle-soak-report.json"
(cd "$repo_root" && record_gate_stamp \
  lifecycle-soak-log.tsv last-lifecycle-soak passed "report:$report")
echo "lifecycle soak: report: $report"
