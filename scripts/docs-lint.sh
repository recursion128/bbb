#!/bin/sh
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
goal="$repo_root/goal.md"
archive="$repo_root/docs/goal-archive.md"
feature_ledger="$repo_root/docs/unsupported-features.md"

# workflow-v5 (2026-07-21) rewrote goal.md down to 513 lines: completed
# foundations moved to the goal-archive milestone index and workflow prose was
# deduplicated with AGENTS.md. The cap is 560 (~9% headroom). When the cap is
# reached, archive completed history (as index lines, not prose) before
# changing the limit; do not raise it to accommodate coverage prose.
max_goal_lines=560

# workflow-v6 (2026-07-28): the goal-archive milestone index gets its own cap
# so it cannot regrow the prose problem goal.md had. When the cap is reached,
# roll the oldest months up into one-line-per-month summaries; do not raise
# the limit for new milestone prose.
max_archive_lines=900

failed=0

line_count=$(wc -l <"$goal")
if [ "$line_count" -gt "$max_goal_lines" ]; then
  echo "goal.md lint: $line_count lines exceeds the $max_goal_lines-line limit" >&2
  echo "goal.md lint fix: drop completed prose (atomic slices need no archive entry; milestones get 1-3 index lines in docs/goal-archive.md) before raising the limit" >&2
  failed=1
fi

archive_line_count=$(wc -l <"$archive")
if [ "$archive_line_count" -gt "$max_archive_lines" ]; then
  echo "goal-archive lint: $archive_line_count lines exceeds the $max_archive_lines-line limit; roll the oldest months up into one-line-per-month summaries instead of raising the cap" >&2
  failed=1
fi

cd "$repo_root"
if completed_history=$(rg -n --no-heading --color never \
  '已闭合|已审计|五格 complete|全行闭合' goal.md); then
  echo "goal.md lint: completed coverage prose is not allowed:" >&2
  echo "$completed_history" >&2
  echo "goal.md lint fix: move evidence into coverage overrides or the goal-archive milestone index" >&2
  failed=1
fi

# workflow-v8 (2026-07-29): deferred gaps stay under their owning Todo domain
# with an explicit restart condition. A top-level Deferred section made the
# hardest-ready panel silently report zero active work.
if legacy_deferred=$(rg -n --no-heading --color never \
  '^# Deferred$' "$feature_ledger"); then
  echo "feature ledger lint: top-level Deferred sections are not allowed:" >&2
  echo "$legacy_deferred" >&2
  echo "feature ledger lint fix: keep the item under its owning Todo domain and mark it \`deferred\` with an explicit 'Reopen when ...' condition" >&2
  failed=1
fi

deferred_without_reopen=$(
  awk '
    function flush_item() {
      if (item ~ /`deferred`/ && item !~ /Reopen when /) {
        print item_line ":" item
      }
      item = ""
      item_line = 0
    }
    /^# Todo$/ {
      flush_item()
      in_todo = 1
      next
    }
    /^# / {
      flush_item()
      in_todo = 0
      next
    }
    in_todo && /^- / {
      flush_item()
      item = $0
      item_line = NR
      next
    }
    in_todo && item != "" && /^  / {
      item = item " " $0
      next
    }
    in_todo && item != "" && NF {
      flush_item()
    }
    END {
      flush_item()
    }
  ' "$feature_ledger"
)
if [ -n "$deferred_without_reopen" ]; then
  echo "feature ledger lint: deferred Todo items need an explicit 'Reopen when ...' condition:" >&2
  echo "$deferred_without_reopen" >&2
  failed=1
fi

# workflow-v6 (2026-07-28): top-level docs/*-design.md files are active
# contracts and must be referenced from an active root (goal.md, AGENTS.md, or
# another top-level doc). docs/goal-archive.md is a history index and does not
# count; move closed contracts to docs/archive/ instead.
for design_doc in docs/*-design.md; do
  [ -e "$design_doc" ] || continue
  design_base=$(basename "$design_doc")
  design_referenced=0
  for ref_root in goal.md AGENTS.md docs/*.md; do
    [ -e "$ref_root" ] || continue
    case "$ref_root" in
      docs/goal-archive.md | "$design_doc")
        continue
        ;;
    esac
    if grep -qF "$design_base" "$ref_root"; then
      design_referenced=1
      break
    fi
  done
  if [ "$design_referenced" -eq 0 ]; then
    echo "docs lint: $design_doc is not referenced by goal.md, AGENTS.md, or another active doc; move closed design docs to docs/archive/" >&2
    failed=1
  fi
done

if [ "$failed" -ne 0 ]; then
  exit 1
fi

echo "docs lint: goal.md $line_count/$max_goal_lines lines, goal-archive $archive_line_count/$max_archive_lines lines, owned deferred gaps, design docs referenced, no completed coverage prose"
