use anyhow::Result;
use net2::{unix::UnixTcpBuilderExt, TcpBuilder};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::{fs, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::{
    connections::Connections,
    ipc::{unix_socket_bootstrap, unix_socket_listen, Message},
};

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    upstreams: Vec<String>,
}

pub struct Server {
    config: Config,
    takeover: bool,
}

impl Server {
    pub fn new(config_path: String) -> Result<Self> {
        let config_file = fs::read_to_string(config_path)?;
        let config: Config = serde_yaml::from_str(&config_file)?;

        Ok(Server {
            config,
            takeover: false,
        })
    }

    pub fn set_takeover(mut self, v: bool) -> Self {
        self.takeover = v;
        self
    }

    pub fn startup(self) -> Result<()> {
        let conns = if self.takeover {
            self.takeover_startup()?
        } else {
            self.normal_startup()?
        };

        let conns_clone = conns.clone();
        std::thread::spawn(move || {
            let (tx, rx) = unix_socket_bootstrap();

            loop {
                match rx.recv().unwrap() {
                    Message::Takeover() => tx
                        .send(Message::Resources(conns_clone.as_ref().into()))
                        .unwrap(),
                    Message::Shutdown() => break,
                    _ => panic!("unhandled message"),
                }
            }

            std::process::exit(0);
        });

        server(conns)?;
        Ok(())
    }

    fn takeover_startup(self) -> Result<Arc<Connections>> {
        let (tx, rx) = unix_socket_listen();
        tx.send(Message::Takeover())?;

        let mut resources = match rx.recv()? {
            Message::Resources(r) => r,
            _ => panic!("wrong response to takeover"),
        };

        let conns = Connections::new();

        {
            let mut servers = conns.upstream.write().unwrap();
            for host in self.config.upstreams.iter() {
                if let Some(fd) = resources.upstreams.remove(host) {
                    let stream = fd.into_inner();
                    servers.insert(host.to_string(), stream);
                } else {
                    let addr: std::net::SocketAddr = host.parse()?;
                    let stream = TcpStream::connect(addr)?;
                    servers.insert(host.to_string(), stream);
                }
            }

            let mut client = conns.client.write().unwrap();
            for (host, socket) in resources.clients.into_iter() {
                client.insert(host.to_string(), socket.into_inner());
            }
        }

        tx.send(Message::Shutdown())?;
        Ok(Arc::new(conns))
    }

    fn normal_startup(self) -> Result<Arc<Connections>> {
        let conns = Connections::new();

        {
            let mut servers = conns.upstream.write().unwrap();
            for host in self.config.upstreams.iter() {
                let addr: std::net::SocketAddr = host.parse()?;
                let stream = TcpStream::connect(addr)?;
                servers.insert(host.to_string(), stream);
            }
        }

        Ok(Arc::new(conns))
    }
}

fn connection_handler<A: 'static + ToString + std::marker::Send>(
    mut socket: TcpStream,
    address: A,
    conns: Arc<Connections>,
) {
    std::thread::spawn(move || {
        loop {
            let mut buf = [0; 1024];
            match socket.read(&mut buf) {
                Ok(n) if n == 0 => break,
                Ok(n) => n,
                Err(e) => {
                    eprintln!("failed to read from socket; err = {:?}", e);
                    break;
                }
            };

            {
                let mut w = conns.upstream.write().unwrap();
                w.retain(|_, socket| {
                    if socket.write_all(&buf).is_err() {
                        false
                    } else {
                        true
                    }
                })
            }
        }
        let mut fds = conns.client.write().unwrap();
        fds.remove(&address.to_string());
    });
}

fn server(conns: Arc<Connections>) -> Result<()> {
    let listener = TcpBuilder::new_v4()?
        .reuse_address(true)?
        .reuse_port(true)?
        .bind("127.0.0.1:8080")?
        .listen(42)?;

    {
        let client = conns.client.write().unwrap();
        client.iter().for_each(|(host, socket)| {
            connection_handler(socket.try_clone().unwrap(), host.to_string(), conns.clone())
        });
    }

    loop {
        if let Ok((socket, address)) = listener.accept() {
            {
                let mut client = conns.client.write().unwrap();
                client.insert(address.to_string(), socket.try_clone()?);
            };
            connection_handler(socket, address, conns.clone());
        }
    }
}
