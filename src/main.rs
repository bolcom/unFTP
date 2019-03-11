mod config;

use crate::config::EnvVar;
use env_logger::Env;
use firetrap::Server;
use log::*;
use std::env;
use std::process::Command;

const APP_NAME: &str = "unFTP";
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const APP_HASH: &str = env!("HASH");

const ENV_UNFTP_ADDRESS: EnvVar = EnvVar::WithDefault("UNFTP_ADDRESS", "127.0.0.1:2121");
const ENV_LOG_REDIS_KEY: EnvVar = EnvVar::NoDefault("LOG_REDIS_KEY");
const ENV_LOG_REDIS_HOST: EnvVar = EnvVar::WithDefault("LOG_REDIS_HOST", "localhost");
const ENV_LOG_REDIS_PORT: EnvVar = EnvVar::WithDefault("LOG_REDIS_PORT", "6379");

fn init_logging() {
    // TODO: Get something that works on windows too
    fn get_host_name() -> String {
        let output = Command::new("hostname")
            .output()
            .expect("failed to execute process");
        format!(
            "{}",
            String::from_utf8_lossy(&output.stdout).replace("\n", "")
        )
    }

    if ENV_LOG_REDIS_KEY.provided() {
        info!(
            "Redis logger coming soon for {}. Will log to {}:{} using key {}",
            get_host_name(),
            ENV_LOG_REDIS_HOST.val(),
            ENV_LOG_REDIS_PORT.val(),
            ENV_LOG_REDIS_KEY.val()
        );
    } else {
        env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
        info!("Env logger initialized.");
    }
}

fn main() {
    init_logging();
    let addr = ENV_UNFTP_ADDRESS.val();
    let server = Server::with_root(env::temp_dir()).greeting("Welcome to unFTP");

    info!(
        "Starting {} server version {}({}) on {}.",
        APP_NAME, APP_VERSION, APP_HASH, addr
    );
    server.listen(&addr);
}
