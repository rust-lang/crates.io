use std::cell::Cell;
use std::env;
use std::error::Error;
use std::fmt;
use std::mem;
use std::sync::Arc;

use conduit::{Request, Response};
use conduit_middleware::Middleware;
use diesel::pg::PgConnection;
use openssl::ssl::{SslConnector, SslConnectorBuilder, SslMethod, SSL_VERIFY_NONE};
use pg::GenericConnection;
use pg::tls::{TlsHandshake, Stream, TlsStream};
use pg;
use r2d2;
use r2d2_postgres::PostgresConnectionManager as PCM;
use r2d2_postgres::TlsMode;
use r2d2_postgres::postgres;
use r2d2_postgres;
use r2d2_diesel::{self, ConnectionManager};
use url::Url;

use app::{App, RequestApp};
use util::{CargoResult, LazyCell, internal};

pub type Pool = r2d2::Pool<PCM>;
pub type Config = r2d2::Config<pg::Connection, r2d2_postgres::Error>;
type PooledConnnection = r2d2::PooledConnection<PCM>;
pub type DieselPool = r2d2::Pool<ConnectionManager<PgConnection>>;
type DieselPooledConn = r2d2::PooledConnection<ConnectionManager<PgConnection>>;

/// Creates a TLS handshake mechanism used by our postgres driver to negotiate
/// the TLS connection.
pub fn tls_handshake() -> Box<TlsHandshake + Sync + Send> {
    struct MyHandshake(SslConnector);

    // Note that rust-postgres provides a suite of TLS drivers to select from,
    // including all the native platform versions. It even correctly verifies
    // hostnames and such by default!
    //
    // Unfortunately for us, though, Heroku doesn't actually use valid
    // certificates for the database connections that we're initiating. In a
    // support ticket they've clarified that the certs are self-signed and
    // relatively ephemeral. As a result we have no choice but to disable
    // verification of the certificate.
    //
    // We use the standard `SslConnector` from the `openssl` crate here as it
    // will configure a number of other security parameters, but we use it in
    // two special ways:
    //
    // 1. We pass `SSL_VERIFY_NONE` to disable verification of the certificate
    //    chain
    // 2. We use a super long and weird method name indicating that we're not
    //    validating the certificate with a domain name, but rather just the
    //    certificate itself.
    //
    // This should get us connecting to Heroku's databases for now and even
    // protect us against passive attackers, but hopefully Heroku offers a
    // better solution in the future for certificate verification...

    impl TlsHandshake for MyHandshake {
        fn tls_handshake(&self,
                         _domain: &str,
                         stream: Stream)
                         -> Result<Box<TlsStream>, Box<Error + Send + Sync>> {
            let stream = self.0.danger_connect_without_providing_domain_for_certificate_verification_and_server_name_indication(stream)?;
            Ok(Box::new(stream))
        }
    }

    impl fmt::Debug for MyHandshake {
        fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
            fmt.debug_struct("MyHandshake").finish()
        }
    }

    let mut builder = SslConnectorBuilder::new(SslMethod::tls()).unwrap();
    builder.builder_mut().set_verify(SSL_VERIFY_NONE);

    Box::new(MyHandshake(builder.build()))
}

// Note that this is intended to be called from scripts, not in the main app, it
// panics!
pub fn connect_now() -> pg::Connection {
    let tls = tls_handshake();
    let mode = if env::var("HEROKU").is_ok() {
        pg::TlsMode::Require(&*tls)
    } else {
        pg::TlsMode::Prefer(&*tls)
    };
    pg::Connection::connect(&::env("DATABASE_URL")[..], mode).unwrap()
}

pub fn pool(url: &str, config: r2d2::Config<postgres::Connection, r2d2_postgres::Error>) -> Pool {
    let mode = if env::var("HEROKU").is_ok() {
        TlsMode::Require(tls_handshake())
    } else {
        TlsMode::Prefer(tls_handshake())
    };
    let mgr = PCM::new(url, mode).unwrap();
    r2d2::Pool::new(config, mgr).unwrap()
}

pub fn diesel_pool(url: &str,
                   config: r2d2::Config<PgConnection, r2d2_diesel::Error>)
                   -> DieselPool {
    let mut url = Url::parse(url).expect("Invalid database URL");
    if env::var("HEROKU").is_ok() && !url.query_pairs().any(|(k, _)| k == "sslmode") {
        url.query_pairs_mut().append_pair("sslmode", "require");
    }
    let manager = ConnectionManager::new(url.into_string());
    r2d2::Pool::new(config, manager).unwrap()
}

