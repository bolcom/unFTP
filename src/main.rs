#[macro_use]
extern crate lazy_static;

extern crate clap;

#[allow(dead_code)]
mod app;
mod args;
mod auth;
mod domain;
mod http;
mod infra;
mod logging;
mod metrics;
mod notify;
mod storage;

use crate::{
    app::libunftp_version,
    args::FtpsClientAuthType,
    auth::{DefaultUserProvider, JsonUserProvider},
    domain::{EventDispatcher, FTPEvent, FTPEventPayload},
    notify::FTPListener,
};
use auth::LookupAuthenticator;
use clap::ArgMatches;
use libunftp::{
    auth as auth_spi,
    notification::{DataListener, PresenceListener},
    options,
    options::{FailedLoginsBlock, FailedLoginsPolicy, FtpsClientAuth, FtpsRequired, SiteMd5, TlsFlags},
    storage::StorageBackend,
    Server,
};
use slog::*;
use std::{
    env, fs,
    net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs},
    path::PathBuf,
    process,
    process::Command,
    result::Result,
    str::FromStr,
    sync::Arc,
    time::Duration,
};
use tokio::{
    runtime::Runtime,
    signal::unix::{signal, SignalKind},
};
#[cfg(feature = "pam_auth")]
use unftp_auth_pam as pam;
use unftp_sbe_gcs::options::AuthMethod;

fn make_auth(
    m: &clap::ArgMatches,
) -> Result<Arc<dyn auth_spi::Authenticator<auth::User> + Send + Sync + 'static>, String> {
    let mut auth: LookupAuthenticator = match m.value_of(args::AUTH_TYPE) {
        None | Some("anonymous") => make_anon_auth(),
        Some("pam") => make_pam_auth(m),
        Some("rest") => make_rest_auth(m),
        Some("json") => make_json_auth(m),
        unknown_type => Err(format!("unknown auth type: {}", unknown_type.unwrap())),
    }?;
    auth.set_usr_detail(match m.value_of(args::USR_JSON_PATH) {
        Some(path) => {
            let json: String =
                fs::read_to_string(path).map_err(|e| format!("could not load user file '{}': {}", path, e))?;
            Box::new(JsonUserProvider::from_json(json.as_str())?)
        }
        None => Box::new(DefaultUserProvider {}),
    });
    Ok(Arc::new(auth))
}

fn make_anon_auth() -> Result<LookupAuthenticator, String> {
    Ok(LookupAuthenticator::new(auth_spi::AnonymousAuthenticator))
}

fn make_pam_auth(m: &clap::ArgMatches) -> Result<LookupAuthenticator, String> {
    #[cfg(not(feature = "pam_auth"))]
    {
        let _ = m;
        Err(String::from("the pam authentication module was disabled at build time"))
    }

    #[cfg(feature = "pam_auth")]
    {
        if let Some(service) = m.value_of(args::AUTH_PAM_SERVICE) {
            let pam_auth = pam::PamAuthenticator::new(service);
            return Ok(LookupAuthenticator::new(pam_auth));
        }
        Err(format!("--{} is required when using pam auth", args::AUTH_PAM_SERVICE))
    }
}

// FIXME: add user support
fn make_rest_auth(m: &clap::ArgMatches) -> Result<LookupAuthenticator, String> {
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

                let authenticator: unftp_auth_rest::RestAuthenticator = match unftp_auth_rest::Builder::new()
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

                Ok(LookupAuthenticator::new(authenticator))
            }
            _ => Err("for auth type rest please specify all auth-rest-* options".to_string()),
        }
    }
}

fn make_json_auth(m: &clap::ArgMatches) -> Result<LookupAuthenticator, String> {
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

        let authenticator = unftp_auth_jsonfile::JsonFileAuthenticator::from_file(path).map_err(|e| e.to_string())?;
        Ok(LookupAuthenticator::new(authenticator))
    }
}

type VfsProducer =
    Box<dyn (Fn() -> storage::RooterVfs<storage::RestrictingVfs, auth::User, storage::SbeMeta>) + Send + Sync>;

// Creates the filesystem storage back-end
fn fs_storage_backend(log: &Logger, m: &clap::ArgMatches) -> VfsProducer {
    let p: PathBuf = m.value_of(args::ROOT_DIR).unwrap().into();
    let sub_log = Arc::new(log.new(o!("module" => "storage")));
    Box::new(move || {
        storage::RooterVfs::new(storage::RestrictingVfs {
            delegate: storage::ChoosingVfs {
                inner: storage::InnerVfs::File(unftp_sbe_fs::Filesystem::new(p.clone())),
                log: sub_log.clone(),
            },
        })
    })
}

