#!/bin/sh
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)

if [ "$#" -eq 0 ]; then
  echo "usage: scripts/vanilla-source-tools.sh <python-command-or-script> [args...]" >&2
  exit 2
fi

exec "$repo_root/scripts/python" "$@"
