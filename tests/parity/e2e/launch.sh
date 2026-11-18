#!/bin/sh
# Runs one end-to-end scenario (scenarios/<name>.sh) in a rust-anything
# sandbox with the whole factory installed in ./factory/bin.
# Usage: launch.sh original|hybrid <scenario>
#
#   original  every executable is v2's (route, job, gate): the oracle.
#   hybrid    today's real dispatch: Rust route, gate and job, with v2's job
#             beside it as job.py for what the Rust job hands over (advance,
#             merge); job.py runs factory/bin/gate, i.e. the Rust gate.
#
# gh and git are the logging doubles of tests/parity/doubles.sh. Everything
# installed here is removed afterwards.
set -u
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../../.." && pwd)
oracle="$root/.oracle/v2/factory/bin"
mode=$1
scenario=$2
case "$mode" in
  original) for b in route job gate; do eval "src_$b=\$oracle/$b"; done ;;
  hybrid) for b in route job gate; do eval "src_$b=\$root/target/release/$b"; done ;;
  *) echo "launch.sh: unknown mode $mode" >&2; exit 2 ;;
esac

repo=$(pwd -P)
mkdir -p factory/bin
for b in route job gate; do
  eval "cp \"\$src_$b\" factory/bin/$b"
  chmod +x factory/bin/$b
done
[ "$mode" = hybrid ] && cp "$oracle/job" factory/bin/job.py && chmod +x factory/bin/job.py

CLAUDE_CONFIG_DIR="$repo/.no-claude-config"
export CLAUDE_CONFIG_DIR
. "$root/tests/parity/doubles.sh"

sh "$here/scenarios/$scenario.sh"
rc=$?

rm -rf "$doubles"
rm -f factory/bin/route factory/bin/job factory/bin/job.py factory/bin/gate
rmdir factory/bin 2>/dev/null
exit "$rc"
