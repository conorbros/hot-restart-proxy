use anyhow::Result;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::sync::Semaphore;

use crate::shutdown::Shutdown;

#[derive(Debug)]
pub struct Handler {
    pub socket: TcpStream,
    pub upstream: String,
    pub shutdown: Shutdown,
    pub _shutdown_complete: mpsc::Sender<()>,
    pub limit_connections: Arc<Semaphore>,
}

impl Handler {
    pub async fn run(&mut self) -> Result<()> {
        while !self.shutdown.is_shutdown() {
            let mut upstream = TcpStream::connect(&self.upstream).await?;

            let (mut client_r, _client_w) = self.socket.split();
            let (_upstream_r, mut upstream_w) = upstream.split();

            tokio::select! {
                _ = tokio::io::copy(&mut client_r, &mut upstream_w) => {}
                _ = self.shutdown.recv() => {
                    return Ok(());
                }
            };
        }

        Ok(())
    }
}

impl Drop for Handler {
    fn drop(&mut self) {
        self.limit_connections.add_permits(1);
    }
}
