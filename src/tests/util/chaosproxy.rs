use std::net::SocketAddr;
use std::str::FromStr as _;
use std::sync::Arc;

use anyhow::{Context, anyhow};
use futures_util::FutureExt as _;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
#[cfg(unix)]
use tokio::net::UnixStream;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast::Sender;
use tokio_postgres::Config;
use tokio_postgres::config::Host;
use tracing::{debug, error};
use url::Url;

pub(crate) struct ChaosProxy {
    address: SocketAddr,
    backend_config: Config,

    break_networking_send: Sender<()>,
    restore_networking_send: Sender<()>,
}

impl ChaosProxy {
    pub(crate) async fn new(backend_config: Config) -> anyhow::Result<Arc<Self>> {
        debug!(?backend_config, "Creating ChaosProxy");

        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        debug!("ChaosProxy listening on {address}");

        let (break_networking_send, _) = tokio::sync::broadcast::channel(16);
        let (restore_networking_send, _) = tokio::sync::broadcast::channel(16);

        let instance = Arc::new(ChaosProxy {
            address,
            backend_config,

            break_networking_send,
            restore_networking_send,
        });

        debug!("Spawning ChaosProxy server loop");
        let instance_clone = instance.clone();
        tokio::spawn(async move {
            if let Err(error) = instance_clone.server_loop(listener).await {
                error!(%error, "ChaosProxy server error");
            }
        });

        Ok(instance)
    }

    pub(crate) async fn proxy_database_url(url: &str) -> anyhow::Result<(Arc<Self>, String)> {
        let backend_config =
            Config::from_str(url).context("failed to parse database url as config")?;

        let mut db_url = Url::parse(url).context("failed to parse database url")?;

        let instance = ChaosProxy::new(backend_config).await?;

        db_url
            .set_ip_host(instance.address.ip())
            .map_err(|_| anyhow!("Failed to set IP host on the URL"))?;

        db_url
            .set_port(Some(instance.address.port()))
            .map_err(|_| anyhow!("Failed to set post on the URL"))?;

        debug!("ChaosProxy database URL: {db_url}");

        Ok((instance, db_url.into()))
    }

    pub(crate) fn break_networking(&self) -> anyhow::Result<usize> {
        self.break_networking_send
            .send(())
            .context("Failed to send the break_networking message")
    }

    pub(crate) fn restore_networking(&self) -> anyhow::Result<usize> {
        self.restore_networking_send
            .send(())
            .context("Failed to send the restore_networking message")
    }

    async fn server_loop(&self, initial_listener: TcpListener) -> anyhow::Result<()> {
        let mut listener = Some(initial_listener);

        let mut break_networking_recv = self.break_networking_send.subscribe();
        let mut restore_networking_recv = self.restore_networking_send.subscribe();

        loop {
            if let Some(l) = &listener {
                debug!("ChaosProxy waiting for connections");
                tokio::select! {
                    accepted = l.accept() => {
                        let (stream, address ) = accepted?;
                        debug!("ChaosProxy accepted connection from {address}");
                        self.accept_connection(stream).await?;
                    },

                    _ = break_networking_recv.recv() => {
                        debug!("ChaosProxy breaking networking");

                        // Setting the listener to `None` results in the listener being dropped,
                        // which closes the network port. A new listener will be established when
                        // networking is restored.
                        listener = None;
                    },
                };
            } else {
                debug!("ChaosProxy networking is broken, waiting for restore signal");
                let _ = restore_networking_recv.recv().await;
                debug!("ChaosProxy restoring networking");
                listener = Some(TcpListener::bind(self.address).await?);
            }
        }
    }

    async fn accept_connection(&self, accepted: TcpStream) -> anyhow::Result<()> {
        let (client_read, client_write) = accepted.into_split();

        let host = self.backend_config.get_hosts().first().unwrap();
        let port = self.backend_config.get_ports().first().unwrap();

        let (backend_to_client, client_to_backend) = match &host {
            Host::Tcp(hostname) => {
                let (backend_read, backend_write) = TcpStream::connect((hostname.as_ref(), *port))
                    .await?
                    .into_split();
                (
                    proxy_data(
                        self.break_networking_send.clone(),
                        client_read,
                        backend_write,
                    )
                    .boxed(),
                    proxy_data(
                        self.break_networking_send.clone(),
                        backend_read,
                        client_write,
                    )
                    .boxed(),
                )
            }
            #[cfg(not(unix))]
            Host::Unix(_) => panic!("Unix sockets not supported on this platform"),
            #[cfg(unix)]
            Host::Unix(path) => {
                let path = path.join(format!(".s.PGSQL.{port}"));
                let (backend_read, backend_write) = UnixStream::connect(path).await?.into_split();
                (
                    proxy_data(
                        self.break_networking_send.clone(),
                        client_read,
                        backend_write,
                    )
                    .boxed(),
                    proxy_data(
                        self.break_networking_send.clone(),
                        backend_read,
                        client_write,
                    )
                    .boxed(),
                )
            }
        };

        tokio::spawn(async move {
            if let Err(error) = backend_to_client.await {
                error!(%error, "ChaosProxy connection error");
            }
        });

        tokio::spawn(async move {
            if let Err(error) = client_to_backend.await {
                error!(%error, "ChaosProxy connection error");
            }
        });

        Ok(())
    }
}

async fn proxy_data(
    break_networking_send: Sender<()>,
    mut from: impl AsyncRead + Unpin,
    mut to: impl AsyncWrite + Unpin,
) -> anyhow::Result<()> {
    let mut break_connections_recv = break_networking_send.subscribe();
    let mut buf = [0; 1024];

    loop {
        tokio::select! {
            len = from.read(&mut buf) => {
                let len = len?;
                if len == 0 {
                    // EOF, the socket was closed
                    return Ok(());
                }
                to.write_all(&buf[0..len]).await?;
            }
            _ = break_connections_recv.recv() => {
                to.shutdown().await?;
                return Ok(());
            }
        }
    }
}
