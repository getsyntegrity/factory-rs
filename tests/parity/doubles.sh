#!/bin/sh
# Sourced by the job and e2e launchers: puts logging `gh` and `git` doubles
# first on PATH. Needs $repo (the sandbox, resolved); sets $doubles (remove it
# afterwards). Every call appends its working directory (relative to the
# sandbox) and argv to ./doubles.log. A call answers from
# doubles/<tool>/<key>.{stdout,stderr,code}, where <key> is its first three
# arguments joined by "_" when such files exist, else its first two (anything
# outside [A-Za-z0-9-] becomes "_"); for the n-th call with the same key,
# <key>.<n>.* win when they exist. Without answers it prints nothing, exit 0.
doubles=$(mktemp -d)
for tool in gh git; do
  cat > "$doubles/$tool" <<DOUBLE
#!/bin/sh
tool=$tool
DOUBLE
  cat >> "$doubles/$tool" <<'DOUBLE'
here=$(pwd -P)
rel=${here#"$DOUBLES_SANDBOX"}
{
  printf '$ %s (cwd .%s)\n' "$tool" "$rel"
  for a in "$@"; do printf '  [%s]\n' "$a"; done
} >> "$DOUBLES_SANDBOX/doubles.log"
key3=$(printf '%s_%s_%s' "${1:-}" "${2:-}" "${3:-}" | sed 's/[^A-Za-z0-9-]/_/g')
key2=$(printf '%s_%s' "${1:-}" "${2:-}" | sed 's/[^A-Za-z0-9-]/_/g')
answers="$DOUBLES_SANDBOX/doubles/$tool/$key3"
ls "$answers".* >/dev/null 2>&1 || answers="$DOUBLES_SANDBOX/doubles/$tool/$key2"
count="$DOUBLES_STATE/$tool-$(basename "$answers")"
n=$(( $(cat "$count" 2>/dev/null || echo 0) + 1 ))
echo "$n" > "$count"
ls "$answers.$n".* >/dev/null 2>&1 && answers="$answers.$n"
[ -f "$answers.stdout" ] && cat "$answers.stdout"
[ -f "$answers.stderr" ] && cat "$answers.stderr" >&2
exit "$(cat "$answers.code" 2>/dev/null || echo 0)"
DOUBLE
  chmod +x "$doubles/$tool"
done
mkdir "$doubles/state"
DOUBLES_SANDBOX="$repo"
DOUBLES_STATE="$doubles/state"
PATH="$doubles:$PATH"
export DOUBLES_SANDBOX DOUBLES_STATE PATH
