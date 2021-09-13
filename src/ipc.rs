use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::TcpStream;
use unix_ipc::{Bootstrapper, Handle, Receiver, Sender};

#[derive(Serialize, Deserialize)]
pub enum Message {
    Takeover(),
    Resources(Resources),
    Shutdown(),
    Sender(Sender<Message>),
}

#[derive(Serialize, Deserialize)]
pub struct Resources {
    pub upstreams: HashMap<String, Handle<TcpStream>>,
    pub clients: HashMap<String, Handle<TcpStream>>,
}

pub fn unix_socket_bootstrap() -> (Bootstrapper<Message>, Receiver<Message>) {
    let bootstrapper = unix_ipc::Bootstrapper::<Message>::bind("/tmp/proto-socket").unwrap();
    let (tx, rx) = unix_ipc::channel::<Message>().unwrap();
    bootstrapper.send(Message::Sender(tx)).unwrap();
    (bootstrapper, rx)
}

pub fn unix_socket_listen() -> (Sender<Message>, Receiver<Message>) {
    let rx = Receiver::<Message>::connect("/tmp/proto-socket").unwrap();
    let tx = match rx.recv().unwrap() {
        Message::Sender(tx) => tx,
        _ => panic!("Expected Message::Sender"),
    };

    (tx, rx)
}
