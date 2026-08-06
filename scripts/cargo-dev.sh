#!/bin/sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
. "$script_dir/gate-stamp-lib.sh"

usage() {
  cat <<'EOF'
Usage:
  scripts/cargo-dev.sh focused [cargo test args...]
  scripts/cargo-dev.sh default [cargo test args...]
  scripts/cargo-dev.sh test [cargo test args...]
  scripts/cargo-dev.sh fast-test [cargo test args...]
  scripts/cargo-dev.sh check [cargo check args...]
  scripts/cargo-dev.sh fmt [cargo fmt args...]
  scripts/cargo-dev.sh timings [cargo test args...]
  scripts/cargo-dev.sh timings-clean <target-suffix> [cargo test args...]
  scripts/cargo-dev.sh sccache-eval <run-suffix> [focused cargo test args...]
  scripts/cargo-dev.sh cross-check [--target <triple>]
  scripts/cargo-dev.sh gate
  scripts/cargo-dev.sh gate-scripts-tests-status
  scripts/cargo-dev.sh gate-cross-check-status
  scripts/cargo-dev.sh batch-gate [<good-commit>..<bad-commit>]
  scripts/cargo-dev.sh size
  scripts/cargo-dev.sh prune [--force]
  scripts/cargo-dev.sh clean-target <target-suffix|/tmp/bbb-target-...>
  scripts/cargo-dev.sh env
  scripts/cargo-dev.sh sccache-status
  scripts/cargo-dev.sh sccache-zero-stats
  scripts/cargo-dev.sh cargo [cargo args...]

Environment:
  BBB_CARGO_TARGET_NAME   Target cache suffix, default: main
  CARGO_TARGET_DIR        Explicit target dir override
  BBB_BATCH_GATE_TARGET_NAME  Batch-gate cache suffix, default: batch-gate
  BBB_PRUNE_TARGET_ROOT   Prune scan root, default: /tmp (used by tests)
  BBB_GATE_FULL=1         Run every gate step regardless of changed paths
  BBB_USE_SCCACHE=1       Use sccache only if it is installed

Examples:
  scripts/cargo-dev.sh focused -p bbb-world command_tree
  scripts/cargo-dev.sh default -p bbb-world command_tree
  scripts/cargo-dev.sh test -p bbb-world command_tree
  scripts/cargo-dev.sh fast-test -p bbb-world command_tree
  BBB_CARGO_TARGET_NAME=world scripts/cargo-dev.sh focused -p bbb-world command_tree
  scripts/cargo-dev.sh check -p bbb-world
  scripts/cargo-dev.sh timings --workspace --timings
  BBB_USE_SCCACHE=1 scripts/cargo-dev.sh timings-clean sccache-clean-20260619 --workspace --timings
  scripts/cargo-dev.sh sccache-eval 20260619 -p bbb-world command_tree --quiet
  BBB_USE_SCCACHE=1 BBB_CARGO_TARGET_NAME=world scripts/cargo-dev.sh focused -p bbb-world command_tree
  scripts/cargo-dev.sh timings-clean clean-baseline-20260619 --workspace --timings
  scripts/cargo-dev.sh clean-target clean-baseline-20260619
  scripts/cargo-dev.sh cross-check
  scripts/cargo-dev.sh cross-check --target x86_64-apple-darwin
  scripts/cargo-dev.sh prune
  scripts/cargo-dev.sh prune --force
  scripts/cargo-dev.sh gate
  scripts/cargo-dev.sh batch-gate master..HEAD
  scripts/cargo-dev.sh batch-gate

A passing gate appends per-step timings to <git-dir>/bbb/gate-log.tsv and, on
a clean tree, records HEAD in <git-dir>/bbb/last-gated. batch-gate without a
range defaults to <last-gated>..HEAD.
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

run_timed_cargo() {
  if [ -x /usr/bin/time ]; then
    /usr/bin/time -p cargo "$@"
  elif command -v bash >/dev/null 2>&1; then
    bash -c 'time -p cargo "$@"' cargo "$@"
  else
    echo "timing requires /usr/bin/time or a bash shell with time support" >&2
    exit 2
  fi
}

copy_timing_report() {
  target="$1"
  suffix="$2"
  timing_dir="$target/cargo-timings"
  if [ ! -d "$timing_dir" ]; then
    echo "timing report not found: $timing_dir"
    return
  fi

  timing_report="$(
    find "$timing_dir" -type f -name 'cargo-timing*.html' ! -name 'cargo-timing.html' |
      sort |
      tail -n 1
  )"
  if [ -z "$timing_report" ] && [ -f "$timing_dir/cargo-timing.html" ]; then
    timing_report="$timing_dir/cargo-timing.html"
  fi
  if [ -z "$timing_report" ]; then
    echo "timing report not found: $timing_dir"
    return
  fi

  mkdir -p /tmp/bbb-cargo-timings
  copied_report="/tmp/bbb-cargo-timings/cargo-timing-${suffix}.html"
  cp "$timing_report" "$copied_report"
  echo "timing report copied: $copied_report"
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
    run_timed_cargo test --workspace --timings --quiet
  )
  du -sh "$clean_target"
  copy_timing_report "$clean_target" "sccache-clean-${run_suffix}"
  show_sccache_stats

  echo
  echo "== new worker target focused test with sccache =="
  sccache --zero-stats
  (
    export RUSTC_WRAPPER=sccache
    export CARGO_TARGET_DIR="$worker_target"
    run_timed_cargo test "$@"
  )
  du -sh "$worker_target"
  show_sccache_stats

  echo
  echo "== new worker target focused test without sccache =="
  (
    unset RUSTC_WRAPPER
    export CARGO_TARGET_DIR="$nosccache_worker_target"
    run_timed_cargo test "$@"
  )
  du -sh "$nosccache_worker_target"

  echo
  echo "== warm focused default profile with sccache on main target =="
  sccache --zero-stats
  (
    export RUSTC_WRAPPER=sccache
    export CARGO_TARGET_DIR=/tmp/bbb-target-main
    run_timed_cargo test "$@"
  )
  show_sccache_stats

  cat <<EOF

