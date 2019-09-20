mod config;
mod redislog;

use crate::config::Arg;

use tokio::runtime::Runtime;
use libunftp::Server;
use libunftp::auth;

use futures::future;
use hyper::rt::Future;
use hyper::service::service_fn;
use hyper::{Body, Method, Request, Response, StatusCode};
use prometheus::{Encoder, TextEncoder};
use std::env;
use std::sync::Arc;
use std::str::FromStr;

use slog::*;
use libunftp::auth::AnonymousUser;

const APP_NAME: &str = "unFTP";
const APP_VERSION: &str = env!("BUILD_VERSION");

const ENV_UNFTP_ADDRESS: Arg = Arg::WithDefault("UNFTP_ADDRESS", "0.0.0.0:2121");
const ENV_UNFTP_HOME: Arg = Arg::NoDefault("UNFTP_HOME");

const ENV_CERTS_FILE: Arg = Arg::NoDefault("CERTS_FILE");
const ENV_KEY_FILE: Arg = Arg::NoDefault("KEY_FILE");

const ENV_METRICS_ADDRESS: Arg = Arg::NoDefault("METRICS_ADDRESS");

const ENV_LOG_REDIS_KEY: Arg = Arg::NoDefault("LOG_REDIS_KEY");
const ENV_LOG_REDIS_HOST: Arg = Arg::NoDefault("LOG_REDIS_HOST");
const ENV_LOG_REDIS_PORT: Arg = Arg::NoDefault("LOG_REDIS_PORT");

const ENV_AUTH_REST_URL: Arg = Arg::NoDefault("AUTH_REST_URL");
const ENV_AUTH_REST_METHOD: Arg = Arg::WithDefault("AUTH_REST_METHOD", "GET");
const ENV_AUTH_REST_BODY: Arg = Arg::NoDefault("AUTH_REST_BODY");
const ENV_AUTH_REST_SELECTOR: Arg = Arg::NoDefault("AUTH_REST_SELECTOR");
const ENV_AUTH_REST_REGEX: Arg = Arg::NoDefault("AUTH_REST_REGEX");

fn redis_logger() -> Option<redislog::Logger> {
    if ENV_LOG_REDIS_HOST.provided() && ENV_LOG_REDIS_PORT.provided() && ENV_LOG_REDIS_KEY.provided() {
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

// FIXME: add user support
fn make_auth() -> Arc<dyn auth::Authenticator<AnonymousUser> + Send + Sync> {
    if ENV_AUTH_REST_URL.provided() {
        if !ENV_AUTH_REST_REGEX.provided() || !ENV_AUTH_REST_SELECTOR.provided() {
            panic!("rest url was provided but selector and regex not")
        }

        if ENV_AUTH_REST_METHOD.val() == "GET" && !ENV_AUTH_REST_BODY.provided() {
            panic!("no body provided for rest request")
        }

        log::info!("Using REST authenticator ({})", ENV_AUTH_REST_URL.val());

        let authenticator: auth::rest::RestAuthenticator = auth::rest::Builder::new()
            .with_username_placeholder("{USER}".to_string())
            .with_password_placeholder("{PASS}".to_string())
            .with_url(ENV_AUTH_REST_URL.val())
            .with_method(Method::from_str(ENV_AUTH_REST_METHOD.val().as_str()).unwrap())
            .with_body(ENV_AUTH_REST_BODY.val())
            .with_selector(ENV_AUTH_REST_SELECTOR.val())
            .with_regex(ENV_AUTH_REST_REGEX.val())
            .build();

        return Arc::new(authenticator);
    }

    log::info!("Using anonymous authenticator");
    Arc::new(auth::AnonymousAuthenticator {})
}

fn metrics_service(req: Request<Body>) -> Box<dyn Future<Item=Response<Body>, Error=hyper::Error> + Send> {
    let mut response = Response::new(Body::empty());
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/metrics") => {
            *response.body_mut() = Body::from(gather_metrics());
        }
        _ => {
            *response.status_mut() = StatusCode::NOT_FOUND;
        }
    }

    Box::new(future::ok(response))
}

fn gather_metrics() -> Vec<u8> {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = vec![];
    encoder.encode(&metric_families, &mut buffer).unwrap();
    buffer
}

fn main() {
    let mut rt = Runtime::new().unwrap();

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
    if ENV_METRICS_ADDRESS.provided() {
        let http_addr = ENV_METRICS_ADDRESS.val().parse()
            .expect(format!("Unable to parse metrics address {}", ENV_METRICS_ADDRESS.val()).as_str());

        let http_log = log.clone();

        let http_server = hyper::Server::bind(&http_addr)
            .serve(|| service_fn(metrics_service))
            .map_err(move |e| error!(http_log, "HTTP Server error: {}", e));

        info!(log, "Starting Prometheus {} exporter.", APP_NAME; "address" => &http_addr);
        let _http_thread = rt.spawn(http_server);
    }

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

    info!(log, "Starting {} server.", APP_NAME; "version" => APP_VERSION, "address" => &addr, "home" => home_dir.clone());

    let server = Server::with_root(home_dir)
        .greeting("Welcome to unFTP")
        .authenticator(make_auth())
        .with_metrics();

    let _ftp_thread = rt.spawn(server.listener(&addr));

    rt.shutdown_on_idle().wait().unwrap();
}
