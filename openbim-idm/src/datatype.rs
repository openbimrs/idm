//! Lexical checks for the XML Schema built-in types that the generated catalog
//! references. Only lexical validity is checked; no schema bytes are involved.

/// Check `value` against the lexical space of an `xs:` built-in type.
///
/// Returns `None` for types without a specific check (for example `xs:string`)
/// so callers can distinguish "valid" from "unchecked".
pub(crate) fn lexical_check(data_type: &str, value: &str) -> Option<bool> {
    let name = data_type.strip_prefix("xs:").unwrap_or(data_type);
    match name {
        "boolean" => Some(matches!(value.trim(), "true" | "false" | "1" | "0")),
        "integer" => Some(is_integer(value.trim())),
        "dateTime" => Some(is_date_time(value.trim())),
        "NCName" => Some(is_ncname(value)),
        "normalizedString" => Some(!value.contains(['\t', '\n', '\r'])),
        _ => None,
    }
}

fn is_integer(value: &str) -> bool {
    let digits = value.strip_prefix(['+', '-']).unwrap_or(value);
    !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
}

fn is_ncname(value: &str) -> bool {
    let mut characters = value.chars();
    let Some(first) = characters.next() else {
        return false;
    };
    (first.is_alphabetic() || first == '_')
        && characters.all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '.'))
}

fn is_date_time(value: &str) -> bool {
    let value = value.strip_prefix('-').unwrap_or(value);
    let Some((date, time)) = value.split_once('T') else {
        return false;
    };
    is_date(date) && is_time_with_zone(time)
}

fn is_date(date: &str) -> bool {
    let mut parts = date.splitn(3, '-');
    let (Some(year), Some(month), Some(day)) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    if year.len() < 4 || !year.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    let (Some(month), Some(day)) = (two_digits(month), two_digits(day)) else {
        return false;
    };
    let year: u64 = year.parse().unwrap_or(0);
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let max_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1..=max_day).contains(&day)
}

fn is_time_with_zone(time: &str) -> bool {
    let (clock, zone) = match time.find(['Z', '+', '-']) {
        Some(position) => time.split_at(position),
        None => (time, ""),
    };
    let mut parts = clock.splitn(3, ':');
    let (Some(hour), Some(minute), Some(second)) = (parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    let (whole, fraction) = match second.split_once('.') {
        Some((whole, fraction)) => (whole, Some(fraction)),
        None => (second, None),
    };
    let (Some(hour), Some(minute), Some(whole)) =
        (two_digits(hour), two_digits(minute), two_digits(whole))
    else {
        return false;
    };
    if fraction.is_some_and(|f| f.is_empty() || !f.bytes().all(|b| b.is_ascii_digit())) {
        return false;
    }
    let clock_ok =
        (hour < 24 && minute < 60 && whole < 60) || (hour == 24 && minute == 0 && whole == 0);
    clock_ok && zone_ok(zone)
}

fn zone_ok(zone: &str) -> bool {
    match zone {
        "" | "Z" => true,
        _ => {
            let Some(offset) = zone.strip_prefix(['+', '-']) else {
                return false;
            };
            let Some((hours, minutes)) = offset.split_once(':') else {
                return false;
            };
            matches!((two_digits(hours), two_digits(minutes)), (Some(h), Some(m)) if (h < 14 && m < 60) || (h == 14 && m == 0))
        }
    }
}

fn two_digits(value: &str) -> Option<u32> {
    (value.len() == 2 && value.bytes().all(|b| b.is_ascii_digit()))
        .then(|| value.parse().ok())
        .flatten()
}

/// Strict check for the catalog's lowercase GUID pattern
/// `[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}`.
pub(crate) fn is_lowercase_guid(value: &str) -> bool {
    const GROUPS: [usize; 5] = [8, 4, 4, 4, 12];
    let parts: Vec<&str> = value.split('-').collect();
    parts.len() == GROUPS.len()
        && parts.iter().zip(GROUPS).all(|(part, length)| {
            part.len() == length
                && part
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checks_builtin_lexical_spaces() {
        assert_eq!(lexical_check("xs:boolean", "true"), Some(true));
        assert_eq!(lexical_check("xs:boolean", "yes"), Some(false));
        assert_eq!(lexical_check("xs:integer", "-12"), Some(true));
        assert_eq!(lexical_check("xs:integer", "1.5"), Some(false));
        assert_eq!(lexical_check("xs:integer", ""), Some(false));
        assert_eq!(
            lexical_check("xs:dateTime", "1970-01-01T00:00:00Z"),
            Some(true)
        );
        assert_eq!(
            lexical_check("xs:dateTime", "2024-02-29T23:59:59.5+01:00"),
            Some(true)
        );
        assert_eq!(
            lexical_check("xs:dateTime", "2023-02-29T00:00:00"),
            Some(false)
        );
        assert_eq!(lexical_check("xs:dateTime", "not-a-date"), Some(false));
        assert_eq!(lexical_check("xs:dateTime", "2024-01-01"), Some(false));
        assert_eq!(lexical_check("xs:NCName", "author-1"), Some(true));
        assert_eq!(lexical_check("xs:NCName", "1author"), Some(false));
        assert_eq!(lexical_check("xs:NCName", "a:b"), Some(false));
        assert_eq!(lexical_check("xs:normalizedString", "a\nb"), Some(false));
        assert_eq!(lexical_check("xs:string", "anything"), None);
    }

    #[test]
    fn guid_pattern_is_strict_lowercase() {
        assert!(is_lowercase_guid("04e6c514-5c8b-4250-b2a6-52ef954a5285"));
        assert!(!is_lowercase_guid("04E6C514-5c8b-4250-b2a6-52ef954a5285"));
        assert!(!is_lowercase_guid("04e6c5145c8b4250b2a652ef954a5285"));
        assert!(!is_lowercase_guid("{04e6c514-5c8b-4250-b2a6-52ef954a5285}"));
    }
}