Disposable measurement targets were kept for review:
  $clean_target
  $worker_target
  $nosccache_worker_target

Clean them after recording results with:
  scripts/cargo-dev.sh clean-target ${clean_target}
  scripts/cargo-dev.sh clean-target ${worker_target}
  scripts/cargo-dev.sh clean-target ${nosccache_worker_target}
EOF
}

# Stable long-lived caches that prune never deletes. Keep in sync with the
# keep-list documented in docs/wiki/development/build-and-cache.md.
PRUNE_KEEP_LIST="main batch-gate playable world renderer net native protocol pack control audio item-model render-types"

is_prune_keep() {
  name="$1"
  for keep in $PRUNE_KEEP_LIST; do
    if [ "$name" = "$keep" ]; then
      return 0
    fi
  done
  return 1
}

prune_safe_dir() {
  root="$1"
  dir="$2"
  case "$dir" in
    "$root"/bbb-target-*)
      ;;
    *)
      echo "refusing to remove path outside $root/bbb-target-*: $dir" >&2
      exit 2
      ;;
  esac
}

run_prune() {
  force=0
  while [ "$#" -gt 0 ]; do
    case "$1" in
      --force|-f)
        force=1
        ;;
      *)
        echo "unknown prune option: $1" >&2
        usage >&2
        exit 2
        ;;
    esac
    shift
  done

  prune_root="${BBB_PRUNE_TARGET_ROOT:-/tmp}"
  case "$prune_root" in
    /*)
      ;;
    *)
      echo "BBB_PRUNE_TARGET_ROOT must be an absolute path: $prune_root" >&2
      exit 2
      ;;
  esac
  if [ ! -d "$prune_root" ]; then
    echo "prune target root not found: $prune_root" >&2
    exit 2
  fi

  repo_root="$(git rev-parse --show-toplevel 2>/dev/null)" || {
    echo "prune must run inside a Git worktree" >&2
    exit 2
  }

  delete_list=""
  delete_count=0
  found=0
  for dir in "$prune_root"/bbb-target-*; do
    [ -d "$dir" ] || continue
    found=1
    name="${dir##*/bbb-target-}"
    size="$(du -sh "$dir" 2>/dev/null | cut -f1)"
    if is_prune_keep "$name"; then
      printf 'keep   %-8s %s\n' "$size" "$dir"
    else
      printf 'delete %-8s %s\n' "$size" "$dir"
      delete_list="$delete_list $dir"
      delete_count=$((delete_count + 1))
    fi
  done
  if [ "$found" = "0" ]; then
    echo "no bbb-target-* directories found under $prune_root"
  fi

  repo_target="$repo_root/target"
  if [ -d "$repo_target" ]; then
    size="$(du -sh "$repo_target" 2>/dev/null | cut -f1)"
    printf 'delete %-8s %s (repo-local target; ignored by git)\n' "$size" "$repo_target"
    delete_list="$delete_list $repo_target"
    delete_count=$((delete_count + 1))
  fi

  if [ "$delete_count" = "0" ]; then
    echo "prune: nothing to delete"
    exit 0
  fi

  if [ "$force" = "0" ]; then
    echo
    echo "prune: dry-run; $delete_count directories would be deleted"
    echo "rerun with --force to delete them"
    exit 0
  fi

  for dir in $delete_list; do
    if [ "$dir" != "$repo_target" ]; then
      prune_safe_dir "$prune_root" "$dir"
    fi
    rm -rf "$dir"
    echo "removed $dir"
  done
  echo "prune: removed $delete_count directories"
}