// Creates the GCS storage back-end
fn gcs_storage_backend(log: &Logger, m: &clap::ArgMatches) -> Result<VfsProducer, String> {
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
            ));
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
    Ok(Box::new(move || {
        storage::RooterVfs::new(storage::RestrictingVfs {
            delegate: storage::ChoosingVfs {
                inner: storage::InnerVfs::Cloud(unftp_sbe_gcs::CloudStorage::with_api_base(
                    base_url.clone(),
                    bucket.clone(),
                    root_dir.clone(),
                    auth_method.clone(),
                )),
                log: sub_log.clone(),
            },
        })
    }))
}

// starts the FTP server as a Tokio task.
fn start_ftp(
    log: &Logger,
    root_log: &Logger,
    m: &clap::ArgMatches,
    shutdown: tokio::sync::broadcast::Receiver<()>,
    done: tokio::sync::mpsc::Sender<()>,
) -> Result<(), String> {
    let event_dispatcher = notify::create_event_dispatcher(Arc::new(log.new(o!("module" => "storage"))), m)?;

    match m.value_of(args::STORAGE_BACKEND_TYPE) {
        None | Some("filesystem") => start_ftp_with_storage(
            log,
            root_log,
            m,
            fs_storage_backend(root_log, m),
            event_dispatcher,
            shutdown,
            done,
        ),
        Some("gcs") => start_ftp_with_storage(
            log,
            root_log,
            m,
            gcs_storage_backend(root_log, m)?,
            event_dispatcher,
            shutdown,
            done,
        ),
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
            Ok(IpAddr::V4(ip)) => Ok(options::PassiveHost::Ip(ip)),
            Ok(IpAddr::V6(_)) => Err(format!(
                "an IP is valid for the '--{}' argument, but it needs to be an IP v4 address",
                args::PASSIVE_HOST
            )),
            Err(_) => resolve_dns(log, ip_or_dns).map(options::PassiveHost::Ip),
        },
    }
}

