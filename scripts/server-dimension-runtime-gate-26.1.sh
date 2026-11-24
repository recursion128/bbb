#!/bin/sh
set -eu

if [ "$#" -ne 1 ] || [ "$1" != live ]; then
  echo "usage: scripts/server-dimension-runtime-gate-26.1.sh live" >&2
  exit 2
fi
if [ "${BBB_OFFICIAL_CAPTURE_ACCEPT_EULA:-0}" != 1 ]; then
  echo "dimension runtime gate: requires BBB_OFFICIAL_CAPTURE_ACCEPT_EULA=1" >&2
  exit 2
fi

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$repo_root/scripts/gate-stamp-lib.sh"

vanilla_root=$("$repo_root/scripts/vanilla" path 26.1 --kind root)
java_bin=${BBB_SERVER_DIMENSION_RUNTIME_JAVA:-$("$repo_root/scripts/vanilla" path 26.1 --kind java)}
server_jar=$("$repo_root/scripts/vanilla" path 26.1 --kind server-jar)
if [ ! -d "$vanilla_root" ] || [ ! -x "$java_bin" ] || [ ! -f "$server_jar" ]; then
  echo "dimension runtime gate: managed Vanilla 26.1 workspace is unavailable" >&2
  exit 2
fi

client_bin=${BBB_SERVER_DIMENSION_RUNTIME_CLIENT_BIN:-}
server_bin=${BBB_SERVER_DIMENSION_RUNTIME_SERVER_BIN:-}
admin_bin=${BBB_SERVER_DIMENSION_RUNTIME_ADMIN_BIN:-}
for candidate in "${CARGO_TARGET_DIR:-}/debug/bbb-client" /tmp/bbb-target-main/debug/bbb-client; do
  if [ -z "$client_bin" ] && [ -x "$candidate" ]; then client_bin=$candidate; fi
done
for candidate in "${CARGO_TARGET_DIR:-}/debug/bbb-server" /tmp/bbb-target-main/debug/bbb-server; do
  if [ -z "$server_bin" ] && [ -x "$candidate" ]; then server_bin=$candidate; fi
done
for candidate in "${CARGO_TARGET_DIR:-}/debug/bbb-admin" /tmp/bbb-target-main/debug/bbb-admin; do
  if [ -z "$admin_bin" ] && [ -x "$candidate" ]; then admin_bin=$candidate; fi
done
for pair in "bbb-client:$client_bin" "bbb-server:$server_bin" "bbb-admin:$admin_bin"; do
  name=${pair%%:*}
  path=${pair#*:}
  if [ -z "$path" ] || [ ! -x "$path" ]; then
    echo "dimension runtime gate: $name is unavailable" >&2
    exit 2
  fi
  require_fresh_bin "$repo_root" "$path" "dimension runtime gate" || exit 2
done

artifact_dir=${BBB_SERVER_DIMENSION_RUNTIME_ARTIFACT_DIR:-}
if [ -z "$artifact_dir" ]; then
  artifact_dir=$(mktemp -d "${TMPDIR:-/tmp}/bbb-dimension-runtime.XXXXXX")
fi
timeout_seconds=${BBB_SERVER_DIMENSION_RUNTIME_TIMEOUT_SECONDS:-90}

exec "$repo_root/scripts/python" python "$repo_root/scripts/server_dimension_runtime_gate.py" \
  live \
  --artifact-dir "$artifact_dir" \
  --client-bin "$client_bin" \
  --server-bin "$server_bin" \
  --admin-bin "$admin_bin" \
  --java-bin "$java_bin" \
  --server-jar "$server_jar" \
  --shared-properties "$repo_root/scripts/data/official-replay-server.properties" \
  --timeout "$timeout_seconds" \
  --gamemode creative --force-gamemode \
  --difficulty peaceful
