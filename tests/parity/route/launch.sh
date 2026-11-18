#!/bin/sh
# Runs one `route` build inside a rust-anything sandbox (the current directory).
# Usage: launch.sh original|rust <route args...>
#
# route finds factory/ from its own location, so the build under test is
# installed at ./factory/bin/route next to the case's fixtures. The Python
# `job` from the frozen oracle is installed beside it unless the case ships its
# own double at factory/bin/job. Everything this script installs is removed
# afterwards so only route's own effects show up in the recorded files.
set -u
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../../.." && pwd)
mode=$1
shift
case "$mode" in
  original) route_src="$root/.oracle/v2/factory/bin/route" ;;
  rust) route_src="$root/target/release/route" ;;
  *) echo "launch.sh: unknown mode $mode" >&2; exit 2 ;;
esac

made_factory=0; made_bin=0; installed_job=0
[ -d factory ] || made_factory=1
[ -d factory/bin ] || made_bin=1
mkdir -p factory/bin
cp "$route_src" factory/bin/route
chmod +x factory/bin/route
if [ -e factory/bin/job ]; then
  chmod +x factory/bin/job
else
  cp "$root/.oracle/v2/factory/bin/job" factory/bin/job
  chmod +x factory/bin/job
  installed_job=1
fi

# job sync prices Claude Code transcripts; point it at a directory that never exists.
CLAUDE_CONFIG_DIR="$PWD/.no-claude-config"
export CLAUDE_CONFIG_DIR

factory/bin/route "$@"
rc=$?

rm -f factory/bin/route
[ "$installed_job" = 1 ] && rm -f factory/bin/job
[ "$made_bin" = 1 ] && rmdir factory/bin 2>/dev/null
[ "$made_factory" = 1 ] && rmdir factory 2>/dev/null
exit "$rc"