gate_step_total=7
gate_step_index=0
gate_step_timings=""

run_gate_step() {
  step_key="$1"
  label="$2"
  shift 2
  gate_step_index=$((gate_step_index + 1))
  echo "== [$gate_step_index/$gate_step_total] $label =="
  step_start=$(date +%s)
  if "$@"; then
    step_end=$(date +%s)
    step_seconds=$((step_end - step_start))
    echo "   $label: ${step_seconds}s"
    gate_step_timings="$gate_step_timings,$step_key:${step_seconds}s"
  else
    status=$?
    echo "gate step failed: $label (exit $status)" >&2
    exit "$status"
  fi
}

# Change-aware gate steps validate one part of the tree, so they only need to
# run when the work under gate touched it. What the gate certifies is everything
# since the last gated commit, so that is the base on a clean tree: gating a
# batch tip against `HEAD^` would only see the last commit and silently skip a
# step that an earlier commit in the same batch made necessary. A dirty tree is
# gated against HEAD, and a clean tree with no usable stamp falls back to the
# parent commit.
# `$1` is the path pattern, `$2` the label used in the run/skip line.
gate_change_base() {
  git_dir=$(git rev-parse --absolute-git-dir)
  stamp="$git_dir/bbb/last-gated"
  [ -r "$stamp" ] || return 1
  base=$(cat "$stamp")
  [ -n "$base" ] || return 1
  git rev-parse --verify --quiet "$base^{commit}" >/dev/null 2>&1 || return 1
  git merge-base --is-ancestor "$base" HEAD || return 1
  [ "$(git rev-parse "$base")" != "$(git rev-parse HEAD)" ] || return 1
  printf '%s\n' "$base"
}

gate_change_status() {
  pattern="$1"
  label="$2"
  if [ "${BBB_GATE_FULL:-0}" = "1" ]; then
    echo "run: BBB_GATE_FULL=1"
    return 0
  fi
  if ! git rev-parse --verify --quiet HEAD >/dev/null 2>&1; then
    echo "run: no HEAD commit to diff against"
    return 0
  fi
  if [ -n "$(git status --porcelain | head -n 1)" ]; then
    changed="$(
      git diff --name-only HEAD
      git ls-files --others --exclude-standard
    )"
    scope="working tree vs HEAD"
  elif change_base=$(gate_change_base); then
    changed="$(git diff --name-only "$change_base" HEAD)"
    scope="HEAD vs last-gated $(git rev-parse --short "$change_base")"
  else
    if ! git rev-parse --verify --quiet 'HEAD^' >/dev/null 2>&1; then
      echo "run: clean tree without a parent commit"
      return 0
    fi
    changed="$(git diff --name-only 'HEAD^' HEAD)"
    scope="HEAD vs HEAD^"
  fi
  if printf '%s\n' "$changed" | grep -Eq "$pattern"; then
    echo "run: $label changed ($scope)"
  else
    echo "skip: no $label change ($scope)"
  fi
}

