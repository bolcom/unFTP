use firetrap::Server;
use log::*;
use std::env;
use env_logger::Env;

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let addr = env::var("UNFTP_ADDRESS").unwrap_or("127.0.0.1:2121".to_string());
    let server = Server::with_root(env::temp_dir()).greeting("Welcome to unFTP");

    info!("Starting server on {}", addr);
    server.listen(&addr);
}
