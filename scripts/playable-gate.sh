#!/bin/sh
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
python_runner="$repo_root/scripts/vanilla-source-tools.sh"
helper="$repo_root/scripts/playable_gate.py"
fixture="$repo_root/scripts/fixtures/playable-gate/session-26.1.json"

usage() {
  cat <<'EOF'
Usage: scripts/playable-gate.sh [auto|fixture|live]

Modes:
  auto       Run live when every prerequisite is explicit and available;
             otherwise run the checked-in recorded fixture and report why.
  fixture    Run only the checked-in recorded fixture.
  live       Require and run the vanilla 26.1 offline server + bbb-native path.

Live environment:
  BBB_PLAYABLE_SERVER_JAR       Explicit vanilla 26.1 dedicated server jar path
  BBB_PLAYABLE_ACCEPT_EULA=1    Required acknowledgement before writing eula.txt
  BBB_PLAYABLE_JAVA             Java executable, defaults to java from PATH
  BBB_PLAYABLE_CLIENT_BIN       bbb-native executable; known external targets are probed
  BBB_PLAYABLE_PORT             Dedicated server port; default is an available loopback port
  BBB_PLAYABLE_CONTROL_PORT     bbb-control port; default is an available loopback port
  BBB_PLAYABLE_ARTIFACT_DIR     Evidence directory; default is a fresh /tmp directory
  BBB_PLAYABLE_TIMEOUT_SECONDS  Client/canonical-state timeout; default 120
  BBB_PLAYABLE_SERVER_READY_TIMEOUT_SECONDS  Server-ready timeout; default 120

The live path uses the bbb-native --server/--connect-server/--control CLI and
the narrow `bbb-native --status-server <addr>` JSON CLI (which must delegate to
`bbb_net::status_ping`), then queries bbb-control for canonical dimension,
chunk, and local-player state.
It never downloads Java or a server jar.  This is a milestone/slow gate and is
not part of the pre-commit gate in goal.md.
EOF
}

mode=${1:-auto}
if [ "$#" -gt 1 ]; then
  usage >&2
  exit 2
fi
case "$mode" in
  auto|fixture|live) ;;
  -h|--help)
    usage
    exit 0
    ;;
  *)
    usage >&2
    exit 2
    ;;
esac

run_python() {
  "$python_runner" python "$helper" "$@"
}

run_fixture() {
  echo "playable gate: running checked-in recorded fixture smoke"
  run_python assert \
    --evidence "$fixture" \
    --protocol-source "$repo_root/crates/bbb-protocol/src/lib.rs"
}

if [ "$mode" = fixture ]; then
  run_fixture
  exit 0
fi

live_reason=
java_bin=${BBB_PLAYABLE_JAVA:-}
if [ -z "$java_bin" ]; then
  java_bin=$(command -v java 2>/dev/null || true)
fi
if [ -z "$java_bin" ] || [ ! -x "$java_bin" ]; then
  live_reason="Java executable unavailable (set BBB_PLAYABLE_JAVA)"
fi

server_jar=${BBB_PLAYABLE_SERVER_JAR:-}
if [ -z "$live_reason" ] && { [ -z "$server_jar" ] || [ ! -f "$server_jar" ]; }; then
  live_reason="vanilla server jar unavailable (set BBB_PLAYABLE_SERVER_JAR)"
fi
if [ -z "$live_reason" ] && [ "${BBB_PLAYABLE_ACCEPT_EULA:-0}" != 1 ]; then
  live_reason="BBB_PLAYABLE_ACCEPT_EULA=1 was not set"
fi
if [ -z "$live_reason" ] && ! command -v setsid >/dev/null 2>&1; then
  live_reason="setsid is unavailable for leak-free process-group cleanup"
fi
if [ -z "$live_reason" ] && ! command -v timeout >/dev/null 2>&1; then
  live_reason="timeout is unavailable for the delegated status probe"
fi

client_bin=${BBB_PLAYABLE_CLIENT_BIN:-}
if [ -z "$client_bin" ]; then
  for candidate in \
    "${CARGO_TARGET_DIR:-}/debug/bbb-native" \
    /tmp/bbb-target-main/debug/bbb-native \
    /tmp/bbb-target-playable/debug/bbb-native \
    "$repo_root/target/debug/bbb-native"
  do
    if [ -n "$candidate" ] && [ -x "$candidate" ]; then
      client_bin=$candidate
      break
    fi
  done
