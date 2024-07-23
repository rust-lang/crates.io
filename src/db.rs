use diesel::{Connection, ConnectionResult, PgConnection, QueryResult};
use diesel_async::pooled_connection::deadpool::{Hook, HookError};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use secrecy::ExposeSecret;
use std::time::Duration;
use url::Url;

use crate::config;

pub fn oneoff_connection_with_config(
    config: &config::DatabasePools,
) -> ConnectionResult<PgConnection> {
    let url = connection_url(config, config.primary.url.expose_secret());
    PgConnection::establish(&url)
}

pub fn oneoff_connection() -> anyhow::Result<PgConnection> {
    let config = config::DatabasePools::full_from_environment(&config::Base::from_environment()?)?;
    oneoff_connection_with_config(&config).map_err(Into::into)
}

pub fn connection_url(config: &config::DatabasePools, url: &str) -> String {
    let mut url = Url::parse(url).expect("Invalid database URL");

    if config.enforce_tls {
        maybe_append_url_param(&mut url, "sslmode", "require");
    }

    // Configure the time it takes for diesel to return an error when there is full packet loss
    // between the application and the database.
    maybe_append_url_param(
        &mut url,
        "tcp_user_timeout",
        &config.tcp_timeout_ms.to_string(),
    );

    url.into()
}

fn maybe_append_url_param(url: &mut Url, key: &str, value: &str) {
    if !url.query_pairs().any(|(k, _)| k == key) {
        url.query_pairs_mut().append_pair(key, value);
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ConnectionConfig {
    pub statement_timeout: Duration,
    pub read_only: bool,
}

impl ConnectionConfig {
    async fn apply(&self, conn: &mut AsyncPgConnection) -> QueryResult<()> {
        diesel::sql_query("SET application_name = 'crates.io'")
            .execute(conn)
            .await?;

        let statement_timeout = self.statement_timeout.as_millis();
        diesel::sql_query(format!("SET statement_timeout = {statement_timeout}"))
            .execute(conn)
            .await?;

        if self.read_only {
            diesel::sql_query("SET default_transaction_read_only = 't'")
                .execute(conn)
                .await?;
        }

        Ok(())
    }
}

impl From<ConnectionConfig> for Hook<AsyncPgConnection> {
    fn from(config: ConnectionConfig) -> Self {
        Hook::async_fn(move |conn, _| {
            Box::pin(async move {
                let result = config.apply(conn).await;
                result.map_err(|err| HookError::message(err.to_string()))
            })
        })
    }
}
