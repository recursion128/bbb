#!/bin/sh
set -eu

usage() {
  cat <<'EOF'
Usage:
  scripts/cargo-dev.sh test [cargo test args...]
  scripts/cargo-dev.sh fast-test [cargo test args...]
  scripts/cargo-dev.sh timings [cargo test args...]
  scripts/cargo-dev.sh timings-clean <target-suffix> [cargo test args...]
  scripts/cargo-dev.sh sccache-eval <run-suffix> [focused cargo test args...]
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
  scripts/cargo-dev.sh sccache-eval 20260619 -p bbb-world command_tree --quiet
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
      suffix="${arg#/tmp/bbb-target-}"
      validate_target_suffix "$suffix"
      printf '/tmp/bbb-target-%s\n' "$suffix"
      ;;
    /*|*/*)
      echo "target suffix must be a name or /tmp/bbb-target-... path: $arg" >&2
      exit 2
      ;;
    *)
      validate_target_suffix "$arg"
      printf '/tmp/bbb-target-%s\n' "$arg"
      ;;
  esac
}

validate_target_suffix() {
  suffix="$1"
  case "$suffix" in
    ""|*[!A-Za-z0-9_-]*)
      echo "target suffix may contain only letters, digits, '_' and '-': $suffix" >&2
      exit 2
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

refuse_existing_target_dir() {
  dir="$1"
  if [ -e "$dir" ]; then
    echo "refusing to use existing measurement target: $dir" >&2
    echo "remove it explicitly with: scripts/cargo-dev.sh clean-target ${dir}" >&2
    exit 2
  fi
}

require_sccache() {
  if ! command -v sccache >/dev/null 2>&1; then
    echo "sccache is not on PATH" >&2
    exit 2
  fi
}

show_sccache_stats() {
  sccache --show-stats || true
}

run_sccache_eval() {
  if [ "$#" -lt 1 ]; then
    echo "sccache-eval requires a run suffix" >&2
    usage >&2
    exit 2
  fi

  run_suffix="$1"
  shift
  case "$run_suffix" in
    ""|*[!A-Za-z0-9_-]*)
      echo "run suffix may contain only letters, digits, '_' and '-': $run_suffix" >&2
      exit 2
      ;;
  esac

  if [ "$#" -eq 0 ]; then
    set -- -p bbb-world command_tree --quiet
  fi

  require_sccache

  clean_target="$(target_dir_for_arg "sccache-clean-${run_suffix}")"
  worker_target="$(target_dir_for_arg "sccache-worker-${run_suffix}")"
  nosccache_worker_target="$(target_dir_for_arg "nosccache-worker-${run_suffix}")"
  ensure_safe_target_dir "$clean_target"
  ensure_safe_target_dir "$worker_target"
  ensure_safe_target_dir "$nosccache_worker_target"
  refuse_existing_target_dir "$clean_target"
  refuse_existing_target_dir "$worker_target"
  refuse_existing_target_dir "$nosccache_worker_target"

  cat <<EOF
sccache evaluation run: $run_suffix
focused cargo test args: $*
clean target: $clean_target
sccache worker target: $worker_target
no-sccache worker target: $nosccache_worker_target
warm main target: /tmp/bbb-target-main
EOF

  echo
  echo "== clean full workspace with sccache =="
  sccache --zero-stats
  (
    export RUSTC_WRAPPER=sccache
    export CARGO_TARGET_DIR="$clean_target"
    /usr/bin/time -p cargo test --workspace --timings --quiet
  )
  du -sh "$clean_target"
  show_sccache_stats

  echo
  echo "== new worker focused test with sccache =="
  sccache --zero-stats
  (
    export RUSTC_WRAPPER=sccache
    export CARGO_TARGET_DIR="$worker_target"
    /usr/bin/time -p cargo test "$@"
  )
  du -sh "$worker_target"
  show_sccache_stats

  echo
  echo "== new worker focused test without sccache =="
  (
    unset RUSTC_WRAPPER
    export CARGO_TARGET_DIR="$nosccache_worker_target"
    /usr/bin/time -p cargo test "$@"
  )
  du -sh "$nosccache_worker_target"

  echo
  echo "== warm focused default profile with sccache on main target =="
  sccache --zero-stats
  (
    export RUSTC_WRAPPER=sccache
    export CARGO_TARGET_DIR=/tmp/bbb-target-main
    /usr/bin/time -p cargo test "$@"
  )
  show_sccache_stats
}

if [ "$#" -eq 0 ]; then
  usage >&2
  exit 2
fi

target_name="${BBB_CARGO_TARGET_NAME:-main}"
default_target_dir="$(target_dir_for_arg "$target_name")"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$default_target_dir}"

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
  sccache-eval)
    run_sccache_eval "$@"
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
