use anyhow::Result;
use net2::{unix::UnixTcpBuilderExt, TcpBuilder};
use std::fs;
use tokio::net::{TcpListener, TcpStream};
use tokio::signal::unix::{signal, SignalKind};

use serde::{Deserialize, Serialize};

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

    pub async fn startup(self) -> Result<()> {
        self.server().await?;
        Ok(())
    }

    async fn server(self) -> Result<()> {
        let listener = TcpListener::bind("127.0.0.1:8080").await?;

        let mut interrupt = signal(SignalKind::interrupt()).unwrap();

        tokio::select! {
            _ = async {
                loop {
                    let (mut socket, _) = listener.accept().await?;
                    let upstream = self.upstream.clone();
                    tokio::spawn(async move {
                        let (mut client_r, _client_w) = socket.split();

                        let mut stream = TcpStream::connect(upstream).await.unwrap();
                        let (_upstream_r, mut upstream_w) = stream.split();
                        tokio::io::copy(&mut client_r, &mut upstream_w).await.unwrap();
                    });
                }

                #[allow(unreachable_code)]
                Ok::<_, tokio::io::Error>(())
            } => {},
            _ = interrupt.recv() => {},
        };

        Ok(())
    }
}
