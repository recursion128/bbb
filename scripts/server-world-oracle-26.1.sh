#!/bin/sh
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$repo_root/scripts/gate-stamp-lib.sh"
python_runner="$repo_root/scripts/python"
helper="$repo_root/scripts/server_world_oracle.py"

usage() {
  cat <<'EOF'
Usage: scripts/server-world-oracle-26.1.sh [SCENARIO ...]

Opens one prepared official 26.1 save twice -- once on the official Mojang
server and once on bbb-server -- with no client connected to either, freezes
both, steps each exactly the same number of ticks, and compares the block and
entity truth they wrote back to their region files.

This is the oracle for the server behaviour a client cannot see. The
differential replay compares what a client observed, so it can only report on
behaviour some inbound packet asked for; a scheduled block tick, a fluid
spreading, an entity falling and a periodic save all happen with nobody asking.
Here the official server is both the authority and the scenario author: it is
booted first with its tick loop frozen, told where to put the water, the sand
and the leaves, and stopped, and the save it wrote is the shared input.

The step is what makes it a measurement. Both sides get `/tick freeze` and
`/tick step <n>t`, and afterwards both saves have to say that `level.dat`'s
`Data.Time` advanced by exactly n from the prepared save's value. A side that
ran a different number of ticks fails the run by name instead of quietly being
compared at the wrong moment.

Scenarios:
  fluid_trench           A water source at one end of a walled stone trench.
                         Settles to a source, seven falling levels and air.
                         Covers server-block-tick-scheduling.
  falling_sand_column    Four sand blocks hung ten blocks above the ground.
                         Settles to a four-block stack on the ground.
                         Covers server-entity-runtime.
  leaf_distance_cascade  Six leaves hung off a log, and the log taken away.
                         The scheduled leaf tick walks the row to distance=7.
                         Covers server-block-tick-scheduling.
  leaf_decay_patch       Nine unsupported leaves under a high randomTickSpeed.
                         The random tick removes every one of them.
                         Covers server-block-tick-scheduling.
  chorus_survival_tick   A chorus flower loses its end-stone support while
                         frozen. Its pending one-tick survival check removes
                         it. Covers server-block-tick-scheduling.
  chorus_random_growth   An age-four chorus flower grows once from end stone,
                         then a stone blocker makes the new flower age five.
                         Covers server-block-tick-scheduling.
  entity_fall_and_land   Two sand blocks dropped from two heights: one lands
                         inside the step and one is still airborne when it
                         ends, so the entity storage is compared with something
                         in it. Covers server-entity-runtime.
  redstone_lamp_release  A lamp lit by a redstone block, and the redstone
                         block taken away again. The save carries the pending
                         four-tick lamp tick; the step is what turns the lamp
                         off. Covers server-redstone.
  redstone_lamp_repowered
                         The same pending tick with the power put back before
                         it fires, so the lamp has to stay lit, next to a
                         witness lamp that is released and does go out.
                         Covers server-redstone.
  redstone_torch_tower   A torch, the block it charges, and a second torch on
                         top of that block, written in the order that leaves
                         the upper torch's two-tick toggle pending. The step
                         puts it out; the lower one stays lit.
                         Covers server-redstone.
  repeater_delay_release A repeater armed by a redstone block, with its
                         eight-tick delay still pending in the save, pointing
                         at a lamp. The step is what switches the repeater on
                         and lights the lamp. Covers server-redstone.
  repeater_pulse_and_lock
                         Three repeaters: one fed by a pulse shorter than its
                         own delay, which has to switch itself on and then back
                         off; one whose pending tick has to do nothing because
                         a repeater beside it locks it; and one plain armed
                         repeater as the witness that the box moved at all.
                         Covers server-redstone.

Naming scenarios runs only those. With no argument every scenario runs.

The result JSON is written to the artifact directory (or BBB_SERVER_WORLD_ORACLE_OUTPUT).
This is a debugging comparison, not acceptance evidence.

Exit status:
  0  every scenario measured something and every compared block and entity
     agrees
  1  the comparison ran and either found differences or found a scenario that
     measured nothing at all; the result JSON names both
  2  the harness could not produce a comparison

Environment:
  BBB_SERVER_WORLD_ORACLE_ARTIFACT_DIR   Result directory; default is a fresh /tmp.
  BBB_SERVER_WORLD_ORACLE_SERVER_BIN     Optional bbb-server executable.
  BBB_SERVER_WORLD_ORACLE_ADMIN_BIN      Optional bbb-admin executable.
  BBB_SERVER_WORLD_ORACLE_WORLD_TEMPLATE An official Mojang 26.1 save to build
                                         the scenario inside a copy of. Absent
                                         captures one.
  BBB_SERVER_WORLD_ORACLE_STEP_TICKS     How many ticks each side is stepped.
  BBB_SERVER_WORLD_ORACLE_OUTPUT         Result JSON path; default is inside the
                                         artifact directory.
  BBB_OFFICIAL_CAPTURE_ACCEPT_EULA=1     Required: this harness boots the
                                         official 26.1 server. Never inferred.

The harness never downloads anything. Prepare repo-managed Vanilla explicitly
with:
  scripts/vanilla prepare 26.1
EOF
}

