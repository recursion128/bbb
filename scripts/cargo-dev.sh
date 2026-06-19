#!/bin/sh
set -eu

usage() {
  cat <<'EOF'
Usage:
  scripts/cargo-dev.sh test [cargo test args...]
  scripts/cargo-dev.sh fast-test [cargo test args...]
  scripts/cargo-dev.sh timings [cargo test args...]
  scripts/cargo-dev.sh timings-clean <target-suffix> [cargo test args...]
  scripts/cargo-dev.sh gate
  scripts/cargo-dev.sh size
  scripts/cargo-dev.sh clean-target <target-suffix|/tmp/bbb-target-...>
  scripts/cargo-dev.sh env
  scripts/cargo-dev.sh sccache-status
  scripts/cargo-dev.sh sccache-zero-stats
  scripts/cargo-dev.sh cargo [cargo args...]

Environment:
  BBB_CARGO_TARGET_NAME   Target cache suffix, default: main
  CARGO_TARGET_DIR        Explicit target dir override
  BBB_USE_SCCACHE=1       Use sccache only if it is installed

Examples:
  scripts/cargo-dev.sh test -p bbb-world command_tree
  scripts/cargo-dev.sh fast-test -p bbb-world command_tree
  BBB_CARGO_TARGET_NAME=world scripts/cargo-dev.sh test -p bbb-world command_tree
  scripts/cargo-dev.sh timings --workspace --timings
  BBB_USE_SCCACHE=1 scripts/cargo-dev.sh timings-clean sccache-clean-20260619 --workspace --timings
  BBB_USE_SCCACHE=1 BBB_CARGO_TARGET_NAME=world scripts/cargo-dev.sh test -p bbb-world command_tree
  scripts/cargo-dev.sh timings-clean clean-baseline-20260619 --workspace --timings
  scripts/cargo-dev.sh clean-target clean-baseline-20260619
  scripts/cargo-dev.sh gate
EOF
}

target_dir_for_arg() {
  arg="$1"
  case "$arg" in
    "")
      echo "target suffix must not be empty" >&2
      exit 2
      ;;
    /tmp/bbb-target-*)
      printf '%s\n' "$arg"
      ;;
    /*|*/*)
      echo "target suffix must be a name or /tmp/bbb-target-... path: $arg" >&2
      exit 2
      ;;
    *[!A-Za-z0-9_-]*)
      echo "target suffix may contain only letters, digits, '_' and '-': $arg" >&2
      exit 2
      ;;
    *)
      printf '/tmp/bbb-target-%s\n' "$arg"
      ;;
  esac
}

ensure_safe_target_dir() {
  dir="$1"
  case "$dir" in
    /tmp/bbb-target-*)
      ;;
    *)
      echo "refusing to operate outside /tmp/bbb-target-*: $dir" >&2
      exit 2
      ;;
  esac
}

if [ "$#" -eq 0 ]; then
  usage >&2
  exit 2
fi

target_name="${BBB_CARGO_TARGET_NAME:-main}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/bbb-target-${target_name}}"

if [ "${BBB_USE_SCCACHE:-0}" = "1" ] && [ -z "${RUSTC_WRAPPER:-}" ]; then
  if ! command -v sccache >/dev/null 2>&1; then
    echo "BBB_USE_SCCACHE=1 was set, but sccache is not on PATH" >&2
    exit 2
  fi
  export RUSTC_WRAPPER=sccache
fi

cmd="$1"
shift

case "$cmd" in
  test)
    exec cargo test "$@"
    ;;
  fast-test)
    exec cargo test --profile fast-test "$@"
    ;;
  timings)
    if [ "$#" -eq 0 ]; then
      set -- --workspace --timings
    fi
    exec /usr/bin/time -p cargo test "$@"
    ;;
  timings-clean)
    if [ "$#" -lt 1 ]; then
      echo "timings-clean requires a disposable target suffix" >&2
      usage >&2
      exit 2
    fi
    clean_target="$(target_dir_for_arg "$1")"
    ensure_safe_target_dir "$clean_target"
    shift
    if [ -e "$clean_target" ]; then
      echo "refusing to use existing clean target: $clean_target" >&2
      echo "remove it explicitly with: scripts/cargo-dev.sh clean-target ${clean_target}" >&2
      exit 2
    fi
    export CARGO_TARGET_DIR="$clean_target"
    if [ "$#" -eq 0 ]; then
      set -- --workspace --timings
    fi
    echo "CARGO_TARGET_DIR=${CARGO_TARGET_DIR}"
    exec /usr/bin/time -p cargo test "$@"
    ;;
  gate)
    export CARGO_TARGET_DIR=/tmp/bbb-target-main
    cargo fmt --check
    git diff --check
    cargo test --workspace
    ;;
  size)
    found=0
    for dir in /tmp/bbb-target-*; do
      if [ -d "$dir" ]; then
        found=1
        du -sh "$dir"
      fi
    done
    if [ "$found" = "0" ]; then
      echo "no /tmp/bbb-target-* directories found"
    fi
    ;;
  clean-target)
    if [ "$#" -ne 1 ]; then
      echo "clean-target requires exactly one target suffix or /tmp/bbb-target-... path" >&2
      usage >&2
      exit 2
    fi
    clean_target="$(target_dir_for_arg "$1")"
    ensure_safe_target_dir "$clean_target"
    if [ -d "$clean_target" ]; then
      rm -rf "$clean_target"
      echo "removed $clean_target"
    else
      echo "not found: $clean_target"
    fi
    ;;
  env)
    echo "CARGO_TARGET_DIR=${CARGO_TARGET_DIR}"
    echo "RUSTC_WRAPPER=${RUSTC_WRAPPER:-}"
    cargo --version
    rustc --version
    ;;
  sccache-status)
    if ! command -v sccache >/dev/null 2>&1; then
      echo "sccache is not on PATH"
      exit 0
    fi
    sccache --version
    sccache --show-stats || true
    ;;
  sccache-zero-stats)
    if ! command -v sccache >/dev/null 2>&1; then
      echo "sccache is not on PATH" >&2
      exit 2
    fi
    sccache --zero-stats
    ;;
  cargo)
    exec cargo "$@"
    ;;
  -h|--help|help)
    usage
    ;;
  *)
    echo "unknown command: ${cmd}" >&2
    usage >&2
    exit 2
    ;;
esac
