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
use libunftp::auth::{self, pam, AnonymousUser};
use libunftp::Server;
use prometheus::{Encoder, TextEncoder};
use slog::*;
use std::process;
use tokio::runtime::Runtime as TokioRuntime;

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
    if let Some(service) = m.value_of(args::AUTH_PAM_SERVICE) {
        log::info!("Using pam authenticator");
        return Arc::new(pam::PAMAuthenticator::new(service));
    }
    panic!("argument 'auth-pam-service' is required");
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

// TODO: Implement
fn _storage_backend<S>(m: &clap::ArgMatches) -> Box<dyn (Fn() -> S) + Send> {
    match m.value_of(args::STORAGE_BACKEND_TYPE) {
        None | Some("filesystem") => {
            // let p = m.value_of("home dir");
            // Box::new(move || {
            //     let p = &p.clone();
            //     libunftp::storage::filesystem::Filesystem::new(p)
            // })
            unimplemented!()
        }
        Some("gcs") => {
            if let Some(_bucket) = m.value_of(args::GCS_BUCKET) {
                // Box::new(move || {
                //     libunftp::storage::cloud_storage::CloudStorage::new(
                //         "bolcom-dev-unftp-dev-738-unftp-dev",
                //         yup_oauth2::service_account_key_from_file(&"/Users/dkosztka/Downloads/bolcom-dev-unftp-dev-738-1379d4070948.json".to_string()).expect("borked"),
                //     )
                // })
                unimplemented!()
            }
            panic!("sbe-gcs-bucket needs to be specified")
        }
        Some(x) => panic!("unknown storage back-end type {}", x),
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

    // HTTP server for exporting Prometheus metrics
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

    let mut server = Server::with_root(home_dir)
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
                "Need to set both {} and {}. FTPS still disabled.", "ftps-certs-file", "ftps-key-file"
            );
            server
        }
        _ => server,
    };

    let _ftp_thread = runtime.spawn(server.listener(&addr));
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
