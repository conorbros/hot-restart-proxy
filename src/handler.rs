use anyhow::Result;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::sync::Semaphore;

use crate::{ipc, shutdown::Shutdown};

#[derive(Debug)]
pub struct Handler {
    pub shutdown: Shutdown,
    pub shutdown_complete: mpsc::UnboundedSender<()>,
    pub handover: mpsc::UnboundedSender<ipc::Socket>,
    pub limit_connections: Arc<Semaphore>,
}

impl Handler {
    pub async fn run(mut self, mut client: TcpStream, mut upstream: TcpStream) -> Result<()> {
        while !self.shutdown.is_shutdown() {
            let (mut client_r, _client_w) = client.split();
            let (_upstream_r, mut upstream_w) = upstream.split();

            tokio::select! {
                _ = tokio::io::copy(&mut client_r, &mut upstream_w) => {}
                _ = self.shutdown.recv() => {
                    // self.handover.send(
                    //     ipc::SocketPair{
                    //         client,
                    //         upstream
                    //     })?;

                    self.handover.send(ipc::Socket::SocketPair(ipc::SocketPair{ client:client, upstream: upstream }))?;
                    return Ok(())
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
