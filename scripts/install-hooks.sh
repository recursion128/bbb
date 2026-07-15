#!/bin/sh
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
source_hook="$repo_root/scripts/git-hooks/pre-commit"
hooks_dir=$(git -C "$repo_root" rev-parse --git-path hooks)

case "$hooks_dir" in
  /*) ;;
  *) hooks_dir="$repo_root/$hooks_dir" ;;
esac

target_hook="$hooks_dir/pre-commit"
mkdir -p "$hooks_dir"

if [ -f "$target_hook" ] && cmp -s "$source_hook" "$target_hook"; then
  echo "pre-commit hook is already installed: $target_hook"
  exit 0
fi

if [ -e "$target_hook" ] || [ -L "$target_hook" ]; then
  echo "refusing to replace existing pre-commit hook: $target_hook" >&2
  exit 1
fi

cp "$source_hook" "$target_hook"
chmod +x "$target_hook"
echo "installed pre-commit hook: $target_hook"
