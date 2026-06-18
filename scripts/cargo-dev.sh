#!/bin/sh
set -eu

usage() {
  cat <<'EOF'
Usage:
  scripts/cargo-dev.sh test [cargo test args...]
  scripts/cargo-dev.sh fast-test [cargo test args...]
  scripts/cargo-dev.sh timings [cargo test args...]
  scripts/cargo-dev.sh gate
  scripts/cargo-dev.sh size
  scripts/cargo-dev.sh env
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
  scripts/cargo-dev.sh gate
EOF
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
  gate)
    cargo fmt --check
    git diff --check
    cargo test --workspace
    ;;
  size)
    found=0
    for dir in /tmp/bbb-target*; do
      if [ -d "$dir" ]; then
        found=1
        du -sh "$dir"
      fi
    done
    if [ "$found" = "0" ]; then
      echo "no /tmp/bbb-target* directories found"
    fi
    ;;
  env)
    echo "CARGO_TARGET_DIR=${CARGO_TARGET_DIR}"
    echo "RUSTC_WRAPPER=${RUSTC_WRAPPER:-}"
    cargo --version
    rustc --version
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
