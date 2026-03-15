use crate::certs::CRUNCHY;
use diesel::{ConnectionResult, QueryResult};
use diesel_async::pooled_connection::ManagerConfig;
use diesel_async::pooled_connection::deadpool::{Hook, HookError};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use native_tls::{Certificate, TlsConnector};
use postgres_native_tls::MakeTlsConnector;
use secrecy::ExposeSecret;
use std::time::Duration;
use url::Url;

use crate::config;

pub async fn oneoff_connection_with_config(
    config: &config::DatabasePools,
) -> ConnectionResult<AsyncPgConnection> {
    let url = connection_url(&config.primary);
    establish_async_connection(&url, config.primary.enforce_tls).await
}

pub async fn oneoff_connection() -> anyhow::Result<AsyncPgConnection> {
    let config = config::DatabasePools::full_from_environment(&config::Base::from_environment()?)?;
    Ok(oneoff_connection_with_config(&config).await?)
}

pub fn connection_url(config: &config::DbPoolConfig) -> String {
    let mut url = Url::parse(config.url.expose_secret()).expect("Invalid database URL");

    // Support `postgres:///db_name` shorthand for easier local development.
    if url.host().is_none() {
        maybe_append_url_param(&mut url, "host", "/run/postgresql");
    }

    if config.enforce_tls {
        maybe_append_url_param(&mut url, "sslmode", "require");
    }

    // Configure the time it takes for diesel to return an error when there is full packet loss
    // between the application and the database.
    maybe_append_url_param(
        &mut url,
        "tcp_user_timeout",
        &config.tcp_timeout.as_millis().to_string(),
    );

    url.into()
}

fn maybe_append_url_param(url: &mut Url, key: &str, value: &str) {
    if !url.query_pairs().any(|(k, _)| k == key) {
        url.query_pairs_mut().append_pair(key, value);
    }
}

/// Create a new [ManagerConfig] for the database connection pool, which can
/// be used with [diesel_async::pooled_connection::AsyncDieselConnectionManager::new_with_config()].
pub fn make_manager_config(enforce_tls: bool) -> ManagerConfig<AsyncPgConnection> {
    let mut manager_config = ManagerConfig::default();
    manager_config.custom_setup =
        Box::new(move |url| Box::pin(establish_async_connection(url, enforce_tls)));
    manager_config
}

/// Establish a new database connection with the given URL.
///
/// Adapted from <https://github.com/weiznich/diesel_async/blob/v0.5.0/examples/postgres/pooled-with-rustls/src/main.rs>.
async fn establish_async_connection(
    url: &str,
    enforce_tls: bool,
) -> ConnectionResult<AsyncPgConnection> {
    use diesel::ConnectionError::BadConnection;

    let cert = Certificate::from_pem(CRUNCHY).map_err(|err| BadConnection(err.to_string()))?;

    let connector = TlsConnector::builder()
        .add_root_certificate(cert)
        // On OSX the native TLS stack is complaining about the long validity
        // period of the certificate, so if locally we don't enforce TLS
        // connections, we also don't enforce the validity of the certificate.
        //
        // Similarly, on CI the native TLS stack is complaining about the
        // certificate being self-signed. On CI we are connecting to a locally
        // running database, so we also don't need to enforce the validity of
        // the certificate either.
        .danger_accept_invalid_certs(!enforce_tls)
        .build()
        .map_err(|err| BadConnection(err.to_string()))?;

    let connector = MakeTlsConnector::new(connector);
    let result = tokio_postgres::connect(url, connector).await;
    let (client, conn) = result.map_err(|err| BadConnection(err.to_string()))?;
    AsyncPgConnection::try_from_client_and_connection(client, conn).await
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