fi
if [ -z "$live_reason" ] && { [ -z "$client_bin" ] || [ ! -x "$client_bin" ]; }; then
  live_reason="bbb-native executable unavailable (set BBB_PLAYABLE_CLIENT_BIN)"
fi
if [ -z "$live_reason" ] && ! "$client_bin" --help 2>&1 | grep -F -- '--status-server' >/dev/null; then
  live_reason="bbb-native lacks required --status-server <addr> delegated status CLI"
fi
if [ -z "$live_reason" ] && [ -z "${DISPLAY:-}${WAYLAND_DISPLAY:-}" ]; then
  live_reason="no DISPLAY or WAYLAND_DISPLAY for the native client"
fi

if [ -n "$live_reason" ]; then
  if [ "$mode" = live ]; then
    echo "playable gate: live prerequisites missing: $live_reason" >&2
    echo "playable gate: live portion NOT RUN" >&2
    exit 2
  fi
  echo "playable gate: live portion NOT RUN: $live_reason"
  run_fixture
  exit 0
fi

timeout_seconds=${BBB_PLAYABLE_TIMEOUT_SECONDS:-120}
ready_timeout_seconds=${BBB_PLAYABLE_SERVER_READY_TIMEOUT_SECONDS:-120}
case "$timeout_seconds:$ready_timeout_seconds" in
  *[!0-9:]*|:*|*:|0:*|*:0)
    echo "playable gate: timeout values must be positive integer seconds" >&2
    exit 2
    ;;
esac

artifact_dir=${BBB_PLAYABLE_ARTIFACT_DIR:-}
if [ -z "$artifact_dir" ]; then
  artifact_dir=$(mktemp -d "${TMPDIR:-/tmp}/bbb-playable-gate.XXXXXX")
else
  mkdir -p "$artifact_dir"
  artifact_dir=$(CDPATH= cd -- "$artifact_dir" && pwd)
fi
server_dir="$artifact_dir/server"
world_dir="$server_dir/world"
server_log="$artifact_dir/server.log"
client_log="$artifact_dir/client.log"
status_json="$artifact_dir/status.json"
status_raw_json="$artifact_dir/status-raw.json"
evidence_json="$artifact_dir/evidence.json"
mkdir -p "$server_dir" "$world_dir"

server_port=${BBB_PLAYABLE_PORT:-}
if [ -z "$server_port" ]; then
  server_port=$(run_python reserve-port)
fi
control_port=${BBB_PLAYABLE_CONTROL_PORT:-}
if [ -z "$control_port" ]; then
  control_port=$(run_python reserve-port)
fi
if [ "$server_port" = "$control_port" ]; then
  echo "playable gate: server and control ports must differ" >&2
  exit 2
fi
case "$server_port:$control_port" in
  *[!0-9:]*|:*|*:|0:*|*:0)
    echo "playable gate: ports must be positive integers" >&2
    exit 2
    ;;
esac

# DedicatedServerProperties.java owns these property names.  The seed and
# small view/simulation distances keep the temporary world deterministic and
# cheap.  EULA acknowledgement is required above and is never inferred.
printf '%s\n' \
  'online-mode=false' \
  'server-ip=127.0.0.1' \
  "server-port=$server_port" \
  'level-name=world' \
  'level-seed=bbb-playable-gate-26.1' \
  'gamemode=creative' \
  'difficulty=peaceful' \
  'spawn-protection=0' \
  'view-distance=2' \
  'simulation-distance=2' \
  'max-players=1' \
  'motd=bbb playable gate 26.1' \
  >"$server_dir/server.properties"
printf 'eula=true\n' >"$server_dir/eula.txt"

