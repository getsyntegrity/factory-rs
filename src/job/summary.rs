//! `job summary [<job>...]`: a table of jobs (issue, stage, round, PR, cost,
//! time) and the total cost. Jobs are synced for display only, never saved.

use serde_json::Value;

use super::sync::{field, sync};
use super::{Job, Workspace, all_jobs, clock, resolve};
use crate::py;

const HEADER: [&str; 7] = ["job", "issue", "stage", "round", "pr", "cost", "time"];

pub fn cmd_summary(args: &[String], config: &Value, ws: &dyn Workspace) -> Result<String, String> {
    // v2 loads (and locks) every job before syncing any.
    let mut jobs = if args.is_empty() {
        all_jobs(ws)?
    } else {
        args.iter()
            .map(|a| resolve(a, ws))
            .collect::<Result<_, _>>()?
    };

    let mut rows: Vec<Vec<String>> = vec![HEADER.iter().map(|h| h.to_string()).collect()];
    let mut total = 0.0;
    for (job, file) in &mut jobs {
        sync(job, file, config, ws)?; // for display only, never saved
        let cost = field(job, file, "cost_usd")?.as_f64().unwrap_or(0.0);
        total += cost;
        let pr = field(job, file, "pr")?;
        rows.push(vec![
            py::str_of(field(job, file, "id")?),
            format!("#{}", py::str_of(field(job, file, "issue")?)),
            py::str_of(field(job, file, "stage")?),
            py::str_of(field(job, file, "round")?),
            if py::truthy(pr) {
                format!("#{}", py::str_of(pr))
            } else {
                "-".into()
            },
            format!("${cost:.2}"),
            elapsed(job, file, ws)?,
        ]);
    }

    let widths: Vec<usize> = (0..HEADER.len())
        .map(|i| rows.iter().map(|r| r[i].chars().count()).max().unwrap_or(0))
        .collect();
    let mut out = String::new();
    for row in &rows {
        let cells: Vec<String> = row
            .iter()
            .zip(&widths)
            .map(|(cell, w)| format!("{cell}{}", " ".repeat(w - cell.chars().count())))
            .collect();
        out.push_str(py::rstrip(&cells.join("  ")));
        out.push('\n');
    }
    out.push_str(&format!(
        "total ${total:.2} (API-equivalent, factory agents only)\n"
    ));
    Ok(out)
}

/// v2 `elapsed(job)`: from `started` to `ended`, or to now while running.
fn elapsed(job: &Job, file: &str, ws: &dyn Workspace) -> Result<String, String> {
    let time = |key: &str| -> Result<i64, String> {
        let value = py::str_of(field(job, file, key)?);
        clock::parse_iso(&value)
            .ok_or_else(|| format!("{file} has a {key} time that is not ISO 8601: {value}"))
    };
    let start = time("started")?;
    let end = if py::truthy(field(job, file, "ended")?) {
        time("ended")?
    } else {
        ws.now()
    };
    Ok(clock::elapsed(start, end))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job::fake::{Fake, NOW, case};

    fn run(fake: &Fake, args: &[&str]) -> crate::Outcome {
        let config = crate::job::load_config(fake).unwrap();
        let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
        crate::job::run(&crate::job::Command::Summary { jobs: args }, &config, fake)
    }

    #[test]
    fn summary_01_no_jobs_prints_header_and_zero_total() {
        let (fake, want) = case!("summary-01-no-jobs");
        assert_eq!(run(&fake, &[]), want.outcome);
        fake.assert_files(&want);
    }

    #[test]
    fn summary_02_every_job_in_id_order_with_cost_and_time() {
        let (fake, want) = case!("summary-02-all-jobs");
        assert_eq!(run(&fake, &[]), want.outcome);
        fake.assert_files(&want);
    }

    #[test]
    fn summary_03_selected_jobs_in_argument_order() {
        let (fake, want) = case!("summary-03-selected-jobs");
        assert_eq!(run(&fake, &["12", "001-add-a-thing"]), want.outcome);
        fake.assert_files(&want);
    }

    #[test]
    fn summary_04_unknown_job_is_refused() {
        let (fake, want) = case!("summary-04-unknown-job");
        assert_eq!(run(&fake, &["001-add-a-thing", "999-nope"]), want.outcome);
        fake.assert_files(&want);
    }

    #[test]
    fn fallback_06_summary_of_one_ended_job() {
        let (fake, want) = case!("fallback-06-summary-ende");
        assert_eq!(run(&fake, &["001-add-a-thing"]), want.outcome);
        fake.assert_files(&want);
    }

    #[test]
    fn unfinished_jobs_run_until_now() {
        let started = clock::format_utc(NOW - 90 * 60 * 1_000_000);
        let fake = Fake::new().job(
            "001-a",
            &format!(
                r#"{{"id": "001-a", "issue": 1, "stage": "build", "round": 1, "pr": null,
                    "started": "{started}", "ended": null, "agents": null, "reviews": {{}}}}"#
            ),
        );
        let out = run(&fake, &[]);
        assert_eq!(
            out.stdout,
            "job    issue  stage  round  pr  cost   time\n\
             001-a  #1     build  1      -   $0.00  1h30m\n\
             total $0.00 (API-equivalent, factory agents only)\n"
        );
    }

    #[test]
    fn bad_start_times_are_named() {
        let fake = Fake::new().job(
            "001-a",
            r#"{"id": "001-a", "issue": 1, "stage": "build", "round": 1, "pr": null, "agents": null, "started": "soon", "ended": null}"#,
        );
        let out = run(&fake, &[]);
        assert_eq!(
            out.stderr,
            "job: factory/jobs/001-a/job.json has a started time that is not ISO 8601: soon\n"
        );
    }
}
