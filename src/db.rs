use std::cell::Cell;
use std::error::Error;
use std::mem;
use std::sync::Arc;

use pg;
use pg::GenericConnection;
use r2d2;
use r2d2_postgres;
use r2d2_postgres::PostgresConnectionManager as PCM;
use conduit::{Request, Response};
use conduit_middleware::Middleware;

use app::{App, RequestApp};
use util::{CargoResult, LazyCell, internal};

pub type Pool = r2d2::Pool<PCM>;
pub type Config = r2d2::Config<pg::Connection, r2d2_postgres::Error>;
type PooledConnnection = r2d2::PooledConnection<PCM>;

pub fn pool(url: &str, config: r2d2::Config<pg::Connection, r2d2_postgres::Error>) -> Pool {
    let mgr = PCM::new(url, pg::SslMode::None).unwrap();
    r2d2::Pool::new(config, mgr).unwrap()
}

pub struct TransactionMiddleware;

pub struct Transaction {
    // fields are destructed top-to-bottom so ensure we destroy them in the
    // right order.
    tx: LazyCell<pg::Transaction<'static>>,
    slot: LazyCell<PooledConnnection>,
    commit: Cell<bool>,

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
            commit: Cell::new(false),
        }
    }

    pub fn conn(&self) -> CargoResult<&pg::Connection> {
        if !self.slot.filled() {
            let conn = try!(self.app.database.get().map_err(|e| {
                internal(format!("failed to get a database connection: {}", e))
            }));
            let conn: PooledConnnection = conn;
            self.slot.fill(conn);
        }
        Ok(&**self.slot.borrow().unwrap())
    }

    fn tx<'a>(&'a self) -> CargoResult<&'a (GenericConnection + 'a)> {
        // Similar to above, the transaction for this request is actually tied
        // to the connection in the request itself, not 'static. We transmute it
        // to static as its paired with the inner connection to achieve the
        // desired effect.
        unsafe {
            if !self.tx.filled() {
                let conn = try!(self.conn());
                let t = try!(conn.transaction());
                let t = mem::transmute::<_, pg::Transaction<'static>>(t);
                self.tx.fill(t);
            }
        }
        let tx = self.tx.borrow();
        let tx: &'a pg::Transaction<'static> = tx.unwrap();
        Ok(tx)
    }

    pub fn rollback(&self) { self.commit.set(false); }
    pub fn commit(&self) { self.commit.set(true); }
}

impl Middleware for TransactionMiddleware {
    fn before(&self, req: &mut Request) -> Result<(), Box<Error+Send>> {
        if !req.extensions().contains::<Transaction>() {
            let app = req.app().clone();
            req.mut_extensions().insert(Transaction::new(app));
        }
        Ok(())
    }

    fn after(&self, req: &mut Request, res: Result<Response, Box<Error+Send>>)
             -> Result<Response, Box<Error+Send>> {
        if res.is_ok() {
            let tx = req.extensions().find::<Transaction>()
                        .expect("Transaction not present in request");
            match tx.tx.borrow() {
                Some(transaction) if tx.commit.get() => {
                    transaction.set_commit();
                }
                _ => {}
            }
        }
        return res;
    }
}

pub trait RequestTransaction {
    /// Return the lazily initialized postgres connection for this request.
    ///
    /// The connection will live for the lifetime of the request.
    fn db_conn(&self) -> CargoResult<&pg::Connection>;

    /// Return the lazily initialized postgres transaction for this request.
    ///
    /// The transaction will live for the duration of the request, and it will
    /// only be set to commit() if a successful response code of 200 is seen.
    fn tx(&self) -> CargoResult<&GenericConnection>;

    /// Flag the transaction to not be committed
    fn rollback(&self);
    /// Flag this transaction to be committed
    fn commit(&self);
}

impl<'a> RequestTransaction for Request + 'a {
    fn db_conn(&self) -> CargoResult<&pg::Connection> {
        self.extensions().find::<Transaction>()
            .expect("Transaction not present in request")
            .conn()
    }

    fn tx(&self) -> CargoResult<&GenericConnection> {
        self.extensions().find::<Transaction>()
            .expect("Transaction not present in request")
            .tx()
    }

    fn rollback(&self) {
        self.extensions().find::<Transaction>()
            .expect("Transaction not present in request")
            .rollback()
    }

    fn commit(&self) {
        self.extensions().find::<Transaction>()
            .expect("Transaction not present in request")
            .commit()
    }
}