server_pid=
client_pid=
cleanup() {
  trap - 0 HUP INT TERM
  for pid in "$client_pid" "$server_pid"; do
    if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
      kill -TERM -"$pid" 2>/dev/null || kill -TERM "$pid" 2>/dev/null || true
    fi
  done
  remaining=5
  while [ "$remaining" -gt 0 ]; do
    any_running=false
    for pid in "$client_pid" "$server_pid"; do
      if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
        any_running=true
      fi
    done
    if [ "$any_running" = false ]; then
      break
    fi
    sleep 1
    remaining=$((remaining - 1))
  done
  for pid in "$client_pid" "$server_pid"; do
    if [ -n "$pid" ]; then
      if kill -0 "$pid" 2>/dev/null; then
        kill -KILL -"$pid" 2>/dev/null || kill -KILL "$pid" 2>/dev/null || true
      fi
      wait "$pid" 2>/dev/null || true
    fi
  done
}
on_signal() {
  cleanup
  exit 130
}
trap cleanup 0
trap on_signal HUP INT TERM

echo "playable gate: artifacts: $artifact_dir"
echo "playable gate: starting vanilla 26.1 offline server on 127.0.0.1:$server_port"
(
  cd "$server_dir"
  exec setsid "$java_bin" -Xms512M -Xmx1G -jar "$server_jar" nogui
) </dev/null >"$server_log" 2>&1 &
server_pid=$!

# DedicatedServer.java logs this stable marker after initialization completes:
#   Done ({})! For help, type "help"
ready_deadline=$(( $(date +%s) + ready_timeout_seconds ))
while ! grep -F 'Done (' "$server_log" >/dev/null 2>&1; do
  if ! kill -0 "$server_pid" 2>/dev/null; then
    echo "playable gate failed: vanilla server exited before its ready marker" >&2
    echo "playable gate evidence: server log: $server_log" >&2
    exit 1
  fi
  if [ "$(date +%s)" -ge "$ready_deadline" ]; then
    echo "playable gate failed: timed out waiting for vanilla server ready marker" >&2
    echo "playable gate evidence: server log: $server_log" >&2
    exit 1
  fi
  sleep 1
done

if ! timeout "$timeout_seconds" \
  "$client_bin" --status-server "127.0.0.1:$server_port" \
  </dev/null >"$status_raw_json" 2>>"$client_log"
then
  echo "playable gate failed: bbb-native delegated status probe failed" >&2
  echo "playable gate evidence: packet/session timeline: $client_log" >&2
  echo "playable gate evidence: server log: $server_log" >&2
  exit 1
fi
if ! run_python normalize-status \
  --input "$status_raw_json" \
  --protocol-source "$repo_root/crates/bbb-protocol/src/lib.rs" \
  --output "$status_json"
then
  echo "playable gate evidence: delegated status JSON: $status_raw_json" >&2
  echo "playable gate evidence: packet/session timeline: $client_log" >&2
  echo "playable gate evidence: server log: $server_log" >&2
  exit 1
fi

echo "playable gate: launching bbb-native with bbb-control on 127.0.0.1:$control_port"
RUST_LOG=${RUST_LOG:-bbb_net=debug,bbb_native=info,bbb_control=info} \
  setsid "$client_bin" \
    --server "127.0.0.1:$server_port" \
    --username bbb-playablegate \
    --connect-server \
    --control "127.0.0.1:$control_port" \
    --client-vsync false \
    --client-framerate-limit 30 \
    </dev/null >"$client_log" 2>&1 &
client_pid=$!

if ! run_python collect-live \
  --control "127.0.0.1:$control_port" \
  --status "$status_json" \
  --client-log "$client_log" \
  --server-log "$server_log" \
  --packet-ledger "$repo_root/docs/packet-coverage-26.1.json" \
  --protocol-source "$repo_root/crates/bbb-protocol/src/lib.rs" \
  --output "$evidence_json" \
  --timeout "$timeout_seconds"
then
  echo "playable gate evidence: packet/session timeline: $client_log" >&2
  echo "playable gate evidence: world snapshot: $evidence_json" >&2
  echo "playable gate evidence: server log: $server_log" >&2
  exit 1
fi

run_python assert \
  --evidence "$evidence_json" \
  --protocol-source "$repo_root/crates/bbb-protocol/src/lib.rs"
echo "playable gate: live vanilla 26.1 session passed"
echo "playable gate evidence: packet/session timeline: $client_log"
echo "playable gate evidence: world snapshot: $evidence_json"
echo "playable gate evidence: server log: $server_log"
