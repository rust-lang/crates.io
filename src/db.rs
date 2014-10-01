use std::fmt::Show;
use std::mem;
use std::sync::Arc;

use pg;
use pg::{PostgresConnection, PostgresStatement, PostgresResult};
use pg::types::ToSql;
use r2d2::{mod, LoggingErrorHandler};
use r2d2_postgres::{PostgresPoolManager, Error};
use conduit::{Request, Response};
use conduit_middleware::Middleware;

use app::{App, RequestApp};
use util::{CargoResult, LazyCell, internal};

pub type Pool = r2d2::Pool<pg::PostgresConnection,
                           Error,
                           PostgresPoolManager,
                           LoggingErrorHandler>;
type PooledConnnection<'a> =
        r2d2::PooledConnection<'a,
                               pg::PostgresConnection,
                               Error,
                               PostgresPoolManager,
                               LoggingErrorHandler>;

pub fn pool(url: &str, config: r2d2::Config) -> Pool {
    let mgr = PostgresPoolManager::new(url, pg::NoSsl);
    r2d2::Pool::new(config, mgr, LoggingErrorHandler).unwrap()
}

pub struct TransactionMiddleware;

pub struct Transaction {
    // fields are destructed top-to-bottom so ensure we destroy them in the
    // right order.
    tx: LazyCell<pg::PostgresTransaction<'static>>,
    slot: LazyCell<PooledConnnection<'static>>,

    // Keep a handle to the app which keeps a handle to the database to ensure
    // that this `'static` is indeed at least a little more accurate (in that
    // it's alive for the period of time this `Transaction` is alive.
    app: Arc<App>,
}

impl Transaction {
    pub fn new(app: Arc<App>) -> Transaction {
        Transaction {
            app: app,
            slot: LazyCell::new(),
            tx: LazyCell::new(),
        }
    }

    pub fn conn(&self) -> CargoResult<&PostgresConnection> {
        // Here we want to tie the lifetime of a single connection the lifetime
        // of this request. Currently the lifetime of a connection is tied to
        // the lifetime of the pool from which it came from, which is the
        // mismatch.
        //
        // The unsafety here is frobbing lifetimes to ensure that they work out.
        // Additionally, any extension in a Request needs to live for the static
        // lifetime (yay!).
        //
        // To solve these problems, the private `Transaction` extension stores a
        // reference both to the pool (to keep it alive) as well as a connection
        // transmuted to the 'static lifetime. This allows us to allocate a
        // connection up front and then repeatedly hand it out.
        unsafe {
            if !self.slot.filled() {
                let conn = try!(self.app.database.get().map_err(|()| {
                    internal("failed to get a database connection")
                }));
                let conn: PooledConnnection = conn;
                self.slot.fill(mem::transmute::<_, PooledConnnection<'static>>(conn));
            }
        }
        Ok(&**self.slot.borrow().unwrap())
    }

    fn tx<'a>(&'a self) -> CargoResult<&'a pg::PostgresTransaction<'a>> {
        // Similar to above, the transaction for this request is actually tied
        // to the connection in the request itself, not 'static. We transmute it
        // to static as its paired with the inner connection to achieve the
        // desired effect.
        unsafe {
            if !self.tx.filled() {
                let conn = try!(self.conn());
                let t = try!(conn.transaction());
                let t = mem::transmute::<_, pg::PostgresTransaction<'static>>(t);
                self.tx.fill(t);
            }
        }
        let tx = self.tx.borrow();
        let tx: &'a pg::PostgresTransaction<'static> = tx.unwrap();
        Ok(tx)
    }
}

impl Middleware for TransactionMiddleware {
    fn before(&self, req: &mut Request) -> Result<(), Box<Show + 'static>> {
        if !req.extensions().contains::<Transaction>() {
            let app = req.app().clone();
            req.mut_extensions().insert(Transaction::new(app));
        }
        Ok(())
    }

    fn after(&self, req: &mut Request, res: Result<Response, Box<Show + 'static>>)
             -> Result<Response, Box<Show + 'static>> {
        if res.is_ok() {
            let tx = req.extensions().find::<Transaction>()
                        .expect("Transaction not present in request");
            match tx.tx.borrow() {
                Some(tx) => {
                    if req.app().config.env != ::Test {
                        tx.set_commit();
                    }
                }
                None => {}
            }
        }
        return res;
    }
}

pub trait RequestTransaction<'a> {
    /// Return the lazily initialized postgres connection for this request.
    ///
    /// The connection will live for the lifetime of the request.
    fn db_conn(self) -> CargoResult<&'a PostgresConnection>;

    /// Return the lazily initialized postgres transaction for this request.
    ///
    /// The transaction will live for the duration of the request, and it will
    /// only be set to commit() if a successful response code of 200 is seen.
    fn tx(self) -> CargoResult<&'a pg::PostgresTransaction<'a>>;
}

impl<'a> RequestTransaction<'a> for &'a Request + 'a {
    fn db_conn(self) -> CargoResult<&'a PostgresConnection> {
        self.extensions().find::<Transaction>()
            .expect("Transaction not present in request")
            .conn()
    }

    fn tx(self) -> CargoResult<&'a pg::PostgresTransaction<'a>> {
        self.extensions().find::<Transaction>()
            .expect("Transaction not present in request")
            .tx()
    }
}

pub trait Connection {
    fn prepare<'a>(&'a self, query: &str) -> PostgresResult<PostgresStatement<'a>>;
    fn execute(&self, query: &str, params: &[&ToSql]) -> PostgresResult<uint>;
}

impl Connection for pg::PostgresConnection {
    fn prepare<'a>(&'a self, query: &str) -> PostgresResult<PostgresStatement<'a>> {
        self.prepare(query)
    }
    fn execute(&self, query: &str, params: &[&ToSql]) -> PostgresResult<uint> {
        self.execute(query, params)
    }
}
//
// impl Connection for pg::pool::PooledPostgresConnection {
//     fn prepare<'a>(&'a self, query: &str) -> PostgresResult<PostgresStatement<'a>> {
//         self.prepare(query)
//     }
//     fn execute(&self, query: &str, params: &[&ToSql]) -> PostgresResult<uint> {
//         self.execute(query, params)
//     }
// }

impl<'a> Connection for pg::PostgresTransaction<'a> {
    fn prepare<'a>(&'a self, query: &str) -> PostgresResult<PostgresStatement<'a>> {
        self.prepare(query)
    }
    fn execute(&self, query: &str, params: &[&ToSql]) -> PostgresResult<uint> {
        self.execute(query, params)
    }
}
