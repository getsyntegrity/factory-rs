# create → route → spec → build → gate → review → approve → queue → merge.
. "$(dirname "$0")/lib.sh"
step factory/bin/job create 5
ID=$(job_id)
J=factory/jobs/$ID
agent_writes $J/intake.md 'DECISION: ACCEPT\nRISK: high\nSURFACES: ui\nRESCOPED: no\n\n## Summary\n- Add the toggle.\n'
step factory/bin/route "$ID"
step factory/bin/job advance "$ID" spec
mkdir -p ".worktrees/$ID"
agent_writes $J/spec.md '# Spec\n\nA toggle.\n'
step factory/bin/job advance "$ID" build
agent_writes $J/build.md 'STATUS: READY\n\n## Summary\n- Added it.\n'
step factory/bin/job advance "$ID" gate
step factory/bin/gate "$ID"
step factory/bin/job advance "$ID" review
for r in code ux ui; do agent_writes $J/round-1/review-$r.md 'VERDICT: PASS\n\nFine.\n'; done
step factory/bin/job sync "$ID"
step factory/bin/job advance "$ID" approve
agent_writes $J/decision.md 'APPROVE\n\n## Why\nAll reviews passed.\n'
step factory/bin/job queue
step factory/bin/job merge "$ID"
step factory/bin/job summary
