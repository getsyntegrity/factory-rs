//! v2 `advance(job, stage)`: the job state machine. Only the transitions the
//! ported commands use are in Rust so far: needs-human → build (`feedback`)
//! and needs-info / needs-human → rejected (`reject`). Every other legal
//! transition still runs in the v2 fallback (`job advance`), and reaching one
//! here is refused instead of guessed.

use serde_json::{Value, json};

use super::shell::Shell;
use super::sync::field;
use super::{Job, Workspace, clock};
use crate::py;

/// v2 `TRANSITIONS`: the stages each stage may move to.
const TRANSITIONS: [(&str, &[&str]); 9] = [
    ("intake", &["needs-info", "rejected", "spec"]),
    ("needs-info", &["intake", "rejected"]),
    ("spec", &["build"]),
    ("build", &["gate", "needs-human"]),
    ("gate", &["review", "build", "needs-human"]),
    ("review", &["approve", "build", "needs-human"]),
    ("approve", &["merging", "needs-human"]),
    ("merging", &["needs-human"]),
    ("needs-human", &["build", "rejected"]),
];
pub const TERMINAL: [&str; 2] = ["merged", "rejected"];
pub const WAITING: [&str; 2] = ["needs-info", "needs-human"];

/// v2 `running(jobs)`: jobs holding a concurrency slot (not finished, not
/// waiting).
pub fn is_running(stage: &str) -> bool {
    !TERMINAL.contains(&stage) && !WAITING.contains(&stage)
}

/// Moves `job` to `stage`, or refuses with v2's message.
pub fn advance(
    job: &mut Job,
    file: &str,
    stage: &str,
    config: &Value,
    ws: &dyn Workspace,
    shell: &Shell,
) -> Result<(), String> {
    let id = py::str_of(field(job, file, "id")?);
    let old = py::str_of(field(job, file, "stage")?);
    let legal = TRANSITIONS
        .iter()
        .any(|(from, to)| *from == old && to.contains(&stage))
        || (stage == "merged" && !TERMINAL.contains(&old.as_str()));
    if !legal {
        return Err(format!("{id}: {old} → {stage} is not a legal transition"));
    }
    let labels = |key: &str| -> Result<String, String> {
        let table = config
            .get("labels")
            .ok_or_else(|| "factory/config.json has no labels".to_string())?;
        table
            .get(key)
            .map(py::str_of)
            .ok_or_else(|| format!("factory/config.json labels has no {key}"))
    };

    if stage == "merged" {
        // `job["pr"] and pr_state(job) == "MERGED"`: GitHub is asked again.
        let pr = field(job, file, "pr")?;
        if !py::truthy(pr) || pr_state(shell, pr)? != "MERGED" {
            return Err("the PR isn't merged".into());
        }
        if old == "needs-human" {
            let issue = py::str_of(field(job, file, "issue")?);
            shell.label("issue", &issue, None, Some(&labels("needs_human")?))?;
        }
    } else if stage == "rejected" && WAITING.contains(&old.as_str()) {
        let Some(rejection) = ws
            .read(&format!("{id}/rejection.md"))
            .map_err(|_| format!("cannot read factory/jobs/{id}/rejection.md"))?
        else {
            return Err("use `job reject` to reject a waiting job".into());
        };
        let rejection = py::universal_newlines(&rejection);
        let Some((_, reason)) = rejection.split_once('\n') else {
            return Err(format!("factory/jobs/{id}/rejection.md has no reason"));
        };
        let issue = py::str_of(field(job, file, "issue")?);
        shell.comment(
            "issue",
            &issue,
            &format!("### Factory: rejected\n\n{}", py::strip(reason)),
        )?;
        let waiting = labels(if old == "needs-info" {
            "needs_info"
        } else {
            "needs_human"
        })?;
        shell.label(
            "issue",
            &issue,
            None,
            Some(&format!("{},{waiting}", labels("queue")?)),
        )?;
        let pr = field(job, file, "pr")?;
        if py::truthy(pr) {
            shell.gh(&["pr", "close", &py::str_of(pr)])?;
            let branch = py::str_of(field(job, file, "branch")?);
            shell.run("git", &["push", "origin", "--delete", &branch], false)?;
        }
    } else if old == "needs-human" {
        to_build(job, file, &id, config, ws)?;
        let issue = py::str_of(field(job, file, "issue")?);
        shell.label("issue", &issue, None, Some(&labels("needs_human")?))?;
    } else {
        return Err(format!(
            "{id}: {old} → {stage} is not ported to Rust yet; use the v2 job"
        ));
    }

    if TERMINAL.contains(&stage) {
        let worktree = py::str_of(field(job, file, "worktree")?);
        if ws.exists(&worktree) {
            shell.run("git", &["worktree", "remove", "--force", &worktree], true)?;
            let branch = py::str_of(field(job, file, "branch")?);
            shell.run("git", &["branch", "-D", &branch], false)?;
        }
    }

    job.insert("stage".into(), json!(stage));
    let ended = if TERMINAL.contains(&stage) || WAITING.contains(&stage) {
        json!(clock::format_utc(ws.now()))
    } else {
        Value::Null
    };
    job.insert("ended".into(), ended);
    Ok(())
}

