#!/bin/sh
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
goal="$repo_root/goal.md"

# workflow-v3 reduced goal.md to 564 lines on 2026-07-15. The 660-line cap
# leaves about 17% headroom. When the cap is reached, archive completed history
# before changing the limit; do not raise it to accommodate coverage prose.
max_goal_lines=660
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
  echo "Fix: move completed history to docs/goal-archive.md and put evidence in coverage overrides before raising the goal.md limit." >&2
  exit 1
fi

echo "goal.md lint: $line_count/$max_goal_lines lines, no completed coverage prose"
