//! v2 `cost_usd()`: the API-equivalent cost of every Claude Code subagent
//! transcript whose first prompt names the job's folder.
//!
//! Each transcript is a JSONL file. Its first line is the prompt; factory
//! prompts start with `Job folder: <path>/`. Later lines carry the assistant
//! messages with model and token usage. A message id seen twice counts once,
//! with its last usage.

use serde_json::{Map, Value};

use crate::py;

/// Prices per million tokens, from config.json `prices_usd_per_mtok`.
pub type Prices = Map<String, Value>;

/// Sums the cost of the transcripts that belong to `job_folder` (absolute
/// path, no trailing slash), rounded like Python's `round(total, 2)`.
pub fn cost_usd(job_folder: &str, transcripts: &[String], prices: &Prices) -> Result<f64, String> {
    let needle = format!("Job folder: {job_folder}/");
    let mut total = 0.0;
    for text in transcripts {
        let text = py::universal_newlines(text);
        let mut lines = text.split_inclusive('\n');
        let Some(first) = lines.next() else { continue };
        if !prompt_text(&parse(first)?)?.contains(&needle) {
            continue;
        }
        // Python dict semantics: the first sighting fixes the position, the
        // last one the value.
        let mut usage_by_message: Vec<(String, Value, Value)> = Vec::new();
        for line in lines {
            let entry = parse(line)?;
            let msg = entry.get("message");
            let usage = msg.and_then(|m| m.get("usage"));
            if entry.get("type").and_then(Value::as_str) == Some("assistant")
                && msg.is_some_and(Value::is_object)
                && usage.is_some_and(py::truthy)
            {
                let msg = msg.unwrap_or(&Value::Null);
                let id = field(msg, "id")?.to_string();
                let model = field(msg, "model")?.clone();
                let usage = usage.cloned().unwrap_or_default();
                match usage_by_message.iter_mut().find(|(seen, _, _)| *seen == id) {
                    Some(slot) => *slot = (id, model, usage),
                    None => usage_by_message.push((id, model, usage)),
                }
            }
        }
        for (_, model, usage) in &usage_by_message {
            total += message_cost(model, usage, prices)?;
        }
    }
    // Python's round(x, 2) and Rust's `{:.2}` both round the exact binary
    // value to nearest, ties to even.
    Ok(format!("{total:.2}").parse().unwrap_or(total))
}

fn message_cost(model: &Value, usage: &Value, prices: &Prices) -> Result<f64, String> {
    let tokens = |key: &str| usage.get(key).and_then(Value::as_f64).unwrap_or(0.0);
    let input = tokens("input_tokens");
    let output = tokens("output_tokens");
    let cache_write = tokens("cache_creation_input_tokens");
    let cache_read = tokens("cache_read_input_tokens");
    if input + output + cache_write + cache_read == 0.0 {
        return Ok(0.0);
    }
    let name = py::str_of(model);
    let Some(p) = model.as_str().and_then(|m| prices.get(m)) else {
        return Err(format!(
            "no price for model {name} in factory/config.json prices_usd_per_mtok"
        ));
    };
    let price = |key: &str| {
        p.get(key).and_then(Value::as_f64).ok_or_else(|| {
            format!("factory/config.json prices_usd_per_mtok has no {key} price for {name}")
        })
    };
    let split = usage.get("cache_creation").filter(|v| py::truthy(v));
    let write_1h = split
        .and_then(|s| s.get("ephemeral_1h_input_tokens"))
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let write_5m = cache_write - write_1h;
    // Same operation order as v2, so the float sums are bit-identical.
    Ok((input * price("input")?
        + output * price("output")?
        + write_5m * price("cache_write_5m")?
        + write_1h * price("cache_write_1h")?
        + cache_read * price("cache_read")?)
        / 1e6)
}

fn parse(line: &str) -> Result<Value, String> {
    serde_json::from_str(line)
        .map_err(|_| "a Claude Code transcript line is not valid JSON".to_string())
}

