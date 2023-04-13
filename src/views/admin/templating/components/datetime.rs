use chrono::{Duration, NaiveDateTime, Utc};
use serde::{ser::SerializeMap, Serialize};

#[derive(Debug, Clone, Copy)]
pub struct DateTime(NaiveDateTime);

impl From<NaiveDateTime> for DateTime {
    fn from(value: NaiveDateTime) -> Self {
        Self(value)
    }
}

impl Serialize for DateTime {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("absolute", &format_time(self.0))?;
        map.serialize_entry("human", &format_duration(Utc::now().naive_utc() - self.0))?;
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
        (_, _, _, seconds, _) if seconds > 0 => {
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
