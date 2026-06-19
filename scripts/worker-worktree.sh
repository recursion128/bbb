#!/bin/sh
set -eu

usage() {
  cat <<'EOF'
Usage:
  scripts/worker-worktree.sh create <name> [base-ref]
  scripts/worker-worktree.sh status [name]
  scripts/worker-worktree.sh cleanup <name> [--force|-f] [--remove-target]
  scripts/worker-worktree.sh env <name>
  scripts/worker-worktree.sh shell-env <name>
  scripts/worker-worktree.sh --help

Examples:
  scripts/worker-worktree.sh create world
  scripts/worker-worktree.sh status
  scripts/worker-worktree.sh env world
  scripts/worker-worktree.sh shell-env world
  scripts/worker-worktree.sh cleanup world
  scripts/worker-worktree.sh cleanup world --force
  scripts/worker-worktree.sh cleanup world --remove-target

Conventions:
  worktree path: ../bbb-wt-<name>
  branch:        worker/<name>
  target dir:    /tmp/bbb-target-<name>

If worker/<name> already exists, create uses a timestamp-suffixed branch.
cleanup refuses dirty worktrees unless --force is passed, deletes the temporary
branch safely unless --force is passed, and keeps the matching target unless
--remove-target is passed.
EOF
}

repo_root() {
  git rev-parse --show-toplevel
}

validate_name() {
  name="$1"
  case "$name" in
    "")
      echo "worker name must not be empty" >&2
      exit 2
      ;;
    -*)
      echo "worker name must not start with '-': $name" >&2
      exit 2
      ;;
    *[!A-Za-z0-9_-]*)
      echo "worker name may contain only letters, digits, '_' and '-': $name" >&2
      exit 2
      ;;
  esac
}

worktree_path() {
  root="$1"
  name="$2"
  parent="$(dirname "$root")"
  printf '%s/bbb-wt-%s\n' "$parent" "$name"
}

branch_name() {
  name="$1"
  printf 'worker/%s\n' "$name"
}

target_dir() {
  name="$1"
  printf '/tmp/bbb-target-%s\n' "$name"
}

branch_exists() {
  root="$1"
  branch="$2"
  git -C "$root" show-ref --verify --quiet "refs/heads/$branch"
}

select_branch_name() {
  root="$1"
  name="$2"
  branch="$(branch_name "$name")"

  if ! branch_exists "$root" "$branch"; then
    printf '%s\n' "$branch"
    return
  fi

  stamp="$(date -u +%Y%m%d%H%M%S)"
  suffix=1
  while :; do
    if [ "$suffix" = "1" ]; then
      candidate="${branch}-${stamp}"
    else
      candidate="${branch}-${stamp}-${suffix}"
    fi

    if ! branch_exists "$root" "$candidate"; then
      printf '%s\n' "$candidate"
      return
    fi
    suffix=$((suffix + 1))
  done
}

is_worker_branch_for_name() {
  name="$1"
  branch="$2"
  case "$branch" in
    worker/"$name"|worker/"$name"-[0-9]*)
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

remove_target_dir() {
  target="$1"
  case "$target" in
    /tmp/bbb-target-*)
      ;;
    *)
      echo "refusing to remove target outside /tmp/bbb-target-*: $target" >&2
      exit 2
      ;;
  esac

  if [ -d "$target" ]; then
    rm -rf "$target"
    echo "removed target: $target"
  else
    echo "target not found: $target"
  fi
}

shell_quote() {
  printf "%s" "$1" | sed "s/'/'\\\\''/g; s/^/'/; s/$/'/"
}

print_worker_env() {
  name="$1"
  path="$2"
  branch="$3"
  target="$4"
  cat <<EOF
worker: $name
worktree: $path
branch: $branch
BBB_CARGO_TARGET_NAME=$name
CARGO_TARGET_DIR=$target

Export for focused tests:
  export BBB_CARGO_TARGET_NAME=$(shell_quote "$name")
  export CARGO_TARGET_DIR=$(shell_quote "$target")
EOF
}

