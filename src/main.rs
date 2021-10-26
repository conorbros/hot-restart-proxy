mod handler;
mod ipc;
mod listener;
mod server;
mod shutdown;

use anyhow::Result;
use clap::{App, Arg};
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::mpsc;
use tokio::sync::watch;

use crate::ipc::{Message, Socket};

#[tokio::main]
async fn main() -> Result<()> {
    let matches = App::new("Hot Reload Proxy")
        .arg(Arg::with_name("takeover").short("t"))
        .get_matches();

    let (trigger_shutdown_tx, trigger_shutdown_rx) = watch::channel::<bool>(false);
    let (handover_tx, handover_rx) = mpsc::unbounded_channel::<Socket>();

    tokio::spawn(async move {
        let mut interrupt = signal(SignalKind::interrupt()).unwrap();
        let mut terminate = signal(SignalKind::terminate()).unwrap();
        let (mut bootstrapper, tx, rx) = ipc::unix_socket_bootstrap().await.unwrap();

        tokio::select! {
            _ = interrupt.recv() => {
                println!("received SIGINT");
                trigger_shutdown_tx.send(true).unwrap();
            },
            _ = terminate.recv() => {
                println!("received SIGTERM");
                trigger_shutdown_tx.send(true).unwrap();
            },
            _ = bootstrapper.send(Message::GiveSender(tx)) => {
                println!("sent sender");

                match rx.recv().await.unwrap() {
                    Message::Takeover() => {
                        trigger_shutdown_tx.send(true).unwrap();
                        println!("takeover received");
                        let resources = ipc::collect_socket_pairs(handover_rx).await.unwrap();
                        bootstrapper.send(Message::Resources(resources)).await.unwrap();
                    }
                    Message::Shutdown()=> {},
                    _ => panic!("unexpected message received")
                }
            }
        }
    });

    server::Server::new("config/config.yaml".to_string())?
        .run(
            matches.is_present("takeover"),
            trigger_shutdown_rx,
            handover_tx,
        )
        .await;

    Ok(())
}
