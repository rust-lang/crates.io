use diesel::result::Error;

pub fn is_read_only_error(error: &Error) -> bool {
    matches!(error, Error::DatabaseError(_, info) if info.message().ends_with("read-only transaction"))
}
