#!/bin/sh
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$repo_root/scripts/gate-stamp-lib.sh"
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
  BBB_PLAYABLE_TRANSFER_PORT    Transfer-target server port; default is an available loopback port
  BBB_PLAYABLE_CONTROL_PORT     bbb-control port; default is an available loopback port
  BBB_PLAYABLE_ARTIFACT_DIR     Evidence directory; default is a fresh /tmp directory
  BBB_PLAYABLE_TIMEOUT_SECONDS  Client/canonical-state timeout; default 120
  BBB_PLAYABLE_SERVER_READY_TIMEOUT_SECONDS  Server-ready timeout; default 120
  BBB_PLAYABLE_CLIENT_LOCALE  Deterministic client locale: en_us (default), ar_sa, or zh_cn
  BBB_PLAYABLE_TERMINATION_MODE  Optional validation-only clean quit or server kick
  BBB_LIFECYCLE_SOAK_MANIFEST    Required manifest when termination mode is set

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
  (cd "$repo_root" && record_gate_stamp playable-log.tsv last-playable-fixture \
    fixture "mode:requested")
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
if [ -z "$live_reason" ] && ! command -v mkfifo >/dev/null 2>&1; then
  live_reason="mkfifo is unavailable for the persistent server command channel"
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
  (cd "$repo_root" && record_gate_stamp playable-log.tsv last-playable-fixture \
    fixture "fallback:$live_reason")
  exit 0
fi

timeout_seconds=${BBB_PLAYABLE_TIMEOUT_SECONDS:-120}
ready_timeout_seconds=${BBB_PLAYABLE_SERVER_READY_TIMEOUT_SECONDS:-120}
client_locale=${BBB_PLAYABLE_CLIENT_LOCALE:-en_us}
termination_mode=${BBB_PLAYABLE_TERMINATION_MODE:-}
termination_manifest=${BBB_LIFECYCLE_SOAK_MANIFEST:-}
termination_timeout_seconds=${BBB_PLAYABLE_TERMINATION_TIMEOUT_SECONDS:-60}
case "$timeout_seconds:$ready_timeout_seconds" in
  *[!0-9:]*|:*|*:|0:*|*:0)
    echo "playable gate: timeout values must be positive integer seconds" >&2
    exit 2
    ;;
esac
case "$termination_timeout_seconds" in
  ''|*[!0-9]*|0)
    echo "playable gate: termination timeout must be positive integer seconds" >&2
    exit 2
    ;;
esac
case "$client_locale" in
  en_us|ar_sa|zh_cn) ;;
  *)
    echo "playable gate: BBB_PLAYABLE_CLIENT_LOCALE must be en_us, ar_sa, or zh_cn" >&2
    exit 2
    ;;
esac
case "$termination_mode" in
  "") ;;
  quit|kick)
    if [ -z "$termination_manifest" ] || [ ! -f "$termination_manifest" ]; then
      echo "playable gate: termination mode requires BBB_LIFECYCLE_SOAK_MANIFEST" >&2
      exit 2
    fi
    ;;
  *)
    echo "playable gate: BBB_PLAYABLE_TERMINATION_MODE must be quit or kick" >&2
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
transfer_server_dir="$artifact_dir/transfer-server"
transfer_world_dir="$transfer_server_dir/world"
transfer_server_log="$artifact_dir/transfer-server.log"
client_log="$artifact_dir/client.log"
status_json="$artifact_dir/status.json"
status_raw_json="$artifact_dir/status-raw.json"
probe_json="$artifact_dir/probe.json"
evidence_json="$artifact_dir/evidence.json"
termination_json="$artifact_dir/termination.json"
restricted_client_log="$artifact_dir/restricted-client.log"
chat_restriction_json="$artifact_dir/chat-restriction.json"
server_command="$artifact_dir/server-command.fifo"
transfer_server_command="$artifact_dir/transfer-server-command.fifo"
server_pack_dir="$artifact_dir/server-pack-http"
server_pack_zip="$server_pack_dir/lifecycle-pack.zip"
server_pack_cache="$artifact_dir/server-pack-cache"
mkdir -p "$server_dir" "$world_dir" "$transfer_server_dir" "$transfer_world_dir"

server_port=${BBB_PLAYABLE_PORT:-}
if [ -z "$server_port" ]; then
  server_port=$(run_python reserve-port)
fi
control_port=${BBB_PLAYABLE_CONTROL_PORT:-}
if [ -z "$control_port" ]; then
  control_port=$(run_python reserve-port)
