#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate clap;

#[allow(dead_code)]
mod app;
mod args;
mod redislog;

use std::env;
use std::path::PathBuf;
use std::process;
use std::result::Result;
use std::str::FromStr;
use std::sync::Arc;

use clap::ArgMatches;
use futures01::future::Future;
use hyper::{
    service::{make_service_fn, service_fn},
    Body, Method, Request, Response, StatusCode,
};
use libunftp::{
    auth::{self, AnonymousUser},
    storage::StorageBackend,
    Server,
};
use prometheus::{Encoder, TextEncoder};
use slog::*;
use tokio_compat::runtime::Runtime as TokioRuntime;

#[cfg(feature = "pam")]
use libunftp::auth::pam;

fn redis_logger(m: &clap::ArgMatches) -> Result<Option<redislog::Logger>, String> {
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
                .map_err(|e| format!("could not initialize Redis logger: {}", e))?;
            Ok(Some(logger))
        }
        (None, None, None) => Ok(None),
        _ => Err("for the redis logger please specify all --log-redis-* options".to_string()),
    }
}

fn make_auth(m: &clap::ArgMatches) -> Result<Arc<dyn auth::Authenticator<AnonymousUser> + Send + Sync>, String> {
    match m.value_of(args::AUTH_TYPE) {
        None | Some("anonymous") => Ok(make_anon_auth()),
        Some("pam") => make_pam_auth(m),
        Some("rest") => make_rest_auth(m),
        Some("json") => make_json_auth(m),
        unkown_type => Err(format!("unknown auth type: {}", unkown_type.unwrap())),
    }
}

fn make_anon_auth() -> Arc<dyn auth::Authenticator<AnonymousUser> + Send + Sync> {
    log::info!("Using anonymous authenticator");
    Arc::new(auth::AnonymousAuthenticator {})
}

fn make_pam_auth(m: &clap::ArgMatches) -> Result<Arc<dyn auth::Authenticator<AnonymousUser> + Send + Sync>, String> {
    #[cfg(not(feature = "pam"))]
    {
        let _ = m;
        Err(format!("the pam authentication module was disabled at build time"))
    }

    #[cfg(feature = "pam")]
    {
        if let Some(service) = m.value_of(args::AUTH_PAM_SERVICE) {
            log::info!("Using pam authenticator");
            return Ok(Arc::new(pam::PAMAuthenticator::new(service)));
        }
        Err(format!("--{} is required when using pam auth", args::AUTH_PAM_SERVICE))
    }
}

// FIXME: add user support
fn make_rest_auth(m: &clap::ArgMatches) -> Result<Arc<dyn auth::Authenticator<AnonymousUser> + Send + Sync>, String> {
    #[cfg(not(feature = "rest"))]
    {
        let _ = m;
        Err(format!("the rest authentication module was disabled at build time"))
    }

    #[cfg(feature = "rest")]
    {
        match (
            m.value_of(args::AUTH_REST_URL),
            m.value_of(args::AUTH_REST_REGEX),
            m.value_of(args::AUTH_REST_SELECTOR),
            m.value_of(args::AUTH_REST_METHOD),
        ) {
            (Some(url), Some(regex), Some(selector), Some(method)) => {
                if method.to_uppercase() != "GET" && m.value_of(args::AUTH_REST_BODY).is_none() {
                    return Err("REST authenticator error: no body provided for rest request".to_string());
                }

                log::info!("Using REST authenticator ({})", url);

                let authenticator: auth::rest::RestAuthenticator = auth::rest::Builder::new()
                    .with_username_placeholder("{USER}".to_string())
                    .with_password_placeholder("{PASS}".to_string())
                    .with_url(String::from(url))
                    .with_method(hyper12::Method::from_str(method).map_err(|e| format!("error creating REST auth: {}", e))?)
                    .with_body(String::from(m.value_of(args::AUTH_REST_BODY).unwrap_or("")))
                    .with_selector(String::from(selector))
                    .with_regex(String::from(regex))
                    .build();

                Ok(Arc::new(authenticator))
            }
            _ => Err("for auth type rest please specify all auth-rest-* options".to_string()),
        }
    }
}

