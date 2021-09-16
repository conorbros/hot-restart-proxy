use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, mpsc, Semaphore};
use tokio::time;

use crate::handler::Handler;
use crate::shutdown::Shutdown;

pub struct Listener {
    pub listener: TcpListener,
    pub upstream: String,
    pub notify_shutdown: broadcast::Sender<()>,
    pub shutdown_complete_rx: mpsc::Receiver<()>,
    pub shutdown_complete_tx: mpsc::Sender<()>,
    pub limit_connections: Arc<Semaphore>,
}

impl Listener {
    pub async fn run(&mut self) -> Result<()> {
        loop {
            self.limit_connections.acquire().await.unwrap().forget();

            let socket = self.accept().await?;

            let mut handler = Handler {
                socket,
                upstream: self.upstream.clone(),
                shutdown: Shutdown::new(self.notify_shutdown.subscribe()),
                _shutdown_complete: self.shutdown_complete_tx.clone(),
                limit_connections: self.limit_connections.clone(),
            };

            tokio::spawn(async move {
                if let Err(err) = handler.run().await {
                    eprintln!("{} connection error", err);
                }
            });
        }
    }

    async fn accept(&mut self) -> Result<TcpStream> {
        let mut backoff = 1;
        loop {
            match self.listener.accept().await {
                Ok((socket, _)) => return Ok(socket),
                Err(err) => {
                    if backoff > 64 {
                        return Err(err.into());
                    }
                }
            }

            time::sleep(Duration::from_secs(backoff)).await;
            backoff *= 2;
        }
    }
}
