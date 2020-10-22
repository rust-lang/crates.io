use chrono::DateTime;
use insta::{dynamic_redaction, internals::Redaction};

pub fn rfc3339_redaction() -> Redaction {
    dynamic_redaction(|value, _| {
        assert!(DateTime::parse_from_rfc3339(value.as_str().unwrap()).is_ok());
        "[datetime]"
    })
}
