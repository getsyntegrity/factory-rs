//! `job queue`: what `/factory run` should do with each labeled issue. First
//! moves jobs whose PR merged on GitHub to `merged`; then prints one row per
//! issue or job, tab-separated, in the order to start them.

use std::cmp::Ordering;

use serde_json::Value;

use super::advance::{TERMINAL, WAITING, advance, is_running, pr_state};
use super::issue::{fetch_issue, replied_since_question};
use super::shell::Shell;
use super::sync::{field, sync};
use super::{Job, Workspace, all_jobs, save};
use crate::py;

/// One printed row: action, issue, job id, stage, note.
type Row = [String; 5];

struct Entry {
    issue: Value,
    job: Job,
    file: String,
}

impl Entry {
    fn get(&self, key: &str) -> Result<&Value, String> {
        field(&self.job, &self.file, key)
    }

    fn text(&self, key: &str) -> Result<String, String> {
        self.get(key).map(py::str_of)
    }

    fn row(&self, action: &str, note: String) -> Result<Row, String> {
        Ok([
            action.into(),
            py::str_of(&self.issue),
            self.text("id")?,
            self.text("stage")?,
            note,
        ])
    }
}

/// Python orders the issue numbers (ints) numerically.
fn by_number(a: &Value, b: &Value) -> Ordering {
    let n = |v: &Value| v.as_f64().unwrap_or(f64::NAN);
    n(a).total_cmp(&n(b))
}

pub fn cmd_queue(config: &Value, ws: &dyn Workspace) -> Result<String, String> {
    let labels = config
        .get("labels")
        .ok_or("factory/config.json has no labels")?;
    let queue = py::str_of(
        labels
            .get("queue")
            .ok_or("factory/config.json labels has no queue")?,
    );
    let concurrency = config
        .get("concurrency")
        .ok_or("factory/config.json has no concurrency")?;
    let shell = Shell::new(ws);

    // `{j["issue"]: j for j in all_jobs()}`: a later job with the same issue
    // replaces an earlier one in its place.
    let mut jobs: Vec<Entry> = Vec::new();
    for (job, file) in all_jobs(ws)? {
        let issue = field(&job, &file, "issue")?.clone();
        let entry = Entry { issue, job, file };
        match jobs.iter_mut().find(|e| e.issue == entry.issue) {
            Some(slot) => *slot = entry,
            None => jobs.push(entry),
        }
    }

    // A job is done once its PR is merged on GitHub.
    for entry in &mut jobs {
        let stage = entry.text("stage")?;
        let pr = entry.get("pr")?.clone();
        if !TERMINAL.contains(&stage.as_str())
            && py::truthy(&pr)
            && pr_state(&shell, &pr)? == "MERGED"
        {
            sync(&mut entry.job, &entry.file, config, ws)?;
            advance(&mut entry.job, &entry.file, "merged", config, ws, &shell)?;
            save(&entry.job, &entry.file, ws)?;
        }
    }

    let issues = shell.gh_json(&[
        "issue",
        "list",
        "--label",
        &queue,
        "--state",
        "open",
        "--limit",
        "200",
        "--json",
        "number,title",
    ])?;
    let mut issues: Vec<Value> = issues.as_array().cloned().unwrap_or_default();
    issues.sort_by(|a, b| by_number(&a["number"], &b["number"]));
    let open: Vec<&Value> = issues.iter().map(|i| &i["number"]).collect();

    let (mut running_rows, mut replied_rows, mut new_rows, mut other_rows) =
        (vec![], vec![], vec![], vec![]);
    for issue in &issues {
        let n = &issue["number"];
        let Some(entry) = jobs.iter().find(|e| e.issue == *n) else {
            let row = [
                "new".into(),
                py::str_of(n),
                "-".into(),
                "-".into(),
                py::str_of(&issue["title"]),
            ];
            new_rows.push(row);
            continue;
        };
        match entry.text("stage")?.as_str() {
            "needs-info" => {
                if replied_since_question(&fetch_issue(&shell, &py::str_of(n))?, &shell)? {
                    replied_rows.push(entry.row("resume", "author replied".into())?);
                } else {
                    other_rows.push(entry.row("wait", "waiting for the author".into())?);
                }
            }
            "needs-human" => {
                let pr = entry.get("pr")?;
                let note = if py::truthy(pr) {
                    format!("merge PR #{}, or give feedback", py::str_of(pr))
                } else {
                    "give feedback".into()
                };
                other_rows.push(entry.row("wait", note)?);
            }
            "rejected" => {
                let note = format!(
                    "rejected; delete factory/jobs/{} to retry",
                    entry.text("id")?
                );
                other_rows.push(entry.row("skip", note)?);
            }
            "merged" => other_rows.push(entry.row("skip", "merged".into())?),
            _ => running_rows.push(entry.row("resume", format!("round {}", entry.text("round")?))?),
        }
    }

    let mut by_issue: Vec<&Entry> = jobs.iter().collect();
    by_issue.sort_by(|a, b| by_number(&a.issue, &b.issue));
    for entry in by_issue {
        let stage = entry.text("stage")?;
        if !open.contains(&&entry.issue)
            && !TERMINAL.contains(&stage.as_str())
            && !WAITING.contains(&stage.as_str())
        {
            other_rows.push(entry.row(
                "skip",
                format!("issue is closed or no longer labeled {queue}"),
            )?);
        }
    }

    // Jobs in a running stage hold a slot whether or not an agent is on them.
    let mut running = 0;
    for entry in &jobs {
        if is_running(&entry.text("stage")?) {
            running += 1;
        }
    }
    let mut slots = concurrency.as_i64().unwrap_or(0) - running;
    let mut rows = running_rows;
    for row in replied_rows.into_iter().chain(new_rows) {
        if slots > 0 {
            rows.push(row);
            slots -= 1;
        } else {
            let [_, n, id, stage, _] = row;
            rows.push([
                "hold".into(),
                n,
                id,
                stage,
                format!("concurrency {} reached", py::str_of(concurrency)),
            ]);
        }
    }
    Ok(rows
        .iter()
        .chain(&other_rows)
        .map(|row| row.join("\t") + "\n")
        .collect())
}

