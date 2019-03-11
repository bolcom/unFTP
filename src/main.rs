mod config;
mod redislog;

use crate::config::Arg;
use env_logger::Env;
use firetrap::Server;
use log::*;
use std::env;

const APP_NAME: &str = "unFTP";
const APP_VERSION: &str = env!("BUILD_VERSION");

const ENV_UNFTP_ADDRESS: Arg = Arg::WithDefault("UNFTP_ADDRESS", "0.0.0.0:2121");
const ENV_LOG_REDIS_KEY: Arg = Arg::NoDefault("LOG_REDIS_KEY");
const ENV_LOG_REDIS_HOST: Arg = Arg::WithDefault("LOG_REDIS_HOST", "localhost");
const ENV_LOG_REDIS_PORT: Arg = Arg::WithDefault("LOG_REDIS_PORT", "6379");

fn init_logging() {
    if ENV_LOG_REDIS_KEY.provided() {
        let logger = redislog::Builder::new(APP_NAME)
            .redis(
                ENV_LOG_REDIS_HOST.val(),
                ENV_LOG_REDIS_PORT.val().parse::<u32>().unwrap(),
                ENV_LOG_REDIS_KEY.val(),
            )
            .build()
            .expect("could not initialize Redis logger");

        log::set_boxed_logger(Box::new(logger))
            .map(|()| log::set_max_level(LevelFilter::Info))
            .expect("could not set Redis logger");
        info!("Redis logger initialized.");
        return;
    }

    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    info!("Env logger initialized.");
}

fn main() {
    init_logging();
    let addr = ENV_UNFTP_ADDRESS.val();
    let server = Server::with_root(env::temp_dir()).greeting("Welcome to unFTP");
    info!("Starting {} server version {} on {}.", APP_NAME, APP_VERSION, addr);
    server.listen(&addr);
}
