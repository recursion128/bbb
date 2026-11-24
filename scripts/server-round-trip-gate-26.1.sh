#!/bin/sh
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$repo_root/scripts/gate-stamp-lib.sh"
python_runner="$repo_root/scripts/python"

usage() {
  cat <<'EOF'
Usage: scripts/server-round-trip-gate-26.1.sh

Runs the Stage 3 official round-trip end to end:
  1. Capture a deterministic official Mojang 26.1 world via
     scripts/capture-official-world-26.1.sh.
  2. Run the BBB_TEST_JAVA_26_1_WORLD ignored persistence tests against the
     captured world (BBB loads the official save).
  3. Load a copy into bbb-server --world and let the shutdown flush publish
     a BBB save.
  4. Have the official server hang one sand block in the air on a frozen tick,
     and let bbb-server be the process that ticks it into a
     minecraft:falling_block and writes the entities/ chunk holding it
     (scripts/server_round_trip_entity.py).
  5. Boot the official 26.1 server on the BBB-saved world, assert a clean
     startup log, assert it still holds the falling block BBB wrote, re-join
     the probe client, and stop.
  6. Create a normal Overworld with the official server, clear its generated
     chunks, let BBB generate and publish Noise FULL chunks, then require the
     official server to load, mutate, flush, and republish that same chunk.

Environment:
  BBB_OFFICIAL_CAPTURE_ACCEPT_EULA=1  Required before any official-server run.
                                      Never inferred.
  BBB_SERVER_PLAYABLE_CLIENT_BIN      Optional bbb-client executable.
  BBB_SERVER_PLAYABLE_SERVER_BIN      Optional bbb-server executable.
  BBB_SERVER_ROUND_TRIP_ADMIN_BIN     Optional bbb-admin executable; the entity
                                      step freezes and steps BBB through it.
  BBB_SERVER_ROUND_TRIP_ARTIFACT_DIR  Evidence directory; default is fresh /tmp.
  BBB_SERVER_PROBE_TIMEOUT_SECONDS    Flat and official probe deadline; default 90.
  BBB_SERVER_NOISE_PROBE_TIMEOUT_SECONDS
                                      Noise FULL publish/resume probe deadline; default 600.

The gate never downloads Java or the official jar. After explicit acceptance,
it writes eula.txt only below its disposable artifact directory and never
commits the capture receipt.
Prepare repo-managed Vanilla explicitly with:
  scripts/vanilla prepare 26.1
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
if [ "${BBB_OFFICIAL_CAPTURE_ACCEPT_EULA:-0}" != 1 ]; then
  echo "round-trip gate: requires BBB_OFFICIAL_CAPTURE_ACCEPT_EULA=1" >&2
  exit 2
fi

client_bin=${BBB_SERVER_PLAYABLE_CLIENT_BIN:-}
server_bin=${BBB_SERVER_PLAYABLE_SERVER_BIN:-}
# The entity step drives BBB's `/tick freeze` and `/tick step` through the
# management endpoint, which is what bbb-admin speaks; BBB has no console.
admin_bin=${BBB_SERVER_ROUND_TRIP_ADMIN_BIN:-}
for candidate in "${CARGO_TARGET_DIR:-}/debug/bbb-client" /tmp/bbb-target-main/debug/bbb-client; do
  if [ -z "$client_bin" ] && [ -x "$candidate" ]; then
    client_bin=$candidate
  fi
done
for candidate in "${CARGO_TARGET_DIR:-}/debug/bbb-server" /tmp/bbb-target-main/debug/bbb-server; do
  if [ -z "$server_bin" ] && [ -x "$candidate" ]; then
    server_bin=$candidate
  fi
done
for candidate in "${CARGO_TARGET_DIR:-}/debug/bbb-admin" /tmp/bbb-target-main/debug/bbb-admin; do
  if [ -z "$admin_bin" ] && [ -x "$candidate" ]; then
    admin_bin=$candidate
  fi
done
if [ -z "$client_bin" ] || [ ! -x "$client_bin" ]; then
  echo "round-trip gate: bbb-client is unavailable" >&2
  exit 2
fi
if [ -z "$server_bin" ] || [ ! -x "$server_bin" ]; then
  echo "round-trip gate: bbb-server is unavailable" >&2
  exit 2
fi
if [ -z "$admin_bin" ] || [ ! -x "$admin_bin" ]; then
  echo "round-trip gate: bbb-admin is unavailable" >&2
  exit 2
fi

require_fresh_bin "$repo_root" "$client_bin" "round-trip gate" || exit 2
require_fresh_bin "$repo_root" "$server_bin" "round-trip gate" || exit 2
require_fresh_bin "$repo_root" "$admin_bin" "round-trip gate" || exit 2

artifact_dir=${BBB_SERVER_ROUND_TRIP_ARTIFACT_DIR:-}
if [ -z "$artifact_dir" ]; then
  artifact_dir=$(mktemp -d "${TMPDIR:-/tmp}/bbb-round-trip.XXXXXX")
else
  mkdir -p "$artifact_dir"
fi

available_port() {
  "$python_runner" python -c \
    'import socket; s=socket.socket(); s.bind(("127.0.0.1", 0)); print(s.getsockname()[1]); s.close()'
}

server_pid=
fifo=
cleanup() {
  trap - 0 HUP INT TERM
  if [ -n "$server_pid" ] && kill -0 "$server_pid" 2>/dev/null; then
    kill -TERM "$server_pid" 2>/dev/null || true
    wait "$server_pid" 2>/dev/null || true
  fi
  if [ -n "$fifo" ]; then
    rm -f "$fifo"
  fi
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
      echo "round-trip gate: process exited before '$marker'; log: $log" >&2
      exit 1
    fi
    if [ "$remaining" -eq 0 ]; then
      echo "round-trip gate: timed out waiting for '$marker'; log: $log" >&2
      exit 1
    fi
    remaining=$((remaining - 1))
    sleep 1
  done
}

# The gate's one reading of "the official server could not open what BBB wrote".
# A save it cannot read never stops it: the chunk or the entity record is
# dropped, the server carries on, and the only place it says so is this log. It
# is a function because the entity step asks the same question a second time,
# after the reload has had a chance to reach the entity storage, and two
# spellings of the same check would be two things to keep in step.
require_clean_official_log() {
  official_errors=$(grep -E 'ERROR|Exception|Caused by' "$official_log" 2>/dev/null || true)
  unexpected_errors=$(printf '%s\n' "$official_errors" |
    grep -Fv "Signed command mismatch between server and client ('list')" |
    grep -Fv 'refuse-config lost connection: Internal Exception: java.lang.IllegalStateException: Client acknowledged config, but none was requested' |
    grep -Fv "refuse-body lost connection: Internal Exception: io.netty.handler.codec.DecoderException: Failed to decode packet 'serverbound/minecraft:chat'" |
    grep -Fv "refuse-nbt lost connection: Internal Exception: io.netty.handler.codec.DecoderException: Failed to decode packet 'serverbound/minecraft:set_creative_mode_slot'" || true)
  if [ -n "$unexpected_errors" ]; then
    echo "round-trip gate: official resume logged errors; log: $official_log" >&2
    exit 1
  fi
}

# Step 1: capture a fresh official 26.1 world (EULA gate owned by the capture script).
echo "round-trip gate: capturing official 26.1 world"
BBB_OFFICIAL_CAPTURE_ARTIFACT_DIR="$artifact_dir/capture" \
BBB_OFFICIAL_CAPTURE_RECEIPT="$artifact_dir/capture-receipt.json" \
  "$repo_root/scripts/capture-official-world-26.1.sh"
capture_world="$artifact_dir/capture/server/world"

# Step 2: BBB loads the official save (ignored persistence live-gate tests).
echo "round-trip gate: running BBB persistence preflight against the capture"
(
  cd "$repo_root"
  BBB_TEST_JAVA_26_1_WORLD="$capture_world" \
    cargo dev test -p bbb-server-persistence -- --ignored
)

# Step 3: BBB loads a copy and publishes its own save on shutdown.
bbb_client_world="$artifact_dir/bbb-client-world"
cp -a "$capture_world" "$bbb_client_world"
bbb_port=$(available_port)
bbb_log="$artifact_dir/bbb-server.log"
echo "round-trip gate: starting BBB on 127.0.0.1:$bbb_port with the captured world"
# Every property the shared file declares that this server has a flag for. The
# capture was written by an official server reading that file, and the save BBB
# publishes here is read back by one more of them, so a value BBB does not match
# is a value the round trip rewrites:
#
#   * `gamemode` is applied over the save's own GameType at every startup
#     (DedicatedServer.java:237), on both servers;
#   * `force-gamemode` (DedicatedServer.java:792-794) decides whether a
#     returning player's saved `playerGameType` outranks it;
#   * `difficulty` is applied the same way by `forceDifficulty`
#     (DedicatedServer.java:395-397), and BBB now writes the live value back
#     into `level.dat` (`Java26LevelDocument::raw_with_saved_state`), so leaving
#     the flag off would republish the capture's `peaceful` as BBB's own `easy`.
#
# scripts/tests/test_capture_replay_expectations.py keeps the three in step
# with scripts/data/official-replay-server.properties.
"$server_bin" --bind "127.0.0.1:$bbb_port" --world "$bbb_client_world" \
  --gamemode creative --force-gamemode --difficulty peaceful \
  --view-distance 2 --simulation-distance 2 --pause-when-empty-seconds 0 >"$bbb_log" 2>&1 &
server_pid=$!
# `server.ready` is logged once the world is open and both shutdown handlers are
# installed, which is what makes the SIGTERM below a flush this step can measure
# rather than a race with startup.
wait_marker '"event":"server.ready"' "$bbb_log" "$server_pid"
kill -TERM "$server_pid" 2>/dev/null || true
wait "$server_pid" 2>/dev/null || true
server_pid=
if ! grep -F '"event":"world.saved"' "$bbb_log" >/dev/null 2>&1; then
  echo "round-trip gate: BBB shutdown did not publish a world save; log: $bbb_log" >&2
  exit 1
fi

java_bin=$("$repo_root/scripts/vanilla" path 26.1 --kind java)
server_jar=$("$repo_root/scripts/vanilla" path 26.1 --kind server-jar)
vanilla_root=$("$repo_root/scripts/vanilla" path 26.1 --kind root)

# Step 4: BBB writes an entities/ chunk of its own.
#
# Everything above hands over blocks only. The capture is a fresh flat world
# with no entity in it, so the entity storage BBB republishes there is the empty
# one it read, and nothing yet says an official server can read an `entities/`
# chunk this server produced. The helper closes that: the official server hangs
# one sand block in the air over a forceloaded column with its tick loop frozen,
# and bbb-server is the process that runs the scheduled `FallingBlock` tick,
# spawns the `FallingBlockEntity`, moves it and saves it. It fails by name on
# every empty shape -- no entities/ directory, no chunk, an empty chunk, a
# record that is not a falling block or is missing the tags a saved one carries.
entity_world="$artifact_dir/entity-world"
echo "round-trip gate: letting BBB tick a hung sand block into a saved falling block"
"$python_runner" python "$repo_root/scripts/server_round_trip_entity.py" write \
  --artifact-dir "$artifact_dir/entity" \
  --server-bin "$server_bin" \
  --client-bin "$client_bin" \
  --admin-bin "$admin_bin" \
  --vanilla-root "$vanilla_root" \
  --java-bin "$java_bin" \
  --server-jar "$server_jar" \
  --input-world "$bbb_client_world" \
  --output-world "$entity_world" \
  >"$artifact_dir/entity-write.json"

# Step 5: the official 26.1 server resumes from the BBB save.
official_dir="$artifact_dir/official-resume"
official_log="$artifact_dir/official-resume.log"
official_port=$(available_port)
fifo="$official_dir/server.stdin"
mkdir -p "$official_dir"
cp -a "$entity_world" "$official_dir/world"
cat "$repo_root/scripts/data/official-replay-server.properties" >"$official_dir/server.properties"
printf '%s\n' \
  "server-port=$official_port" \
  'level-name=world' \
  'motd=bbb round-trip resume 26.1' \
  >>"$official_dir/server.properties"
printf 'eula=true\n' >"$official_dir/eula.txt"
mkfifo "$fifo"

echo "round-trip gate: resuming the BBB save on the official 26.1 server"
(
  cd "$official_dir"
  exec "$java_bin" -Xms512M -Xmx1G -jar "$server_jar" nogui <"$fifo"
) >"$official_log" 2>&1 &
server_pid=$!
exec 3>"$fifo"
wait_marker 'Done (' "$official_log" "$server_pid"
require_clean_official_log

# The entity half of the hand-over, before anything else touches this server.
#
# The forced ticket in the save has just made Mojang's own `EntityStorage` read
# the chunk BBB wrote, so a record it could not decode is already in the log the
# check above and below read. What is left is that the entity survived the trip,
# and that this server really wrote it rather than leaving BBB's file alone:
# `--published` holds the reload against the save BBB published and requires
# the entity column's own `RegionFile` write stamp to have moved
# (RegionFile.java:309), which only a server that loaded the column and saved it
# again can do.
#
# Both have to be asked of a stopped clock. `FallingBlockEntity.tick` puts the
# block back and discards the entity the moment it lands
# (FallingBlockEntity.java:194-198), which would empty the entity storage for
# the honest reason rather than the interesting one. `/tick freeze`
# (TickCommand.java:52-53) stops it mid-air, `save-all flush` publishes what the
# official server is holding, and `/tick unfreeze` hands the running server back
# to the probe client below -- "The game is running normally" is that command's
# own answer (`commands.tick.status.running`, TickCommand.java:125, rendered by
# en_us.json:3041).
printf 'tick freeze\nsave-all flush\n' >&3
wait_marker 'Saved the game' "$official_log" "$server_pid"
require_clean_official_log
"$python_runner" python "$repo_root/scripts/server_round_trip_entity.py" verify \
  --world "$official_dir/world" \
  --actor "the official 26.1 server" \
  --published "$artifact_dir/entity-write.json" \
  >"$artifact_dir/official-resume-entity.json"
printf 'tick unfreeze\n' >&3
wait_marker 'The game is running normally' "$official_log" "$server_pid"

probe_json="$artifact_dir/official-resume-probe.json"
mkdir -p "$artifact_dir/client"
# Keep the process supervisor behind the client's own probe deadline so the
# client can publish its attributable timeout before it is terminated.
probe_timeout_seconds=${BBB_SERVER_PROBE_TIMEOUT_SECONDS:-90}
probe_process_timeout_seconds=$((probe_timeout_seconds + 10))
noise_probe_timeout_seconds=${BBB_SERVER_NOISE_PROBE_TIMEOUT_SECONDS:-600}
noise_probe_process_timeout_seconds=$((noise_probe_timeout_seconds + 10))

"$repo_root/scripts/process" run --timeout "$probe_process_timeout_seconds" -- \
  "$client_bin" \
    --game-dir "$artifact_dir/client" \
    --vanilla-root "$vanilla_root" \
    --server "127.0.0.1:$official_port" \
    --username round-trip-probe \
    --probe-server \
    --probe-timeout-seconds "$probe_timeout_seconds" \
    --probe-server-interactions \
    >"$probe_json"
"$python_runner" python "$repo_root/scripts/server_playable_gate.py" assert --input "$probe_json"

printf 'stop\n' >&3
exec 3>&-
wait "$server_pid" 2>/dev/null || true
server_pid=
rm -f "$fifo"
fifo=

noise_helper="$repo_root/scripts/server_round_trip_noise.py"
noise_seed_dir="$artifact_dir/noise-seed"
noise_seed_log="$artifact_dir/noise-seed.log"
noise_seed_port=$(available_port)
fifo="$noise_seed_dir/server.stdin"
mkdir -p "$noise_seed_dir"
cat "$repo_root/scripts/data/official-replay-server.properties" >"$noise_seed_dir/server.properties"
printf '%s\n' \
  "server-port=$noise_seed_port" \
  'level-name=world' \
  'level-seed=0' \
  'level-type=minecraft:normal' \
  'generate-structures=true' \
  'motd=bbb noise round-trip seed 26.1' \
  >>"$noise_seed_dir/server.properties"
printf 'eula=true\n' >"$noise_seed_dir/eula.txt"
mkfifo "$fifo"

echo "round-trip gate: creating official normal Overworld settings"
official_log="$noise_seed_log"
(
  cd "$noise_seed_dir"
  exec "$java_bin" -Xms512M -Xmx1G -jar "$server_jar" nogui <"$fifo"
) >"$official_log" 2>&1 &
server_pid=$!
exec 3>"$fifo"
wait_marker 'Done (' "$official_log" "$server_pid"
require_clean_official_log
printf 'stop\n' >&3
exec 3>&-
wait "$server_pid" 2>/dev/null || true
server_pid=
require_clean_official_log
rm -f "$fifo"
fifo=

noise_bbb_client_world="$artifact_dir/noise-bbb-client-world"
"$python_runner" python "$noise_helper" prepare \
  --input-world "$noise_seed_dir/world" \
  --output-world "$noise_bbb_client_world"

noise_bbb_port=$(available_port)
noise_bbb_log="$artifact_dir/noise-bbb-server.log"
noise_probe_json="$artifact_dir/noise-bbb-probe.json"
mkdir -p "$artifact_dir/noise-bbb-client"
echo "round-trip gate: letting BBB publish normal Overworld Noise FULL chunks"
"$server_bin" --bind "127.0.0.1:$noise_bbb_port" --world "$noise_bbb_client_world" \
  --gamemode creative --force-gamemode --difficulty peaceful \
  --view-distance 2 --simulation-distance 2 --pause-when-empty-seconds 0 >"$noise_bbb_log" 2>&1 &
server_pid=$!
wait_marker '"event":"server.ready"' "$noise_bbb_log" "$server_pid"
"$repo_root/scripts/process" run --timeout "$noise_probe_process_timeout_seconds" -- \
  "$client_bin" \
    --game-dir "$artifact_dir/noise-bbb-client" \
    --vanilla-root "$vanilla_root" \
    --server "127.0.0.1:$noise_bbb_port" \
    --username noise-bbb-probe \
    --probe-server \
    --probe-timeout-seconds "$noise_probe_timeout_seconds" \
    --probe-server-interactions \
    >"$noise_probe_json"
"$python_runner" python "$repo_root/scripts/server_playable_gate.py" assert \
  --input "$noise_probe_json"
kill -TERM "$server_pid" 2>/dev/null || true
wait "$server_pid" 2>/dev/null || true
server_pid=
if ! grep -F '"event":"world.saved"' "$noise_bbb_log" >/dev/null 2>&1; then
  echo "round-trip gate: BBB noise shutdown did not publish a world save; log: $noise_bbb_log" >&2
  exit 1
fi

noise_published_json="$artifact_dir/noise-bbb-published.json"
"$python_runner" python "$noise_helper" inspect \
  --world "$noise_bbb_client_world" \
  --probe "$noise_probe_json" \
  --actor bbb-server \
  >"$noise_published_json"

noise_official_dir="$artifact_dir/noise-official-resume"
noise_official_log="$artifact_dir/noise-official-resume.log"
noise_official_port=$(available_port)
fifo="$noise_official_dir/server.stdin"
mkdir -p "$noise_official_dir"
cp -a "$noise_bbb_client_world" "$noise_official_dir/world"
cat "$repo_root/scripts/data/official-replay-server.properties" >"$noise_official_dir/server.properties"
printf '%s\n' \
  "server-port=$noise_official_port" \
  'level-name=world' \
  'motd=bbb noise round-trip resume 26.1' \
  >>"$noise_official_dir/server.properties"
printf 'eula=true\n' >"$noise_official_dir/eula.txt"
mkfifo "$fifo"

marker_coordinates=$("$python_runner" python -c \
  'import json, sys; marker=json.load(open(sys.argv[1], encoding="utf-8"))["marker"]; print(marker["x"], marker["y"], marker["z"])' \
  "$noise_published_json")
set -- $marker_coordinates
marker_x=$1
marker_y=$2
marker_z=$3
marker_note_y=$((marker_y + 1))
marker_control_y=$((marker_y + 2))

echo "round-trip gate: resuming BBB Noise FULL chunks on the official 26.1 server"
official_log="$noise_official_log"
(
  cd "$noise_official_dir"
  exec "$java_bin" -Xms512M -Xmx1G -jar "$server_jar" nogui <"$fifo"
) >"$official_log" 2>&1 &
server_pid=$!
exec 3>"$fifo"
wait_marker 'Done (' "$official_log" "$server_pid"
require_clean_official_log

# The marker need not be inside the official server's configured spawn ticket.
# `forceload add` converts these block coordinates to one chunk and confirms
# the ticket after `setChunkForced` succeeds (ForceLoadCommand.java:145-177).
printf 'forceload add %s %s\n' "$marker_x" "$marker_z" >&3
wait_marker 'Marked chunk' "$official_log" "$server_pid"

# The condition must succeed before the control write, or an official
# regeneration could manufacture the only mutation used as reload evidence.
printf 'execute if block %s %s %s minecraft:stone if block %s %s %s minecraft:note_block run say bbb-noise-full-marker-loaded\n' \
  "$marker_x" "$marker_y" "$marker_z" \
  "$marker_x" "$marker_note_y" "$marker_z" >&3
wait_marker 'bbb-noise-full-marker-loaded' "$official_log" "$server_pid"
printf 'setblock %s %s %s minecraft:gold_block\n' \
  "$marker_x" "$marker_control_y" "$marker_z" >&3
wait_marker 'Changed the block at' "$official_log" "$server_pid"

noise_official_probe_json="$artifact_dir/noise-official-resume-probe.json"
mkdir -p "$artifact_dir/noise-official-resume-client"
# The official resume may generate and stream columns outside BBB's published
# window before replay updates surface, so it owns the same long Noise budget
# as the publishing side rather than the flat-world budget.
"$repo_root/scripts/process" run --timeout "$noise_probe_process_timeout_seconds" -- \
  "$client_bin" \
    --game-dir "$artifact_dir/noise-official-resume-client" \
    --vanilla-root "$vanilla_root" \
    --server "127.0.0.1:$noise_official_port" \
    --username noise-java-probe \
    --probe-server \
    --probe-timeout-seconds "$noise_probe_timeout_seconds" \
    --probe-server-interactions \
    >"$noise_official_probe_json"
"$python_runner" python "$repo_root/scripts/server_playable_gate.py" assert \
  --input "$noise_official_probe_json"

printf 'save-all flush\n' >&3
wait_marker 'Saved the game' "$official_log" "$server_pid"
require_clean_official_log
printf 'stop\n' >&3
exec 3>&-
wait "$server_pid" 2>/dev/null || true
server_pid=
require_clean_official_log
rm -f "$fifo"
fifo=

"$python_runner" python "$noise_helper" verify \
  --world "$noise_official_dir/world" \
  --published "$noise_published_json" \
  --actor "the official 26.1 server" \
  >"$artifact_dir/noise-official-verified.json"

echo "round-trip gate: official→BBB→official round-trip passed; artifacts: $artifact_dir"
