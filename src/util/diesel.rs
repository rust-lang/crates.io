use diesel::connection::LoadConnection;
use diesel::pg::Pg;
use diesel::result::Error;

pub trait Conn: LoadConnection<Backend = Pg> {}

impl<T> Conn for T where T: LoadConnection<Backend = Pg> {}

pub fn is_read_only_error(error: &Error) -> bool {
    matches!(error, Error::DatabaseError(_, info) if info.message().ends_with("read-only transaction"))
}
