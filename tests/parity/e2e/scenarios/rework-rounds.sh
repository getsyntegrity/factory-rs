# gate fails → build (round 2) → gate passes → review wants changes →
# build (round 3) → gate → review fails again at the round limit →
# needs-human.
. "$(dirname "$0")/lib.sh"
step factory/bin/job create 5
ID=$(job_id)
J=factory/jobs/$ID
agent_writes $J/intake.md 'DECISION: ACCEPT\nRISK: medium\nSURFACES: security\n'
step factory/bin/route "$ID"
step factory/bin/job advance "$ID" spec
mkdir -p ".worktrees/$ID"
agent_writes $J/spec.md '# Spec\n'
step factory/bin/job advance "$ID" build
agent_writes $J/build.md 'STATUS: READY\n'
step factory/bin/job advance "$ID" gate
touch .fail-checks
step factory/bin/gate "$ID"
rm .fail-checks
step factory/bin/job advance "$ID" build
agent_writes $J/build.md 'STATUS: READY\n'
step factory/bin/job advance "$ID" gate
step factory/bin/gate "$ID"
step factory/bin/job advance "$ID" review
agent_writes $J/round-2/review-code.md 'VERDICT: CHANGES\n\nRename it.\n'
agent_writes $J/round-2/review-security.md 'VERDICT: PASS\n'
step factory/bin/job advance "$ID" build
agent_writes $J/build.md 'STATUS: READY\n'
step factory/bin/job advance "$ID" gate
step factory/bin/gate "$ID"
step factory/bin/job advance "$ID" review
agent_writes $J/round-3/review-code.md 'VERDICT: CHANGES\n\nStill not right.\n'
agent_writes $J/round-3/review-security.md 'VERDICT: PASS\n'
step factory/bin/job advance "$ID" needs-human
step factory/bin/job summary