fn make_json_auth(m: &clap::ArgMatches) -> Result<Arc<dyn auth::Authenticator<AnonymousUser> + Send + Sync>, String> {
    #[cfg(not(feature = "jsonfile_auth"))]
    {
        let _ = m;
        Err(format!("the jsonfile authentication module was disabled at build time"))
    }

    #[cfg(feature = "jsonfile_auth")]
    {
        let path = m
            .value_of(args::AUTH_JSON_PATH)
            .ok_or_else(|| "please provide the json credentials file by specifying auth-json-path".to_string())?;

        let authenticator = auth::jsonfile_auth::JsonFileAuthenticator::new(path).map_err(|e| e.to_string())?;
        Ok(Arc::new(authenticator))
    }
}

async fn metrics_service(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let mut response = Response::new(Body::empty());
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/metrics") => {
            *response.body_mut() = Body::from(gather_metrics());
        }
        _ => {
            *response.status_mut() = StatusCode::NOT_FOUND;
        }
    }

    Ok(response)
}

fn gather_metrics() -> Vec<u8> {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = vec![];
    encoder.encode(&metric_families, &mut buffer).unwrap();
    buffer
}

// Creates the filesystem storage back-end
fn fs_storage_backend(
    m: &clap::ArgMatches,
) -> Box<dyn (Fn() -> libunftp::storage::filesystem::Filesystem) + Send + Sync> {
    let p: PathBuf = m.value_of(args::ROOT_DIR).unwrap().into();
    Box::new(move || libunftp::storage::filesystem::Filesystem::new(p.clone()))
}

// Creates the GCS storage back-end
fn gcs_storage_backend(
    m: &clap::ArgMatches,
) -> Result<Box<dyn (Fn() -> libunftp::storage::cloud_storage::CloudStorage) + Send + Sync>, String> {
    let b: String = m
        .value_of(args::GCS_BUCKET)
        .ok_or_else(|| format!("--{} is required when using storage type gcs", args::GCS_BUCKET))?
        .into();
    let p: PathBuf = m
        .value_of(args::GCS_KEY_FILE)
        .ok_or_else(|| format!("--{} is required when using storage type gcs", args::GCS_KEY_FILE))?
        .into();
    Ok(Box::new(move || {
        libunftp::storage::cloud_storage::CloudStorage::new(
            b.clone(),
            yup_oauth2::service_account_key_from_file(p.clone())
                .map_err(|e| format!("could not load GCS back-end key file: {}", e))
                .unwrap(),
        )
    }))
}

// starts the FTP server as a Tokio task.
fn start_ftp(log: &Logger, m: &clap::ArgMatches, runtime: &mut TokioRuntime) -> Result<(), String> {
    match m.value_of(args::STORAGE_BACKEND_TYPE) {
        None | Some("filesystem") => start_ftp_with_storage(&log, m, fs_storage_backend(m), runtime),
        Some("gcs") => start_ftp_with_storage(&log, m, gcs_storage_backend(m)?, runtime),
        Some(x) => Err(format!("unknown storage back-end type {}", x)),
    }
}

