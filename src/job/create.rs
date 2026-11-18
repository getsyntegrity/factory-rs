//! `job create <issue>`: a new job for an open issue labeled for the factory;
//! prints the job id.

use serde_json::{Value, json};

use super::advance::is_running;
use super::issue::{fetch_issue, get, slugify, write_issue_md};
use super::shell::Shell;
use super::sync::field;
use super::{Workspace, all_jobs, clock, save};
use crate::py;

pub fn cmd_create(number: &str, config: &Value, ws: &dyn Workspace) -> Result<String, String> {
    let config_value = |key: &str| {
        config
            .get(key)
            .ok_or_else(|| format!("factory/config.json has no {key}"))
    };
    let wanted: Option<i64> = number.parse().ok();
    let jobs = all_jobs(ws)?;
    let mut existing = None;
    for (job, file) in &jobs {
        if wanted.is_some() && field(job, file, "issue")?.as_i64() == wanted && existing.is_none() {
            existing = Some((job, file));
        }
    }
    if let Some((job, file)) = existing {
        return Err(format!(
            "issue #{number} already has job {} ({})",
            py::str_of(field(job, file, "id")?),
            py::str_of(field(job, file, "stage")?)
        ));
    }

    // v2 `require_slot(None)`.
    let mut busy = Vec::new();
    for (job, file) in &jobs {
        if is_running(&py::str_of(field(job, file, "stage")?)) {
            busy.push(py::str_of(field(job, file, "id")?));
        }
    }
    let concurrency = config_value("concurrency")?;
    if (busy.len() as i64) >= concurrency.as_i64().unwrap_or(0) {
        return Err(format!(
            "concurrency {} reached; running: {}",
            py::str_of(concurrency),
            busy.join(", ")
        ));
    }

    let shell = Shell::new(ws);
    let issue = fetch_issue(&shell, number)?;
    let state = py::str_of(get(&issue, number, &["state"])?);
    if state != "OPEN" {
        return Err(format!("issue #{number} is {}", state.to_lowercase()));
    }
    let queue = py::str_of(
        config_value("labels")?
            .get("queue")
            .ok_or_else(|| "factory/config.json labels has no queue".to_string())?,
    );
    let labeled = get(&issue, number, &["labels"])?
        .as_array()
        .is_some_and(|labels| {
            labels
                .iter()
                .any(|l| l.get("name").map(py::str_of) == Some(queue.clone()))
        });
    if !labeled {
        return Err(format!("issue #{number} isn't labeled {queue}"));
    }

    ws.mkdir("", true)
        .map_err(|_| "cannot create factory/jobs".to_string())?;
    let mut highest = 0;
    for name in ws
        .list("")
        .map_err(|_| "cannot list factory/jobs".to_string())?
    {
        let head: String = name.chars().take(3).collect();
        if head.is_empty() || !head.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        let n: i64 = name
            .split('-')
            .next()
            .and_then(|n| n.parse().ok())
            .ok_or_else(|| format!("factory/jobs/{name} does not start with a job number"))?;
        highest = highest.max(n);
    }
    let title = get(&issue, number, &["title"])?;
    let id = format!("{:03}-{}", highest + 1, slugify(&py::str_of(title)));
    ws.mkdir(&id, false)
        .map_err(|_| format!("factory/jobs/{id} already exists"))?;

    let job = json!({
        "id": id, "issue": get(&issue, number, &["number"])?, "title": title,
        "issue_url": get(&issue, number, &["url"])?,
        "pr": null, "pr_url": null,
        "branch": format!("factory/{id}"),
        "worktree": format!("{}/.worktrees/{id}", ws.repo_root()),
        "stage": "intake", "round": 0, "round_limit": config_value("max_rounds")?,
        "intake": null, "agents": null, "gate": null, "reviews": {},
        "cost_usd": 0.0, "started": clock::format_utc(ws.now()), "ended": null,
    });
    let issue_number = py::str_of(&job["issue"]);
    write_issue_md(&id, &issue_number, ws, &shell)?;
    let job = job.as_object().cloned().unwrap_or_default();
    save(&job, &format!("factory/jobs/{id}/job.json"), ws)?;
    Ok(format!("{id}\n"))
}

#[cfg(test)]
mod tests {
    use crate::Outcome;
    use crate::job::fake::{Fake, case};
    use crate::job::{Command, load_config, run};

    fn create(fake: &Fake, issue: &str) -> Outcome {
        let config = load_config(fake).unwrap();
        run(
            &Command::Create {
                issue: issue.into(),
            },
            &config,
            fake,
        )
    }

    macro_rules! recorded {
        ($test:ident, $case:literal, $issue:literal) => {
            #[test]
            fn $test() {
                let (fake, want) = case!("job-queue", $case);
                assert_eq!(create(&fake, $issue), want.outcome);
                fake.assert_files(&want);
            }
        };
    }

    recorded!(
        create_01_writes_job_and_quoted_issue,
        "create-01-new-job",
        "5"
    );
    recorded!(
        create_02_numbers_after_the_highest_folder,
        "create-02-next-number",
        "5"
    );
    recorded!(create_03_one_job_per_issue, "create-03-already-has-jo", "5");
    recorded!(
        create_04_respects_concurrency,
        "create-04-concurrency-re",
        "5"
    );
    recorded!(create_05_only_open_issues, "create-05-closed-issue", "5");
    recorded!(create_06_only_labeled_issues, "create-06-not-labeled", "5");
    recorded!(
        create_07_wordless_title_and_null_body,
        "create-07-wordless-title",
        "5"
    );
    recorded!(
        create_08_issue_view_failure,
        "create-08-issue-view-fai",
        "5"
    );
    recorded!(
        create_10_number_as_given_then_as_returned,
        "create-10-zero-padded-nu",
        "005"
    );

    #[test]
    fn started_is_now_in_v2_format() {
        let (fake, _) = case!("job-queue", "create-01-new-job");
        create(&fake, "5");
        let job: serde_json::Value =
            serde_json::from_str(&fake.written("001-add-dark-mode-toggle/job.json").unwrap())
                .unwrap();
        assert_eq!(job["started"], "2026-11-18T03:04:05Z");
        assert_eq!(job["worktree"], "/repo/.worktrees/001-add-dark-mode-toggle");
    }

    #[test]
    fn folder_names_with_digits_but_no_number_are_named() {
        let fake = Fake::new()
            .file("123abc/notes.md", "x")
            .sandbox_file("doubles/gh/api_user.stdout", "op\n")
            .sandbox_file(
                "doubles/gh/issue_view_5.stdout",
                r#"{"number": 5, "title": "T", "state": "OPEN", "labels": [{"name": "factory"}]}"#,
            );
        let config = r#"{"labels": {"queue": "factory"}, "base_branch": "main", "concurrency": 3,
                         "max_rounds": 3, "prices_usd_per_mtok": {}}"#;
        let out = create(&fake.config(config), "5");
        assert_eq!(
            out,
            Outcome::die(
                "job",
                "factory/jobs/123abc does not start with a job number"
            )
        );
    }
}