/// v2 `pr_state(job)`: the PR's state on GitHub (`OPEN`, `MERGED`, ...).
pub fn pr_state(shell: &Shell, pr: &Value) -> Result<String, String> {
    let view = shell.gh_json(&["pr", "view", &py::str_of(pr), "--json", "state"])?;
    Ok(view.get("state").map(py::str_of).unwrap_or_default())
}

/// needs-human → build: only after `job feedback` wrote a feedback file
/// newer than decision.md; starts a round with `max_rounds` more to go. The
/// caller then drops the needs-human label.
fn to_build(
    job: &mut Job,
    file: &str,
    id: &str,
    config: &Value,
    ws: &dyn Workspace,
) -> Result<(), String> {
    let mut feedback: Vec<String> = ws
        .list(id)
        .map_err(|_| format!("cannot list factory/jobs/{id}"))?
        .into_iter()
        .filter(|n| n.starts_with("feedback-") && n.ends_with(".md") && n.len() >= 12)
        .collect();
    feedback.sort();
    let mtime = |name: &str| {
        ws.mtime(&format!("{id}/{name}"))
            .map_err(|_| format!("factory/jobs/{id}/{name} is missing"))
    };
    let newer = match feedback.last() {
        Some(last) => mtime(last)? > mtime("decision.md")?,
        None => false,
    };
    if !newer {
        return Err("use `job feedback` to send a job back to the builder".into());
    }
    let round = field(job, file, "round")?
        .as_i64()
        .ok_or_else(|| format!("{file} has a round that is not a number"))?;
    let max_rounds = config
        .get("max_rounds")
        .and_then(Value::as_i64)
        .ok_or_else(|| "factory/config.json has no max_rounds".to_string())?;
    job.insert("round_limit".into(), json!(round + max_rounds));
    next_round(job, id, round, ws)
}

