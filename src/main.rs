#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate clap;

#[allow(dead_code)]
mod app;
mod args;
mod http;
mod logging;
mod metrics;
mod storage;
mod user;

use clap::ArgMatches;
use libunftp::options::FtpsRequired;
use libunftp::{auth, options, storage::StorageBackend, Server};
use slog::*;
use std::{
    env,
    net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs},
    path::PathBuf,
    process,
    result::Result,
    str::FromStr,
    sync::Arc,
};
use tokio::{
    runtime::Runtime,
    signal::unix::{signal, SignalKind},
};
use user::LookupAuthenticator;

#[cfg(feature = "pam_auth")]
use libunftp::auth::pam;
use libunftp::storage::cloud_storage::options::AuthMethod;

fn make_auth(m: &clap::ArgMatches) -> Result<Arc<dyn auth::Authenticator<user::User> + Send + Sync>, String> {
    match m.value_of(args::AUTH_TYPE) {
        None | Some("anonymous") => Ok(make_anon_auth()),
        Some("pam") => make_pam_auth(m),
        Some("rest") => make_rest_auth(m),
        Some("json") => make_json_auth(m),
        unknown_type => Err(format!("unknown auth type: {}", unknown_type.unwrap())),
    }
}

fn make_anon_auth() -> Arc<dyn auth::Authenticator<user::User> + Send + Sync> {
    Arc::new(LookupAuthenticator::new(auth::AnonymousAuthenticator))
}

fn make_pam_auth(m: &clap::ArgMatches) -> Result<Arc<dyn auth::Authenticator<user::User> + Send + Sync>, String> {
    #[cfg(not(feature = "pam_auth"))]
    {
        let _ = m;
        Err(String::from("the pam authentication module was disabled at build time"))
    }

    #[cfg(feature = "pam_auth")]
    {
        if let Some(service) = m.value_of(args::AUTH_PAM_SERVICE) {
            let pam_auth = pam::PAMAuthenticator::new(service);
            return Ok(Arc::new(LookupAuthenticator::new(pam_auth)));
        }
        Err(format!("--{} is required when using pam auth", args::AUTH_PAM_SERVICE))
    }
}

// FIXME: add user support
fn make_rest_auth(m: &clap::ArgMatches) -> Result<Arc<dyn auth::Authenticator<user::User> + Send + Sync>, String> {
    #[cfg(not(feature = "rest_auth"))]
    {
        let _ = m;
        Err(format!("the rest authentication module was disabled at build time"))
    }

    #[cfg(feature = "rest_auth")]
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

                let authenticator: auth::rest::RestAuthenticator = match auth::rest::Builder::new()
                    .with_username_placeholder("{USER}".to_string())
                    .with_password_placeholder("{PASS}".to_string())
                    .with_url(String::from(url))
                    .with_method(
                        hyper::Method::from_str(method).map_err(|e| format!("error creating REST auth: {}", e))?,
                    )
                    .with_body(String::from(m.value_of(args::AUTH_REST_BODY).unwrap_or("")))
                    .with_selector(String::from(selector))
                    .with_regex(String::from(regex))
                    .build()
                {
                    Ok(res) => res,
                    Err(e) => return Err(format!("Unable to create RestAuthenticator: {}", e)),
                };

                Ok(Arc::new(LookupAuthenticator::new(authenticator)))
            }
            _ => Err("for auth type rest please specify all auth-rest-* options".to_string()),
        }
    }
}

fn make_json_auth(m: &clap::ArgMatches) -> Result<Arc<dyn auth::Authenticator<user::User> + Send + Sync>, String> {
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

        let authenticator = auth::jsonfile::JsonFileAuthenticator::new(path).map_err(|e| e.to_string())?;
        Ok(Arc::new(LookupAuthenticator::new(authenticator)))
    }
}

// Creates the filesystem storage back-end
fn fs_storage_backend(log: &Logger, m: &clap::ArgMatches) -> Box<dyn (Fn() -> storage::StorageBE) + Send + Sync> {
    let p: PathBuf = m.value_of(args::ROOT_DIR).unwrap().into();
    let sub_log = Arc::new(log.new(o!("module" => "storage")));
    Box::new(move || storage::StorageBE {
        inner: storage::InnerStorage::File(libunftp::storage::filesystem::Filesystem::new(p.clone())),
        log: sub_log.clone(),
    })
}

