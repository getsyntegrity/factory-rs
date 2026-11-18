# needs-info → queue sees the reply → intake → spec → build → blocked →
# needs-human → feedback → build → blocked again → needs-human → reject.
. "$(dirname "$0")/lib.sh"
step factory/bin/job create 5
ID=$(job_id)
J=factory/jobs/$ID
agent_writes $J/intake.md 'DECISION: NEEDS-INFO\nRISK: low\nSURFACES: none\n\n## Questions\n- Which page?\n'
step factory/bin/job advance "$ID" needs-info
step factory/bin/job queue
step factory/bin/job advance "$ID" intake
agent_writes $J/intake.md 'DECISION: ACCEPT\nRISK: low\nSURFACES: none\n\n## Summary\n- Settings page.\n'
step factory/bin/route "$ID"
step factory/bin/job advance "$ID" spec
mkdir -p ".worktrees/$ID"
agent_writes $J/spec.md '# Spec\n'
step factory/bin/job advance "$ID" build
agent_writes $J/build.md 'STATUS: BLOCKED\n\n## Blocked\nNo API key.\n'
step factory/bin/job advance "$ID" needs-human
printf '### feedback\n'
printf 'Use the staging key.\n' | factory/bin/job feedback "$ID"; printf 'exit=%s\n' "$?"
agent_writes $J/build.md 'STATUS: BLOCKED\n\n## Blocked\nStill no key.\n'
step factory/bin/job advance "$ID" needs-human
printf '### reject\n'
printf 'Not worth it.\n' | factory/bin/job reject "$ID"; printf 'exit=%s\n' "$?"
step factory/bin/job summary