/// v2 `next_round()`.
fn next_round(job: &mut Job, id: &str, round: i64, ws: &dyn Workspace) -> Result<(), String> {
    let round = round + 1;
    job.insert("round".into(), json!(round));
    ws.mkdir(&format!("{id}/round-{round}"), true)
        .map_err(|_| format!("cannot create factory/jobs/{id}/round-{round}"))?;
    job.insert("gate".into(), Value::Null);
    job.insert("reviews".into(), json!({}));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job::fake::{Fake, NOW};

    fn config() -> Value {
        json!({"labels": {"queue": "factory", "needs_info": "factory:needs-info", "needs_human": "factory:needs-human"},
               "max_rounds": 3})
    }

    fn job(json: Value) -> Job {
        json.as_object().cloned().unwrap()
    }

    fn go(fake: &Fake, j: &mut Job, stage: &str) -> Result<(), String> {
        advance(
            j,
            "factory/jobs/a/job.json",
            stage,
            &config(),
            fake,
            &Shell::new(fake),
        )
    }

    #[test]
    fn illegal_transitions_are_refused_like_v2() {
        let fake = Fake::new();
        let mut j = job(json!({"id": "a", "stage": "spec"}));
        assert_eq!(
            go(&fake, &mut j, "approve"),
            Err("a: spec → approve is not a legal transition".into())
        );
        let mut j = job(json!({"id": "a", "stage": "rejected"}));
        assert_eq!(
            go(&fake, &mut j, "merged"),
            Err("a: rejected → merged is not a legal transition".into())
        );
        let mut j = job(json!({"id": "a", "stage": "nowhere"}));
        assert_eq!(
            go(&fake, &mut j, "build"),
            Err("a: nowhere → build is not a legal transition".into())
        );
    }

    #[test]
    fn legal_transitions_not_ported_yet_are_refused_not_guessed() {
        let fake = Fake::new();
        let mut j = job(json!({"id": "a", "stage": "spec"}));
        assert_eq!(
            go(&fake, &mut j, "build"),
            Err("a: spec → build is not ported to Rust yet; use the v2 job".into())
        );
        let mut j = job(json!({"id": "a", "stage": "build"}));
        assert_eq!(
            go(&fake, &mut j, "gate"),
            Err("a: build → gate is not ported to Rust yet; use the v2 job".into())
        );
        assert!(fake.written("doubles.log").is_none(), "nothing ran");
    }

    #[test]
    fn needs_human_to_build_uses_the_last_feedback_by_name() {
        // sorted() puts feedback-10 before feedback-9; v2 compares feedback-9.
        let fake = Fake::new()
            .file("a/decision.md", "ESCALATE\n")
            .file("a/feedback-9.md", "old")
            .file("a/feedback-10.md", "old");
        fake.write("a/feedback-10.md", "new").unwrap();
        let mut j = job(json!({"id": "a", "issue": 5, "stage": "needs-human", "round": 3}));
        assert_eq!(
            go(&fake, &mut j, "build"),
            Err("use `job feedback` to send a job back to the builder".into())
        );
    }

    #[test]
    fn needs_human_to_build_starts_a_new_round_with_a_new_limit() {
        let fake = Fake::new().file("a/decision.md", "ESCALATE\n");
        fake.write("a/feedback-1.md", "new").unwrap();
        let mut j = job(
            json!({"id": "a", "issue": 5, "stage": "needs-human", "round": 3, "round_limit": 3,
                               "gate": "FAIL", "reviews": {"code": "CHANGES"}, "ended": "x"}),
        );
        go(&fake, &mut j, "build").unwrap();
        assert_eq!(
            Value::Object(j),
            json!({"id": "a", "issue": 5, "stage": "build", "round": 4, "round_limit": 6,
                   "gate": null, "reviews": {}, "ended": null})
        );
        assert_eq!(fake.dirs(), ["a/round-4"]);
        assert_eq!(
            fake.written("doubles.log").unwrap(),
            "$ gh (cwd .)\n  [issue]\n  [edit]\n  [5]\n  [--remove-label]\n  [factory:needs-human]\n"
        );
    }

    #[test]
    fn rejecting_stamps_ended_with_now_in_v2_format() {
        let fake = Fake::new().file("a/rejection.md", "REJECTED\n\nno\n");
        let mut j = job(
            json!({"id": "a", "issue": 5, "stage": "needs-info", "pr": null,
                               "worktree": "gone", "branch": "factory/a", "ended": null}),
        );
        go(&fake, &mut j, "rejected").unwrap();
        assert_eq!(j["stage"], json!("rejected"));
        assert_eq!(j["ended"], json!(clock::format_utc(NOW)));
        assert_eq!(j["ended"], json!("2026-11-18T03:04:05Z"));
    }

    #[test]
    fn rejecting_needs_the_rejection_file() {
        let fake = Fake::new();
        let mut j = job(json!({"id": "a", "issue": 5, "stage": "needs-human"}));
        assert_eq!(
            go(&fake, &mut j, "rejected"),
            Err("use `job reject` to reject a waiting job".into())
        );
    }

    #[test]
    fn missing_config_entries_are_named() {
        let fake = Fake::new().file("a/decision.md", "ESCALATE\n");
        fake.write("a/feedback-1.md", "new").unwrap();
        let mut j = job(json!({"id": "a", "issue": 5, "stage": "needs-human", "round": 3}));
        let shell = Shell::new(&fake);
        assert_eq!(
            advance(&mut j, "f", "build", &json!({"labels": {}}), &fake, &shell),
            Err("factory/config.json has no max_rounds".into())
        );
    }

    #[test]
    fn merged_needs_a_merged_pr_and_asks_github_again() {
        let fake =
            Fake::new().sandbox_file("doubles/gh/pr_view_7.stdout", "{\"state\": \"OPEN\"}\n");
        let mut j = job(json!({"id": "a", "issue": 5, "stage": "review", "pr": 7}));
        assert_eq!(
            go(&fake, &mut j, "merged"),
            Err("the PR isn't merged".into())
        );
        let mut j = job(json!({"id": "a", "issue": 5, "stage": "review", "pr": null}));
        assert_eq!(
            go(&fake, &mut j, "merged"),
            Err("the PR isn't merged".into())
        );
    }

    #[test]
    fn merged_from_needs_human_drops_the_label() {
        let fake =
            Fake::new().sandbox_file("doubles/gh/pr_view_7.stdout", "{\"state\": \"MERGED\"}\n");
        let mut j = job(
            json!({"id": "a", "issue": 5, "stage": "needs-human", "pr": 7,
                               "worktree": "gone", "branch": "factory/a"}),
        );
        go(&fake, &mut j, "merged").unwrap();
        assert_eq!(j["stage"], json!("merged"));
        assert_eq!(j["ended"], json!("2026-11-18T03:04:05Z"));
        assert!(
            fake.written("doubles.log")
                .unwrap()
                .ends_with("  [--remove-label]\n  [factory:needs-human]\n")
        );
    }
}
