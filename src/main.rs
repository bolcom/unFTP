mod app;
mod args;
mod redislog;

#[macro_use]
extern crate clap;

use std::env;
use std::str::FromStr;
use std::sync::Arc;

use clap::ArgMatches;
use futures::future;
use hyper::rt::Future;
use hyper::service::service_fn;
use hyper::{Body, Method, Request, Response, StatusCode};
use libunftp::auth::{self, AnonymousUser};
use libunftp::storage::StorageBackend;
use libunftp::Server;
use prometheus::{Encoder, TextEncoder};
use slog::*;
use std::path::PathBuf;
use std::process;
use tokio::runtime::Runtime as TokioRuntime;

#[cfg(feature = "pam")]
use libunftp::auth::pam;

fn redis_logger(m: &clap::ArgMatches) -> Option<redislog::Logger> {
    match (
        m.value_of(args::REDIS_KEY),
        m.value_of(args::REDIS_HOST),
        m.value_of(args::REDIS_PORT),
    ) {
        (Some(key), Some(host), Some(port)) => {
            let logger = redislog::Builder::new(app::NAME)
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

fn make_auth(m: &clap::ArgMatches) -> Arc<dyn auth::Authenticator<AnonymousUser> + Send + Sync> {
    match m.value_of(args::AUTH_TYPE) {
        None | Some("anonymous") => make_anon_auth(),
        Some("pam") => make_pam_auth(m),
        Some("rest") => make_rest_auth(m),
        _ => panic!("unknown auth type"),
    }
}

fn make_anon_auth() -> Arc<dyn auth::Authenticator<AnonymousUser> + Send + Sync> {
    log::info!("Using anonymous authenticator");
    Arc::new(auth::AnonymousAuthenticator {})
}

fn make_pam_auth(m: &clap::ArgMatches) -> Arc<dyn auth::Authenticator<AnonymousUser> + Send + Sync> {
    #[cfg(not(feature = "pam"))]
    {
        let _ = m;
        panic!("pam auth was disabled at build time");
    }

    #[cfg(feature = "pam")]
    {
        if let Some(service) = m.value_of(args::AUTH_PAM_SERVICE) {
            log::info!("Using pam authenticator");
            return Arc::new(pam::PAMAuthenticator::new(service));
        }
        panic!("argument 'auth-pam-service' is required");
    }
}

// FIXME: add user support
fn make_rest_auth(m: &clap::ArgMatches) -> Arc<dyn auth::Authenticator<AnonymousUser> + Send + Sync> {
    match (
        m.value_of(args::AUTH_REST_URL),
        m.value_of(args::AUTH_REST_REGEX),
        m.value_of(args::AUTH_REST_SELECTOR),
        m.value_of(args::AUTH_REST_METHOD),
    ) {
        (Some(url), Some(regex), Some(selector), Some(method)) => {
            if method.to_uppercase() != "GET" && m.value_of(args::AUTH_REST_BODY).is_none() {
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
        _ => {
            panic!("for auth type rest please specify all auth-rest-* options");
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

// Creates the filesystem storage back-end
fn fs_storage_backend(m: &clap::ArgMatches) -> Box<dyn (Fn() -> libunftp::storage::filesystem::Filesystem) + Send> {
    let p: PathBuf = m.value_of(args::ROOT_DIR).unwrap().into();
    Box::new(move || libunftp::storage::filesystem::Filesystem::new(p.clone()))
}

// Creates the GCS storage back-end
fn gcs_storage_backend(
    m: &clap::ArgMatches,
) -> Box<dyn (Fn() -> libunftp::storage::cloud_storage::CloudStorage) + Send> {
    let b: String = m.value_of(args::GCS_BUCKET).unwrap().into();
    let p: PathBuf = m.value_of(args::GCS_KEY_FILE).unwrap().into();
    Box::new(move || {
        libunftp::storage::cloud_storage::CloudStorage::new(
            b.clone(),
            yup_oauth2::service_account_key_from_file(p.clone()).expect("oops"),
        )
    })
}

// starts the FTP server as a Tokio task.
fn start_ftp(log: &Logger, m: &clap::ArgMatches, runtime: &mut TokioRuntime) {
    match m.value_of(args::STORAGE_BACKEND_TYPE) {
        None | Some("filesystem") => start_ftp_with_storage(&log, m, fs_storage_backend(m), runtime),
        Some("gcs") => {
            if let Some(_bucket) = m.value_of(args::GCS_BUCKET) {
                start_ftp_with_storage(&log, m, gcs_storage_backend(m), runtime)
            } else {
                panic!("sbe-gcs-bucket needs to be specified")
            }
        }
        Some(x) => panic!("unknown storage back-end type {}", x),
    }
}

// Given a storage back-end, starts the FTP server as a Tokio task.
fn start_ftp_with_storage<S>(
    log: &Logger,
    arg_matches: &ArgMatches,
    storage_backend: Box<dyn (Fn() -> S) + Send>,
    runtime: &mut TokioRuntime,
) where
    S: StorageBackend<AnonymousUser> + Send + Sync + 'static,
    S::File: tokio::io::AsyncRead + Send,
    S::Metadata: Sync + Send,
{
    let addr = String::from(arg_matches.value_of(args::BIND_ADDRESS).unwrap());

    let mut server = Server::new(storage_backend)
        .greeting("Welcome to unFTP")
        .authenticator(make_auth(&arg_matches))
        .passive_ports(49152..65535)
        .with_metrics();

    // Setup FTPS
    server = match (
        arg_matches.value_of(args::FTPS_CERTS_FILE),
        arg_matches.value_of(args::FTPS_KEY_FILE),
    ) {
        (Some(certs_file), Some(key_file)) => {
            info!(log, "FTPS enabled");
            server.certs(certs_file, key_file)
        }
        (Some(_), None) | (None, Some(_)) => {
            warn!(
                log,
                "Need to set both {} and {}. FTPS still disabled.",
                args::FTPS_CERTS_FILE,
                args::FTPS_KEY_FILE
            );
            server
        }
        _ => {
            info!(log, "FTPS not enabled");
            server
        }
    };

    runtime.spawn(server.listener(&addr));
}

// starts an HTTP server and exports Prometheus metrics.
fn start_http(log: &Logger, arg_matches: &ArgMatches, runtime: &mut TokioRuntime) {
    if let Some(addr) = arg_matches.value_of(args::HTTP_BIND_ADDR) {
        let http_addr = addr
            .parse()
            .unwrap_or_else(|_| panic!("Unable to parse metrics address {}", addr));

        let http_log = log.clone();

        let http_server = hyper::Server::bind(&http_addr)
            .serve(|| service_fn(metrics_service))
            .map_err(move |e| error!(http_log, "HTTP Server error: {}", e));

        info!(log, "Starting Prometheus {} exporter.", app::NAME; "address" => &http_addr);
        let _http_thread = runtime.spawn(http_server);
    }
}

fn run(arg_matches: ArgMatches) -> std::result::Result<(), String> {
    // Logging
    let min_log_level = match arg_matches.occurrences_of(args::VERBOSITY) {
        0 => (slog::Level::Info, log::Level::Info),
        1 => (slog::Level::Debug, log::Level::Debug),
        2 | _ => (slog::Level::Trace, log::Level::Trace),
    };
    let drain = match redis_logger(&arg_matches) {
        Some(l) => slog_async::Async::new(l.filter_level(min_log_level.0).fuse())
            .build()
            .fuse(),
        None => {
            let decorator = slog_term::PlainDecorator::new(std::io::stdout());
            let drain = slog_term::CompactFormat::new(decorator)
                .build()
                .filter_level(min_log_level.0)
                .fuse();
            slog_async::Async::new(drain).build().fuse()
        }
    };
    let root = Logger::root(drain, o!());
    let log = root.new(o!("module" => "main"));
    let _scope_guard = slog_scope::set_global_logger(root);
    slog_stdlog::init_with_level(min_log_level.1).unwrap();

    let addr = String::from(arg_matches.value_of(args::BIND_ADDRESS).unwrap());
    let home_dir = String::from(arg_matches.value_of(args::ROOT_DIR).unwrap());
    let auth_type = String::from(arg_matches.value_of(args::AUTH_TYPE).unwrap());
    let sbe_type = String::from(arg_matches.value_of(args::STORAGE_BACKEND_TYPE).unwrap());

    info!(log, "Starting {} server.", app::NAME;
    "version" => app::VERSION,
    "address" => &addr,
    "home" => home_dir.clone(),
    "auth-type" => auth_type,
    "sbe-type" => sbe_type
    );

    let mut runtime = TokioRuntime::new().unwrap();

    start_http(&log, &arg_matches, &mut runtime);
    start_ftp(&log, &arg_matches, &mut runtime);
    runtime.shutdown_on_idle().wait().unwrap();

    Ok(())
}

fn main() {
    let tmp_dir = env::temp_dir();
    let tmp_dir = tmp_dir.as_path().to_str().unwrap();
    let arg_matches = args::clap_app(tmp_dir).get_matches();
    if let Err(e) = run(arg_matches) {
        println!("Error: {}", e);
        process::exit(1);
    };
}
