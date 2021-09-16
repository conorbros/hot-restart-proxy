mod connections;
mod handler;
mod ipc;
mod listener;
mod server;
mod shutdown;

use anyhow::Result;
use clap::{App, Arg};
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::watch::channel;

#[tokio::main]
async fn main() -> Result<()> {
    console_subscriber::init();

    let matches = App::new("Hot Reload Proxy")
        .arg(Arg::with_name("takeover").short("t"))
        .get_matches();

    let (trigger_shutdown_tx, trigger_shutdown_rx) = channel::<bool>(false);

    tokio::spawn(async move {
        let mut interrupt = signal(SignalKind::interrupt()).unwrap();
        let mut terminate = signal(SignalKind::terminate()).unwrap();

        tokio::select! {
            _ = interrupt.recv() => {
                println!("received SIGINT");
            },
            _ = terminate.recv() => {
                println!("received SIGTERM");
            },
        };

        trigger_shutdown_tx.send(true).unwrap();
    });

    server::Server::new("config/config.yaml".to_string())?
        .set_takeover(matches.is_present("takeover"))
        .run(trigger_shutdown_rx)
        .await;

    Ok(())
}
