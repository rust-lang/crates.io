use super::request::RequestBuilder;
use cargo_registry::models::{ApiToken, Email, NewUser, User};
use cargo_registry::util::CargoResult;
use conduit::Method;
use conduit_middleware::MiddlewareBuilder;
use diesel::prelude::*;
use std::sync::Arc;

pub struct App {
    app: Arc<cargo_registry::App>,
    middleware: MiddlewareBuilder,
    _bomb: crate::record::Bomb,
}

impl App {
    pub fn new() -> Self {
        let (bomb, app, middleware) = crate::app();
        Self {
            app,
            middleware,
            _bomb: bomb,
        }
    }

    /// Obtain the database connection and pass it to the closure
    ///
    /// Our tests share a database connection with the app server, so it's
    /// important that the conenction is dropped before requests are made to
    /// ensure it's available for the server to use. The connection will be
    /// returned to the server after the given function returns.
    pub fn db<T, F>(&self, f: F) -> CargoResult<T>
    where
        F: FnOnce(&PgConnection) -> CargoResult<T>,
    {
        let conn = self.app.diesel_database.get()?;
        f(&conn)
    }

    /// Create a new user in the database with the given id
    pub fn create_user(&self, username: &str) -> CargoResult<User> {
        use cargo_registry::schema::emails;

        self.db(|conn| {
            let new_user = NewUser {
                email: Some("something@example.com"),
                ..crate::new_user(username)
            };
            let user = new_user.create_or_update(conn)?;
            diesel::update(Email::belonging_to(&user))
                .set(emails::verified.eq(true))
                .execute(conn)?;
            Ok(user)
        })
    }

    /// Sets the database in read only mode.
    ///
    /// Any attempts to modify the database after calling this function will
    /// result in an error.
    pub fn set_read_only(&self) -> CargoResult<()> {
        self.db(|conn| {
            diesel::sql_query("SET TRANSACTION READ ONLY").execute(conn)?;
            Ok(())
        })
    }

    /// Create an HTTP `GET` request
    pub fn get(&self, path: &str) -> RequestBuilder<'_> {
        RequestBuilder::new(&self.middleware, Method::Get, path)
    }

    /// Create an HTTP `PUT` request
    pub fn put(&self, path: &str) -> RequestBuilder<'_> {
        RequestBuilder::new(&self.middleware, Method::Put, path)
    }

    /// Create an HTTP `DELETE` request
    pub fn delete(&self, path: &str) -> RequestBuilder<'_> {
        RequestBuilder::new(&self.middleware, Method::Delete, path)
    }

    /// Returns the first API token for the given user, or creates a new one
    pub fn token_for(&self, user: &User) -> CargoResult<ApiToken> {
        self.db(|conn| {
            ApiToken::belonging_to(user)
                .first(conn)
                .optional()?
                .map(Ok)
                .unwrap_or_else(|| {
                    ApiToken::insert(conn, user.id, "test_token").map_err(Into::into)
                })
        })
    }
}
