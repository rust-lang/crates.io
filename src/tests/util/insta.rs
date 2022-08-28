pub use ::insta::*;

pub fn id_redaction(expected_id: i32) -> insta::internals::Redaction {
    insta::dynamic_redaction(move |value, _path| {
        assert_eq!(value.as_i64().unwrap(), expected_id as i64);
        "[id]"
    })
}

pub fn any_id_redaction() -> insta::internals::Redaction {
    insta::dynamic_redaction(move |value, _path| {
        assert_some!(value.as_i64());
        "[id]"
    })
}
