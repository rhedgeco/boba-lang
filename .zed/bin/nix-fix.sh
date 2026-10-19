#!/usr/bin/env sh
# Nix format-on-save fixer for Zed.
#
# Zed's `external` formatter pipes the buffer in on stdin and expects the
# formatted result on stdout. statix supports this directly (`statix fix
# --stdin`), but deadnix only edits files in place. To run BOTH on save we
# stage the buffer in a temp file, apply the in-place fixers, then re-run
# statix in streaming mode and emit the result to stdout.
#
# Binaries come from the devenv profile so this works regardless of PATH.
set -eu

ROOT="${ZED_WORKTREE_ROOT:-.}"
BIN="$ROOT/.devenv/profile/bin"

tmp="$(mktemp --suffix=.nix)"
trap 'rm -f "$tmp"' EXIT

# Stage the incoming buffer.
cat > "$tmp"

# deadnix can only edit files in place; apply it to the staged copy.
"$BIN/deadnix" --edit --quiet "$tmp" >/dev/null 2>&1 || true

# statix can stream: feed the deadnix-cleaned file through and emit to stdout.
"$BIN/statix" fix --stdin < "$tmp"
