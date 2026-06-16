use chrono::NaiveDate;

/// Parses a `YYYY-MM-DD` date, the format used by both the CloudFront `date`
/// field and the date portion of the Fastly `date_time` timestamp.
///
/// This avoids chrono's general-purpose format machinery, which is noticeably
/// slower when the format is known and fixed.
pub fn parse_date(date: &str) -> Option<NaiveDate> {
    let bytes = date.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return None;
    }

    // The `-` checks above guarantee these slices fall on UTF-8 char
    // boundaries (a `-` byte can never be part of a multi-byte character), so
    // the indexing cannot panic. The non-ASCII test cases below cover this.
    let year = date[0..4].parse().ok()?;
    let month = date[5..7].parse().ok()?;
    let day = date[8..10].parse().ok()?;
    NaiveDate::from_ymd_opt(year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_date() {
        assert_eq!(
            parse_date("2024-01-16"),
            NaiveDate::from_ymd_opt(2024, 1, 16)
        );

        // Wrong length, separators, or non-digits are rejected.
        assert_eq!(parse_date(""), None);
        assert_eq!(parse_date("2024-1-6"), None);
        assert_eq!(parse_date("2024/01/16"), None);
        assert_eq!(parse_date("2024-01-16T00"), None);
        assert_eq!(parse_date("abcd-01-16"), None);
        assert_eq!(parse_date("2024-13-16"), None);

        // Non-ASCII input must return `None` rather than panic on a slice that
        // could fall inside a multi-byte UTF-8 character (`é` is two bytes, so
        // `"2024-01-é"` is exactly ten bytes and reaches the digit slices).
        assert_eq!(parse_date("2024-01-é"), None);
        assert_eq!(parse_date("é2024-01-16"), None);
    }
}