fi
restricted_control_port=$(run_python reserve-port)
while [ "$restricted_control_port" = "$server_port" ] ||
  [ "$restricted_control_port" = "$control_port" ]
do
  restricted_control_port=$(run_python reserve-port)
done
transfer_server_port=${BBB_PLAYABLE_TRANSFER_PORT:-}
if [ -z "$transfer_server_port" ]; then
  transfer_server_port=$(run_python reserve-port)
  while [ "$transfer_server_port" = "$server_port" ] ||
    [ "$transfer_server_port" = "$control_port" ] ||
    [ "$transfer_server_port" = "$restricted_control_port" ]
  do
    transfer_server_port=$(run_python reserve-port)
  done
fi
server_pack_port=$(run_python reserve-port)
while [ "$server_pack_port" = "$server_port" ] ||
  [ "$server_pack_port" = "$control_port" ] ||
  [ "$server_pack_port" = "$restricted_control_port" ] ||
  [ "$server_pack_port" = "$transfer_server_port" ]
do
  server_pack_port=$(run_python reserve-port)
done
server_pack_id=5d77e69d-1c8f-4fd8-9884-4ce89238f170
server_pack_url="http://127.0.0.1:$server_pack_port/lifecycle-pack.zip"
profile_id=$(run_python write-operator \
  --output "$server_dir/ops.json" \
  --username bbb-playablegate)
server_pack_sha1=$(run_python write-server-pack --output "$server_pack_zip")
mkfifo "$server_command" "$transfer_server_command"
# Keep one read/write descriptor alive for the whole server lifetime. Opening a
# FIFO only on the server's read side would block startup before the collector
# has a chance to write the reconfiguration command.
exec 3<>"$server_command"
exec 4<>"$transfer_server_command"
if [ "$server_port" = "$control_port" ] ||
  [ "$server_port" = "$transfer_server_port" ] ||
  [ "$control_port" = "$transfer_server_port" ] ||
  [ "$restricted_control_port" = "$server_port" ] ||
  [ "$restricted_control_port" = "$control_port" ] ||
  [ "$restricted_control_port" = "$transfer_server_port" ]
then
  echo "playable gate: server, transfer-server, and control ports must differ" >&2
  exit 2
fi
for playable_port in "$server_port" "$transfer_server_port" "$control_port" "$restricted_control_port"; do
  case "$playable_port" in
    ''|*[!0-9]*|0)
      echo "playable gate: ports must be positive integers" >&2
      exit 2
      ;;
  esac
done

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
  "resource-pack=$server_pack_url" \
  "resource-pack-id=$server_pack_id" \
  "resource-pack-sha1=$server_pack_sha1" \
  'require-resource-pack=false' \
  >"$server_dir/server.properties"
printf 'eula=true\n' >"$server_dir/eula.txt"

# The target is a separate official DedicatedServer. Its distinct seed, fresh
# world, and separate join log let the collector prove that the same
# native/control process completed Transfer-intent Login -> Configuration ->
# Play without retaining the first server's level, chunks, or pack session.
printf '%s\n' \
  'online-mode=false' \
  'server-ip=127.0.0.1' \
  "server-port=$transfer_server_port" \
  'level-name=world' \
  'level-seed=bbb-playable-transfer-26.1' \
  'gamemode=creative' \
  'difficulty=peaceful' \
  'spawn-protection=0' \
  'view-distance=2' \
  'simulation-distance=2' \
  'max-players=1' \
  'motd=bbb playable transfer target 26.1' \
  'accepts-transfers=true' \
  >"$transfer_server_dir/server.properties"
printf 'eula=true\n' >"$transfer_server_dir/eula.txt"