case "${1:-}" in
  -h|--help)
    usage
    exit 0
    ;;
esac

# Every stage of this harness boots Mojang's own server: it authors the
# scenario, and it is one of the two sides being compared. Accepting the EULA is
# never inferred.
if [ "${BBB_OFFICIAL_CAPTURE_ACCEPT_EULA:-0}" != 1 ]; then
  echo "server world oracle: this harness boots the official 26.1 server; set" >&2
  echo "  BBB_OFFICIAL_CAPTURE_ACCEPT_EULA=1 to accept the Mojang EULA" >&2
  exit 2
fi

artifact_dir=${BBB_SERVER_WORLD_ORACLE_ARTIFACT_DIR:-}
if [ -z "$artifact_dir" ]; then
  artifact_dir=$(mktemp -d "${TMPDIR:-/tmp}/bbb-server-world-oracle.XXXXXX")
else
  mkdir -p "$artifact_dir"
fi

resolve_bin() {
  explicit=$1
  name=$2
  if [ -n "$explicit" ]; then
    printf '%s\n' "$explicit"
    return 0
  fi
  for candidate in "${CARGO_TARGET_DIR:-}/debug/$name" /tmp/bbb-target-main/debug/"$name"; do
    if [ -x "$candidate" ]; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done
  return 1
}

server_bin=$(resolve_bin "${BBB_SERVER_WORLD_ORACLE_SERVER_BIN:-}" bbb-server) || {
  echo "server world oracle: bbb-server is unavailable" >&2
  exit 2
}
admin_bin=$(resolve_bin "${BBB_SERVER_WORLD_ORACLE_ADMIN_BIN:-}" bbb-admin) || {
  echo "server world oracle: bbb-admin is unavailable" >&2
  exit 2
}

require_fresh_bin "$repo_root" "$server_bin" "server world oracle" || exit 2
require_fresh_bin "$repo_root" "$admin_bin" "server world oracle" || exit 2

java_bin=$("$repo_root/scripts/vanilla" path 26.1 --kind java) || {
  echo "server world oracle: managed Java is unavailable; run scripts/vanilla prepare 26.1" >&2
  exit 2
}
server_jar=$("$repo_root/scripts/vanilla" path 26.1 --kind server-jar) || {
  echo "server world oracle: managed server jar is unavailable; run scripts/vanilla prepare 26.1" >&2
  exit 2
}
vanilla_root=$("$repo_root/scripts/vanilla" path 26.1 --kind root)
if [ ! -x "$java_bin" ] || [ ! -f "$server_jar" ]; then
  echo "server world oracle: managed Java/server jar is unavailable; run scripts/vanilla prepare 26.1" >&2
  exit 2
fi

# `--world` can only open a save that already exists, and the capture script is
# this repository's single owner of producing an official 26.1 save. The harness
# reuses it rather than authoring a second world-making path.
world_template=${BBB_SERVER_WORLD_ORACLE_WORLD_TEMPLATE:-}
if [ -z "$world_template" ]; then
  echo "server world oracle: capturing an official 26.1 world"
  # The capture script's other job is to re-pin the repository's own capture
  # manifest, and every capture writes a different one -- the official server is
  # not deterministic across runs. This harness only wants the world, so it
  # sends the manifest into the artifact directory instead of rewriting
  # scripts/data/server-official-capture-26.1.json.
  BBB_OFFICIAL_CAPTURE_ARTIFACT_DIR="$artifact_dir/capture" \
  BBB_OFFICIAL_CAPTURE_RECEIPT="$artifact_dir/capture/official-capture-receipt.json" \
    "$repo_root/scripts/capture-official-world-26.1.sh" >"$artifact_dir/capture.log" 2>&1 || {
      echo "server world oracle: the official world capture failed; log: $artifact_dir/capture.log" >&2
      exit 2
    }
  world_template="$artifact_dir/capture/server/world"
fi

scenarios=""
for scenario in ${1+"$@"}; do
  scenarios="$scenarios --scenario $scenario"
done

output=${BBB_SERVER_WORLD_ORACLE_OUTPUT:-}

status=0
# shellcheck disable=SC2086
"$python_runner" python "$helper" \
  --artifact-dir "$artifact_dir" \
  --server-bin "$server_bin" \
  --admin-bin "$admin_bin" \
  --vanilla-root "$vanilla_root" \
  --java-bin "$java_bin" \
  --server-jar "$server_jar" \
  --world-template "$world_template" \
  ${BBB_SERVER_WORLD_ORACLE_STEP_TICKS:+--step-ticks "$BBB_SERVER_WORLD_ORACLE_STEP_TICKS"} \
  ${output:+--output "$output"} \
  $scenarios || status=$?

echo "server world oracle: artifacts: $artifact_dir"
exit "$status"
