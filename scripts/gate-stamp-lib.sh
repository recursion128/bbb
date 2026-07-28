# Sourced by scripts/cargo-dev.sh and scripts/playable-gate.sh. Records gate
# outcomes as per-checkout local state under <git-dir>/bbb (never tracked):
# batch-gate reads last-gated as its default <good> endpoint, the goal.md
# audit step reads the pointers as regression evidence, and workflow step 8
# reads the gate-log timing columns to verify gate cost stays flat or falls.
#
# record_gate_stamp <log-basename> <pointer-basename-or-empty> <status> <detail>
# Appends "<utc-time>\t<HEAD>\t<status>\t<detail>" to the log and, when a
# pointer basename is given, points it at HEAD. Outside a git checkout this is
# a silent no-op so gates keep working in exported trees.

record_gate_stamp() {
  stamp_git_dir=$(git rev-parse --absolute-git-dir 2>/dev/null) || return 0
  stamp_head=$(git rev-parse HEAD 2>/dev/null) || return 0
  stamp_dir="$stamp_git_dir/bbb"
  mkdir -p "$stamp_dir"
  printf '%s\t%s\t%s\t%s\n' \
    "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$stamp_head" "$3" "$4" \
    >>"$stamp_dir/$1"
  if [ -n "$2" ]; then
    printf '%s\n' "$stamp_head" >"$stamp_dir/$2"
  fi
}