fn field<'a>(msg: &'a Value, key: &str) -> Result<&'a Value, String> {
    msg.get(key)
        .ok_or_else(|| format!("a Claude Code transcript message has no {key}"))
}

/// v2 `prompt_text()`: the prompt's text, text blocks joined by a space.
fn prompt_text(first: &Value) -> Result<String, String> {
    let content = first
        .get("message")
        .and_then(|m| m.get("content"))
        .ok_or_else(|| "a Claude Code transcript prompt has no message content".to_string())?;
    Ok(match content {
        Value::String(s) => s.clone(),
        Value::Array(blocks) => blocks
            .iter()
            .filter(|b| b.is_object())
            .map(|b| b.get("text").and_then(Value::as_str).unwrap_or_default())
            .collect::<Vec<_>>()
            .join(" "),
        _ => String::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const FOLDER: &str = "/repo/factory/jobs/001-a";

    fn prices() -> Prices {
        json!({
            "claude-sonnet-5-5": {"input": 2.0, "output": 10.0, "cache_write_5m": 2.5, "cache_write_1h": 4.0, "cache_read": 0.20},
            "claude-haiku-5-5": {"input": 0.10, "output": 0.50, "cache_write_5m": 0.125, "cache_write_1h": 0.20, "cache_read": 0.01}
        })
        .as_object()
        .cloned()
        .unwrap()
    }

    fn lines(entries: &[Value]) -> String {
        entries.iter().map(|e| format!("{e}\n")).collect()
    }

    fn prompt(text: &str) -> Value {
        json!({"type": "user", "message": {"content": text}})
    }

    fn usage(id: &str, model: &str, usage: Value) -> Value {
        json!({"type": "assistant", "message": {"id": id, "model": model, "usage": usage}})
    }

    fn sonnet_turn(output: u64) -> Value {
        usage(
            "m1",
            "claude-sonnet-5-5",
            json!({"input_tokens": 10000, "output_tokens": output, "cache_creation_input_tokens": 20000,
                   "cache_read_input_tokens": 100000, "cache_creation": {"ephemeral_1h_input_tokens": 5000}}),
        )
    }

    #[test]
    fn no_transcripts_cost_nothing() {
        assert_eq!(cost_usd(FOLDER, &[], &prices()), Ok(0.0));
    }

    #[test]
    fn prices_every_token_kind_last_usage_per_message_wins_and_rounds_like_python() {
        let t = lines(&[
            prompt("Job folder: /repo/factory/jobs/001-a/\nGo."),
            sonnet_turn(5000),
            json!({"type": "user", "message": {"content": "tool result"}}),
            sonnet_turn(6000),
            usage(
                "m2",
                "claude-haiku-5-5",
                json!({"input_tokens": 333, "output_tokens": 777}),
            ),
        ]);
        // Python: 0.1575 + 0.0004218 = 0.1579218, round(_, 2) = 0.16
        assert_eq!(cost_usd(FOLDER, &[t], &prices()), Ok(0.16));
    }

    #[test]
    fn transcripts_of_other_jobs_and_empty_files_are_skipped() {
        let other = lines(&[
            prompt("Job folder: /repo/factory/jobs/001-ab/"),
            sonnet_turn(1),
        ]);
        let prefix_only = lines(&[
            prompt("Job folder: /repo/factory/jobs/001-a"),
            sonnet_turn(1),
        ]);
        let empty = String::new();
        assert_eq!(
            cost_usd(FOLDER, &[other, prefix_only, empty], &prices()),
            Ok(0.0)
        );
    }

    #[test]
    fn prompt_blocks_are_joined_and_non_dict_blocks_ignored() {
        let first = json!({"message": {"content": [
            {"type": "text", "text": "Context"}, "loose", {"type": "image"},
            {"text": "Job folder: /repo/factory/jobs/001-a/"}]}});
        let t = lines(&[
            first,
            usage("m", "claude-haiku-5-5", json!({"output_tokens": 20000})),
        ]);
        assert_eq!(cost_usd(FOLDER, &[t], &prices()), Ok(0.01));
    }

    #[test]
    fn needle_may_span_two_text_blocks_joined_by_a_space() {
        let first = json!({"message": {"content": [{"text": "Job folder:"}, {"text": "/repo/factory/jobs/001-a/"}]}});
        let t = lines(&[
            first,
            usage("m", "claude-haiku-5-5", json!({"output_tokens": 20000})),
        ]);
        assert_eq!(cost_usd(FOLDER, &[t], &prices()), Ok(0.01));
    }

    #[test]
    fn non_assistant_or_usage_less_entries_are_ignored() {
        let t = lines(&[
            prompt("Job folder: /repo/factory/jobs/001-a/"),
            json!({"type": "user", "message": {"id": "u", "model": "x", "usage": {"input_tokens": 1}}}),
            json!({"type": "assistant", "message": {"id": "e", "model": "x", "usage": {}}}),
            json!({"type": "assistant", "message": "not a dict"}),
            json!({"type": "assistant"}),
        ]);
        assert_eq!(cost_usd(FOLDER, &[t], &prices()), Ok(0.0));
    }

    #[test]
    fn zero_token_messages_need_no_price() {
        let t = lines(&[
            prompt("Job folder: /repo/factory/jobs/001-a/"),
            usage("z", "claude-unknown", json!({"input_tokens": 0})),
        ]);
        assert_eq!(cost_usd(FOLDER, &[t], &prices()), Ok(0.0));
    }

    #[test]
    fn unknown_model_with_tokens_is_refused_like_v2() {
        let t = lines(&[
            prompt("Job folder: /repo/factory/jobs/001-a/"),
            usage("m", "claude-mystery-1", json!({"output_tokens": 5})),
        ]);
        assert_eq!(
            cost_usd(FOLDER, &[t], &prices()),
            Err(
                "no price for model claude-mystery-1 in factory/config.json prices_usd_per_mtok"
                    .into()
            )
        );
    }

    #[test]
    fn crlf_transcripts_read_like_python_text_mode() {
        let t = lines(&[
            prompt("Job folder: /repo/factory/jobs/001-a/"),
            usage("m", "claude-haiku-5-5", json!({"output_tokens": 20000})),
        ])
        .replace('\n', "\r\n");
        assert_eq!(cost_usd(FOLDER, &[t], &prices()), Ok(0.01));
    }

    #[test]
    fn broken_transcript_lines_are_named() {
        let bad_first = "not json\n".to_string();
        assert_eq!(
            cost_usd(FOLDER, &[bad_first], &prices()),
            Err("a Claude Code transcript line is not valid JSON".into())
        );
        let blank_later = format!("{}\n", prompt("Job folder: /repo/factory/jobs/001-a/"));
        assert_eq!(
            cost_usd(FOLDER, &[format!("{blank_later}\n")], &prices()),
            Err("a Claude Code transcript line is not valid JSON".into())
        );
        let no_message = "{}\n".to_string();
        assert_eq!(
            cost_usd(FOLDER, &[no_message], &prices()),
            Err("a Claude Code transcript prompt has no message content".into())
        );
    }

    #[test]
    fn incomplete_prices_are_named() {
        let mut prices = prices();
        prices.insert("claude-haiku-5-5".into(), json!({"input": 0.1}));
        let t = lines(&[
            prompt("Job folder: /repo/factory/jobs/001-a/"),
            usage("m", "claude-haiku-5-5", json!({"input_tokens": 1})),
        ]);
        assert_eq!(
            cost_usd(FOLDER, &[t], &prices),
            Err(
                "factory/config.json prices_usd_per_mtok has no output price for claude-haiku-5-5"
                    .into()
            )
        );
    }
}
