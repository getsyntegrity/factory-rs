//! v2 `sync(job)`: refresh the fields derived from the job's files: intake
//! (intake.md header), gate (round-N/checks.md), reviews (round-N/review-*.md
//! verdicts, during review stages) and cost (Claude Code transcripts).

use std::collections::HashMap;

use serde_json::{Map, Value, json};

use super::{Job, Workspace, cost};
use crate::py;

/// Stages whose reviews are recomputed from the review files.
const REVIEW_STAGES: [&str; 5] = ["review", "approve", "merging", "merged", "needs-human"];

/// v2 `header()`: `KEY: value` lines at the top of an agent's file, up to the
/// first blank line. Keys are stripped and upper-cased; later keys win.
pub fn header(text: &str) -> HashMap<String, String> {
    let mut fields = HashMap::new();
    for line in py::splitlines(text) {
        if py::strip(line).is_empty() {
            break;
        }
        let (key, value) = line.split_once(':').unwrap_or((line, ""));
        fields.insert(py::strip(key).to_uppercase(), py::strip(value).to_string());
    }
    fields
}

/// v2 `re.fullmatch(r"VERDICT:\s*(PASS|CHANGES)", line)`.
fn verdict(line: &str) -> Option<&'static str> {
    let word = line
        .strip_prefix("VERDICT:")?
        .trim_start_matches(py::is_space);
    ["PASS", "CHANGES"].into_iter().find(|v| *v == word)
}

/// v2 `first_line()`: the stripped first line, `None` when the file is missing.
fn first_line(ws: &dyn Workspace, path: &str) -> Result<Option<String>, String> {
    Ok(read(ws, path)?.map(|text| {
        let text = py::universal_newlines(&text);
        py::strip(text.split('\n').next().unwrap_or_default()).to_string()
    }))
}

fn read(ws: &dyn Workspace, path: &str) -> Result<Option<String>, String> {
    ws.read(path)
        .map_err(|_| format!("cannot read factory/jobs/{path}"))
}

/// A job.json field v2 reads with `job[key]`.
pub fn field<'a>(job: &'a Job, file: &str, key: &str) -> Result<&'a Value, String> {
    job.get(key).ok_or_else(|| format!("{file} has no {key}"))
}

/// The job's id, which v2 joins into paths.
pub fn job_id(job: &Job, file: &str) -> Result<String, String> {
    field(job, file, "id")?
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| format!("{file} has an id that is not text"))
}

/// Refreshes `job` in place. `file` names the job.json for diagnostics.
pub fn sync(job: &mut Job, file: &str, config: &Value, ws: &dyn Workspace) -> Result<(), String> {
    let id = job_id(job, file)?;

    if let Some(text) = read(ws, &format!("{id}/intake.md"))? {
        let h = header(&py::universal_newlines(&text));
        let text_or_null = |key: &str| h.get(key).map_or(Value::Null, |v| json!(v));
        let surfaces: Vec<&str> = h
            .get("SURFACES")
            .map_or("", String::as_str)
            .split(',')
            .map(py::strip)
            .filter(|s| !s.is_empty() && *s != "none")
            .collect();
        job.insert(
            "intake".into(),
            json!({
                "decision": text_or_null("DECISION"),
                "rescoped": h.get("RESCOPED").map(String::as_str) == Some("yes"),
                "risk": text_or_null("RISK"),
                "surfaces": surfaces,
            }),
        );
    }

    let round = field(job, file, "round")?;
    if py::truthy(round) {
        let rd = format!("{id}/round-{}", py::str_of(round));
        let gate = read(ws, &format!("{rd}/checks.md"))?
            .and_then(|text| header(&py::universal_newlines(&text)).remove("GATE"))
            .map_or(Value::Null, Value::String);
        job.insert("gate".into(), gate);

        let agents = field(job, file, "agents")?;
        // `job["agents"] and job["stage"] in ...`: stage is read only when
        // agents is truthy.
        let in_review = py::truthy(agents)
            && field(job, file, "stage")?
                .as_str()
                .is_some_and(|s| REVIEW_STAGES.contains(&s));
        if in_review {
            let names: Vec<String> = match agents.get("reviewers") {
                Some(Value::Object(reviewers)) => reviewers.keys().cloned().collect(),
                _ => return Err(format!("{file} has no agents.reviewers table")),
            };
            let mut reviews = Map::new();
            for r in names {
                let state = match first_line(ws, &format!("{rd}/review-{r}.md"))? {
                    None => "pending",
                    Some(line) => verdict(&line).unwrap_or("invalid"),
                };
                reviews.insert(r, json!(state));
            }
            job.insert("reviews".into(), Value::Object(reviews));
        }
    }

    let prices = config
        .get("prices_usd_per_mtok")
        .and_then(Value::as_object)
        .ok_or_else(|| "factory/config.json has no prices_usd_per_mtok".to_string())?;
    let transcripts = ws
        .transcripts()
        .map_err(|_| "cannot read Claude Code transcripts".to_string())?;
    let cost = cost::cost_usd(&ws.job_folder(&id), &transcripts, prices)?;
    job.insert("cost_usd".into(), json!(cost));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_stops_at_the_first_blank_line() {
        let h = header("DECISION: ACCEPT\nRISK: low\n\nSURFACES: ui\n");
        assert_eq!(h.get("DECISION").map(String::as_str), Some("ACCEPT"));
        assert_eq!(h.get("RISK").map(String::as_str), Some("low"));
        assert_eq!(h.get("SURFACES"), None);
    }

    #[test]
    fn header_keys_are_stripped_and_upper_cased_and_values_stripped() {
        let h = header(" risk :  high \nno colon\nsurfaces:a:b\nrisk: low\n");
        assert_eq!(h.get("RISK").map(String::as_str), Some("low"));
        assert_eq!(h.get("NO COLON").map(String::as_str), Some(""));
        assert_eq!(h.get("SURFACES").map(String::as_str), Some("a:b"));
    }

    #[test]
    fn header_treats_whitespace_only_lines_as_blank_and_splits_like_python() {
        let h = header("A: 1\u{2028}B: 2\n \t\nC: 3");
        assert_eq!(h.len(), 2);
        assert_eq!(h.get("B").map(String::as_str), Some("2"));
    }

    #[test]
    fn verdict_accepts_only_exact_pass_or_changes() {
        assert_eq!(verdict("VERDICT: PASS"), Some("PASS"));
        assert_eq!(verdict("VERDICT:CHANGES"), Some("CHANGES"));
        assert_eq!(verdict("VERDICT:\u{a0}\u{1c}PASS"), Some("PASS"));
        for line in [
            "VERDICT: pass",
            "VERDICT: PASSED",
            "verdict: PASS",
            "VERDICT: PASS!",
            "",
        ] {
            assert_eq!(verdict(line), None, "{line}");
        }
    }
}