print_worker_shell_env() {
  name="$1"
  path="$2"
  target="$3"
  printf 'cd %s\n' "$(shell_quote "$path")"
  printf 'export BBB_CARGO_TARGET_NAME=%s\n' "$(shell_quote "$name")"
  printf 'export CARGO_TARGET_DIR=%s\n' "$(shell_quote "$target")"
}

worker_env_branch() {
  root="$1"
  name="$2"
  path="$(worktree_path "$root" "$name")"
  branch="$(branch_name "$name")"

  if git -C "$path" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    current_branch="$(git -C "$path" branch --show-current 2>/dev/null || true)"
    if [ -n "$current_branch" ]; then
      branch="$current_branch"
    fi
  fi

  printf '%s\n' "$branch"
}

create_worker() {
  name="$1"
  base_ref="${2:-HEAD}"
  validate_name "$name"

  root="$(repo_root)"
  path="$(worktree_path "$root" "$name")"
  default_branch="$(branch_name "$name")"
  branch="$(select_branch_name "$root" "$name")"
  target="$(target_dir "$name")"

  if [ -e "$path" ]; then
    echo "worktree path already exists: $path" >&2
    exit 2
  fi

  if [ "$branch" != "$default_branch" ]; then
    echo "branch already exists: $default_branch" >&2
    echo "using temporary branch: $branch" >&2
  fi

  git -C "$root" worktree add -b "$branch" "$path" "$base_ref"
  print_worker_env "$name" "$path" "$branch" "$target"
  cat <<EOF

Run focused tests with:
  cd $(shell_quote "$path")
  export BBB_CARGO_TARGET_NAME=$(shell_quote "$name")
  export CARGO_TARGET_DIR=$(shell_quote "$target")
  scripts/cargo-dev.sh test -p <crate> <filter>
EOF
}

status_one() {
  root="$1"
  name="$2"
  validate_name "$name"
  path="$(worktree_path "$root" "$name")"
  target="$(target_dir "$name")"
  target_status="target=missing"
  if [ -e "$target" ]; then
    target_status="target=present"
  fi

  if ! git -C "$path" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    if [ -e "$path" ]; then
      printf '%s: status=not-git path=%s %s\n' "$name" "$path" "$target_status"
    else
      printf '%s: status=missing path=%s %s\n' "$name" "$path" "$target_status"
    fi
    return
  fi

  head="$(git -C "$path" rev-parse --short HEAD 2>/dev/null || true)"
  current_branch="$(git -C "$path" branch --show-current 2>/dev/null || true)"
  dirty="$(git -C "$path" status --short 2>/dev/null || true)"
  if [ -n "$dirty" ]; then
    dirty_count="$(printf '%s\n' "$dirty" | wc -l | tr -d ' ')"
    state="dirty(${dirty_count})"
  else
    state="clean"
  fi

  if [ -z "$current_branch" ]; then
    current_branch="detached"
  fi

  printf '%s: branch=%s status=%s head=%s path=%s %s\n' \
    "$name" "$current_branch" "$state" "$head" "$path" "$target_status"
}

status_workers() {
  root="$(repo_root)"
  if [ "$#" -eq 1 ]; then
    status_one "$root" "$1"
    return
  fi

  found=0
  parent="$(dirname "$root")"
  for path in "$parent"/bbb-wt-*; do
    [ -e "$path" ] || continue
    found=1
    name="${path##*/bbb-wt-}"
    status_one "$root" "$name"
    echo
  done
  if [ "$found" = "0" ]; then
    echo "no worker worktrees found"
  fi
}