# The scripts python suite only validates scripts/.
gate_scripts_tests_status() {
  gate_change_status '^scripts/' 'scripts/'
}

# The second-platform check only sees Rust sources and the manifests that
# decide which crates it can reach.
gate_cross_check_status() {
  gate_change_status '^(crates/|Cargo\.toml$|Cargo\.lock$)' 'crate source'
}

run_gate() {
  gate_start=$(date +%s)
  gate_step_timings=""
  if [ -n "${BBB_GATE_TEST_COMMAND:-}" ]; then
    if [ "${BBB_GATE_TEST_MODE:-0}" != "1" ]; then
      echo "BBB_GATE_TEST_COMMAND is reserved for isolated script tests" >&2
      exit 2
    fi
    gate_step_total=1
    run_gate_step test-command "gate test command" sh -c "$BBB_GATE_TEST_COMMAND"
  else
    run_gate_step docs-lint "docs-lint" scripts/docs-lint.sh
    run_gate_step fmt "cargo fmt --all --check" cargo fmt --all --check
    scripts_tests_status="$(gate_scripts_tests_status)"
    case "$scripts_tests_status" in
      run:*)
        run_gate_step scripts-tests "scripts python tests" \
          scripts/vanilla-source-tools.sh python -m unittest discover -s scripts/tests
        ;;
      *)
        gate_step_index=$((gate_step_index + 1))
        echo "== [$gate_step_index/$gate_step_total] scripts python tests =="
        echo "   skipped: ${scripts_tests_status#skip: } (BBB_GATE_FULL=1 forces)"
        gate_step_timings="$gate_step_timings,scripts-tests:skip"
        ;;
    esac
    run_gate_step diff-check "git diff --check" git diff --check
    run_gate_step check "cargo check --workspace" cargo check --workspace
    cross_check_status="$(gate_cross_check_status)"
    case "$cross_check_status" in
      run:*)
        run_gate_step cross-check "second-platform cross-check" \
          python3 scripts/cross_check.py
        ;;
      *)
        gate_step_index=$((gate_step_index + 1))
        echo "== [$gate_step_index/$gate_step_total] second-platform cross-check =="
        echo "   skipped: ${cross_check_status#skip: } (BBB_GATE_FULL=1 forces)"
        gate_step_timings="$gate_step_timings,cross-check:skip"
        ;;
    esac
    run_gate_step test "cargo test --workspace" cargo test --workspace
  fi
  gate_end=$(date +%s)
  gate_total=$((gate_end - gate_start))
  if [ -z "$(git status --porcelain 2>/dev/null | head -n 1)" ]; then
    gate_tree_state=clean
    gate_pointer=last-gated
  else
    gate_tree_state=dirty
    gate_pointer=""
  fi
  record_gate_stamp gate-log.tsv "$gate_pointer" "$gate_tree_state" \
    "total:${gate_total}s${gate_step_timings}"
  echo "gate: all $gate_step_total steps passed in ${gate_total}s"
}