// Creates the GCS storage back-end
fn gcs_storage_backend(
    log: &Logger,
    m: &clap::ArgMatches,
) -> Result<Box<dyn (Fn() -> storage::StorageBE) + Send + Sync>, String> {
    let bucket: String = m
        .value_of(args::GCS_BUCKET)
        .ok_or_else(|| format!("--{} is required when using storage type gcs", args::GCS_BUCKET))?
        .into();
    let base_url: String = m
        .value_of(args::GCS_BASE_URL)
        .ok_or_else(|| format!("--{} is required when using storage type gcs", args::GCS_BUCKET))?
        .into();
    let root_dir: PathBuf = m
        .value_of(args::GCS_ROOT)
        .ok_or_else(|| format!("--{} is required when using storage type gcs", args::GCS_ROOT))?
        .into();
    let auth_method: AuthMethod = match (m.value_of(args::GCS_SERVICE_ACCOUNT), m.value_of(args::GCS_KEY_FILE)) {
        (None, None) => AuthMethod::WorkloadIdentity(None),
        (Some(_), Some(_)) => {
            return Err(format!(
                "Please specify either --{} or --{}, not both",
                args::GCS_SERVICE_ACCOUNT,
                args::GCS_KEY_FILE
            ))
        }
        (Some(sevice_account), None) => AuthMethod::WorkloadIdentity(Some(sevice_account.into())),
        (None, Some(key_file_path)) => {
            let key_file: PathBuf = key_file_path.into();
            let service_account_key = std::fs::read(key_file)
                .map_err(|e| format!("could not load GCS back-end service account key from file: {}", e))?;
            AuthMethod::ServiceAccountKey(service_account_key)
        }
    };

    slog::info!(log, "GCS back-end auth method: {}", auth_method);

    let sub_log = Arc::new(log.new(o!("module" => "storage")));
    Ok(Box::new(move || storage::StorageBE {
        inner: storage::InnerStorage::Cloud(libunftp::storage::cloud_storage::CloudStorage::new(
            base_url.clone(),
            bucket.clone(),
            root_dir.clone(),
            auth_method.clone(),
        )),
        log: sub_log.clone(),
    }))
}

// starts the FTP server as a Tokio task.
fn start_ftp(log: &Logger, root_log: &Logger, m: &clap::ArgMatches) -> Result<(), String> {
    match m.value_of(args::STORAGE_BACKEND_TYPE) {
        None | Some("filesystem") => start_ftp_with_storage(log, root_log, m, fs_storage_backend(root_log, m)),
        Some("gcs") => start_ftp_with_storage(log, root_log, m, gcs_storage_backend(root_log, m)?),
        Some(x) => Err(format!("unknown storage back-end type {}", x)),
    }
}

fn resolve_dns(log: &Logger, dns_name: &str) -> Result<Ipv4Addr, String> {
    slog::info!(log, "Resolving domain name '{}'", dns_name);
    // Normalize the address. If lookup_host (https://doc.rust-lang.org/1.6.0/std/net/fn.lookup_host.html)
    // is stable we won't need this.
    let host_port = dns_name.split(':').take(1).map(|s| format!("{}:21", s)).next().unwrap();
    let mut addrs_iter = host_port.to_socket_addrs().unwrap();
    loop {
        match addrs_iter.next() {
            None => break Err(format!("Could not resolve DNS address '{}'", dns_name)),
            Some(SocketAddr::V4(addr)) => {
                slog::info!(log, "Resolved '{}' to {}", dns_name, addr.ip());
                break Ok(*addr.ip());
            }
            Some(SocketAddr::V6(_)) => continue,
        }
    }
}

fn get_passive_host_option(log: &Logger, arg_matches: &ArgMatches) -> Result<options::PassiveHost, String> {
    let passive_host_str = arg_matches.value_of(args::PASSIVE_HOST);
    match passive_host_str {
        None | Some("from-connection") => Ok(options::PassiveHost::FromConnection),
        Some(ip_or_dns) => match ip_or_dns.parse() {
            Ok(IpAddr::V4(ip)) => Ok(options::PassiveHost::IP(ip)),
            Ok(IpAddr::V6(_)) => Err(format!(
                "an IP is valid for the '--{}' argument, but it needs to be an IP v4 address",
                args::PASSIVE_HOST
            )),
            Err(_) => resolve_dns(log, ip_or_dns).map(options::PassiveHost::IP),
        },
    }
}

