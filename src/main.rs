mod config;
mod redislog;

extern crate slog;
extern crate slog_async;
extern crate slog_term;

use crate::config::Arg;
use firetrap::Server;
use std::env;

use slog::*;

const APP_NAME: &str = "unFTP";
const APP_VERSION: &str = env!("BUILD_VERSION");

const ENV_UNFTP_ADDRESS: Arg = Arg::WithDefault("UNFTP_ADDRESS", "0.0.0.0:2121");
const ENV_UNFTP_HOME: Arg = Arg::NoDefault("UNFTP_HOME");
const ENV_CERTS_FILE: Arg = Arg::NoDefault("CERTS_FILE");
const ENV_KEY_FILE: Arg = Arg::NoDefault("KEY_FILE");
const ENV_LOG_REDIS_KEY: Arg = Arg::NoDefault("LOG_REDIS_KEY");
const ENV_LOG_REDIS_HOST: Arg = Arg::WithDefault("LOG_REDIS_HOST", "localhost");
const ENV_LOG_REDIS_PORT: Arg = Arg::WithDefault("LOG_REDIS_PORT", "6379");

fn redis_logger() -> Option<redislog::Logger> {
    if ENV_LOG_REDIS_KEY.provided() {
        let logger = redislog::Builder::new(APP_NAME)
            .redis(
                ENV_LOG_REDIS_HOST.val(),
                ENV_LOG_REDIS_PORT.val().parse::<u32>().unwrap(),
                ENV_LOG_REDIS_KEY.val(),
            )
            .build()
            .expect("could not initialize Redis logger");
        return Some(logger);
    }
    None
}

fn main() {
    let drain = match redis_logger() {
        Some(l) => slog_async::Async::new(l.fuse()).build().fuse(),
        None => {
            let decorator = slog_term::PlainDecorator::new(std::io::stdout());
            let drain = slog_term::CompactFormat::new(decorator).build();
            slog_async::Async::new(drain.fuse()).build().fuse()
        }
    };

    let root = Logger::root(drain.fuse(), o!());
    let log = root.new(o!("module" => "main"));
    let addr = ENV_UNFTP_ADDRESS.val();
    let home_dir = ENV_UNFTP_HOME.val_or_else(|_| env::temp_dir().as_path().to_str().unwrap().to_string());
    let use_ftps: bool = ENV_CERTS_FILE.provided() && ENV_KEY_FILE.provided();
    if !use_ftps && (ENV_CERTS_FILE.provided() || ENV_KEY_FILE.provided()) {
        warn!(log, "Need to set both {} and {}. FTPS still disabled.", ENV_CERTS_FILE.name(), ENV_KEY_FILE.name())
    }

    info!(log, "Starting {} server.", APP_NAME;
    "version" => APP_VERSION,
    "address" => &addr,
    "home" => home_dir.clone());

    let server = Server::with_root(home_dir).greeting("Welcome to unFTP");
    server.listen(&addr);
}