server_pid=
transfer_server_pid=
client_pid=
restricted_client_pid=
server_pack_http_pid=
cleanup() {
  trap - 0 HUP INT TERM
  for pid in "$client_pid" "$restricted_client_pid" "$server_pid" "$transfer_server_pid" "$server_pack_http_pid"; do
    if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
      kill -TERM -"$pid" 2>/dev/null || kill -TERM "$pid" 2>/dev/null || true
    fi
  done
  remaining=5
  while [ "$remaining" -gt 0 ]; do
    any_running=false
    for pid in "$client_pid" "$restricted_client_pid" "$server_pid" "$transfer_server_pid" "$server_pack_http_pid"; do
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
  for pid in "$client_pid" "$restricted_client_pid" "$server_pid" "$transfer_server_pid" "$server_pack_http_pid"; do
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
echo "playable gate: serving lifecycle resource pack on 127.0.0.1:$server_pack_port"
setsid "$python_runner" python -m http.server "$server_pack_port" \
  --bind 127.0.0.1 \
  --directory "$server_pack_dir" \
  >"$artifact_dir/server-pack-http.log" 2>&1 &
server_pack_http_pid=$!
echo "playable gate: starting vanilla 26.1 offline server on 127.0.0.1:$server_port"
(
  cd "$server_dir"
  exec setsid "$java_bin" \
    -DMC_DEBUG_ENABLED \
    -DMC_DEBUG_DEV_COMMANDS \
    -Xms512M -Xmx1G -jar "$server_jar" nogui
) <"$server_command" >"$server_log" 2>&1 &
server_pid=$!
echo "playable gate: starting vanilla 26.1 transfer target on 127.0.0.1:$transfer_server_port"
(
  cd "$transfer_server_dir"
  exec setsid "$java_bin" -Xms512M -Xmx1G -jar "$server_jar" nogui
) <"$transfer_server_command" >"$transfer_server_log" 2>&1 &
transfer_server_pid=$!

# DedicatedServer.java logs this stable marker after initialization completes:
#   Done ({})! For help, type "help"
wait_server_ready() {
  ready_log=$1
  ready_pid=$2
  ready_label=$3
  ready_deadline=$(( $(date +%s) + ready_timeout_seconds ))
  while ! grep -F 'Done (' "$ready_log" >/dev/null 2>&1; do
    if ! kill -0 "$ready_pid" 2>/dev/null; then
      echo "playable gate failed: $ready_label exited before its ready marker" >&2
      echo "playable gate evidence: $ready_label log: $ready_log" >&2
      exit 1
    fi
    if [ "$(date +%s)" -ge "$ready_deadline" ]; then
      echo "playable gate failed: timed out waiting for $ready_label ready marker" >&2
      echo "playable gate evidence: $ready_label log: $ready_log" >&2
      exit 1
    fi
    sleep 1
  done
}
wait_server_ready "$server_log" "$server_pid" "vanilla server"
wait_server_ready "$transfer_server_log" "$transfer_server_pid" "vanilla transfer server"

if ! timeout "$timeout_seconds" \
  "$client_bin" --status-server "127.0.0.1:$server_port" \
  </dev/null >"$status_raw_json" 2>>"$client_log"
then
  echo "playable gate failed: bbb-native delegated status probe failed" >&2
  echo "playable gate evidence: packet/session timeline: $client_log" >&2
  echo "playable gate evidence: server log: $server_log" >&2
  echo "playable gate evidence: transfer server log: $transfer_server_log" >&2
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
  echo "playable gate evidence: transfer server log: $transfer_server_log" >&2
  exit 1
fi

if ! timeout "$timeout_seconds" \
  "$client_bin" \
    --server "127.0.0.1:$server_port" \
    --username bbb-probe \
    --probe-server \
    </dev/null >"$probe_json" 2>>"$client_log"
then
  echo "playable gate failed: bbb-native headless Login/Configuration/Play probe failed" >&2
  echo "playable gate evidence: headless probe JSON: $probe_json" >&2
  echo "playable gate evidence: packet/session timeline: $client_log" >&2
  echo "playable gate evidence: server log: $server_log" >&2
  exit 1
fi
if ! run_python validate-probe --input "$probe_json"; then
  echo "playable gate evidence: headless probe JSON: $probe_json" >&2
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
    --server-resource-pack-policy accept \
    --server-resource-pack-cache-dir "$server_pack_cache" \
    --client-locale "$client_locale" \
    --client-save-chat-drafts true \
    --client-vsync false \
    --client-framerate-limit 30 \
    </dev/null >"$client_log" 2>&1 &
client_pid=$!

if ! run_python collect-live \
  --control "127.0.0.1:$control_port" \
  --status "$status_json" \
  --client-log "$client_log" \
  --server-log "$server_log" \
  --transfer-server-log "$transfer_server_log" \
  --transfer-server-port "$transfer_server_port" \
  --packet-ledger "$repo_root/docs/evidence/vanilla-26.1/packet-coverage.json" \
  --protocol-source "$repo_root/crates/bbb-protocol/src/lib.rs" \
  --output "$evidence_json" \
  --timeout "$timeout_seconds" \
  --server-command "$server_command" \
  --profile-id "$profile_id" \
  --server-pack-id "$server_pack_id" \
  --client-locale "$client_locale"
then
  echo "playable gate evidence: packet/session timeline: $client_log" >&2
  echo "playable gate evidence: world snapshot: $evidence_json" >&2
  echo "playable gate evidence: server log: $server_log" >&2
  echo "playable gate evidence: transfer server log: $transfer_server_log" >&2
  exit 1
fi

echo "playable gate: launching system-visibility ChatScreen acceptance client"
RUST_LOG=${RUST_LOG:-bbb_net=debug,bbb_native=info,bbb_control=info} \
  setsid "$client_bin" \
    --server "127.0.0.1:$server_port" \
    --username bbb-restricted \
    --connect-server \
    --control "127.0.0.1:$restricted_control_port" \
    --server-resource-pack-policy decline \
    --client-locale "$client_locale" \
    --client-chat-visibility system \
    --client-vsync false \
    --client-framerate-limit 30 \
    </dev/null >"$restricted_client_log" 2>&1 &
restricted_client_pid=$!
if ! run_python collect-chat-restriction \
  --control "127.0.0.1:$restricted_control_port" \
  --timeout "$timeout_seconds" \
  --output "$chat_restriction_json"
then
  echo "playable gate evidence: restricted client log: $restricted_client_log" >&2
  exit 1
fi
remaining=$timeout_seconds
while kill -0 "$restricted_client_pid" 2>/dev/null && [ "$remaining" -gt 0 ]; do
  restricted_state=$(ps -o stat= -p "$restricted_client_pid" 2>/dev/null || true)
  case "$restricted_state" in
    *Z*) break ;;
  esac
  sleep 1
  remaining=$((remaining - 1))