// Given a storage back-end, starts the FTP server as a Tokio task.
fn start_ftp_with_storage<S>(
    log: &Logger,
    root_log: &Logger,
    arg_matches: &ArgMatches,
    storage_backend: Box<dyn (Fn() -> S) + Send + Sync>,
) -> Result<(), String>
where
    S: StorageBackend<user::User> + Send + Sync + 'static,
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

    let passive_host = get_passive_host_option(log, arg_matches)?;
    info!(log, "Using passive host option '{:?}'", passive_host);

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

    let mut server = Server::with_authenticator(storage_backend, make_auth(&arg_matches)?)
        .greeting("Welcome to unFTP")
        .passive_ports(start_port..end_port)
        .idle_session_timeout(idle_timeout)
        .logger(root_log.new(o!("lib" => "libunftp")))
        .passive_host(passive_host)
        .metrics();

    // Setup proxy protocol mode.
    if let Some(port) = arg_matches.value_of(args::PROXY_EXTERNAL_CONTROL_PORT) {
        let port_num = String::from(port)
            .parse::<u16>()
            .map_err(|e| format!("unable to parse proxy protocol external control port {}: {}", port, e))?;
        server = server.proxy_protocol_mode(port_num);
    }

    // Setup FTPS
    server = match (
        arg_matches.value_of(args::FTPS_CERTS_FILE),
        arg_matches.value_of(args::FTPS_KEY_FILE),
    ) {
        (Some(certs_file), Some(key_file)) => {
            info!(log, "FTPS enabled");
            let server = server.ftps(certs_file, key_file);
            let ftps_req_args: Result<Vec<FtpsRequired>, String> = [
                args::FTPS_REQUIRED_ON_CONTROL_CHANNEL,
                args::FTPS_REQUIRED_ON_DATA_CHANNEL,
            ]
            .iter()
            .map(|arg| -> Result<FtpsRequired, String> {
                let ftps_required = match arg_matches.value_of(arg) {
                    None => libunftp::options::FtpsRequired::None,
                    Some(str) => match str.parse::<args::FtpsRequiredType>()? {
                        args::FtpsRequiredType::all => libunftp::options::FtpsRequired::All,
                        args::FtpsRequiredType::accounts => libunftp::options::FtpsRequired::Accounts,
                        args::FtpsRequiredType::none => libunftp::options::FtpsRequired::None,
                    },
                };
                Ok(ftps_required)
            })
            .collect();
            let ftps_req_args = ftps_req_args?;
            let (ftps_required_control, ftps_required_data) = (ftps_req_args[0], ftps_req_args[1]);

            info!(log, "FTPS requirement for clients on control channel: {}", ftps_required_control; "mode" => format!("{:?}", ftps_required_control));
            info!(log, "FTPS requirement for clients on data channel: {}", ftps_required_data; "mode" => format!("{:?}", ftps_required_data));
            server.ftps_required(ftps_required_control, ftps_required_data)
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
            warn!(log, "FTPS not enabled");
            server
        }
    };

    tokio::spawn(server.listen(addr));
    Ok(())
}

struct ExitSignal(pub &'static str);

async fn listen_for_signals() -> Result<ExitSignal, String> {
    let mut term_sig =
        signal(SignalKind::terminate()).map_err(|e| format!("could not listen for TERM signals: {}", e))?;
    let mut int_sig = signal(SignalKind::interrupt()).map_err(|e| format!("Could not listen for INT signal: {}", e))?;

    let sig_name = tokio::select! {
        Some(_signal) = term_sig.recv() => {
            "SIG_TERM"
        },
        Some(_signal) = int_sig.recv() => {
            "SIG_INT"
        },
    };
    Ok(ExitSignal(sig_name))
}

async fn main_task(arg_matches: ArgMatches<'_>, log: &Logger, root_log: &Logger) -> Result<ExitSignal, String> {
    let ftp_addr: SocketAddr = arg_matches
        .value_of(args::BIND_ADDRESS)
        .unwrap()
        .parse()
        .map_err(|_| "could not parse FTP address")?;
    if let Some(addr) = arg_matches.value_of(args::HTTP_BIND_ADDRESS) {
        let addr = String::from(addr);
        let log = log.clone();
        tokio::spawn(async move {
            if let Err(e) = http::start(&log, &*addr, ftp_addr).await {
                error!(log, "HTTP Server error: {}", e)
            }
        });
    }

    start_ftp(&log, &root_log, &arg_matches)?;

    listen_for_signals().await
}

fn run(arg_matches: ArgMatches) -> Result<(), String> {
    let root_logger = logging::create_logger(&arg_matches)?;
    let log = root_logger.new(o!("module" => "main"));

    let addr = String::from(arg_matches.value_of(args::BIND_ADDRESS).unwrap());
    let http_addr = String::from(arg_matches.value_of(args::HTTP_BIND_ADDRESS).unwrap());
    let auth_type = String::from(arg_matches.value_of(args::AUTH_TYPE).unwrap());
    let sbe_type = String::from(arg_matches.value_of(args::STORAGE_BACKEND_TYPE).unwrap());

    let home_dir = String::from(match &*sbe_type {
        "gcs" => arg_matches.value_of(args::GCS_ROOT).unwrap(),
        _ => arg_matches.value_of(args::ROOT_DIR).unwrap(),
    });

    info!(log, "Starting {} server.", app::NAME;
    "version" => app::VERSION,
    "libunftp-version" => app::libunftp_version(),
    "ftp-address" => &addr,
    "http-address" => &http_addr,
    "home" => home_dir,
    "auth-type" => auth_type,
    "sbe-type" => sbe_type,
    );

    let runtime = Runtime::new().map_err(|e| format!("could not construct runtime: {}", e))?;
    let ExitSignal(signal) = runtime.block_on(main_task(arg_matches, &log, &root_logger))?;
    info!(log, "Received signal {}, shutting down...", signal);
    Ok(())
}

fn main() {
    let tmp_dir = env::temp_dir();
    let tmp_dir = tmp_dir.as_path().to_str().unwrap();
    let arg_matches = args::clap_app(tmp_dir).get_matches();
    if let Err(e) = run(arg_matches) {
        eprintln!("\nError: {}", e);
        process::exit(1);
    };
}
