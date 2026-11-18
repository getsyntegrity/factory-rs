#!/bin/sh
# Runs one `job` build inside a rust-anything sandbox (the current directory).
# Usage: launch.sh original|rust <job args...>
#
# job finds the repository from its own location (factory/bin/job), so the
# build under test is installed at ./factory/bin/job next to the fixtures. The
# Rust build also gets v2's Python job beside it as factory/bin/job.py, its
# fallback for every subcommand it does not own. `job merge` runs
# factory/bin/gate, so the matching gate build (v2's, or the Rust one) is
# installed too.
#
# Transcripts: a case may ship Claude Code transcripts under
# claude-config/projects/REPO/. job looks for them under
# $CLAUDE_CONFIG_DIR/projects/<repo path with non-alphanumerics as "-">, and
# their prompts name the job folder by absolute path, so the fixtures are copied
# outside the sandbox with REPO and @REPO@ replaced by the sandbox path.
# `gh` and `git` are doubles put first on PATH. Every call appends its working
# directory (relative to the sandbox) and argv to ./doubles.log, so replay
# compares the calls themselves. A call answers from
# doubles/<tool>/<key>.{stdout,stderr,code}, where <key> is its first three
# arguments joined by "_" when such files exist, else its first two (anything
# outside [A-Za-z0-9-] becomes "_"); without those files it prints nothing
# and exits 0. For the n-th call with the same key, <key>.<n>.{stdout,...}
# win when they exist, so a double can change state (a PR that merges).
#
# Everything this script installs is removed afterwards.
set -u
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../../.." && pwd)
oracle_job="$root/.oracle/v2/factory/bin/job"
oracle_gate="$root/.oracle/v2/factory/bin/gate"
mode=$1
shift
case "$mode" in
  original) job_src="$oracle_job"; gate_src="$oracle_gate" ;;
  rust) job_src="$root/target/release/job"; gate_src="$root/target/release/gate" ;;
  *) echo "launch.sh: unknown mode $mode" >&2; exit 2 ;;
esac

repo=$(pwd -P)
made_factory=0; made_bin=0
[ -d factory ] || made_factory=1
[ -d factory/bin ] || made_bin=1
mkdir -p factory/bin
cp "$job_src" factory/bin/job
chmod +x factory/bin/job
[ "$mode" = rust ] && cp "$oracle_job" factory/bin/job.py && chmod +x factory/bin/job.py
cp "$gate_src" factory/bin/gate
chmod +x factory/bin/gate

config=$(mktemp -d)
if [ -d claude-config ]; then
  cp -R claude-config/. "$config/"
  if [ -d "$config/projects/REPO" ]; then
    mangled=$(printf '%s' "$repo" | sed 's/[^A-Za-z0-9]/-/g')
    mv "$config/projects/REPO" "$config/projects/$mangled"
    find "$config" -type f -name '*.jsonl' -exec sed -i "s#@REPO@#$repo#g" {} +
  fi
fi
CLAUDE_CONFIG_DIR="$config"
export CLAUDE_CONFIG_DIR

. "$root/tests/parity/doubles.sh"

factory/bin/job "$@"
rc=$?

rm -rf "$config" "$doubles"
rm -f factory/bin/job factory/bin/job.py factory/bin/gate
[ "$made_bin" = 1 ] && rmdir factory/bin 2>/dev/null
[ "$made_factory" = 1 ] && rmdir factory 2>/dev/null
exit "$rc"
