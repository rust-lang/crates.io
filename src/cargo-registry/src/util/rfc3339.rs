//! Convenience functions for serializing and deserializing times in RFC 3339 format.
//! Used for returning time values in JSON API responses.
//! Example: `2012-02-22T14:53:18+00:00`.

use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{self, Deserialize, Deserializer, Serializer};

pub fn serialize<S>(dt: &NaiveDateTime, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let s = DateTime::<Utc>::from_utc(*dt, Utc).to_rfc3339();
    serializer.serialize_str(&s)
}
pub fn deserialize<'de, D>(deserializer: D) -> Result<NaiveDateTime, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let dt = DateTime::parse_from_rfc3339(&s).map_err(serde::de::Error::custom)?;
    Ok(dt.naive_utc())
}

/// Wrapper for dealing with Option<NaiveDateTime>
pub mod option {
    use chrono::NaiveDateTime;
    use serde::{Deserializer, Serializer};

    pub fn serialize<S>(dt: &Option<NaiveDateTime>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *dt {
            Some(dt) => super::serialize(&dt, serializer),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<NaiveDateTime>, D::Error>
    where
        D: Deserializer<'de>,
    {
        match super::deserialize(deserializer) {
            Ok(dt) => Ok(Some(dt)),
            Err(_) => Ok(None),
        }
    }
}
