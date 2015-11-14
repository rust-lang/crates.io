use pg::rows::Row;
use pg::GenericConnection;

use util::{CargoResult, ChainError};
use util::errors::NotFound;

pub trait Model: Sized {
    fn from_row(row: &Row) -> Self;
    fn table_name(_: Option<Self>) -> &'static str;

    fn find(conn: &GenericConnection, id: i32) -> CargoResult<Self> {
        let sql = format!("SELECT * FROM {} WHERE id = $1",
                          Model::table_name(None::<Self>));
        let stmt = try!(conn.prepare(&sql));
        let rows = try!(stmt.query(&[&id]));
        let row = try!(rows.into_iter().next().chain_error(|| NotFound));
        Ok(Model::from_row(&row))
    }

    fn count(conn: &GenericConnection) -> CargoResult<i64> {
        let sql = format!("SELECT COUNT(*) FROM {}", Model::table_name(None::<Self>));
        let stmt = try!(conn.prepare(&sql));
        let rows = try!(stmt.query(&[]));
        Ok(rows.iter().next().unwrap().get("count"))
    }
}
