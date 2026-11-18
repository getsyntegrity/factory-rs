//! Python behavior that v2 makes observable: `repr()`, `str()`, truthiness,
//! string stripping and line splitting, and `json.dumps()` (default
//! separators, `indent=2`, `ensure_ascii=True`, float `repr`).

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
        Value::Number(n) => number(n),
        Value::Null | Value::Bool(_) => value.to_string(),
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

/// Python's `str.isspace()` for one character.
pub fn is_space(c: char) -> bool {
    c.is_whitespace() || ('\x1c'..='\x1f').contains(&c)
}

/// `s.strip()`.
pub fn strip(s: &str) -> &str {
    s.trim_matches(is_space)
}

/// `s.rstrip()`.
pub fn rstrip(s: &str) -> &str {
    s.trim_end_matches(is_space)
}

/// What `Path.read_text()` returns: `\r\n` and `\r` read as `\n`.
pub fn universal_newlines(s: &str) -> String {
    s.replace("\r\n", "\n").replace('\r', "\n")
}

/// `s.splitlines()`.
pub fn splitlines(s: &str) -> Vec<&str> {
    let is_break = |c: char| {
        matches!(
            c,
            '\n' | '\r' | '\x0b' | '\x0c' | '\x1c'..='\x1e' | '\u{85}' | '\u{2028}' | '\u{2029}'
        )
    };
    let mut lines = Vec::new();
    let mut rest = s;
    while let Some(at) = rest.find(is_break) {
        lines.push(&rest[..at]);
        let skip = if rest[at..].starts_with("\r\n") {
            2
        } else {
            rest[at..].chars().next().map_or(1, char::len_utf8)
        };
        rest = &rest[at + skip..];
    }
    if !rest.is_empty() {
        lines.push(rest);
    }
    lines
}

/// `repr(x)` for a float: the shortest round-tripping digits, in exponent
/// form when the decimal exponent is below -4 or at least 16.
pub fn float_repr(x: f64) -> String {
    if !x.is_finite() {
        return if x.is_nan() {
            "nan".into()
        } else if x > 0.0 {
            "inf".into()
        } else {
            "-inf".into()
        };
    }
    let sci = format!("{x:e}");
    let (mantissa, exp) = sci.split_once('e').unwrap_or((&sci, "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    let (sign, mantissa) = mantissa
        .strip_prefix('-')
        .map_or(("", mantissa), |m| ("-", m));
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    if !(-4..16).contains(&exp) {
        let (first, rest) = digits.split_at(1);
        let rest = if rest.is_empty() {
            String::new()
        } else {
            format!(".{rest}")
        };
        let exp_sign = if exp < 0 { '-' } else { '+' };
        return format!("{sign}{first}{rest}e{exp_sign}{:02}", exp.abs());
    }
    if exp < 0 {
        return format!("{sign}0.{}{digits}", "0".repeat((-exp - 1) as usize));
    }
    let int_len = exp as usize + 1;
    if digits.len() <= int_len {
        format!("{sign}{digits}{}.0", "0".repeat(int_len - digits.len()))
    } else {
        format!("{sign}{}.{}", &digits[..int_len], &digits[int_len..])
    }
}

fn number(n: &serde_json::Number) -> String {
    match n.as_f64() {
        Some(x) if n.is_f64() => float_repr(x),
        _ => n.to_string(),
    }
}

/// `json.dumps(value, indent=2)`.
pub fn dumps_indent(value: &Value) -> String {
    let mut out = String::new();
    write_indented(value, 0, &mut out);
    out
}

fn write_indented(value: &Value, depth: usize, out: &mut String) {
    let pad = |d: usize| "  ".repeat(d);
    match value {
        Value::Array(items) if !items.is_empty() => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                out.push_str(if i == 0 { "\n" } else { ",\n" });
                out.push_str(&pad(depth + 1));
                write_indented(item, depth + 1, out);
            }
            out.push('\n');
            out.push_str(&pad(depth));
            out.push(']');
        }
        Value::Object(map) if !map.is_empty() => {
            out.push('{');
            for (i, (key, item)) in map.iter().enumerate() {
                out.push_str(if i == 0 { "\n" } else { ",\n" });
                out.push_str(&pad(depth + 1));
                out.push_str(&dumps_str(key));
                out.push_str(": ");
                write_indented(item, depth + 1, out);
            }
            out.push('\n');
            out.push_str(&pad(depth));
            out.push('}');
        }
        other => out.push_str(&dumps(other)),
    }
}

