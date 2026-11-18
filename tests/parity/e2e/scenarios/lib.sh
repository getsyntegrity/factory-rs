# Helpers for the scenarios. `step` prints the command, runs it, and prints
# its exit code; stdout and stderr stay on their own streams.
step() {
  printf '### %s\n' "$*"
  printf '### %s\n' "$*" >&2
  "$@"
  printf 'exit=%s\n' "$?"
}

# What an agent would write: <path> <text with \n escapes>.
agent_writes() {
  mkdir -p "$(dirname "$1")"
  printf '%b' "$2" > "$1"
  printf '### agent writes %s\n' "$1"
}

job_id() {
  ls factory/jobs | grep -v '^\.' | head -1
}
