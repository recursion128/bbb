#!/bin/sh
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$repo_root/scripts/gate-stamp-lib.sh"
python_runner="$repo_root/scripts/python"
helper="$repo_root/scripts/capture_official_world.py"

receipt_path=${BBB_OFFICIAL_CAPTURE_RECEIPT:-}
if [ -z "$receipt_path" ]; then
  receipt_path="$repo_root/scripts/data/server-official-capture-26.1.json"
fi

usage() {
  cat <<'EOF'
Usage: scripts/capture-official-world-26.1.sh [verify <world-dir>]

Modes:
  (default)        Boot the official Mojang 26.1 dedicated server locally,
                   join the probe client, save-all, stop, then write the
                   capture manifest and identity receipt.
  verify <dir>     Recompute the manifest of <dir> and compare it against the
                   committed receipt. No EULA, no server boot.

Environment:
  BBB_OFFICIAL_CAPTURE_ACCEPT_EULA=1  Required for capture before writing
                                      eula.txt. Never inferred.
  BBB_SERVER_PLAYABLE_CLIENT_BIN      Optional bbb-client executable.
  BBB_OFFICIAL_CAPTURE_ARTIFACT_DIR   Capture directory; default is fresh /tmp.
  BBB_OFFICIAL_CAPTURE_RECEIPT        Receipt path; default is
                                      scripts/data/server-official-capture-26.1.json.

The capture never downloads Java or the official jar and never infers EULA
acceptance. Prepare repo-managed Vanilla explicitly with:
  scripts/vanilla prepare 26.1
EOF
}

mode=capture
case "${1:-}" in
  "")
    ;;
  verify)
    mode=verify
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

if [ "$mode" = verify ]; then
  if [ "$#" -ne 2 ]; then
    usage >&2
    exit 2
  fi
  "$python_runner" python "$helper" verify --world-dir "$2" --receipt "$receipt_path"
  exit 0
fi

if [ "$#" -ne 0 ]; then
  usage >&2
  exit 2
fi
if [ "${BBB_OFFICIAL_CAPTURE_ACCEPT_EULA:-0}" != 1 ]; then
  echo "official capture: requires BBB_OFFICIAL_CAPTURE_ACCEPT_EULA=1" >&2
  exit 2
fi

java_bin=$("$repo_root/scripts/vanilla" path 26.1 --kind java) || {
  echo "official capture: managed Java is unavailable; run scripts/vanilla prepare 26.1" >&2
  exit 2
}
server_jar=$("$repo_root/scripts/vanilla" path 26.1 --kind server-jar) || {
  echo "official capture: managed server jar is unavailable; run scripts/vanilla prepare 26.1" >&2
  exit 2
}
vanilla_root=$("$repo_root/scripts/vanilla" path 26.1 --kind root)
if [ ! -x "$java_bin" ] || [ ! -f "$server_jar" ]; then
  echo "official capture: managed Java/server jar is unavailable; run scripts/vanilla prepare 26.1" >&2
  exit 2
fi

client_bin=${BBB_SERVER_PLAYABLE_CLIENT_BIN:-}
for candidate in "${CARGO_TARGET_DIR:-}/debug/bbb-client" /tmp/bbb-target-main/debug/bbb-client; do
  if [ -z "$client_bin" ] && [ -x "$candidate" ]; then
    client_bin=$candidate
  fi
done
if [ -z "$client_bin" ] || [ ! -x "$client_bin" ]; then
  echo "official capture: bbb-client is unavailable" >&2
  exit 2
fi

require_fresh_bin "$repo_root" "$client_bin" "official capture" || exit 2

artifact_dir=${BBB_OFFICIAL_CAPTURE_ARTIFACT_DIR:-}
if [ -z "$artifact_dir" ]; then
  artifact_dir=$(mktemp -d "${TMPDIR:-/tmp}/bbb-official-capture.XXXXXX")
else
  mkdir -p "$artifact_dir"
fi