/// `str(value)` for a JSON-loaded value.
pub fn str_of(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => repr_of(other),
    }
}

fn repr_of(value: &Value) -> String {
    match value {
        Value::Null => "None".into(),
        Value::Bool(true) => "True".into(),
        Value::Bool(false) => "False".into(),
        Value::Number(n) => number(n),
        Value::String(s) => repr(Some(s)),
        Value::Array(items) => format!(
            "[{}]",
            items.iter().map(repr_of).collect::<Vec<_>>().join(", ")
        ),
        Value::Object(map) => format!(
            "{{{}}}",
            map.iter()
                .map(|(k, v)| format!("{}: {}", repr(Some(k)), repr_of(v)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

/// `bool(value)` for a JSON-loaded value.
pub fn truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(items) => !items.is_empty(),
        Value::Object(map) => !map.is_empty(),
    }
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

    #[test]
    fn strip_removes_python_whitespace_only() {
        assert_eq!(strip(" \x1c\x1fa b\u{a0}\u{3000}\u{85}"), "a b");
        assert_eq!(strip("\u{200b}a\u{200b}"), "\u{200b}a\u{200b}");
        assert_eq!(rstrip("  a b \t\n"), "  a b");
    }

    #[test]
    fn universal_newlines_reads_cr_and_crlf_as_lf() {
        assert_eq!(universal_newlines("a\r\nb\rc\n\r\n"), "a\nb\nc\n\n");
    }

    #[test]
    fn splitlines_splits_on_every_python_line_boundary() {
        let s = "a\x0bb\x0cc\x1cd\x1de\x1ef\u{85}g\u{2028}h\u{2029}i\r\nj\rk\nl";
        assert_eq!(
            splitlines(s),
            ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l"]
        );
        assert_eq!(splitlines("a\n\nb\n"), ["a", "", "b"]);
        assert!(splitlines("").is_empty());
    }

    #[test]
    fn float_repr_matches_python() {
        let cases = [
            (0.0, "0.0"),
            (-0.0, "-0.0"),
            (0.14, "0.14"),
            (100.0, "100.0"),
            (1e16, "1e+16"),
            (1e15, "1000000000000000.0"),
            (123456789012345.6, "123456789012345.6"),
            (1e-5, "1e-05"),
            (0.0001, "0.0001"),
            (1.5e-7, "1.5e-07"),
            (2.5e300, "2.5e+300"),
            (4000.25, "4000.25"),
        ];
        for (x, want) in cases {
            assert_eq!(float_repr(x), want, "{x}");
        }
    }

    #[test]
    fn dumps_indent_matches_python() {
        let v = json!({"a": {}, "b": [], "c": [1, {"d": null}], "e": "ñ", "f": 1e16});
        assert_eq!(
            dumps_indent(&v),
            "{\n  \"a\": {},\n  \"b\": [],\n  \"c\": [\n    1,\n    {\n      \"d\": null\n    }\n  ],\n  \"e\": \"\\u00f1\",\n  \"f\": 1e+16\n}"
        );
    }

    #[test]
    fn dumps_uses_float_repr() {
        assert_eq!(
            dumps(&json!([1e16, 1e-5, 2.5, 7])),
            "[1e+16, 1e-05, 2.5, 7]"
        );
    }

    #[test]
    fn str_of_matches_python_str() {
        assert_eq!(str_of(&json!(true)), "True");
        assert_eq!(str_of(&json!(false)), "False");
        assert_eq!(str_of(&json!(null)), "None");
        assert_eq!(str_of(&json!(3)), "3");
        assert_eq!(str_of(&json!(1.0)), "1.0");
        assert_eq!(str_of(&json!("x")), "x");
        assert_eq!(
            str_of(&json!({"a": [1, "b", null, true]})),
            "{'a': [1, 'b', None, True]}"
        );
        assert_eq!(str_of(&json!([])), "[]");
    }

    #[test]
    fn truthy_matches_python_bool() {
        for v in [
            json!(null),
            json!(false),
            json!(0),
            json!(0.0),
            json!(""),
            json!([]),
            json!({}),
        ] {
            assert!(!truthy(&v), "{v}");
        }
        for v in [
            json!(true),
            json!(2),
            json!(-0.5),
            json!("0"),
            json!([0]),
            json!({"a": null}),
        ] {
            assert!(truthy(&v), "{v}");
        }
    }
}
