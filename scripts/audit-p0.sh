#!/bin/sh
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
allowlist="$repo_root/scripts/data/p0-audit-allowlist.txt"

if ! command -v rg >/dev/null 2>&1; then
  echo "P0 audit requires rg" >&2
  exit 2
fi
if [ ! -f "$allowlist" ]; then
  echo "P0 audit allowlist not found: $allowlist" >&2
  exit 2
fi

hits_file=$(mktemp "${TMPDIR:-/tmp}/bbb-p0-audit.XXXXXX")
trap 'rm -f "$hits_file"' 0 HUP INT TERM

cd "$repo_root"
rg -n --no-heading --color never 'residual|fallback|unsupported' crates/ \
  -g '!target/**' >"$hits_file" || {
  status=$?
  if [ "$status" -ne 1 ]; then
    exit "$status"
  fi
}

LC_ALL=C awk '
  FNR == NR {
    if ($0 ~ /^[[:space:]]*($|#)/) {
      next
    }
    separator = index($0, "\t")
    if (separator == 0) {
      printf "invalid P0 audit allowlist entry at line %d: expected <path-regex><TAB><line-regex>\n", FNR > "/dev/stderr"
      invalid = 1
      next
    }
    path_pattern[++pattern_count] = substr($0, 1, separator - 1)
    line_pattern[pattern_count] = substr($0, separator + 1)
    if (path_pattern[pattern_count] == "" || line_pattern[pattern_count] == "") {
      printf "invalid P0 audit allowlist entry at line %d: regexes must be non-empty\n", FNR > "/dev/stderr"
      invalid = 1
    }
    next
  }

  {
    hit_count++
    first = index($0, ":")
    remainder = substr($0, first + 1)
    second = index(remainder, ":")
    path = substr($0, 1, first - 1)
    content = substr(remainder, second + 1)
    classified = 0
    for (i = 1; i <= pattern_count; i++) {
      if (path ~ path_pattern[i] && content ~ line_pattern[i]) {
        classified = 1
        break
      }
    }
    if (!classified) {
      print $0
      unclassified++
    }
  }

  END {
    if (invalid) {
      exit 2
    }
    if (unclassified) {
      exit 1
    }
    printf "P0 audit: %d hits, all classified against %d allowlist entries\n", \
      hit_count, pattern_count
  }
' "$allowlist" "$hits_file"
