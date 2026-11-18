#!/bin/sh
# Runs one `gate` build inside a rust-anything sandbox (the current directory).
# Usage: launch.sh original|rust <gate args...>
#
# gate finds factory/ from its own location, so the build under test is
# installed at ./factory/bin/gate next to the case's fixtures. `git` is a
# double put first on PATH: for each subcommand (`rev-parse`, `diff`) it
# answers from the case's git-double/<subcommand>.{stdout,stderr,code} and
# fails with exit 99 unless its argv equals git-double/<subcommand>.args (one
# argument per line) and its working directory is the sandbox's
# git-double/cwd. Everything this script installs is removed afterwards.
set -u
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../../.." && pwd)
mode=$1
shift
case "$mode" in
  original) gate_src="$root/.oracle/v2/factory/bin/gate" ;;
  rust) gate_src="$root/target/release/gate" ;;
  *) echo "launch.sh: unknown mode $mode" >&2; exit 2 ;;
esac

made_factory=0; made_bin=0
[ -d factory ] || made_factory=1
[ -d factory/bin ] || made_bin=1
mkdir -p factory/bin
cp "$gate_src" factory/bin/gate
chmod +x factory/bin/gate

doubles=$(mktemp -d)
cat > "$doubles/git" <<'DOUBLE'
#!/bin/sh
d="$GIT_DOUBLE_DIR"
cmd=$1
if [ ! -f "$d/$cmd.args" ]; then
  echo "git double: no answer for: $*" >&2
  exit 98
fi
got=$(for a in "$@"; do printf '%s\n' "$a"; done)
want=$(cat "$d/$cmd.args")
if [ "$got" != "$want" ]; then
  echo "git double: unexpected argv: $*" >&2
  exit 99
fi
if [ "$(pwd -P)" != "$GIT_DOUBLE_SANDBOX/$(cat "$d/cwd")" ]; then
  echo "git double: unexpected cwd: $(pwd -P)" >&2
  exit 99
fi
[ -f "$d/$cmd.stdout" ] && cat "$d/$cmd.stdout"
[ -f "$d/$cmd.stderr" ] && cat "$d/$cmd.stderr" >&2
exit "$(cat "$d/$cmd.code" 2>/dev/null || echo 0)"
DOUBLE
chmod +x "$doubles/git"
GIT_DOUBLE_DIR="$PWD/git-double"
GIT_DOUBLE_SANDBOX=$(pwd -P)
PATH="$doubles:$PATH"
export GIT_DOUBLE_DIR GIT_DOUBLE_SANDBOX PATH

factory/bin/gate "$@"
rc=$?

rm -rf "$doubles"
rm -f factory/bin/gate
[ "$made_bin" = 1 ] && rmdir factory/bin 2>/dev/null
[ "$made_factory" = 1 ] && rmdir factory 2>/dev/null
exit "$rc"
