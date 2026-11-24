# Sourced by the POSIX live gates. Records gate outcomes as
# local state under <git-common-dir>/bbb (shared by every worktree, never
# tracked): batch-gate reads last-gated as its default <good> endpoint, and
# workflow maintenance reads the timing logs.
#
# record_gate_stamp <log-basename> <pointer-basename-or-empty> <status> <detail> [validated-head]
# Appends "<utc-time>\t<HEAD>\t<status>\t<detail>" to the log and, when a
# pointer basename is given, points it at HEAD. Outside a git checkout this is
# a silent no-op so gates keep working in exported trees.

record_gate_stamp() {
  stamp_git_dir=$(git rev-parse --path-format=absolute --git-common-dir 2>/dev/null) || return 0
  stamp_head=${5:-}
  if [ -z "$stamp_head" ]; then
    stamp_head=$(git rev-parse HEAD 2>/dev/null) || return 0
  fi
  stamp_dir="$stamp_git_dir/bbb"
  mkdir -p "$stamp_dir"
  printf '%s\t%s\t%s\t%s\n' \
    "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$stamp_head" "$3" "$4" \
    >>"$stamp_dir/$1"
  if [ -n "$2" ]; then
    printf '%s\n' "$stamp_head" >"$stamp_dir/$2"
  fi
}

# require_fresh_bin <repo-root> <binary-path> <label>
#
# Refuses a harness binary that is older than a source it was built from.
#
# Every harness here resolves its binary by falling back to a shared build
# directory when no `BBB_*_BIN` names one, and that fallback is silent. A
# checkout that has moved on since the last build therefore runs *old* code
# against a *new* expectation, and the harness reports the difference as a
# behaviour divergence -- which is the most expensive possible way to say "you
# forgot to rebuild". This turns that into a refusal naming the file that is
# newer.
#
# The dependency set is Cargo's own: `--emit=dep-info` leaves `<binary>.d`
# beside the binary listing every file that went into it, so a crate the binary
# does not depend on cannot trigger a refusal. That precision is the whole
# point -- editing `bbb-client-net` does not relink `bbb-server`, and a guard that
# compared against every `crates/**/*.rs` would refuse to run a server binary
# that is perfectly current. When there is no dep-info the check is skipped:
# a binary named explicitly through `BBB_*_BIN` is a declared intent, and the
# hazard this guards is the silent fallback, not the explicit choice.
#
# `find` does the comparison, so this needs no GNU-only `-printf` and no
# `-nt` test operator, and behaves the same on both supported platforms.
require_fresh_bin() {
  fresh_bin=$2
  fresh_label=$3
  [ -x "$fresh_bin" ] || return 0
  [ -f "$fresh_bin.d" ] || return 0
  # Everything after the `<target>:` prefix is a path the build read, minus two
  # kinds of dependency that cannot mean "rebuild me":
  #
  # * anything under `.git`, because a build script that stamps the commit into
  #   the binary lists the ref file, and that would make every binary stale one
  #   commit after it was built with no code having changed;
  # * anything under `docs/evidence`, because those are generated outputs, not
  #   inputs a rebuild would change.
  fresh_deps=$(
    sed 's/^[^:]*://' "$fresh_bin.d" |
      tr ' ' '\n' |
      grep -v '/\.git/' |
      grep -v '/docs/evidence/' |
      tr '\n' ' '
  )
  [ -n "$fresh_deps" ] || return 0
  # Unquoted on purpose: the dep list is whitespace-separated and every path in
  # this workspace is space-free.
  # shellcheck disable=SC2086
  fresh_newer=$(find $fresh_deps -newer "$fresh_bin" -print 2>/dev/null | head -1)
  if [ -n "$fresh_newer" ]; then
    echo "$fresh_label: $fresh_bin was built before $fresh_newer" >&2
    echo "$fresh_label fix: rebuild the workspace before running this harness, or point the matching BBB_*_BIN at a current binary" >&2
    return 1
  fi
  return 0
}
