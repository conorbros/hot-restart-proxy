use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, mpsc, Semaphore};
use tokio::time;

use crate::{handler::Handler, ipc, shutdown::Shutdown};

pub struct Listener {
    pub listener: Option<TcpListener>,
    pub upstream: String,
    pub notify_shutdown: broadcast::Sender<()>,
    pub shutdown_complete_rx: mpsc::UnboundedReceiver<()>,
    pub shutdown_complete_tx: mpsc::UnboundedSender<()>,
    pub handover_tx: mpsc::UnboundedSender<ipc::Socket>,
    pub limit_connections: Arc<Semaphore>,
}

impl Listener {
    pub async fn run(&mut self) -> Result<()> {
        loop {
            self.limit_connections.acquire().await.unwrap().forget();

            let client = self.accept().await?;
            let upstream = TcpStream::connect(&self.upstream).await?;

            let handler = Handler {
                shutdown: Shutdown::new(self.notify_shutdown.subscribe()),
                shutdown_complete: self.shutdown_complete_tx.clone(),
                handover: self.handover_tx.clone(),
                limit_connections: self.limit_connections.clone(),
            };

            tokio::spawn(async move {
                if let Err(err) = handler.run(client, upstream).await {
                    eprintln!("{} connection error", err);
                }
            });
        }
    }

    pub async fn handover_run(&mut self, socket_pairs: Vec<ipc::SocketPair>) -> Result<()> {
        for socket_pair in socket_pairs {
            self.limit_connections.acquire().await.unwrap().forget();

            let handler = Handler {
                shutdown: Shutdown::new(self.notify_shutdown.subscribe()),
                shutdown_complete: self.shutdown_complete_tx.clone(),
                handover: self.handover_tx.clone(),
                limit_connections: self.limit_connections.clone(),
            };

            tokio::spawn(async move {
                if let Err(err) = handler.run(socket_pair.client, socket_pair.upstream).await {
                    eprintln!("{} connection error", err);
                }
            });
        }

        Ok(())
    }

    async fn accept(&mut self) -> Result<TcpStream> {
        let mut backoff = 1;

        match &self.listener {
            Some(listener) => loop {
                match listener.accept().await {
                    Ok((socket, _)) => return Ok(socket),
                    Err(err) => {
                        if backoff > 64 {
                            return Err(err.into());
                        }
                    }
                }

                time::sleep(Duration::from_secs(backoff)).await;
                backoff *= 2;
            },
            _ => panic!("listener was not set"),
        }
    }
}