pub struct TransactionMiddleware;

pub struct Transaction {
    // fields are destructed top-to-bottom so ensure we destroy them in the
    // right order.
    //
    // Note that `slot` and `PooledConnnection` are intentionally behind a `Box`
    // for safety reasons. The `tx` field will actually be containing a borrow
    // into `PooledConnnection`, but this `Transaction` can be moved around in
    // memory, so we need the borrow to be from a stable address. The `Box` will
    // provide this stable address.
    tx: LazyCell<pg::transaction::Transaction<'static>>,
    slot: LazyCell<Box<PooledConnnection>>,
    commit: Cell<Option<bool>>,

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
            commit: Cell::new(None),
        }
    }

    pub fn conn
        (&self)
         -> CargoResult<&r2d2::PooledConnection<r2d2_postgres::PostgresConnectionManager>> {
        if !self.slot.filled() {
            let conn =
                self.app
                    .database
                    .get()
                    .map_err(|e| {
                        internal(&format_args!("failed to get a database connection: {}", e))
                    })?;
            self.slot.fill(Box::new(conn));
        }
        Ok(&**self.slot.borrow().unwrap())
    }

    fn tx(&self) -> CargoResult<&GenericConnection> {
        // Similar to above, the transaction for this request is actually tied
        // to the connection in the request itself, not 'static. We transmute it
        // to static as its paired with the inner connection to achieve the
        // desired effect.
        unsafe {
            if !self.tx.filled() {
                let conn = self.conn()?;
                let t = conn.transaction()?;
                let t = mem::transmute::<_, pg::transaction::Transaction<'static>>(t);
                self.tx.fill(t);
            }
        }
        let tx = self.tx.borrow();
        let tx: &pg::transaction::Transaction<'static> = tx.unwrap();
        Ok(tx)
    }

    pub fn rollback(&self) {
        self.commit.set(Some(false));
    }

    pub fn commit(&self) {
        if self.commit.get().is_none() {
            self.commit.set(Some(true));
        }
    }
}

impl Middleware for TransactionMiddleware {
    fn before(&self, req: &mut Request) -> Result<(), Box<Error + Send>> {
        let app = req.app().clone();
        req.mut_extensions().insert(Transaction::new(app));
        Ok(())
    }

    fn after(&self,
             req: &mut Request,
             res: Result<Response, Box<Error + Send>>)
             -> Result<Response, Box<Error + Send>> {
        let tx = req.mut_extensions()
            .pop::<Transaction>()
            .expect("Transaction not present in request");
        if let Some(transaction) = tx.tx.into_inner() {
            if res.is_ok() && tx.commit.get() == Some(true) {
                transaction.set_commit();
            }
            transaction
                .finish()
                .map_err(|e| Box::new(e) as Box<Error + Send>)?;
        }
        res
    }
}

pub trait RequestTransaction {
    /// Return the lazily initialized postgres connection for this request.
    ///
    /// The connection will live for the lifetime of the request.
    fn db_conn(&self) -> CargoResult<DieselPooledConn>;

    /// Return the lazily initialized postgres transaction for this request.
    ///
    /// The transaction will live for the duration of the request, and it will
    /// only be set to commit() if a successful response code of 200 is seen.
    fn tx(&self) -> CargoResult<&GenericConnection>;

    /// Flag the transaction to not be committed. Does not affect Diesel connections
    fn rollback(&self);
    /// Flag this transaction to be committed. Does not affect Diesel connections.
    fn commit(&self);
}

impl<T: Request + ?Sized> RequestTransaction for T {
    fn db_conn(&self) -> CargoResult<DieselPooledConn> {
        self.app().diesel_database.get().map_err(Into::into)
    }

    fn tx(&self) -> CargoResult<&GenericConnection> {
        self.extensions()
            .find::<Transaction>()
            .expect("Transaction not present in request")
            .tx()
    }

    fn rollback(&self) {
        self.extensions()
            .find::<Transaction>()
            .expect("Transaction not present in request")
            .rollback()
    }

    fn commit(&self) {
        self.extensions()
            .find::<Transaction>()
            .expect("Transaction not present in request")
            .commit()
    }
}