#[cfg(test)]
mod tests {
    use crate::Outcome;
    use crate::job::fake::case;
    use crate::job::{Command, load_config, run};

    fn queue(fake: &crate::job::fake::Fake) -> Outcome {
        let config = load_config(fake).unwrap();
        run(&Command::Queue, &config, fake)
    }

    macro_rules! recorded {
        ($test:ident, $case:literal) => {
            #[test]
            fn $test() {
                let (fake, want) = case!("job-queue", $case);
                assert_eq!(queue(&fake), want.outcome);
                fake.assert_files(&want);
            }
        };
    }

    recorded!(
        queue_01_new_issues_fill_the_slots_then_hold,
        "queue-01-new-issues-and"
    );
    recorded!(queue_02_every_row_kind_in_order, "queue-02-every-row-kind");
    recorded!(queue_03_merged_prs_finish_their_jobs, "queue-03-auto-merge");
    recorded!(queue_04_issue_list_failure, "queue-04-issue-list-fail");

    /// Accepted divergence DIV-JOB-07: v2 crashes (JSONDecodeError) when
    /// `gh issue list` prints something that is not JSON.
    #[test]
    fn div_job_07_issue_list_that_is_not_json_is_named() {
        let (fake, want) = case!("job-queue", "queue-06-invalid-json");
        let out = queue(&fake);
        assert_eq!(
            out,
            Outcome::die(
                "job",
                "`gh issue list --label factory --state open --limit 200 --json number,title` did not print JSON (line 1, column 2)"
            )
        );
        assert_eq!(out.code, want.outcome.code);
        fake.assert_files(&want);
    }
}
