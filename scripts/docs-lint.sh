#!/bin/sh
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
goal="$repo_root/goal.md"

# workflow-v5 (2026-07-21) rewrote goal.md down to 513 lines: completed
# foundations moved to the goal-archive milestone index and workflow prose was
# deduplicated with AGENTS.md. The cap is 560 (~9% headroom). When the cap is
# reached, archive completed history (as index lines, not prose) before
# changing the limit; do not raise it to accommodate coverage prose.
max_goal_lines=560
line_count=$(wc -l <"$goal")
failed=0

if [ "$line_count" -gt "$max_goal_lines" ]; then
  echo "goal.md lint: $line_count lines exceeds the $max_goal_lines-line limit" >&2
  failed=1
fi

cd "$repo_root"
if completed_history=$(rg -n --no-heading --color never \
  '已闭合|已审计|五格 complete|全行闭合' goal.md); then
  echo "goal.md lint: completed coverage prose is not allowed:" >&2
  echo "$completed_history" >&2
  failed=1
fi

if [ "$failed" -ne 0 ]; then
  echo "Fix: drop completed prose (atomic slices need no archive entry; milestones get 1-3 index lines in docs/goal-archive.md) and put evidence in coverage overrides before raising the goal.md limit." >&2
  exit 1
fi

echo "goal.md lint: $line_count/$max_goal_lines lines, no completed coverage prose"
