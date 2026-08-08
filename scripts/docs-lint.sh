#!/bin/sh
set -eu

repo_root=${BBB_DOCS_LINT_REPO_ROOT:-$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)}
goal="$repo_root/goal.md"
archive="$repo_root/docs/goal-archive.md"
feature_ledger="$repo_root/docs/unsupported-features.md"
script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)

# workflow-v5 (2026-07-21) rewrote goal.md down to 513 lines: completed
# foundations moved to the goal-archive milestone index and workflow prose was
# deduplicated with AGENTS.md. The cap is 560 (~9% headroom). When the cap is
# reached, archive completed history (as index lines, not prose) before
# changing the limit; do not raise it to accommodate coverage prose.
max_goal_lines=560

# workflow-v10 (2026-08-02): historical contracts moved to Git-only retention
# and the goal archive became a compact routing index. Keep enough headroom for
# new engineering milestones without allowing completion prose to regrow.
max_archive_lines=100

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

# workflow-v9 (2026-07-30): every deferred decision is owned by Todo, every
# deferred decision has one mechanical restart condition, and every Not-needed
# decision keeps an auditable Vanilla/source anchor. Audit complete Markdown
# items rather than only their first line.
feature_item_issues=$(
  awk '
    function has_deferred_marker(text) {
      return tolower(text) ~ /(^|[^[:alnum:]_-])deferred([^[:alnum:]_-]|$)/
    }
    function flush_item() {
      if (item_section == "not needed" && has_deferred_marker(item)) {
        print "misplaced\t" item_line ":" item
      }
      if (has_deferred_marker(item) && item !~ /Reopen when /) {
        print "reopen\t" item_line ":" item
      }
      if (item_section == "not needed" && item !~ /[Vv]anilla/ && item !~ /`/) {
        print "evidence\t" item_line ":" item
      }
      item = ""
      item_line = 0
      item_section = ""
    }
    /^# / {
      flush_item()
      section = tolower(substr($0, 3))
      next
    }
    (section == "todo" || section == "not needed") && /^- / {
      flush_item()
      item = $0
      item_line = NR
      item_section = section
      next
    }
    item != "" && /^  / {
      item = item " " $0
      next
    }
    item != "" && NF {
      flush_item()
    }
    END {
      flush_item()
    }
  ' "$feature_ledger"
)
misplaced_deferred=$(printf '%s\n' "$feature_item_issues" | sed -n 's/^misplaced	//p')
if [ -n "$misplaced_deferred" ]; then
  echo "feature ledger lint: deferred items must stay under their owning # Todo domain, not # Not needed:" >&2
  echo "$misplaced_deferred" >&2
  failed=1
fi

deferred_without_reopen=$(printf '%s\n' "$feature_item_issues" | sed -n 's/^reopen	//p')
if [ -n "$deferred_without_reopen" ]; then
  echo "feature ledger lint: deferred items need an explicit 'Reopen when ...' condition:" >&2
  echo "$deferred_without_reopen" >&2
  failed=1
fi

not_needed_without_evidence=$(printf '%s\n' "$feature_item_issues" | sed -n 's/^evidence	//p')
if [ -n "$not_needed_without_evidence" ]; then
  echo "feature ledger lint: # Not needed items need a Vanilla statement or source-code anchor:" >&2
  echo "$not_needed_without_evidence" >&2
  failed=1
fi

# The synchronized custom block/entity task was a concrete roadmap/ledger
# contradiction: Vanilla 26.1 does not synchronize either registry. Reject only
# the old numbered-task shape so explanatory maintenance prose remains legal.
if stale_custom_identity=$(rg -n --no-heading --color never \
  '^[[:space:]]*([-*]|[0-9]+\.) .*\*\*(同步 registry 到 custom presentation|[Ss]ynchronized custom block/entity identity)' \
  "$goal"); then
  echo "goal.md lint: synchronized custom block/entity identity cannot be an open roadmap task:" >&2
  echo "$stale_custom_identity" >&2
  failed=1
fi

# Route statuses are the only roadmap-side machine state. When the Todo ledger
# has no active items, an active route would recreate an invisible queue; when
# active items exist, at least one route must own them.
roadmap_statuses=$(
  awk '
    /<!-- roadmap-status:start -->/ {
      inside = 1
      next
    }
    /<!-- roadmap-status:end -->/ {
      inside = 0
      next
    }
    inside && NF {
      print
    }
  ' "$goal"
)

for route in \
  p1-playable \
  p1-generated-facts \
  p2-non-entity \
  p2-hud \
  p3-resources \
  p4-delivery \
  architecture-convergence
do
  route_count=$(printf '%s\n' "$roadmap_statuses" |
    grep -Ec "^- $route: (active|maintenance|closed|deferred)$" || true)
  if [ "$route_count" -ne 1 ]; then
    echo "goal.md lint: roadmap status needs exactly one '$route: active|maintenance|closed|deferred' entry" >&2
    failed=1
  fi
done

invalid_roadmap_statuses=$(printf '%s\n' "$roadmap_statuses" |
  grep -Ev '^- (p1-playable|p1-generated-facts|p2-non-entity|p2-hud|p3-resources|p4-delivery|architecture-convergence): (active|maintenance|closed|deferred)$' || true)
if [ -n "$invalid_roadmap_statuses" ]; then
  echo "goal.md lint: malformed or unknown roadmap status entries:" >&2
  echo "$invalid_roadmap_statuses" >&2
  failed=1
fi

active_todo_items=$(
  awk '
    function has_deferred_marker(text) {
      return tolower(text) ~ /(^|[^[:alnum:]_-])deferred([^[:alnum:]_-]|$)/
    }
    function flush_item() {
      lower = tolower(item)
      if (section == "todo" && item != "" &&
          !has_deferred_marker(item) && lower !~ /`not-needed`/) {
        active += 1
      }
      item = ""
    }
    /^# / {
      flush_item()
      section = tolower(substr($0, 3))
      next
    }
    section == "todo" && /^- / {
      flush_item()
      item = $0
      next
    }
    item != "" && /^  / {
      item = item " " $0
      next
    }
    item != "" && NF {
      flush_item()
    }
    END {
      flush_item()
      print active + 0
    }
  ' "$feature_ledger"
)
active_roadmap_routes=$(printf '%s\n' "$roadmap_statuses" |
  grep -Ec ': active$' || true)
if [ "$active_todo_items" -eq 0 ] && [ "$active_roadmap_routes" -ne 0 ]; then
  echo "goal.md lint: active Todo is zero, so roadmap statuses cannot retain an active queue" >&2
  failed=1
elif [ "$active_todo_items" -ne 0 ] && [ "$active_roadmap_routes" -eq 0 ]; then
  echo "goal.md lint: active Todo items exist but no roadmap route owns them" >&2
  failed=1
fi

if [ "$failed" -ne 0 ]; then
  exit 1
fi

# The checked JSON is an exact generated artifact, not merely a collection of
# independently valid hashes. The full gate consumes a verified, explicitly
# prepared workspace and never downloads it implicitly.
if ! vanilla_root=$(
  "$script_dir/vanilla" path 26.1 --kind root
); then
  failed=1
  vanilla_root=
fi
client_inventory_generator="$repo_root/scripts/generate-client-mechanism-inventory.py"
client_inventory_output="$repo_root/docs/evidence/vanilla-26.1/client-mechanism-inventory.json"
if [ -n "$vanilla_root" ] && [ -f "$client_inventory_generator" ]; then
  if ! "$repo_root/scripts/python" \
    "$client_inventory_generator" \
    --vanilla-root "$vanilla_root" \
    --output "$client_inventory_output" \
    --check; then
    failed=1
  fi
else
  echo "docs lint: client inventory exact-output check skipped (Vanilla tree or repo generator absent)"
fi

file_drop_generator="$repo_root/scripts/generate-file-drop-capability.py"
file_drop_output="$repo_root/docs/evidence/vanilla-26.1/file-drop-capability.json"
if [ -n "$vanilla_root" ] && [ -f "$file_drop_generator" ]; then
  if ! "$repo_root/scripts/python" \
    "$file_drop_generator" \
    --vanilla-root "$vanilla_root" \
    --output "$file_drop_output" \
    --check; then
    failed=1
  fi
else
  echo "docs lint: file-drop capability exact-output check skipped (Vanilla tree or repo generator absent)"
fi

menu_screen_generator="$repo_root/scripts/generate-menu-screen-catalog.py"
menu_screen_output="$repo_root/crates/bbb-native/src/input/inventory/menu_screens_26_1.rs"
if [ -n "$vanilla_root" ] && [ -f "$menu_screen_generator" ]; then
  if ! "$repo_root/scripts/python" \
    "$menu_screen_generator" \
    --vanilla-root "$vanilla_root" \
    --output "$menu_screen_output" \
    --check; then
    failed=1
  fi
else
  echo "docs lint: menu-screen catalog exact-output check skipped (Vanilla tree or repo generator absent)"
fi

key_mapping_generator="$repo_root/scripts/generate-key-mapping-catalog.py"
key_mapping_output="$repo_root/docs/evidence/vanilla-26.1/key-mapping-catalog.json"
key_mapping_rust_output="$repo_root/crates/bbb-native/src/client_options/key_mappings_26_1.rs"
if [ -n "$vanilla_root" ] && [ -f "$key_mapping_generator" ]; then
  if ! "$repo_root/scripts/python" \
    "$key_mapping_generator" \
    --vanilla-root "$vanilla_root" \
    --output "$key_mapping_output" \
    --rust-output "$key_mapping_rust_output" \
    --check; then
    failed=1
  fi
else
  echo "docs lint: key-mapping catalog exact-output check skipped (Vanilla tree or repo generator absent)"
fi

particle_coverage_generator="$repo_root/scripts/generate-particle-provider-coverage.py"
particle_coverage_output="$repo_root/docs/evidence/vanilla-26.1/particle-provider-coverage.json"
if [ -n "$vanilla_root" ] && [ -f "$particle_coverage_generator" ]; then
  if ! "$repo_root/scripts/python" \
    "$particle_coverage_generator" \
    --vanilla-root "$vanilla_root" \
    --output "$particle_coverage_output" \
    --check; then
    failed=1
  fi
else
  echo "docs lint: particle provider coverage exact-output check skipped (Vanilla tree or repo generator absent)"
fi

if ! "$script_dir/python" python "$script_dir/docs_wiki_lint.py" \
  --repo-root "$repo_root"; then
  failed=1
fi

parity_acceptance="$repo_root/scripts/parity_acceptance.py"
parity_report="$repo_root/docs/evidence/vanilla-26.1/parity-acceptance.json"
if [ -n "$vanilla_root" ] && [ -f "$parity_acceptance" ] && [ -f "$parity_report" ]; then
  if ! "$repo_root/scripts/python" python "$parity_acceptance" check \
    --repo-root "$repo_root" \
    --vanilla-root "$vanilla_root" \
    --output "$parity_report"; then
    failed=1
  fi
else
  echo "docs lint: parity acceptance check skipped (Vanilla tree, checker, or report absent)"
fi

if [ "$failed" -ne 0 ]; then
  exit 1
fi

echo "docs lint: goal.md $line_count/$max_goal_lines lines, goal-archive $archive_line_count/$max_archive_lines lines, route/ledger states consistent, owned deferred/not-needed decisions, Wiki-only docs layout, no completed coverage prose"