// Given a storage back-end, starts the FTP server as a Tokio task.
fn start_ftp_with_storage<S>(
    log: &Logger,
    root_log: &Logger,
    arg_matches: &ArgMatches,
    storage_backend: Box<dyn (Fn() -> S) + Send + Sync>,
    event_dispatcher: Arc<dyn EventDispatcher<FTPEvent>>,
    mut shutdown: tokio::sync::broadcast::Receiver<()>,
    done: tokio::sync::mpsc::Sender<()>,
) -> Result<(), String>
where
    S: StorageBackend<auth::User> + Send + Sync + 'static,
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

    let md5_setting = match (
        arg_matches.value_of(args::STORAGE_BACKEND_TYPE),
        arg_matches.is_present(args::ENABLE_SITEMD5),
    ) {
        (Some("gcs"), _) => SiteMd5::All,
        (_, true) => SiteMd5::Accounts,
        (_, false) => SiteMd5::None,
    };

    let hostname = get_host_name();
    let instance_name = arg_matches.value_of(args::INSTANCE_NAME).unwrap().to_owned();

    let authenticator = make_auth(arg_matches)?;

    let l = log.clone();

    let listener = Arc::new(FTPListener {
        event_dispatcher: event_dispatcher.clone(),
        instance_name: instance_name.clone(),
        hostname: hostname.clone(),
    });

    let mut server = Server::with_authenticator(storage_backend, authenticator)
        .greeting("Welcome to unFTP")
        .passive_ports(start_port..end_port)
        .idle_session_timeout(idle_timeout)
        .logger(root_log.new(o!("lib" => "libunftp")))
        .passive_host(passive_host)
        .sitemd5(md5_setting)
        .notify_data(listener.clone() as Arc<dyn DataListener>)
        .notify_presence(listener as Arc<dyn PresenceListener>)
        .shutdown_indicator(async move {
            shutdown.recv().await.ok();
            info!(l, "Shutting down FTP server");
            libunftp::options::Shutdown::new().grace_period(Duration::from_secs(11))
        })
        .metrics();

    // Setup proxy protocol mode.
    if let Some(port) = arg_matches.value_of(args::PROXY_EXTERNAL_CONTROL_PORT) {
        let port_num = String::from(port)
            .parse::<u16>()
            .map_err(|e| format!("unable to parse proxy protocol external control port {}: {}", port, e))?;
        server = server.proxy_protocol_mode(port_num);
    }

    // Set up failed logins policy (anti-bruteforce)
    if let Some(arg) = arg_matches.value_of(args::FAILED_LOGINS_POLICY) {
        let policy = match arg.parse::<args::FailedLoginsPolicyType>()? {
            args::FailedLoginsPolicyType::ip => {
                FailedLoginsPolicy::new(3, Duration::from_secs(300), FailedLoginsBlock::IP)
            }
            args::FailedLoginsPolicyType::user => {
                FailedLoginsPolicy::new(3, Duration::from_secs(300), FailedLoginsBlock::User)
            }
            args::FailedLoginsPolicyType::combination => {
                FailedLoginsPolicy::new(3, Duration::from_secs(300), FailedLoginsBlock::UserAndIP)
            }
        };
        server = server.failed_logins_policy(policy);
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
            server
                .ftps_required(ftps_required_control, ftps_required_data)
                .ftps_tls_flags(TlsFlags::V1_2 | TlsFlags::RESUMPTION_SESS_ID | TlsFlags::RESUMPTION_TICKETS)
        }
        (Some(_), None) | (None, Some(_)) => {
            warn!(
                log,
                "Need to set both --{} and --{}. FTPS still disabled.",
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

    // MTLS
    server = match (
        arg_matches
            .value_of(args::FTPS_CLIENT_AUTH)
            .unwrap()
            .parse::<args::FtpsClientAuthType>()?,
        arg_matches.value_of(args::FTPS_TRUST_STORE),
    ) {
        (FtpsClientAuthType::off, _) => server.ftps_client_auth(FtpsClientAuth::Off),
        (FtpsClientAuthType::request, None) | (FtpsClientAuthType::require, None) => {
            warn!(
                log,
                "Need to set both --{} and --{}. MTLS still disabled.",
                args::FTPS_CLIENT_AUTH,
                args::FTPS_TRUST_STORE
            );
            server.ftps_client_auth(FtpsClientAuth::Off)
        }
        (FtpsClientAuthType::request, Some(file)) => {
            if !PathBuf::from(file).exists() {
                return Err(format!("file specified for --{} not found", args::FTPS_TRUST_STORE));
            }
            server.ftps_client_auth(FtpsClientAuth::Request).ftps_trust_store(file)
        }
        (FtpsClientAuthType::require, Some(file)) => {
            if !PathBuf::from(file).exists() {
                return Err(format!("file specified for --{} not found", args::FTPS_TRUST_STORE));
            }
            server.ftps_client_auth(FtpsClientAuth::Require).ftps_trust_store(file)
        }
    };

    let log = log.clone();
    tokio::spawn(async move {
        if let Err(e) = server.listen(addr).await {
            error!(log, "FTP server error: {:?}", e)
        }
        info!(log, "FTP exiting");
        drop(done)
    });

    tokio::spawn(async move {
        event_dispatcher
            .dispatch(FTPEvent {
                source_instance: instance_name,
                hostname,
                payload: FTPEventPayload::Startup {
                    unftp_version: app::VERSION.to_string(),
                    libunftp_version: libunftp_version().to_string(),
                },
                username: None,
                trace_id: None,
                sequence_number: None,
            })
            .await
    });

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

async fn main_task(arg_matches: ArgMatches, log: &Logger, root_log: &Logger) -> Result<ExitSignal, String> {
    let (shutdown_sender, http_receiver) = tokio::sync::broadcast::channel(1);
    let (http_done_sender, mut shutdown_done_received) = tokio::sync::mpsc::channel(1);
    let ftp_done_sender = http_done_sender.clone();

    let ftp_addr: SocketAddr = arg_matches
        .value_of(args::BIND_ADDRESS)
        .unwrap()
        .parse()
        .map_err(|_| "could not parse FTP address")?;

    if let Some(addr) = arg_matches.value_of(args::HTTP_BIND_ADDRESS) {
        let addr = String::from(addr);
        let log = log.clone();
        tokio::spawn(async move {
            if let Err(e) = http::start(&log, &*addr, ftp_addr, http_receiver, http_done_sender).await {
                error!(log, "HTTP Server error: {}", e)
            }
        });
    }

    start_ftp(
        log,
        root_log,
        &arg_matches,
        shutdown_sender.subscribe(),
        ftp_done_sender,
    )?;

    let signal = listen_for_signals().await?;
    info!(log, "Received signal {}, shutting down...", signal.0);

    drop(shutdown_sender);

    // When every sender has gone out of scope, the recv call
    // will return with an error. We ignore the error.
    let _ = shutdown_done_received.recv().await;

    Ok(signal)
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
    let _ = runtime.block_on(main_task(arg_matches, &log, &root_logger))?;
    info!(log, "Exiting...");
    Ok(())
}

fn get_host_name() -> String {
    if let Ok(host) = env::var("HOST") {
        return host;
    }
    if let Ok(host) = env::var("HOSTNAME") {
        return host;
    }
    match Command::new("hostname").output() {
        Ok(output) => String::from_utf8_lossy(&output.stdout).replace("\n", ""),
        Err(_) => "unknown".to_string(),
    }
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
