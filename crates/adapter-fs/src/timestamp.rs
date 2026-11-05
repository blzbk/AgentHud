//! Times read from session files and from file metadata, as seconds since
//! the Unix epoch, without a date crate.

use std::time::{SystemTime, UNIX_EPOCH};

/// Seconds since the epoch of `time`, 0 when it predates the epoch.
#[must_use]
pub fn epoch_seconds(time: SystemTime) -> i64 {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|elapsed| i64::try_from(elapsed.as_secs()).ok())
        .unwrap_or_default()
}

/// Seconds since the epoch of an RFC 3339 timestamp such as
/// `2026-11-03T12:24:00.123Z` or `2026-11-03T13:24:00+01:00`.
#[must_use]
pub fn parse_utc_timestamp(text: &str) -> Option<i64> {
    let number = |range: std::ops::Range<usize>| -> Option<i64> {
        let digits = text.get(range)?;
        digits
            .bytes()
            .all(|byte| byte.is_ascii_digit())
            .then(|| digits.parse().ok())?
    };
    let bytes = text.as_bytes();
    if bytes.len() < 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || !matches!(bytes[10], b'T' | b't' | b' ')
    {
        return None;
    }
    let (year, month, day) = (number(0..4)?, number(5..7)?, number(8..10)?);
    let (hour, minute, second) = (number(11..13)?, number(14..16)?, number(17..19)?);
    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 60
    {
        return None;
    }
    let mut rest = &text[19..];
    if let Some(fraction) = rest.strip_prefix('.') {
        let digits = fraction.bytes().take_while(u8::is_ascii_digit).count();
        rest = &fraction[digits..];
    }
    let offset = match rest {
        "Z" | "z" => 0,
        _ => {
            let sign = match rest.as_bytes().first()? {
                b'+' => 1,
                b'-' => -1,
                _ => return None,
            };
            let offset = rest.get(1..)?;
            if offset.len() != 5 || offset.as_bytes()[2] != b':' {
                return None;
            }
            let hours: i64 = offset.get(0..2)?.parse().ok()?;
            let minutes: i64 = offset.get(3..5)?.parse().ok()?;
            sign * (hours * 3600 + minutes * 60)
        }
    };
    let days = days_from_civil(year, month, day);
    Some(days * 86_400 + hour * 3600 + minute * 60 + second - offset)
}

/// Days since 1970-01-01 of a proleptic Gregorian date (Howard Hinnant's
/// `days_from_civil`).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year.rem_euclid(400);
    let month_index = (month + 9) % 12;
    let day_of_year = (153 * month_index + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, UNIX_EPOCH};

    use super::{epoch_seconds, parse_utc_timestamp};

    #[test]
    fn converts_system_times() {
        assert_eq!(epoch_seconds(UNIX_EPOCH + Duration::from_secs(42)), 42);
        assert_eq!(epoch_seconds(UNIX_EPOCH - Duration::from_secs(1)), 0);
    }

    #[test]
    fn parses_utc_timestamps() {
        assert_eq!(parse_utc_timestamp("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(
            parse_utc_timestamp("2026-11-03T12:00:00.000Z"),
            Some(1_793_707_200)
        );
        assert_eq!(
            parse_utc_timestamp("2026-11-03T13:00:00+01:00"),
            Some(1_793_707_200)
        );
        assert_eq!(
            parse_utc_timestamp("2000-02-29T00:00:00Z"),
            Some(951_782_400)
        );
    }

    #[test]
    fn rejects_malformed_timestamps() {
        for text in [
            "",
            "yesterday",
            "2026-11-03",
            "2026-13-03T12:00:00Z",
            "2026-11-03T12:00:00",
        ] {
            assert_eq!(parse_utc_timestamp(text), None, "{text}");
        }
    }
}
