use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{broadcast, mpsc};
use tokio::sync::{watch, Semaphore};

use crate::connections::SocketPair;
use crate::listener::Listener;

const MAX_CONNECTIONS: usize = 250;

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    upstream: String,
}

#[derive(Clone)]
pub struct Server {
    upstream: String,
    takeover: bool,
}

impl Server {
    pub fn new(config_path: String) -> Result<Self> {
        let config_file = fs::read_to_string(config_path)?;
        let config: Config = serde_yaml::from_str(&config_file)?;

        Ok(Server {
            upstream: config.upstream,
            takeover: false,
        })
    }

    pub fn set_takeover(mut self, v: bool) -> Self {
        self.takeover = v;
        self
    }

    pub async fn run(self, mut shutdown_rx: watch::Receiver<bool>) {
        let tcp_listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();

        let (notify_shutdown, _) = broadcast::channel(1);
        let (shutdown_complete_tx, shutdown_complete_rx) = mpsc::unbounded_channel::<SocketPair>();

        let mut listener = Listener {
            listener: tcp_listener,
            upstream: self.upstream,
            notify_shutdown,
            shutdown_complete_rx,
            shutdown_complete_tx,
            limit_connections: Arc::new(Semaphore::new(MAX_CONNECTIONS)),
        };

        tokio::select! {
            res = listener.run() => {
                if let Err(err) = res {
                    eprintln!("{}", err);
                }
            }
            _ = shutdown_rx.changed() => {
                println!("shutting down");
            }
        }

        let Listener {
            mut shutdown_complete_rx,
            shutdown_complete_tx,
            notify_shutdown,
            ..
        } = listener;

        drop(notify_shutdown);
        drop(shutdown_complete_tx);

        println!("{:?}", shutdown_complete_rx.recv().await);
        println!("{:?}", shutdown_complete_rx.recv().await);
        println!("{:?}", shutdown_complete_rx.recv().await);
    }
}