// Given a storage back-end, starts the FTP server as a Tokio task.
fn start_ftp_with_storage<S>(
    log: &Logger,
    arg_matches: &ArgMatches,
    storage_backend: Box<dyn (Fn() -> S) + Send + Sync>,
    runtime: &mut TokioRuntime,
) -> Result<(), String>
where
    S: StorageBackend<AnonymousUser> + Send + Sync + 'static,
    S::File: libunftp::storage::AsAsyncReads + Send + Sync,
    S::Metadata: Sync + Send,
{
    let addr = String::from(arg_matches.value_of(args::BIND_ADDRESS).unwrap());

    let ports: std::vec::Vec<&str> = arg_matches
        .value_of(args::PASSIVE_PORTS)
        .unwrap()
        .split(|c: char| !c.is_numeric())
        .filter(|s| !s.is_empty())
        .collect();

    if ports.len() != 2 {
        return Err(format!(
            "please specify a valid port range e.g. 50000-60000 for --{}",
            args::PASSIVE_PORTS
        ));
    }
    let start_port: u16 = ports[0]
        .parse()
        .map_err(|_| "start of port range needs to be numeric")?;
    let end_port: u16 = ports[1].parse().map_err(|_| "end of port range needs to be numeric")?;

    info!(log, "Using passive port range {}..{}", start_port, end_port);

    let idle_timeout_str = arg_matches.value_of(args::IDLE_SESSION_TIMEOUT).unwrap();
    let idle_timeout = String::from(idle_timeout_str).parse::<u64>().map_err(move |e| {
        format!(
            "unable to parse given value '{}' for --{}: {}. Please use a numeric value",
            idle_timeout_str,
            args::IDLE_SESSION_TIMEOUT,
            e
        )
    })?;

    info!(log, "Idle session timeout is set to {} seconds", idle_timeout);

    let mut server = Server::new(storage_backend)
        .greeting("Welcome to unFTP")
        .authenticator(make_auth(&arg_matches)?)
        .passive_ports(start_port..end_port)
        .idle_session_timeout(idle_timeout)
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

    runtime.spawn_std(server.listener(addr));
    Ok(())
}

// starts an HTTP server and exports Prometheus metrics.
async fn start_http(log: &Logger, bind_addr: &str) -> Result<(), String> {
    let http_addr = bind_addr
        .parse()
        .map_err(|e| format!("unable to parse HTTP address {}: {}", bind_addr, e))?;

    let make_svc = make_service_fn(|_conn| {
        async {
            // service_fn converts our function into a `Service`
            Ok::<_, hyper::Error>(service_fn(metrics_service))
        }
    });

    let http_server = hyper::Server::bind(&http_addr).serve(make_svc);

    info!(log, "Starting Prometheus {} exporter.", app::NAME; "address" => &http_addr);

    if let Err(e) = http_server.await {
        error!(log, "HTTP Server error: {}", e)
    }
    Ok(())
}

fn run(arg_matches: ArgMatches) -> Result<(), String> {
    // Logging
    let min_log_level = match arg_matches.occurrences_of(args::VERBOSITY) {
        0 => (slog::Level::Info, log::Level::Info),
        1 => (slog::Level::Debug, log::Level::Debug),
        2 | _ => (slog::Level::Trace, log::Level::Trace),
    };
    let drain = match redis_logger(&arg_matches)? {
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
    "libunftp-version" => app::libunftp_version(),
    "address" => &addr,
    "home" => home_dir,
    "auth-type" => auth_type,
    "sbe-type" => sbe_type,
    );

    let mut runtime = TokioRuntime::new().unwrap();

    if let Some(addr) = arg_matches.value_of(args::HTTP_BIND_ADDR) {
        let addr = String::from(addr);
        let log = log.clone();
        runtime.spawn_std(async move {
            if let Err(e) = start_http(&log, &*addr).await {
                error!(log, "HTTP Server error: {}", e)
            }
        });
    }

    start_ftp(&log, &arg_matches, &mut runtime)?;
    runtime.shutdown_on_idle().wait().unwrap();

    Ok(())
}

fn main() {
    let tmp_dir = env::temp_dir();
    let tmp_dir = tmp_dir.as_path().to_str().unwrap();
    let arg_matches = args::clap_app(tmp_dir).get_matches();
    if let Err(e) = run(arg_matches) {
        println!("\nError: {}", e);
        process::exit(1);
    };
}
