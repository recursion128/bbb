#!/bin/sh
set -eu

usage() {
  cat <<'EOF'
Usage:
  scripts/worker-worktree.sh create <name> [base-ref]
  scripts/worker-worktree.sh status [name]
  scripts/worker-worktree.sh cleanup <name>
  scripts/worker-worktree.sh env <name>

Examples:
  scripts/worker-worktree.sh create world
  scripts/worker-worktree.sh status
  scripts/worker-worktree.sh env world
  scripts/worker-worktree.sh cleanup world

Conventions:
  worktree path: ../bbb-wt-<name>
  branch:        bbb-worker-<name>
  target dir:    /tmp/bbb-target-<name>
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
  printf 'bbb-worker-%s\n' "$name"
}

target_dir() {
  name="$1"
  printf '/tmp/bbb-target-%s\n' "$name"
}

print_worker_env() {
  name="$1"
  path="$2"
  branch="$3"
  target="$4"
  cat <<EOF
worker=$name
worktree=$path
branch=$branch
CARGO_TARGET_DIR=$target
EOF
}

create_worker() {
  name="$1"
  base_ref="${2:-HEAD}"
  validate_name "$name"

  root="$(repo_root)"
  path="$(worktree_path "$root" "$name")"
  branch="$(branch_name "$name")"
  target="$(target_dir "$name")"

  if [ -e "$path" ]; then
    echo "worktree path already exists: $path" >&2
    exit 2
  fi
  if git -C "$root" show-ref --verify --quiet "refs/heads/$branch"; then
    echo "branch already exists: $branch" >&2
    echo "run status/cleanup or choose another worker name" >&2
    exit 2
  fi

  git -C "$root" worktree add -b "$branch" "$path" "$base_ref"
  print_worker_env "$name" "$path" "$branch" "$target"
  cat <<EOF

Run focused tests with:
  cd "$path"
  CARGO_TARGET_DIR=$target cargo test -p <crate> <filter>
EOF
}

status_one() {
  root="$1"
  name="$2"
  validate_name "$name"
  path="$(worktree_path "$root" "$name")"
  branch="$(branch_name "$name")"
  target="$(target_dir "$name")"

  print_worker_env "$name" "$path" "$branch" "$target"
  if [ -d "$path/.git" ] || [ -f "$path/.git" ]; then
    head="$(git -C "$path" rev-parse --short HEAD 2>/dev/null || true)"
    current_branch="$(git -C "$path" branch --show-current 2>/dev/null || true)"
    dirty="$(git -C "$path" status --short 2>/dev/null || true)"
    echo "exists=yes"
    echo "current_branch=$current_branch"
    echo "head=$head"
    if [ -n "$dirty" ]; then
      echo "dirty=yes"
      printf '%s\n' "$dirty"
    else
      echo "dirty=no"
    fi
  else
    echo "exists=no"
  fi
  if [ -d "$target" ]; then
    du -sh "$target"
  else
    echo "target_exists=no"
  fi
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
  validate_name "$name"

  root="$(repo_root)"
  path="$(worktree_path "$root" "$name")"
  branch="$(branch_name "$name")"

  if [ ! -e "$path" ]; then
    echo "worktree not found: $path"
  else
    dirty="$(git -C "$path" status --short)"
    if [ -n "$dirty" ]; then
      echo "refusing to remove dirty worktree: $path" >&2
      printf '%s\n' "$dirty" >&2
      exit 2
    fi
    git -C "$root" worktree remove "$path"
    echo "removed worktree: $path"
  fi

  if git -C "$root" show-ref --verify --quiet "refs/heads/$branch"; then
    if git -C "$root" branch -d "$branch"; then
      echo "deleted branch: $branch"
    else
      echo "branch kept because git refused safe deletion: $branch" >&2
      echo "review/integrate it before deleting manually" >&2
    fi
  else
    echo "branch not found: $branch"
  fi
}

env_worker() {
  name="$1"
  validate_name "$name"
  root="$(repo_root)"
  print_worker_env \
    "$name" \
    "$(worktree_path "$root" "$name")" \
    "$(branch_name "$name")" \
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
    if [ "$#" -ne 1 ]; then
      usage >&2
      exit 2
    fi
    cleanup_worker "$1"
    ;;
  env)
    if [ "$#" -ne 1 ]; then
      usage >&2
      exit 2
    fi
    env_worker "$1"
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
