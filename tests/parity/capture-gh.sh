#!/bin/sh
# Records real `gh` answers as fixtures for the gh double (tests/parity/doubles.sh).
#
# Install a copy named `gh` first on PATH, with
#   REAL_GH=/path/to/real/gh   CAPTURE_DIR=/where/to/write
# Every call runs the real gh unchanged (same argv, stdin, environment) and
# writes, for the n-th call with the same key:
#   $CAPTURE_DIR/gh/<key>.<n>.stdout, .stderr, .code
# where <key> is the first three arguments joined by "_" with anything outside
# [A-Za-z0-9-] as "_" (the double's three-argument key), and appends the call
# to $CAPTURE_DIR/calls.log in doubles.log format. The output is passed
# through, so the factory behaves exactly as with the real gh.
#
# The files hold raw GitHub answers: sanitise them before committing
# (MIGRATION.md, "Capture real gh answers").
set -u
: "${REAL_GH:?set REAL_GH to the real gh}" "${CAPTURE_DIR:?set CAPTURE_DIR}"
key=$(printf '%s_%s_%s' "${1:-}" "${2:-}" "${3:-}" | sed 's/[^A-Za-z0-9-]/_/g')
dir="$CAPTURE_DIR/gh"
mkdir -p "$dir"
n=$(( $(cat "$dir/.count-$key" 2>/dev/null || echo 0) + 1 ))
echo "$n" > "$dir/.count-$key"
"$REAL_GH" "$@" > "$dir/$key.$n.stdout" 2> "$dir/$key.$n.stderr"
rc=$?
echo "$rc" > "$dir/$key.$n.code"
{
  printf '$ gh (cwd %s)\n' "$(pwd -P)"
  for a in "$@"; do printf '  [%s]\n' "$a"; done
} >> "$CAPTURE_DIR/calls.log"
cat "$dir/$key.$n.stdout"
cat "$dir/$key.$n.stderr" >&2
exit "$rc"
