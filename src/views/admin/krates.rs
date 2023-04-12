use chrono::NaiveDateTime;
use serde::Serializer;

#[derive(Serialize)]
pub struct CrateVersion {
    pub id: i32,
    pub name: String,
    pub num: String,
    #[serde(serialize_with = "format_date_time")]
    pub created_at: NaiveDateTime,
    pub published_by_username: String,
    pub published_by_avatar: Option<String>,
}

fn format_date_time<S>(time: &NaiveDateTime, ser: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    ser.serialize_str(&time.format("%a %-d %b %Y %H:%M:%S").to_string())
}
