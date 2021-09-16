mod connections;
mod ipc;
mod server;

use anyhow::Result;
use clap::{App, Arg};

#[tokio::main]
async fn main() -> Result<()> {
    console_subscriber::init();

    let matches = App::new("Hot Reload Proxy")
        .arg(Arg::with_name("takeover").short("t"))
        .get_matches();

    server::Server::new("config/config.yaml".to_string())?
        .set_takeover(matches.is_present("takeover"))
        .startup()
        .await?;

    Ok(())
}
