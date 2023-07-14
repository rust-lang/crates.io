use chrono::{Duration, NaiveDateTime, Utc};
use serde::{ser::SerializeMap, Serialize};

#[derive(Debug, Clone, Copy)]
pub struct DateTime {
    time: NaiveDateTime,
    #[cfg(test)]
    now: chrono::DateTime<Utc>,
}

impl DateTime {
    fn now(&self) -> chrono::DateTime<Utc> {
        #[cfg(test)]
        return self.now;

        #[cfg(not(test))]
        return Utc::now();
    }
}

impl From<NaiveDateTime> for DateTime {
    fn from(time: NaiveDateTime) -> Self {
        Self {
            time,
            #[cfg(test)]
            now: Utc::now(),
        }
    }
}

impl Serialize for DateTime {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("absolute", &format_time(self.time))?;
        map.serialize_entry(
            "human",
            &format_duration(self.now().naive_utc() - self.time),
        )?;
        map.end()
    }
}

fn format_time(time: NaiveDateTime) -> String {
    time.format("%a %-d %b %Y %H:%M:%S").to_string()
}

fn format_duration(duration: Duration) -> String {
    // This originally delegated out to chrono-human-duration or
    // pretty-duration, but the former had bugs around handling plurals and the
    // latter could only handle std::time::Duration and didn't have any options
    // to not include every unit. What we really need is so simple that it's
    // easier to just implement it here.
    let abs = if duration < Duration::zero() {
        -duration
    } else {
        duration
    };
    let adverb = adverb(duration);

    match (
        abs.num_weeks(),
        abs.num_days(),
        abs.num_hours(),
        abs.num_minutes(),
        abs.num_seconds(),
    ) {
        (_, days, _, _, _) if days >= 365 => {
            // Technically, leap years exist. Practically, it doesn't matter for
            // a fuzzy duration anyway.
            format!("{} year{} {}", days / 365, plural(days / 365), adverb)
        }
        (_, days, _, _, _) if days >= 30 => {
            // Same for months, honestly.
            format!("{} month{} {}", days / 30, plural(days / 30), adverb)
        }
        (weeks, _, _, _, _) if weeks > 0 => {
            format!("{} week{} {}", weeks, plural(weeks), adverb)
        }
        (_, days, _, _, _) if days > 0 => {
            format!("{} day{} {}", days, plural(days), adverb)
        }
        (_, _, hours, _, _) if hours > 0 => {
            format!("{} hour{} {}", hours, plural(hours), adverb)
        }
        (_, _, _, minutes, _) if minutes > 0 => {
            format!("{} minute{} {}", minutes, plural(minutes), adverb)
        }
        (_, _, _, _, seconds) if seconds > 0 => {
            format!("{} second{} {}", seconds, plural(seconds), adverb)
        }
        _ => String::from("just now"),
    }
}

fn plural(value: i64) -> &'static str {
    if value == 1 {
        ""
    } else {
        "s"
    }
}

fn adverb(duration: Duration) -> &'static str {
    // These durations are technically reversed; see the code in
    // `DateTime::serialize` for the full details of how the durations are
    // calculated.
    if duration > Duration::zero() {
        "ago"
    } else {
        "from now"
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use serde_json::Value;

    use super::*;

    #[test]
    fn datetime_formatting() -> anyhow::Result<()> {
        let now = Utc
            .with_ymd_and_hms(2023, 5, 31, 21, 43, 42)
            .single()
            .unwrap();

        for (input, human) in [
            ("2023-05-31T21:43:42Z", "just now"),
            ("2023-05-31T21:43:32Z", "10 seconds ago"),
            ("2023-05-31T21:42:42Z", "1 minute ago"),
            ("2023-05-31T20:43:42Z", "1 hour ago"),
            ("2023-05-31T19:43:42Z", "2 hours ago"),
            ("2023-05-30T21:43:42Z", "1 day ago"),
            ("2023-05-24T21:43:42Z", "1 week ago"),
            ("2023-05-17T21:43:42Z", "2 weeks ago"),
            ("2022-06-01T21:43:42Z", "12 months ago"),
            ("2021-05-31T21:43:42Z", "2 years ago"),
            ("2024-05-31T21:43:42Z", "1 year from now"),
        ] {
            let dt = DateTime {
                time: NaiveDateTime::parse_from_str(input, "%Y-%m-%dT%H:%M:%SZ")?,
                now,
            };

            let out = serde_json::to_value(dt)?;
            match out {
                Value::Object(map) => {
                    assert_eq!(map.get("absolute").unwrap(), &format_time(dt.time));
                    assert_eq!(map.get("human").unwrap(), human);
                }
                _ => panic!("unexpected JSON value type: {out:?}"),
            }
        }

        Ok(())
    }
}
