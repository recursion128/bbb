#!/bin/sh
set -eu

# Desktop launcher wrapper for the Linux x86_64 bbb-client binary.
# Resolves a sibling binary in extracted release archives first, then /usr/local/bin or PATH.

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
if [ -x "$SCRIPT_DIR/bbb-client" ]; then
    BBB_CLIENT_BIN="$SCRIPT_DIR/bbb-client"
elif [ -x "/usr/local/bin/bbb-client" ]; then
    BBB_CLIENT_BIN="/usr/local/bin/bbb-client"
elif command -v bbb-client >/dev/null 2>&1; then
    BBB_CLIENT_BIN=$(command -v bbb-client)
else
    printf '%s\n' "bbb-client binary not found next to $0, in /usr/local/bin, or on PATH" >&2
    exit 1
fi

has_flag() {
    needle=$1
    shift
    for arg in "$@"; do
        case "$arg" in
            "$needle"|"$needle"=*) return 0 ;;
        esac
    done
    return 1
}

GAME_DIR=${BBB_GAME_DIR:-${XDG_DATA_HOME:-$HOME/.local/share}/bbb-client}

if ! has_flag --help "$@" && ! has_flag -h "$@" && ! has_flag --version "$@" && ! has_flag -V "$@"; then
    if ! has_flag --game-dir "$@"; then
        mkdir -p "$GAME_DIR"
        set -- --game-dir "$GAME_DIR" "$@"
    fi
    if [ -n "${BBB_VANILLA_ROOT:-}" ] && ! has_flag --vanilla-root "$@"; then
        set -- --vanilla-root "$BBB_VANILLA_ROOT" "$@"
    fi
    if [ -n "${BBB_SERVER:-}" ] && ! has_flag --server "$@" && ! has_flag --quick-play-multiplayer "$@" && ! has_flag --status-server "$@"; then
        set -- --server "$BBB_SERVER" "$@"
    fi
    case "${BBB_FULLSCREEN:-}" in
        1|true|TRUE|yes|YES)
            if ! has_flag --fullscreen "$@"; then
                set -- --fullscreen "$@"
            fi
            ;;
    esac
fi

exec "$BBB_CLIENT_BIN" "$@"
