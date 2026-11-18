//! Commands for jobs waiting on a person: `job feedback <job>` (needs-human
//! → build, with the feedback from stdin) and `job reject <job>`
//! (needs-info / needs-human → rejected, with the reason from stdin).

use std::io;

use serde_json::Value;

use super::advance::advance;
use super::shell::Shell;
use super::sync::{field, sync};
use super::{Job, Workspace, resolve, save};
use crate::py;

/// The job, its job.json path, its id and stage (as v2 prints them), and the
/// stripped text from stdin.
struct Request {
    job: Job,
    file: String,
    id: String,
    stage: String,
    text: String,
}

fn request(arg: &str, ws: &dyn Workspace) -> Result<Request, String> {
    let (job, file) = resolve(arg, ws)?;
    let text = ws.stdin().map_err(|_| "cannot read stdin".to_string())?;
    let id = py::str_of(field(&job, &file, "id")?);
    let stage = py::str_of(field(&job, &file, "stage")?);
    Ok(Request {
        text: py::strip(&text).to_string(),
        job,
        file,
        id,
        stage,
    })
}

fn write(ws: &dyn Workspace, path: &str, text: &str) -> Result<(), String> {
    ws.write(path, text)
        .map_err(|_| format!("cannot write factory/jobs/{path}"))
}

/// `job feedback <job>`: needs-human → build, with stdin as feedback-<k>.md.
pub fn cmd_feedback(arg: &str, config: &Value, ws: &dyn Workspace) -> Result<String, String> {
    let Request {
        mut job,
        file,
        id,
        stage,
        text,
    } = request(arg, ws)?;
    if stage != "needs-human" {
        return Err(format!("{id} is in {stage}, not needs-human"));
    }
    if text.is_empty() {
        return Err("no feedback on stdin".into());
    }
    let existing = match ws.list(&id) {
        Ok(names) => names,
        Err(e) if e.kind() == io::ErrorKind::NotFound => Vec::new(),
        Err(_) => return Err(format!("cannot list factory/jobs/{id}")),
    };
    let k = existing
        .iter()
        .filter(|n| n.starts_with("feedback-") && n.ends_with(".md") && n.len() >= 12)
        .count()
        + 1;
    let shell = Shell::new(ws);
    let body = format!(
        "# Feedback {k}\n\nFrom @{} after the job escalated.\n\n{text}\n",
        shell.operator()?
    );
    write(ws, &format!("{id}/feedback-{k}.md"), &body)?;
    sync(&mut job, &file, config, ws)?;
    advance(&mut job, &file, "build", config, ws, &shell)?;
    save(&job, &file, ws)?;
    let round = py::str_of(field(&job, &file, "round")?);
    Ok(format!("{id}: feedback-{k}.md, round {round}\n"))
}