cleanup_worker() {
  name="$1"
  shift
  validate_name "$name"
  force=0
  remove_target=0
  while [ "$#" -gt 0 ]; do
    case "$1" in
      --force|-f)
        force=1
        ;;
      --remove-target)
        remove_target=1
        ;;
      --keep-target)
        remove_target=0
        ;;
      *)
        echo "unknown cleanup option: $1" >&2
        usage >&2
        exit 2
        ;;
    esac
    shift
  done

  root="$(repo_root)"
  path="$(worktree_path "$root" "$name")"
  target="$(target_dir "$name")"
  branch=""
  branch_cleanup_failed=0

  if [ ! -e "$path" ]; then
    echo "worktree not found: $path"
  else
    if ! git -C "$path" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
      echo "refusing to remove non-git worktree path: $path" >&2
      exit 2
    fi
    current_branch="$(git -C "$path" branch --show-current)"
    if ! is_worker_branch_for_name "$name" "$current_branch"; then
      echo "refusing to remove worktree on unexpected branch: $path" >&2
      echo "expected: $(branch_name "$name") or timestamp-suffixed variant" >&2
      echo "actual: ${current_branch:-detached HEAD}" >&2
      exit 2
    fi
    branch="$current_branch"
    dirty="$(git -C "$path" status --short)"
    if [ -n "$dirty" ] && [ "$force" = "0" ]; then
      echo "refusing to remove dirty worktree: $path" >&2
      printf '%s\n' "$dirty" >&2
      echo "rerun with --force only if these changes can be discarded" >&2
      exit 2
    fi
    if [ "$force" = "1" ]; then
      git -C "$root" worktree remove --force "$path"
    else
      git -C "$root" worktree remove "$path"
    fi
    echo "removed worktree: $path"
  fi

  if [ -n "$branch" ] && branch_exists "$root" "$branch"; then
    if [ "$force" = "1" ]; then
      delete_branch_args="-D"
    else
      delete_branch_args="-d"
    fi
    if git -C "$root" branch "$delete_branch_args" "$branch"; then
      echo "deleted branch: $branch"
    else
      echo "branch kept because git refused safe deletion: $branch" >&2
      echo "review/integrate it before deleting manually" >&2
      branch_cleanup_failed=1
    fi
  elif [ -n "$branch" ]; then
    echo "branch not found: $branch"
  else
    echo "branch cleanup skipped because no worker branch was identified"
  fi

  if [ "$remove_target" = "1" ]; then
    if [ "$branch_cleanup_failed" = "1" ]; then
      echo "kept target because branch cleanup did not complete safely: $target" >&2
      exit 1
    fi
    remove_target_dir "$target"
  else
    echo "kept target: $target"
  fi
  if [ "$branch_cleanup_failed" = "1" ]; then
    exit 1
  fi
}

env_worker() {
  name="$1"
  validate_name "$name"
  root="$(repo_root)"
  print_worker_env \
    "$name" \
    "$(worktree_path "$root" "$name")" \
    "$(worker_env_branch "$root" "$name")" \
    "$(target_dir "$name")"
}

shell_env_worker() {
  name="$1"
  validate_name "$name"
  root="$(repo_root)"
  print_worker_shell_env \
    "$name" \
    "$(worktree_path "$root" "$name")" \
    "$(target_dir "$name")"
}

if [ "$#" -eq 0 ]; then
  usage >&2
  exit 2
fi

cmd="$1"
shift

case "$cmd" in
  create)
    if [ "$#" -lt 1 ] || [ "$#" -gt 2 ]; then
      usage >&2
      exit 2
    fi
    create_worker "$@"
    ;;
  status)
    if [ "$#" -gt 1 ]; then
      usage >&2
      exit 2
    fi
    status_workers "$@"
    ;;
  cleanup)
    if [ "$#" -lt 1 ] || [ "$#" -gt 3 ]; then
      usage >&2
      exit 2
    fi
    cleanup_worker "$@"
    ;;
  env)
    if [ "$#" -ne 1 ]; then
      usage >&2
      exit 2
    fi
    env_worker "$1"
    ;;
  shell-env)
    if [ "$#" -ne 1 ]; then
      usage >&2
      exit 2
    fi
    shell_env_worker "$1"
    ;;
  -h|--help|help)
    usage
    ;;
  *)
    echo "unknown command: $cmd" >&2
    usage >&2
    exit 2
    ;;
esac