available_port() {
  "$python_runner" python -c \
    'import socket; s=socket.socket(); s.bind(("127.0.0.1", 0)); print(s.getsockname()[1]); s.close()'
}
port=$(available_port)

server_dir="$artifact_dir/server"
server_log="$artifact_dir/official-server.log"
probe_json="$artifact_dir/capture-probe.json"
stdin_fifo="$artifact_dir/server.stdin"
mkdir -p "$server_dir"
mkfifo "$stdin_fifo"

server_pid=
cleanup() {
  trap - 0 HUP INT TERM
  if [ -n "$server_pid" ] && kill -0 "$server_pid" 2>/dev/null; then
    kill -TERM "$server_pid" 2>/dev/null || true
    wait "$server_pid" 2>/dev/null || true
  fi
  rm -f "$stdin_fifo"
}
on_signal() {
  cleanup
  exit 130
}
trap cleanup 0
trap on_signal HUP INT TERM

wait_marker() {
  marker=$1
  log=$2
  pid=$3
  remaining=120
  while ! grep -F "$marker" "$log" >/dev/null 2>&1; do
    if ! kill -0 "$pid" 2>/dev/null; then
      echo "official capture: server exited before '$marker'; log: $log" >&2
      exit 1
    fi
    if [ "$remaining" -eq 0 ]; then
      echo "official capture: timed out waiting for '$marker'; log: $log" >&2
      exit 1
    fi
    remaining=$((remaining - 1))
    sleep 1
  done
}

# The settings the replay needs are shared with every other script that boots
# an official server for it; only the world this one creates is its own.
cat "$repo_root/scripts/data/official-replay-server.properties" >"$server_dir/server.properties"
printf '%s\n' \
  "server-port=$port" \
  'level-name=world' \
  'level-seed=bbb-official-capture-26.1' \
  'level-type=minecraft:flat' \
  'generate-structures=false' \
  'motd=bbb official world capture 26.1' \
  >>"$server_dir/server.properties"
printf 'eula=true\n' >"$server_dir/eula.txt"

echo "official capture: starting official 26.1 on 127.0.0.1:$port"
(
  cd "$server_dir"
  exec "$java_bin" -Xms512M -Xmx1G -jar "$server_jar" nogui <"$stdin_fifo"
) >"$server_log" 2>&1 &
server_pid=$!
# Keep the write end open so the server never sees EOF on its console input.
exec 3>"$stdin_fifo"
wait_marker 'Done (' "$server_log" "$server_pid"

# One replay session, which now also drives the client's refusal sessions one
# after another. The budget is the same on every script that runs this probe;
# scripts/tests/test_capture_replay_expectations.py keeps them in step.
probe_timeout_seconds=${BBB_SERVER_PROBE_TIMEOUT_SECONDS:-90}

mkdir -p "$artifact_dir/client"
"$repo_root/scripts/process" run --timeout "$probe_timeout_seconds" -- \
  "$client_bin" \
    --game-dir "$artifact_dir/client" \
    --vanilla-root "$vanilla_root" \
    --server "127.0.0.1:$port" \
    --username capture-probe \
    --probe-server \
    --probe-server-interactions \
    >"$probe_json"
"$python_runner" python "$repo_root/scripts/server_playable_gate.py" assert --input "$probe_json"

printf 'save-all\n' >&3
wait_marker 'Saved the game' "$server_log" "$server_pid"
printf 'stop\n' >&3
exec 3>&-
wait "$server_pid" 2>/dev/null || true
server_pid=

"$python_runner" python "$helper" capture \
  --world-dir "$server_dir/world" \
  --java-bin "$java_bin" \
  --server-jar "$server_jar" \
  --seed bbb-official-capture-26.1 \
  --output "$receipt_path"

world_identity=$("$python_runner" python -c \
  'import json, sys; print(json.load(open(sys.argv[1], encoding="utf-8"))["world_identity"])' \
  "$receipt_path")
echo "official capture: world at $server_dir/world; receipt at $receipt_path; world_identity $world_identity"
