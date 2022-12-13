use crate::config::Config;

mod config;

#[tokio::main]
async fn main() {
    // Set the RUST_LOG, if it hasn't been explicitly defined
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "lsp_server=debug,tower_http=debug")
    }
    tracing_subscriber::fmt::init();
    let _ = Config::new();
}
