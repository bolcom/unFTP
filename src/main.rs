mod config;
mod redislog;

use std::env;
use std::str::FromStr;
use std::sync::Arc;

use clap::App;
use futures::future;
use hyper::rt::Future;
use hyper::service::service_fn;
use hyper::{Body, Method, Request, Response, StatusCode};
use libunftp::auth;
use libunftp::auth::AnonymousUser;
use libunftp::Server;
use prometheus::{Encoder, TextEncoder};
use slog::*;
use tokio::runtime::Runtime;

const APP_NAME: &str = "unFTP";
const APP_VERSION: &str = env!("BUILD_VERSION");

fn clap_app<'a>(tmp_dir: &'a str) -> clap::App<'a, 'a> {
    App::new(APP_NAME)
        .version(APP_VERSION)
        .about("When you need to FTP but don't want to")
        .author("The bol.com unFTP team")
        .arg(
            clap::Arg::with_name("bind-address")
                .long("bind-address")
                .value_name("HOST_PORT")
                .help("Sets the host and port to listen on for FTP control connections")
                .default_value("0.0.0.0:2121")
                .env("UNFTP_ADDRESS")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("fs-home-dir")
                .long("fs-home-dir")
                .value_name("HOME_DIR")
                .help("Sets the home directory for the filesystem back-end")
                .default_value(tmp_dir)
                .env("UNFTP_HOME")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("ftps-certs-file")
                .long("ftps-certs-file")
                .value_name("PEM_FILE")
                .help("Sets the path the the certificates used for TLS security")
                .env("UNFTP_CERTS_FILE")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("ftps-key-file")
                .long("ftps-key-file")
                .value_name("PEM_FILE")
                .help("Sets the path to the private key file used for TLS security")
                .env("UNFTP_CERTS_FILE")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("log-redis-key")
                .long("log-redis-key")
                .value_name("KEY")
                .help("Sets the key name for storage in Redis")
                .env("UNFTP_LOG_REDIS_KEY")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("log-redis-host")
                .long("log-redis-host")
                .value_name("KEY")
                .help("Sets the hostname for the Redis server where logging should go")
                .env("UNFTP_LOG_REDIS_HOST")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("log-redis-port")
                .long("log-redis-port")
                .value_name("PORT")
                .help("Sets the port for the Redis server where logging should go")
                .env("UNFTP_LOG_REDIS_PORT")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("metrics-bind-address")
                .long("metrics-bind-address")
                .value_name("HOST_PORT")
                .help("Sets the host and port for the HTTP server used by prometheus metrics collection")
                .env("UNFTP_METRICS_ADDRESS")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("auth-rest-url")
                .long("auth-rest-url")
                .value_name("URL")
                .help("-")
                .env("UNFTP_AUTH_REST_URL")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("auth-rest-method")
                .long("auth-rest-method")
                .value_name("URL")
                .help("-")
                .env("UNFTP_AUTH_REST_METHOD")
                .default_value("GET")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("auth-rest-body")
                .long("auth-rest-body")
                .value_name("URL")
                .help("-")
                .env("UNFTP_AUTH_REST_BODY")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("auth-rest-selector")
                .long("auth-rest-selector")
                .value_name("SELECTOR")
                .help("-")
                .env("UNFTP_AUTH_REST_SELECTOR")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("auth-rest-regex")
                .long("auth-rest-regex")
                .value_name("REGEX")
                .help("-")
                .env("UNFTP_AUTH_REST_REGEX")
                .takes_value(true),
        )
}

fn redis_logger(m: &clap::ArgMatches) -> Option<redislog::Logger> {
    match (
        m.value_of("log-redis-key"),
        m.value_of("log-redis-host"),
        m.value_of("log-redis-port"),
    ) {
        (Some(key), Some(host), Some(port)) => {
            let logger = redislog::Builder::new(APP_NAME)
                .redis(
                    String::from(host),
                    String::from(port).parse::<u32>().unwrap(),
                    String::from(key),
                )
                .build()
                .expect("could not initialize Redis logger");
            Some(logger)
        }
        (None, None, None) => None,
        _ => {
            // TODO: Warn user
            None
        }
    }
}

