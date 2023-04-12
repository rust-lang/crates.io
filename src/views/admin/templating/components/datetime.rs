use chrono::{NaiveDateTime, Utc};
use chrono_human_duration::ChronoHumanDuration;
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
        map.serialize_entry(
            "human",
            &(Utc::now().naive_utc() - self.0).format_human().to_string(),
        )?;
        map.end()
    }
}

fn format_time(time: NaiveDateTime) -> String {
    time.format("%a %-d %b %Y %H:%M:%S").to_string()
}