run_batch_gate() {
  if [ "$#" -gt 1 ]; then
    echo "batch-gate accepts at most one <good-commit>..<bad-commit> range" >&2
    usage >&2
    exit 2
  fi

  if [ "$#" -eq 1 ]; then
    range="$1"
  else
    stamp_git_dir=$(git rev-parse --absolute-git-dir 2>/dev/null) || {
      echo "batch-gate must run inside a Git worktree" >&2
      exit 2
    }
    last_gated_file="$stamp_git_dir/bbb/last-gated"
    if [ ! -f "$last_gated_file" ]; then
      echo "batch-gate found no recorded last-gated commit ($last_gated_file); pass an explicit <good-commit>..<bad-commit> range" >&2
      exit 2
    fi
    range="$(cat "$last_gated_file")..HEAD"
    echo "batch-gate: defaulting to last-gated range: $range"
  fi
  case "$range" in
    *...*)
      echo "batch-gate requires a two-dot range, not a symmetric three-dot range: $range" >&2
      exit 2
      ;;
    *..*)
      good_ref="${range%%..*}"
      bad_ref="${range#*..}"
      ;;
    *)
      echo "batch-gate range must have the form <good-commit>..<bad-commit>: $range" >&2
      exit 2
      ;;
  esac
  if [ -z "$good_ref" ] || [ -z "$bad_ref" ]; then
    echo "batch-gate range endpoints must not be empty: $range" >&2
    exit 2
  fi

  repo_root="$(git rev-parse --show-toplevel 2>/dev/null)" || {
    echo "batch-gate must run inside a Git worktree" >&2
    exit 2
  }
  good_commit="$(git rev-parse --verify "${good_ref}^{commit}" 2>/dev/null)" || {
    echo "batch-gate cannot resolve good endpoint: $good_ref" >&2
    exit 2
  }
  bad_commit="$(git rev-parse --verify "${bad_ref}^{commit}" 2>/dev/null)" || {
    echo "batch-gate cannot resolve bad endpoint: $bad_ref" >&2
    exit 2
  }
  if [ "$good_commit" = "$bad_commit" ]; then
    echo "batch-gate endpoints resolve to the same commit: $good_commit" >&2
    exit 2
  fi
  if ! git merge-base --is-ancestor "$good_commit" "$bad_commit"; then
    echo "batch-gate good endpoint is not an ancestor of bad endpoint" >&2
    echo "good: $good_commit" >&2
    echo "bad:  $bad_commit" >&2
    exit 2
  fi
  if ! git rev-list --first-parent "$bad_commit" | grep -Fx "$good_commit" >/dev/null; then
    echo "batch-gate good endpoint is not on the bad endpoint's first-parent chain" >&2
    echo "use a linear/first-parent range so 'first bad commit' is unambiguous" >&2
    exit 2
  fi
  if [ -n "${BBB_BATCH_GATE_COMMAND:-}" ]; then
    if [ "${BBB_BATCH_GATE_TEST_MODE:-0}" != "1" ]; then
      echo "BBB_BATCH_GATE_COMMAND is reserved for isolated script tests" >&2
      exit 2
    fi
    batch_gate_command="$BBB_BATCH_GATE_COMMAND"
  else
    batch_gate_command="scripts/cargo-dev.sh gate"
  fi
  batch_target_name="${BBB_BATCH_GATE_TARGET_NAME:-batch-gate}"
  batch_target_dir="$(target_dir_for_arg "$batch_target_name")"

  batch_root="$(mktemp -d "${TMPDIR:-/tmp}/bbb-batch-gate.XXXXXX")"
  batch_worktree="$batch_root/worktree"
  batch_logs="${BBB_BATCH_GATE_LOG_DIR:-$(mktemp -d "${TMPDIR:-/tmp}/bbb-batch-gate-logs.XXXXXX")}"
  commits_file="$batch_root/commits"
  results_dir="$batch_root/results"
  case "$batch_logs" in
    /*)
      ;;
    *)
      rm -rf "$batch_root"
      echo "BBB_BATCH_GATE_LOG_DIR must be an absolute path" >&2
      exit 2
      ;;
  esac
  mkdir -p "$batch_logs" "$results_dir"

  cleanup_batch_gate() {
    if [ -d "$batch_worktree" ]; then
      git -C "$repo_root" worktree remove --force "$batch_worktree" >/dev/null 2>&1 || true
    fi
    rm -rf "$batch_root"
  }
  trap cleanup_batch_gate EXIT
  trap 'exit 129' HUP
  trap 'exit 130' INT
  trap 'exit 143' TERM

  printf '%s\n' "$good_commit" >"$commits_file"
  git rev-list --first-parent --reverse "$good_commit..$bad_commit" >>"$commits_file"
  commit_count="$(wc -l <"$commits_file" | tr -d ' ')"
  last_index=$((commit_count - 1))

  echo "batch-gate range: $good_commit..$bad_commit"
  echo "batch-gate first-parent commits: $last_index"
  echo "batch-gate temporary worktree: $batch_worktree"
  echo "batch-gate logs: $batch_logs"
  echo "batch-gate target: $batch_target_dir"
  echo "batch-gate authoritative command: $batch_gate_command"
  git -C "$repo_root" worktree add --detach --quiet "$batch_worktree" "$bad_commit"

  gate_result=0
  run_batch_gate_commit() {
    commit="$1"
    short_commit="$(git rev-parse --short=12 "$commit")"
    result_file="$results_dir/$commit"
    log_file="$batch_logs/$short_commit.log"
    if [ -f "$result_file" ]; then
      gate_result="$(cat "$result_file")"
      return 0
    fi

    git -C "$batch_worktree" checkout --detach --quiet "$commit"
    subject="$(git -C "$batch_worktree" log -1 --format=%s)"
    {
      echo "commit: $commit"
      echo "subject: $subject"
      echo "command: $batch_gate_command"
      echo
      (
        cd "$batch_worktree"
        export BBB_CARGO_TARGET_NAME="$batch_target_name"
        export CARGO_TARGET_DIR="$batch_target_dir"
        if [ -n "${BBB_BATCH_GATE_COMMAND:-}" ]; then
          export BBB_BATCH_GATE_COMMIT="$commit"
          export BBB_BATCH_GATE_WORKTREE="$batch_worktree"
          sh -c "$batch_gate_command"
        else
          sh scripts/cargo-dev.sh gate
        fi
      )
    } >"$log_file" 2>&1 && gate_result=0 || gate_result=$?
    printf '%s\n' "$gate_result" >"$result_file"
    if [ "$gate_result" -eq 0 ]; then
      echo "batch-gate: PASS $short_commit $subject (log: $log_file)"
    else
      echo "batch-gate: FAIL $short_commit $subject (exit $gate_result; log: $log_file)"
    fi
  }

  tip_commit="$(sed -n "${commit_count}p" "$commits_file")"
  run_batch_gate_commit "$tip_commit"
  if [ "$gate_result" -eq 0 ]; then
    echo "batch-gate: range tip passed; no bisection needed"
    exit 0
  fi

  baseline_commit="$(sed -n '1p' "$commits_file")"
  run_batch_gate_commit "$baseline_commit"
  if [ "$gate_result" -ne 0 ]; then
    echo "batch-gate: baseline commit also fails; cannot attribute a regression within the range" >&2
    exit 1
  fi

  low=0
  high="$last_index"
  while [ $((high - low)) -gt 1 ]; do
    middle=$(((low + high) / 2))
    middle_line=$((middle + 1))
    middle_commit="$(sed -n "${middle_line}p" "$commits_file")"
    run_batch_gate_commit "$middle_commit"
    if [ "$gate_result" -eq 0 ]; then
      low="$middle"
    else
      high="$middle"
    fi
  done

  first_bad_line=$((high + 1))
  first_bad_commit="$(sed -n "${first_bad_line}p" "$commits_file")"
  first_bad_subject="$(git -C "$repo_root" log -1 --format=%s "$first_bad_commit")"
  echo "batch-gate: first bad commit: $first_bad_commit $first_bad_subject" >&2
  exit 1
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
  focused|default|test)
    exec cargo test "$@"
    ;;
  fast-test)
    # opt-level 0 frames are roughly 3x larger than the default profile's
    # (measured on the depth-64 dialog reference walk: ~33.4 KiB/level vs
    # ~10.3 KiB/level). Give fast-test threads explicit headroom so the same
    # tests fit in every profile; the default gate profile keeps the standard
    # 2 MiB stacks as its frame-size discipline.
    RUST_MIN_STACK="${RUST_MIN_STACK:-16777216}" exec cargo test --profile fast-test "$@"
    ;;
  check)
    exec cargo check "$@"
    ;;
  fmt)
    exec cargo fmt --all "$@"
    ;;
  timings)
    if [ "$#" -eq 0 ]; then
      set -- --workspace --timings
    fi
    run_timed_cargo test "$@"
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
    run_timed_cargo test "$@"
    ;;
  sccache-eval)
    run_sccache_eval "$@"
    ;;
  gate)
    run_gate
    ;;
  cross-check)
    exec python3 "$script_dir/cross_check.py" "$@"
    ;;
  gate-scripts-tests-status)
    gate_scripts_tests_status
    ;;
  gate-cross-check-status)
    gate_cross_check_status
    ;;
  batch-gate)
    run_batch_gate "$@"
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
  prune)
    run_prune "$@"
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
