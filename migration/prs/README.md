# Stacked pull requests (to open by hand)

The branches are pushed to `origin` (getsyntegrity/factory-rs). No PR was
opened from the agent session: opening them needs a `gh` session, which was
not authorized for the agent (see `odd/tasks/verification-gaps.md`, L2).

Order and bases (each PR targets the previous branch, so each diff shows one
slice). `feat/job-sync-agents` has no PR yet; PR 0 makes the stack start at
`develop`. Do not merge out of order.

| # | Head | Base | Title | Body |
| --- | --- | --- | --- | --- |
| 0 | `feat/job-sync-agents` | `develop` | feat(job): port job sync and agents | [pr-0.md](pr-0.md) |
| 1 | `feat/gate` | `feat/job-sync-agents` | feat(gate): port gate to Rust | [pr-1.md](pr-1.md) |
| 2 | `feat/job-waiting-commands` | `feat/gate` | feat(job): port summary, feedback and reject | [pr-2.md](pr-2.md) |
| 3 | `feat/job-create-queue` | `feat/job-waiting-commands` | feat(job): port create and queue | [pr-3.md](pr-3.md) |

To open them yourself:

```sh
gh pr create --repo getsyntegrity/factory-rs --draft --head feat/job-sync-agents --base develop --title "feat(job): port job sync and agents" --body-file migration/prs/pr-0.md
gh pr create --repo getsyntegrity/factory-rs --draft --head feat/gate --base feat/job-sync-agents --title "feat(gate): port gate to Rust" --body-file migration/prs/pr-1.md
gh pr create --repo getsyntegrity/factory-rs --draft --head feat/job-waiting-commands --base feat/gate --title "feat(job): port summary, feedback and reject" --body-file migration/prs/pr-2.md
gh pr create --repo getsyntegrity/factory-rs --draft --head feat/job-create-queue --base feat/job-waiting-commands --title "feat(job): port create and queue" --body-file migration/prs/pr-3.md
```
