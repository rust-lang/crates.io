use anyhow::{anyhow, Context};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpListener, TcpStream,
    },
    runtime::Runtime,
    sync::broadcast::Sender,
};
use tracing::error;
use url::Url;

pub(crate) struct ChaosProxy {
    address: SocketAddr,
    backend_address: SocketAddr,

    runtime: Runtime,

    break_networking_send: Sender<()>,
    restore_networking_send: Sender<()>,
}

impl ChaosProxy {
    pub(crate) fn new(backend_address: SocketAddr) -> anyhow::Result<Arc<Self>> {
        let runtime = Runtime::new().expect("failed to create Tokio runtime");
        let listener = runtime.block_on(TcpListener::bind("127.0.0.1:0"))?;

        let (break_networking_send, _) = tokio::sync::broadcast::channel(16);
        let (restore_networking_send, _) = tokio::sync::broadcast::channel(16);

        let instance = Arc::new(ChaosProxy {
            address: listener.local_addr()?,
            backend_address,

            runtime,

            break_networking_send,
            restore_networking_send,
        });

        let instance_clone = instance.clone();
        instance.runtime.spawn(async move {
            if let Err(error) = instance_clone.server_loop(listener).await {
                error!(%error, "ChaosProxy server error");
            }
        });

        Ok(instance)
    }

    pub(crate) fn proxy_database_url(url: &str) -> anyhow::Result<(Arc<Self>, String)> {
        let mut db_url = Url::parse(url).context("failed to parse database url")?;
        let backend_addr = db_url
            .socket_addrs(|| Some(5432))
            .context("could not resolve database url")?
            .first()
            .copied()
            .ok_or_else(|| anyhow!("the database url does not point to any IP"))?;

        let instance = ChaosProxy::new(backend_addr)?;

        db_url
            .set_ip_host(instance.address.ip())
            .map_err(|_| anyhow!("Failed to set IP host on the URL"))?;

        db_url
            .set_port(Some(instance.address.port()))
            .map_err(|_| anyhow!("Failed to set post on the URL"))?;

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
                tokio::select! {
                    accepted = l.accept() => {
                        self.accept_connection(accepted?.0).await?;
                    },

                    _ = break_networking_recv.recv() => {
                        // Setting the listener to `None` results in the listener being dropped,
                        // which closes the network port. A new listener will be established when
                        // networking is restored.
                        listener = None;
                    },
                };
            } else {
                let _ = restore_networking_recv.recv().await;
                listener = Some(TcpListener::bind(self.address).await?);
            }
        }
    }

    async fn accept_connection(&self, accepted: TcpStream) -> anyhow::Result<()> {
        let (client_read, client_write) = accepted.into_split();
        let (backend_read, backend_write) = TcpStream::connect(&self.backend_address)
            .await?
            .into_split();

        let break_networking_send = self.break_networking_send.clone();
        tokio::spawn(async move {
            if let Err(error) = proxy_data(break_networking_send, client_read, backend_write).await
            {
                error!(%error, "ChaosProxy connection error");
            }
        });

        let break_networking_send = self.break_networking_send.clone();
        tokio::spawn(async move {
            if let Err(error) = proxy_data(break_networking_send, backend_read, client_write).await
            {
                error!(%error, "ChaosProxy connection error");
            }
        });

        Ok(())
    }
}

async fn proxy_data(
    break_networking_send: Sender<()>,
    mut from: OwnedReadHalf,
    mut to: OwnedWriteHalf,
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