/// `job reject <job>`: needs-info / needs-human → rejected, with stdin as
/// the reason in rejection.md.
pub fn cmd_reject(arg: &str, config: &Value, ws: &dyn Workspace) -> Result<String, String> {
    let Request {
        mut job,
        file,
        id,
        stage,
        text,
    } = request(arg, ws)?;
    if stage != "needs-info" && stage != "needs-human" {
        return Err(format!("{id} is in {stage}, not needs-info or needs-human"));
    }
    if text.is_empty() {
        return Err("no reason on stdin".into());
    }
    let shell = Shell::new(ws);
    let body = format!(
        "REJECTED\n\nBy @{} while the job was {stage}.\n\n{text}\n",
        shell.operator()?
    );
    write(ws, &format!("{id}/rejection.md"), &body)?;
    sync(&mut job, &file, config, ws)?;
    advance(&mut job, &file, "rejected", config, ws, &shell)?;
    save(&job, &file, ws)?;
    Ok(format!("{id}: rejected\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Outcome;
    use crate::job::fake::{Fake, case};
    use crate::job::{Command, load_config, run};

    fn feedback(fake: &Fake, job: &str) -> Outcome {
        let config = load_config(fake).unwrap();
        run(&Command::Feedback { job: job.into() }, &config, fake)
    }

    fn reject(fake: &Fake, job: &str) -> Outcome {
        let config = load_config(fake).unwrap();
        run(&Command::Reject { job: job.into() }, &config, fake)
    }

    macro_rules! recorded {
        ($test:ident, $cmd:ident, $harness:literal, $case:literal, $job:literal) => {
            #[test]
            fn $test() {
                let (fake, want) = case!($harness, $case);
                assert_eq!($cmd(&fake, $job), want.outcome);
                fake.assert_files(&want);
            }
        };
    }

    const ID: &str = "001-add-a-thing";

    recorded!(
        feedback_01_sends_the_job_back_to_build,
        feedback,
        "job",
        "feedback-01-sends-back",
        "001-add-a-thing"
    );
    recorded!(
        feedback_02_numbers_the_next_file,
        feedback,
        "job",
        "feedback-02-second-feedb",
        "001-add-a-thing"
    );
    recorded!(
        feedback_03_only_needs_human,
        feedback,
        "job",
        "feedback-03-wrong-stage",
        "001-add-a-thing"
    );
    recorded!(
        feedback_04_needs_text,
        feedback,
        "job",
        "feedback-04-empty-stdin",
        "001-add-a-thing"
    );
    recorded!(
        feedback_05_operator_failure,
        feedback,
        "job",
        "feedback-05-operator-fai",
        "001-add-a-thing"
    );
    recorded!(
        feedback_06_label_failure_keeps_the_file_not_the_job,
        feedback,
        "job",
        "feedback-06-label-fails",
        "001-add-a-thing"
    );
    recorded!(
        feedback_08_by_issue_number,
        feedback,
        "job",
        "feedback-08-by-issue",
        "1"
    );
    recorded!(
        reject_01_needs_info,
        reject,
        "job-reject",
        "reject-01-needs-info",
        "001-add-a-thing"
    );
    recorded!(
        reject_02_closes_pr_and_removes_worktree,
        reject,
        "job-reject",
        "reject-02-needs-human-wi",
        "001-add-a-thing"
    );
    recorded!(
        reject_03_only_waiting_jobs,
        reject,
        "job-reject",
        "reject-03-wrong-stage",
        "001-add-a-thing"
    );
    recorded!(
        reject_04_needs_a_reason,
        reject,
        "job-reject",
        "reject-04-empty-stdin",
        "001-add-a-thing"
    );
    recorded!(
        reject_05_comment_failure,
        reject,
        "job-reject",
        "reject-05-comment-fails",
        "001-add-a-thing"
    );
    recorded!(
        reject_06_worktree_removal_failure,
        reject,
        "job-reject",
        "reject-06-worktree-remov",
        "001-add-a-thing"
    );
    recorded!(
        reject_07_pr_close_failure,
        reject,
        "job-reject",
        "reject-07-pr-close-fails",
        "001-add-a-thing"
    );

    #[test]
    fn reject_writes_the_exact_times_the_recording_normalized() {
        let (fake, _) = case!("job-reject", "reject-01-needs-info");
        reject(&fake, ID);
        let job: Value =
            serde_json::from_str(&fake.written(&format!("{ID}/job.json")).unwrap()).unwrap();
        assert_eq!(job["started"], "2026-11-18T00:00:00Z");
        assert_eq!(job["ended"], "2026-11-18T03:04:05Z");
    }

    /// Accepted divergence DIV-JOB-06: v2 crashes (FileNotFoundError) after
    /// writing the feedback file; Rust keeps those effects and names the
    /// cause.
    #[test]
    fn div_job_06_feedback_without_decision_md_is_named() {
        let (fake, want) = case!("feedback-07-no-decision");
        let out = feedback(&fake, ID);
        assert_eq!(
            out,
            Outcome::die("job", "factory/jobs/001-add-a-thing/decision.md is missing")
        );
        assert_eq!(out.code, want.outcome.code);
        fake.assert_files(&want);
    }
}
