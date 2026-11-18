//! v2's time handling: `now()` as `%Y-%m-%dT%H:%M:%SZ` (UTC) and
//! `elapsed()` between two ISO 8601 times. Times are microseconds since the
//! Unix epoch.

/// `datetime.fromisoformat(s.replace("Z", "+00:00"))` for the forms the
/// factory writes: `YYYY-MM-DDTHH:MM:SS[.ffffff](Z|±HH:MM)`.
pub fn parse_iso(s: &str) -> Option<i64> {
    let num = |part: &str| -> Option<i64> {
        (!part.is_empty() && part.bytes().all(|b| b.is_ascii_digit())).then(|| part.parse().ok())?
    };
    let (date, rest) = s.split_once('T')?;
    let mut d = date.split('-');
    let (year, month, day) = (num(d.next()?)?, num(d.next()?)?, num(d.next()?)?);
    if d.next().is_some()
        || date.len() != 10
        || !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
    {
        return None;
    }
    let (time, offset) = if let Some(t) = rest.strip_suffix('Z') {
        (t, 0)
    } else {
        let at = rest.rfind(['+', '-'])?;
        let (t, off) = rest.split_at(at);
        let (oh, om) = off[1..].split_once(':')?;
        let minutes = num(oh)? * 60 + num(om)?;
        (
            t,
            if off.starts_with('-') {
                -minutes
            } else {
                minutes
            },
        )
    };
    let (hms, fraction) = time.split_once('.').unwrap_or((time, ""));
    let mut t = hms.split(':');
    let (h, m, sec) = (num(t.next()?)?, num(t.next()?)?, num(t.next()?)?);
    if t.next().is_some() || hms.len() != 8 || h > 23 || m > 59 || sec > 59 || fraction.len() > 6 {
        return None;
    }
    let micros = if fraction.is_empty() {
        0
    } else {
        num(&format!("{fraction:0<6}"))?
    };
    let seconds =
        days_from_civil(year, month, day) * 86_400 + h * 3600 + m * 60 + sec - offset * 60;
    Some(seconds * 1_000_000 + micros)
}

/// `datetime.fromtimestamp(t, timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")`.
pub fn format_utc(micros: i64) -> String {
    let seconds = micros.div_euclid(1_000_000);
    let (days, rest) = (seconds.div_euclid(86_400), seconds.rem_euclid(86_400));
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rest / 3600,
        rest % 3600 / 60,
        rest % 60
    )
}

/// v2 `elapsed()`: whole minutes (floored) as `Nm`, or `HhMMm` from an hour.
pub fn elapsed(start: i64, end: i64) -> String {
    let minutes = (end - start).div_euclid(60_000_000);
    if minutes >= 60 {
        format!("{}h{:02}m", minutes / 60, minutes % 60)
    } else {
        format!("{minutes}m")
    }
}

/// Days since 1970-01-01 (Howard Hinnant's algorithm).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(month <= 2), month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: &str) -> i64 {
        parse_iso(s).unwrap()
    }

    #[test]
    fn elapsed_matches_v2() {
        for (start, end, want) in [
            ("2026-11-18T00:00:00Z", "2026-11-18T01:05:00Z", "1h05m"),
            ("2026-11-18T00:00:00Z", "2026-11-18T00:59:59Z", "59m"),
            ("2026-11-17T00:00:00Z", "2026-11-18T01:00:00Z", "25h00m"),
            ("2026-11-18T00:00:30Z", "2026-11-18T00:00:00Z", "-1m"),
            (
                "2026-11-18T00:00:00.500000+02:00",
                "2026-11-17T23:00:00Z",
                "59m",
            ),
            ("2024-02-28T23:00:00Z", "2024-03-01T00:00:00Z", "25h00m"),
        ] {
            assert_eq!(elapsed(t(start), t(end)), want, "{start} {end}");
        }
    }

    #[test]
    fn parses_the_epoch_and_offsets() {
        assert_eq!(parse_iso("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(
            parse_iso("2026-11-18T03:04:05+00:00"),
            Some(1_794_971_045_000_000)
        );
        assert_eq!(
            parse_iso("2026-11-18T05:04:05+02:00"),
            Some(1_794_971_045_000_000)
        );
        assert_eq!(
            parse_iso("2026-11-18T03:04:05.25Z"),
            Some(1_794_971_045_250_000)
        );
    }

    #[test]
    fn rejects_other_shapes() {
        for s in [
            "",
            "2026-11-18",
            "2026-11-18 03:04:05Z",
            "2026-13-01T00:00:00Z",
            "2026-11-18T03:04:05",
            "x",
        ] {
            assert_eq!(parse_iso(s), None, "{s}");
        }
    }

    #[test]
    fn formats_utc_seconds() {
        assert_eq!(format_utc(1_794_971_045_999_999), "2026-11-18T03:04:05Z");
        assert_eq!(format_utc(951_782_400_000_000), "2000-02-29T00:00:00Z");
        assert_eq!(format_utc(0), "1970-01-01T00:00:00Z");
    }
}
