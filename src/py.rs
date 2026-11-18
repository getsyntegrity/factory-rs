//! Python text formats that v2 makes observable: `repr()` of a string and
//! `json.dumps()` with its default separators and `ensure_ascii=True`.

use serde_json::Value;

/// `repr(value)` for a `str | None`, as Python 3 prints it.
pub fn repr(value: Option<&str>) -> String {
    let Some(s) = value else {
        return "None".to_string();
    };
    let quote = if s.contains('\'') && !s.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut out = String::with_capacity(s.len() + 2);
    out.push(quote);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c if c.is_ascii_control() => out.push_str(&format!("\\x{:02x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push(quote);
    out
}

/// `json.dumps(value)` with Python's defaults.
pub fn dumps(value: &Value) -> String {
    match value {
        Value::Null | Value::Bool(_) | Value::Number(_) => value.to_string(),
        Value::String(s) => dumps_str(s),
        Value::Array(items) => {
            let items: Vec<String> = items.iter().map(dumps).collect();
            format!("[{}]", items.join(", "))
        }
        Value::Object(map) => {
            let entries: Vec<String> = map
                .iter()
                .map(|(k, v)| format!("{}: {}", dumps_str(k), dumps(v)))
                .collect();
            format!("{{{}}}", entries.join(", "))
        }
    }
}

fn dumps_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\x08' => out.push_str("\\b"),
            '\x0c' => out.push_str("\\f"),
            ' '..='~' => out.push(c),
            c => {
                let mut units = [0u16; 2];
                for unit in c.encode_utf16(&mut units) {
                    out.push_str(&format!("\\u{unit:04x}"));
                }
            }
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn repr_of_none_is_none() {
        assert_eq!(repr(None), "None");
    }

    #[test]
    fn repr_uses_single_quotes_by_default() {
        assert_eq!(repr(Some("extreme")), "'extreme'");
    }

    #[test]
    fn repr_switches_to_double_quotes_when_only_single_quotes_appear() {
        assert_eq!(repr(Some("it's")), "\"it's\"");
    }

    #[test]
    fn repr_escapes_single_quote_when_both_quotes_appear() {
        assert_eq!(repr(Some("a'b\"c")), "'a\\'b\"c'");
    }

    #[test]
    fn repr_escapes_backslash_and_control_characters() {
        assert_eq!(
            repr(Some("a\\b\t\n\r\x01\x7f")),
            "'a\\\\b\\t\\n\\r\\x01\\x7f'"
        );
    }

    #[test]
    fn repr_keeps_printable_non_ascii() {
        assert_eq!(repr(Some("año")), "'año'");
    }

    #[test]
    fn dumps_uses_python_separators_and_keeps_key_order() {
        let v = json!({"reviewers": {"code": "sonnet", "ux": "sonnet", "ui": "haiku"}});
        assert_eq!(
            dumps(&v),
            r#"{"reviewers": {"code": "sonnet", "ux": "sonnet", "ui": "haiku"}}"#
        );
    }

    #[test]
    fn dumps_escapes_like_ensure_ascii() {
        let v = json!("q\"b\\n\n\t\x08\x0c\r\x01\x7fñ😀");
        assert_eq!(
            dumps(&v),
            r#""q\"b\\n\n\t\b\f\r\u0001\u007f\u00f1\ud83d\ude00""#
        );
    }

    #[test]
    fn dumps_scalars_and_arrays() {
        let v = json!([null, true, false, 3, 1.5, [], {}]);
        assert_eq!(dumps(&v), "[null, true, false, 3, 1.5, [], {}]");
    }
}
