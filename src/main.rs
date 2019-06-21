mod config;
mod redislog;

extern crate futures;
extern crate hyper;
extern crate prometheus;
extern crate slog;
extern crate slog_async;
extern crate slog_term;

use crate::config::Arg;
use firetrap::Server;
use futures::future;
use hyper::rt::{self, Future};
use hyper::service::service_fn;
use hyper::{Body, Method, Request, Response, StatusCode};
use prometheus::{Encoder, TextEncoder};
use std::env;
use std::thread;

use slog::*;

const APP_NAME: &str = "unFTP";
const APP_VERSION: &str = env!("BUILD_VERSION");

const ENV_UNFTP_ADDRESS: Arg = Arg::WithDefault("UNFTP_ADDRESS", "0.0.0.0:2121");
const ENV_UNFTP_HOME: Arg = Arg::NoDefault("UNFTP_HOME");
const ENV_CERTS_FILE: Arg = Arg::NoDefault("CERTS_FILE");
const ENV_KEY_FILE: Arg = Arg::NoDefault("KEY_FILE");
const ENV_METRICS_ADDRESS: Arg = Arg::WithDefault("METRICS_ADDRESS", "0.0.0.0:9522"); // Re-use default port allocation of the TFTP Exporter
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

type BoxFuture = Box<Future<Item = Response<Body>, Error = hyper::Error> + Send>;

fn metrics_service(req: Request<Body>) -> BoxFuture {
    let mut response = Response::new(Body::empty());
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/metrics") => {
            *response.body_mut() = Body::from(gather_metrics());
        }
        _ => {
            *response.status_mut() = StatusCode::NOT_FOUND;
        }
    };

    Box::new(future::ok(response))
}

fn gather_metrics() -> Vec<u8> {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = vec![];
    encoder.encode(&metric_families, &mut buffer).unwrap();
    return buffer;
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

    // HTTP server for exporting Prometheus metrics
    let http_addr = ENV_METRICS_ADDRESS
        .val()
        .parse()
        .expect(format!("Unable to parse metrics address {}", ENV_METRICS_ADDRESS.val()).as_str());
    let http_log = log.clone();
    let http_server = hyper::Server::bind(&http_addr)
        .serve(|| service_fn(metrics_service))
        .map_err(move |e| error!(http_log, "HTTP Server error: {}", e));
    info!(log, "Starting Prometheus {} exporter.", APP_NAME; "address" => &http_addr);
    let http_thread = thread::spawn(move || {
        rt::run(http_server);
    });

    let addr = ENV_UNFTP_ADDRESS.val();
    let home_dir = ENV_UNFTP_HOME.val_or_else(|_| env::temp_dir().as_path().to_str().unwrap().to_string());
    let use_ftps: bool = ENV_CERTS_FILE.provided() && ENV_KEY_FILE.provided();
    if !use_ftps && (ENV_CERTS_FILE.provided() || ENV_KEY_FILE.provided()) {
        warn!(
            log,
            "Need to set both {} and {}. FTPS still disabled.",
            ENV_CERTS_FILE.name(),
            ENV_KEY_FILE.name()
        )
    }

    info!(log, "Starting {} server.", APP_NAME;
    "version" => APP_VERSION,
    "address" => &addr,
    "home" => home_dir.clone());

    let server = Server::with_root(home_dir).greeting("Welcome to unFTP").with_metrics();
    let ftp_thread = thread::spawn(move || {
        server.listen(&addr);
    });

    http_thread
        .join()
        .expect(format!("The Prometheus {} exporter server thread has panicked", APP_NAME).as_str());
    ftp_thread
        .join()
        .expect(format!("The {} server thread has panicked", APP_NAME).as_str());
}
