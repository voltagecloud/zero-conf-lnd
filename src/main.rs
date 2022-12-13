use lnd::start_channel_acceptor;

use crate::{config::Config, lnd::create_client};

mod config;
mod lnd;

#[tokio::main]
async fn main() {
    // Set the RUST_LOG, if it hasn't been explicitly defined
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "lsp_server=debug,tower_http=debug")
    }
    tracing_subscriber::fmt::init();

    let cfg = Config::new();
    let lnd_client = create_client(cfg.lnd).await;
    start_channel_acceptor(lnd_client, cfg.channel_acceptance).await
}
