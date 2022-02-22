use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio_unix_ipc::{serde::Handle, Bootstrapper, Receiver, Sender};

#[derive(Serialize, Deserialize)]
pub enum Message {
    Takeover(),           // Initiate the takeover, old process should respond with resources
    Resources(Resources), // Send the resources over, i.e. socket FDs
    Shutdown(),           // Takeover is complete, old process should shutdown
    GiveSender(Sender<Message>), // Old process receives the Sender half of channel
}

#[derive(Debug)]
pub enum Socket {
    SocketPair(SocketPair),
    Listener(tokio::net::TcpListener),
}

#[derive(Debug)]
pub struct SocketPair {
    pub client: tokio::net::TcpStream,
    pub upstream: tokio::net::TcpStream,
}

// A socket pair that can be sent over IPC
#[derive(Serialize, Deserialize)]
pub struct IPCSocketPair {
    pub client: Handle<std::net::TcpStream>,
    pub upstream: Handle<std::net::TcpStream>,
}

#[derive(Serialize, Deserialize)]
pub struct Resources {
    pub socket_pairs: Vec<IPCSocketPair>,
    pub listeners: Vec<Handle<std::net::TcpListener>>,
}

pub async fn unix_socket_bootstrap() -> Result<(Bootstrapper, Sender<Message>, Receiver<Message>)> {
    let bootstrapper = tokio_unix_ipc::Bootstrapper::bind("/tmp/proto-socket")?;
    let (tx, rx) = tokio_unix_ipc::channel::<Message>()?;
    println!("bootstrapped socket");

    Ok((bootstrapper, tx, rx))
}

pub async fn unix_socket_listen() -> Result<(Sender<Message>, Receiver<Message>)> {
    let rx = Receiver::<Message>::connect("/tmp/proto-socket").await?;
    println!("socket listening");
    let tx = match rx.recv().await.unwrap() {
        Message::GiveSender(tx) => tx,
        _ => panic!("Expected Message::Sender"),
    };

    println!("sender received");

    Ok((tx, rx))
}

pub async fn collect_socket_pairs(
    mut shutdown_complete_rx: tokio::sync::mpsc::UnboundedReceiver<Socket>,
) -> Result<Resources> {
    let mut resources = Resources {
        socket_pairs: Vec::new(),
        listeners: Vec::new(),
    };

    while let Some(val) = shutdown_complete_rx.recv().await {
        match val {
            Socket::Listener(listener) => {
                resources.listeners.push(Handle::new(listener.into_std()?))
            }
            Socket::SocketPair(socket_pair) => {
                resources.socket_pairs.push(IPCSocketPair {
                    client: Handle::new(socket_pair.client.into_std()?),
                    upstream: Handle::new(socket_pair.upstream.into_std()?),
                });
            }
        }
    }

    Ok(resources)
}
