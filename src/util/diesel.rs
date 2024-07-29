use diesel::connection::LoadConnection;
use diesel::pg::Pg;

pub trait Conn: LoadConnection<Backend = Pg> {}

impl<T> Conn for T where T: LoadConnection<Backend = Pg> {}
