#!/bin/sh
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$repo_root/scripts/gate-stamp-lib.sh"
python_runner="$repo_root/scripts/python"
helper="$repo_root/scripts/platform_acceptance.py"
manifest="$repo_root/scripts/fixtures/platform-acceptance/linux-x86_64-wayland-vulkan.json"
lifecycle_manifest="$repo_root/scripts/fixtures/lifecycle-soak/manifest-26.1.json"

usage() {
  cat <<'EOF'
Usage: scripts/platform-acceptance-gate.sh

Environment:
  BBB_PLAYABLE_ACCEPT_EULA=1    Required EULA acknowledgement
  BBB_PLAYABLE_JAVA             Explicit Java executable
  BBB_VANILLA_ROOT              Optional managed Vanilla cache root
  BBB_PLATFORM_ARTIFACT_DIR     Result directory; default is fresh /tmp
  WAYLAND_DISPLAY               Maintained Wayland compositor socket

The gate requires a clean HEAD already recorded by the full gate. It performs
a fresh locked release build and never downloads Java, assets or a server jar.
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

run_python() {
  "$python_runner" python "$helper" "$@"
}

# The only declared release row is Linux x86_64 / Wayland / Vulkan, and the
# gate drives it from a uinput seat. Say so on another host rather than failing
# somewhere inside the compositor bring-up.
host_os=$(uname -s)
if [ "$host_os" != "Linux" ]; then
  echo "platform acceptance failed: the declared release row is Linux x86_64 / Wayland / Vulkan; this host is $host_os" >&2
  echo "macOS and Linux development share the full gate (scripts/cargo-dev.sh gate); the seat-driven release gates do not" >&2
  exit 2
fi

if [ -n "$(git -C "$repo_root" status --porcelain)" ]; then
  echo "platform acceptance failed: worktree must be clean" >&2
  exit 2
fi
start_head=$(git -C "$repo_root" rev-parse HEAD)
git_dir=$(git -C "$repo_root" rev-parse --absolute-git-dir)
stamp_dir="$git_dir/bbb"
if [ ! -f "$stamp_dir/last-gated" ] ||
  [ "$(cat "$stamp_dir/last-gated")" != "$start_head" ]
then
  echo "platform acceptance failed: current HEAD has not passed the full gate" >&2
  exit 2
fi
if [ ! -f "$stamp_dir/last-lifecycle-soak" ] ||
  [ ! -f "$stamp_dir/last-lifecycle-soak-report.json" ]
then
  echo "platform acceptance failed: lifecycle-soak stamp/report is missing" >&2
  exit 2
fi
lifecycle_head=$(cat "$stamp_dir/last-lifecycle-soak")
if ! git -C "$repo_root" merge-base --is-ancestor "$lifecycle_head" "$start_head"; then
  echo "platform acceptance failed: lifecycle-soak stamp is not a HEAD ancestor" >&2
  exit 2
fi
if [ "$(uname -s)" != Linux ] || [ "$(uname -m)" != x86_64 ]; then
  echo "platform acceptance failed: host must be Linux x86_64" >&2
  exit 2
fi
if [ -z "${WAYLAND_DISPLAY:-}" ]; then
  echo "platform acceptance failed: WAYLAND_DISPLAY is required" >&2
  exit 2
fi
if ! vanilla_root=$("$repo_root/scripts/vanilla" path 26.1 --kind root); then
  echo "platform acceptance failed: prepare the managed Vanilla 26.1 workspace" >&2
  exit 2
fi
export BBB_VANILLA_ROOT="$vanilla_root"
export BBB_PLAYABLE_INPUT_BACKEND=linux-release
if [ -z "${BBB_PLAYABLE_JAVA:-}" ]; then
  BBB_PLAYABLE_JAVA=$("$repo_root/scripts/vanilla" path 26.1 --kind java)
  export BBB_PLAYABLE_JAVA
fi
if ! "$repo_root/scripts/dev" doctor --profile linux-release; then
  echo "platform acceptance failed: Linux release host capabilities are incomplete" >&2
  exit 2
fi
if [ "${BBB_PLAYABLE_ACCEPT_EULA:-0}" != 1 ]; then
  echo "platform acceptance failed: BBB_PLAYABLE_ACCEPT_EULA=1 is required" >&2
  exit 2
fi

run_python validate-manifest --manifest "$manifest"

artifact_dir=${BBB_PLATFORM_ARTIFACT_DIR:-}
if [ -z "$artifact_dir" ]; then
  artifact_dir=$(mktemp -d "${TMPDIR:-/tmp}/bbb-linux-platform.XXXXXX")
else
  mkdir -p "$artifact_dir"
  artifact_dir=$(CDPATH= cd -- "$artifact_dir" && pwd)
fi
release_target=$(mktemp -d "${TMPDIR:-/tmp}/bbb-target-platform.XXXXXX")
release_bin="$release_target/release/bbb-native"
build_log="$artifact_dir/release-build.log"
build_info="$artifact_dir/build-info.json"
fixture_log="$artifact_dir/fixture.log"
focused_dir="$artifact_dir/focused-tests"
playable_dir="$artifact_dir/playable-live"
report="$artifact_dir/report.json"
mkdir -p "$focused_dir" "$playable_dir"

echo "platform acceptance: fresh locked release build: $release_target"
if ! CARGO_TARGET_DIR="$release_target" \
  "$repo_root/scripts/cargo-dev.sh" cargo build --locked --release -p bbb-native \
  >"$build_log" 2>&1
then
  echo "platform acceptance failed: release build; see $build_log" >&2
  exit 1
fi
"$release_bin" --build-info >"$build_info"

for focused_test in \
  local_input_projection_includes_focus_keys_and_mouse_delta \
  saturated_mailbox_never_drops_key_mouse_or_control_releases \
  shift_f3_i_copies_block_recreate_command_to_clipboard_and_reports_feedback \
  dialog_ime_preedit_maps_utf8_cursor_ranges_and_blinks_at_vanilla_period \
  sign_editor_done_mouse_and_ime_preedit_share_one_submit_path \
  network_terminal_shutdown_is_idempotent_and_requests_disconnect
do
  echo "platform acceptance: focused test: $focused_test"
  if ! CARGO_TARGET_DIR=/tmp/bbb-target-main \
    "$repo_root/scripts/cargo-dev.sh" test -p bbb-native "$focused_test" \
    >"$focused_dir/$focused_test.log" 2>&1
  then
    echo "platform acceptance failed: focused test $focused_test" >&2
    echo "platform acceptance evidence: $focused_dir/$focused_test.log" >&2
    exit 1
  fi
done

if ! "$repo_root/scripts/playable-gate.sh" fixture >"$fixture_log" 2>&1; then
  echo "platform acceptance failed: checked-in playable fixture" >&2
  echo "platform acceptance evidence: $fixture_log" >&2
  exit 1
fi

echo "platform acceptance: release live Wayland/Vulkan session: $playable_dir"
if ! DISPLAY= \
  RUST_LOG=bbb_net=debug,bbb_native=info,bbb_control=info,bbb_renderer=info \
  BBB_PLAYABLE_CLIENT_BIN="$release_bin" \
  BBB_PLAYABLE_ARTIFACT_DIR="$playable_dir" \
  BBB_PLAYABLE_TERMINATION_MODE=quit \
  BBB_LIFECYCLE_SOAK_MANIFEST="$lifecycle_manifest" \
  "$repo_root/scripts/playable-gate.sh" live
then
  echo "platform acceptance failed: release live playable" >&2
  echo "platform acceptance evidence: $playable_dir" >&2
  exit 1
fi

run_python report \
  --manifest "$manifest" \
  --head "$start_head" \
  --build-info "$build_info" \
  --release-binary "$release_bin" \
  --playable-artifact-dir "$playable_dir" \
  --lifecycle-report "$stamp_dir/last-lifecycle-soak-report.json" \
  --lifecycle-stamp "$stamp_dir/last-lifecycle-soak" \
  --focused-receipt-dir "$focused_dir" \
  --output "$report"

if [ "$(git -C "$repo_root" rev-parse HEAD)" != "$start_head" ] ||
  [ -n "$(git -C "$repo_root" status --porcelain)" ]
then
  echo "platform acceptance failed: repository changed during the gate" >&2
  exit 1
fi
cp "$report" "$stamp_dir/last-linux-platform-report.json"
(cd "$repo_root" && record_gate_stamp \
  linux-platform-log.tsv last-linux-platform passed "report:$report")
echo "platform acceptance: report: $report"
