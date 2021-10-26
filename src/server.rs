use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpSocket, TcpStream};
use tokio::sync::{broadcast, mpsc};
use tokio::sync::{watch, Semaphore};

use crate::ipc::{self, unix_socket_listen, Message, SocketPair};
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

    async fn get_resources() -> Result<(Vec<SocketPair>, tokio_unix_ipc::Sender<Message>)> {
        let (tx, rx) = unix_socket_listen().await.unwrap();
        tx.send(Message::Takeover()).await.unwrap();

        let resources = match rx.recv().await.unwrap() {
            Message::Resources(r) => r,
            _ => panic!("invalid response to Message::Takeover"),
        };

        let mut socket_pairs = Vec::<SocketPair>::new();

        for socket_pair in resources.socket_pairs {
            socket_pairs.push(SocketPair {
                client: TcpStream::from_std(socket_pair.client.into_inner())?,
                upstream: TcpStream::from_std(socket_pair.upstream.into_inner())?,
            })
        }

        Ok((socket_pairs, tx))
    }

    pub async fn run(
        self,
        takeover: bool,
        mut shutdown_rx: watch::Receiver<bool>,
        handover_tx: mpsc::UnboundedSender<ipc::Socket>,
    ) {
        println!("listening on 127.0.0.1:8080");

        let (notify_shutdown, _) = broadcast::channel(1);
        let (shutdown_complete_tx, shutdown_complete_rx) = mpsc::unbounded_channel::<()>();

        let mut listener = Listener {
            listener: None,
            upstream: self.upstream,
            notify_shutdown,
            shutdown_complete_rx,
            shutdown_complete_tx,
            handover_tx: handover_tx.clone(),
            limit_connections: Arc::new(Semaphore::new(MAX_CONNECTIONS)),
        };

        if takeover {
            let (socket_pairs, tx) = Server::get_resources().await.unwrap();
            listener.handover_run(socket_pairs).await.unwrap();

            tx.send(Message::Shutdown()).await.unwrap();
        } else {
            let tcp_listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
            listener.listener = Some(tcp_listener);
        }

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
            shutdown_complete_tx,
            notify_shutdown,
            ..
        } = listener;

        drop(notify_shutdown);
        drop(shutdown_complete_tx);

        match listener.listener {
            Some(l) => handover_tx.send(ipc::Socket::Listener(l)).unwrap(),
            _ => {}
        };
    }
}