done
if kill -0 "$restricted_client_pid" 2>/dev/null; then
  restricted_state=$(ps -o stat= -p "$restricted_client_pid" 2>/dev/null || true)
  case "$restricted_state" in
    *Z*) ;;
    *)
      echo "playable gate failed: restricted client did not exit after app.quit" >&2
      exit 1
      ;;
  esac
fi
if ! wait "$restricted_client_pid"; then
  echo "playable gate failed: restricted client exited unsuccessfully" >&2
  exit 1
fi
restricted_client_pid=
run_python merge-chat-restriction \
  --evidence "$evidence_json" \
  --restriction "$chat_restriction_json"

run_python assert \
  --evidence "$evidence_json" \
  --protocol-source "$repo_root/crates/bbb-protocol/src/lib.rs"
if [ -n "$termination_mode" ]; then
  run_python terminate-live \
    --control "127.0.0.1:$control_port" \
    --mode "$termination_mode" \
    --manifest "$termination_manifest" \
    --transfer-server-command "$transfer_server_command" \
    --transfer-server-log "$transfer_server_log" \
    --client-pid "$client_pid" \
    --timeout "$termination_timeout_seconds" \
    --output "$termination_json"

  terminated_client_pid=$client_pid
  termination_deadline=$(( $(date +%s) + termination_timeout_seconds ))
  while kill -0 "$terminated_client_pid" 2>/dev/null; do
    client_state=$(ps -o stat= -p "$terminated_client_pid" 2>/dev/null || true)
    case "$client_state" in
      Z*|"") break ;;
    esac
    if [ "$(date +%s)" -ge "$termination_deadline" ]; then
      echo "playable gate failed: client did not exit after termination request" >&2
      echo "playable gate evidence: termination: $termination_json" >&2
      exit 1
    fi
    sleep 1
  done
  if wait "$terminated_client_pid"; then
    client_exit_status=0
  else
    client_exit_status=$?
  fi
  client_pid=
  run_python finalize-termination \
    --evidence "$termination_json" \
    --client-pid "$terminated_client_pid" \
    --exit-status "$client_exit_status"
fi
echo "playable gate: live smoke passed"
echo "playable gate: live vanilla 26.1 session passed"
echo "playable gate evidence: packet/session timeline: $client_log"
echo "playable gate evidence: headless probe: $probe_json"
echo "playable gate evidence: world snapshot: $evidence_json"
echo "playable gate evidence: server log: $server_log"
echo "playable gate evidence: transfer server log: $transfer_server_log"
if [ -n "$termination_mode" ]; then
  echo "playable gate evidence: termination: $termination_json"
fi
(cd "$repo_root" && record_gate_stamp playable-log.tsv last-playable-live \
  live "evidence:$evidence_json")