// FIXME: add user support
fn make_auth(m: &clap::ArgMatches) -> Arc<dyn auth::Authenticator<AnonymousUser> + Send + Sync> {
    match (
        m.value_of("auth-rest-url"),
        m.value_of("auth-rest-regex"),
        m.value_of("auth-rest-selector"),
        m.value_of("auth-rest-method"),
    ) {
        (Some(url), Some(regex), Some(selector), Some(method)) => {
            if method.to_uppercase() != "GET" && m.value_of("auth-rest-body").is_none() {
                panic!("no body provided for rest request")
            }

            log::info!("Using REST authenticator ({})", url);

            let authenticator: auth::rest::RestAuthenticator = auth::rest::Builder::new()
                .with_username_placeholder("{USER}".to_string())
                .with_password_placeholder("{PASS}".to_string())
                .with_url(String::from(url))
                .with_method(Method::from_str(method).unwrap())
                .with_body(String::from(m.value_of("auth-rest-body").unwrap()))
                .with_selector(String::from(selector))
                .with_regex(String::from(regex))
                .build();

            Arc::new(authenticator)
        }
        (Some(_url), _, _, _) => panic!("rest url was provided but selector and regex not"),
        _ => {
            log::info!("Using anonymous authenticator");
            Arc::new(auth::AnonymousAuthenticator {})
        }
    }
}

fn metrics_service(req: Request<Body>) -> Box<dyn Future<Item = Response<Body>, Error = hyper::Error> + Send> {
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
    let tmp_dir = env::temp_dir();

    let arg_matches = clap_app(tmp_dir.as_path().to_str().unwrap()).get_matches();

    let drain = match redis_logger(&arg_matches) {
        Some(l) => slog_async::Async::new(l.fuse()).build().fuse(),
        None => {
            let decorator = slog_term::PlainDecorator::new(std::io::stdout());
            let drain = slog_term::CompactFormat::new(decorator).build().fuse();
            slog_async::Async::new(drain).build().fuse()
        }
    };

    let root = Logger::root(drain, o!());
    let log = root.new(o!("module" => "main"));

    let _scope_guard = slog_scope::set_global_logger(root);
    let _log_guard = slog_stdlog::init_with_level(log::Level::Debug).unwrap();

    let addr = String::from(arg_matches.value_of("bind-address").unwrap());
    let home_dir = String::from(arg_matches.value_of("fs-home-dir").unwrap());

    info!(log, "Starting {} server.", APP_NAME; "version" => APP_VERSION, "address" => &addr, "home" => home_dir.clone());

    // HTTP server for exporting Prometheus metrics
    if let Some(addr) = arg_matches.value_of("metrics-bind-address") {
        let http_addr = addr
            .parse()
            .expect(format!("Unable to parse metrics address {}", addr).as_str());

        let http_log = log.clone();

        let http_server = hyper::Server::bind(&http_addr)
            .serve(|| service_fn(metrics_service))
            .map_err(move |e| error!(http_log, "HTTP Server error: {}", e));

        info!(log, "Starting Prometheus {} exporter.", APP_NAME; "address" => &http_addr);
        let _http_thread = rt.spawn(http_server);
    }

    let server = Server::with_root(home_dir)
        .greeting("Welcome to unFTP")
        .authenticator(make_auth(&arg_matches))
        .with_metrics();

    // Setup FTPS
    match (
        arg_matches.value_of("ftps-certs-file"),
        arg_matches.value_of("ftps-key-file"),
    ) {
        (Some(_certs_file), Some(_key_file)) => {
            // TODO: Re-enable when libunftp API was changed to not take static strings.
            //server.certs(certs_file, key_file)
        }
        (Some(_), None) | (None, Some(_)) => {
            warn!(
                log,
                "Need to set both {} and {}. FTPS still disabled.", "ftps-certs-file", "ftps-key-file"
            );
        }
        _ => {}
    };

    let _ftp_thread = rt.spawn(server.listener(&addr));

    rt.shutdown_on_idle().wait().unwrap();
}
