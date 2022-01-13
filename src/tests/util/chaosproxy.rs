use anyhow::{Context, Error};
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
use url::Url;

pub(crate) struct ChaosProxy {
    address: SocketAddr,
    backend_address: SocketAddr,

    runtime: Runtime,
    listener: TcpListener,

    break_networking_send: Sender<()>,
    restore_networking_send: Sender<()>,
}

impl ChaosProxy {
    pub(crate) fn new(backend_address: SocketAddr) -> Result<Arc<Self>, Error> {
        let runtime = Runtime::new().expect("failed to create Tokio runtime");
        let listener = runtime.block_on(TcpListener::bind("127.0.0.1:0"))?;

        let (break_networking_send, _) = tokio::sync::broadcast::channel(16);
        let (restore_networking_send, _) = tokio::sync::broadcast::channel(16);

        let instance = Arc::new(ChaosProxy {
            address: listener.local_addr()?,
            backend_address,

            listener,
            runtime,

            break_networking_send,
            restore_networking_send,
        });

        let instance_clone = instance.clone();
        instance.runtime.spawn(async move {
            if let Err(err) = instance_clone.server_loop().await {
                eprintln!("ChaosProxy server error: {err}");
            }
        });

        Ok(instance)
    }

    pub(crate) fn proxy_database_url(url: &str) -> Result<(Arc<Self>, String), Error> {
        let mut db_url = Url::parse(url).context("failed to parse database url")?;
        let backend_addr = db_url
            .socket_addrs(|| Some(5432))
            .context("could not resolve database url")?
            .get(0)
            .copied()
            .ok_or_else(|| anyhow::anyhow!("the database url does not point to any IP"))?;

        let instance = ChaosProxy::new(backend_addr).unwrap();
        db_url.set_ip_host(instance.address.ip()).unwrap();
        db_url.set_port(Some(instance.address.port())).unwrap();
        Ok((instance, db_url.into()))
    }

    pub(crate) fn break_networking(&self) {
        self.break_networking_send
            .send(())
            .expect("failed to send the break_networking message");
    }

    pub(crate) fn restore_networking(&self) {
        self.restore_networking_send
            .send(())
            .expect("failed to send the restore_networking message");
    }

    async fn server_loop(self: Arc<Self>) -> Result<(), Error> {
        let mut break_networking_recv = self.break_networking_send.subscribe();
        let mut restore_networking_recv = self.restore_networking_send.subscribe();

        loop {
            let (client_read, client_write) = tokio::select! {
                accepted = self.listener.accept() => accepted?.0.into_split(),

                // When networking is broken stop accepting connections until restore_networking()
                _ = break_networking_recv.recv() => {
                    let _ = restore_networking_recv.recv().await;
                    continue;
                },
            };
            let (backend_read, backend_write) = TcpStream::connect(&self.backend_address)
                .await?
                .into_split();

            let self_clone = self.clone();
            self.runtime.spawn(async move {
                if let Err(err) = self_clone.proxy_data(client_read, backend_write).await {
                    eprintln!("ChaosProxy connection error: {err}");
                }
            });

            let self_clone = self.clone();
            tokio::spawn(async move {
                if let Err(err) = self_clone.proxy_data(backend_read, client_write).await {
                    eprintln!("ChaosProxy connection error: {err}");
                }
            });
        }
    }

    async fn proxy_data(
        &self,
        mut from: OwnedReadHalf,
        mut to: OwnedWriteHalf,
    ) -> Result<(), Error> {
        let mut break_connections_recv = self.break_networking_send.subscribe();
        let mut buf = [0; 1024];

        loop {
            tokio::select! {
                len = from.read(&mut buf) => {
                    let len = len?;
                    if len == 0 {
                        // EOF, the socket was closed
                        return Ok(());
                    }
                    to.write(&buf[0..len]).await?;
                }
                _ = break_connections_recv.recv() => {
                    to.shutdown().await?;
                    return Ok(());
                }
            }
        }
    }
}
