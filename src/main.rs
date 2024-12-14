#[macro_use]
extern crate lazy_static;

extern crate clap;

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

use crate::infra::userdetail_http::HTTPUserDetailProvider;
use crate::{
    app::libunftp_version, args::FtpsClientAuthType, auth::DefaultUserProvider, notify::FTPListener,
};
use args::AuthType;
use auth::LookupAuthenticator;
use base64::{engine, Engine};
use clap::ArgMatches;
use domain::events::{EventDispatcher, FTPEvent, FTPEventPayload};
use domain::user;
use flate2::read::GzDecoder;
use infra::usrdetail_json::JsonUserProvider;
use libunftp::{
    auth as auth_spi,
    notification::{DataListener, PresenceListener},
    options,
    options::{
        FailedLoginsBlock, FailedLoginsPolicy, FtpsClientAuth, FtpsRequired, SiteMd5, TlsFlags,
    },
    storage::StorageBackend,
    ServerBuilder,
};
use slog::*;
use std::io::{Read, Seek};
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

#[cfg(feature = "pam_auth")]
use unftp_auth_pam as pam;
use unftp_sbe_gcs::options::AuthMethod;
use unftp_sbe_restrict::RestrictingVfs;
use unftp_sbe_rooter::RooterVfs;

fn load_user_file(
    path: &str,
) -> Result<std::string::String, Box<dyn std::error::Error + Send + Sync + 'static>> {
    let mut f = fs::File::open(path)?;

    // The user file can be plaintext, gzipped, or gzipped+base64-encoded
    // The gzip-base64 format is useful for overcoming configmap size limits in Kubernetes
    let mut magic: [u8; 4] = [0; 4];
    let n = f.read(&mut magic[..])?;
    let is_gz = n > 2 && magic[0] == 0x1F && magic[1] == 0x8B && magic[2] == 0x8;
    // the 3 magic bytes translate to "H4sI" in base64
    let is_base64gz =
        n > 3 && magic[0] == b'H' && magic[1] == b'4' && magic[2] == b's' && magic[3] == b'I';

    f.rewind()?;
    if is_gz | is_base64gz {
        let mut gzdata: Vec<u8> = Vec::new();
        if is_base64gz {
            let mut b = Vec::new();
            f.read_to_end(&mut b)?;
            b.retain(|&x| x != b'\n' && x != b'\r');
            gzdata = engine::general_purpose::STANDARD.decode(b)?;
        } else {
            f.read_to_end(&mut gzdata)?;
        }
        let mut d = GzDecoder::new(&gzdata[..]);
        let mut s = String::new();
        d.read_to_string(&mut s)?;
        Ok(s)
    } else {
        let mut s = String::new();
        f.read_to_string(&mut s)?;
        Ok(s)
    }
}

fn make_auth(
    m: &clap::ArgMatches,
) -> Result<Arc<dyn auth_spi::Authenticator<user::User> + Send + Sync + 'static>, String> {
    let default_auth_type = AuthType::Anonymous.to_string();
    let input_auth_type = m.value_of(args::AUTH_TYPE).unwrap_or(&default_auth_type);
    let auth_type_variant = match input_auth_type.parse::<AuthType>() {
        Ok(auth_type_variant) => auth_type_variant,
        Err(strum::ParseError::VariantNotFound) => {
            return Err(format!("unknown auth type: {}", input_auth_type))
        }
    };

    let mut auth: LookupAuthenticator = match auth_type_variant {
        AuthType::Anonymous => make_anon_auth(),
        AuthType::Pam => make_pam_auth(m),
        AuthType::Rest => make_rest_auth(m),
        AuthType::Json => make_json_auth(m),
    }?;

    if auth_type_variant != AuthType::Pam && m.is_present(args::AUTH_PAM_SERVICE) {
        return Err(format!(
            "parameter {} set while auth_type is set to {}",
            args::AUTH_PAM_SERVICE,
            auth_type_variant
        ));
    } else if auth_type_variant != AuthType::Json && m.is_present(args::AUTH_JSON_PATH) {
        return Err(format!(
            "parameter {} set while auth_type is set to {}",
            args::AUTH_JSON_PATH,
            auth_type_variant
        ));
    } else if auth_type_variant != AuthType::Rest
        && [
            args::AUTH_REST_URL,
            args::AUTH_REST_REGEX,
            args::AUTH_REST_SELECTOR,
        ]
        .iter()
        .any(|&arg| m.is_present(arg))
    {
        return Err(format!(
            "REST auth parameter(s) set while auth_type is set to {}",
            auth_type_variant
        ));
    }

    auth.set_usr_detail(
        match (
            m.value_of(args::USR_JSON_PATH),
            m.value_of(args::USR_HTTP_URL),
        ) {
            (Some(path), None) => {
                let json: String = load_user_file(path)
                    .map_err(|e| format!("could not load user file '{}': {}", path, e))?;
                Box::new(JsonUserProvider::from_json(json.as_str())?)
            }
            (None, Some(url)) => Box::new(HTTPUserDetailProvider::new(url)),
            (None, None) => Box::new(DefaultUserProvider {}),
            _ => {
                return Err(format!(
                    "please specify either '{}' or '{}' but not both",
                    args::USR_JSON_PATH,
                    args::USR_HTTP_URL
                ))
            }
        },
    );
    Ok(Arc::new(auth))
}

fn make_anon_auth() -> Result<LookupAuthenticator, String> {
    Ok(LookupAuthenticator::new(auth_spi::AnonymousAuthenticator))
}

fn make_pam_auth(m: &clap::ArgMatches) -> Result<LookupAuthenticator, String> {
    #[cfg(not(feature = "pam_auth"))]
    {
        let _ = m;
        Err(String::from(
            "the pam authentication module was disabled at build time",
        ))
    }

    #[cfg(feature = "pam_auth")]
    {
        if let Some(service) = m.value_of(args::AUTH_PAM_SERVICE) {
            let pam_auth = pam::PamAuthenticator::new(service);
            return Ok(LookupAuthenticator::new(pam_auth));
        }
        Err(format!(
            "--{} is required when using pam auth",
            args::AUTH_PAM_SERVICE
        ))
    }
}

fn make_rest_auth(m: &clap::ArgMatches) -> Result<LookupAuthenticator, String> {
    #[cfg(not(feature = "rest_auth"))]
    {
        let _ = m;
        Err(format!(
            "the rest authentication module was disabled at build time"
        ))
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
                    return Err(
                        "REST authenticator error: no body provided for rest request".to_string(),
                    );
                }

                let body = String::from(m.value_of(args::AUTH_REST_BODY).unwrap_or(""));
                let mut builder = unftp_auth_rest::Builder::new()
                    .with_url(String::from(url))
                    .with_method(
                        hyper::Method::from_str(method)
                            .map_err(|e| format!("error creating REST auth: {}", e))?,
                    )
                    .with_body(String::from(m.value_of(args::AUTH_REST_BODY).unwrap_or("")))
                    .with_selector(String::from(selector))
                    .with_regex(String::from(regex));

                if url.contains("{USER}") || body.contains("{USER}") {
                    builder = builder.with_username_placeholder("{USER}".to_string());
                }

                if url.contains("{PASS}") || body.contains("{PASS}") {
                    builder = builder.with_password_placeholder("{PASS}".to_string());
                }

                if url.contains("{IP}") || body.contains("{IP}") {
                    builder = builder.with_source_ip_placeholder("{IP}".to_string());
                }

                let authenticator: unftp_auth_rest::RestAuthenticator = match builder.build() {
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
        Err(format!(
            "the jsonfile authentication module was disabled at build time"
        ))
    }

    #[cfg(feature = "jsonfile_auth")]
    {
        let path = m.value_of(args::AUTH_JSON_PATH).ok_or_else(|| {
            "please provide the json credentials file by specifying auth-json-path".to_string()
        })?;

        let authenticator = unftp_auth_jsonfile::JsonFileAuthenticator::from_file(path)
            .map_err(|e| e.to_string())?;
        Ok(LookupAuthenticator::new(authenticator))
    }
}

type VfsProducer = Box<
    dyn (Fn() -> RooterVfs<
            RestrictingVfs<storage::ChoosingVfs, user::User, storage::SbeMeta>,
            user::User,
            storage::SbeMeta,
        >) + Send
        + Sync,
>;

// Creates the filesystem storage back-end
fn fs_storage_backend(log: &Logger, m: &clap::ArgMatches) -> VfsProducer {
    let p: PathBuf = m.value_of(args::ROOT_DIR).unwrap().into();
    let sub_log = Arc::new(log.new(o!("module" => "storage")));
    Box::new(move || {
        RooterVfs::new(RestrictingVfs::new(storage::ChoosingVfs {
            inner: storage::InnerVfs::File(unftp_sbe_fs::Filesystem::new(p.clone())),
            log: sub_log.clone(),
        }))
    })
}

// Creates the GCS storage back-end
fn gcs_storage_backend(log: &Logger, m: &clap::ArgMatches) -> Result<VfsProducer, String> {
    let bucket: String = m
        .value_of(args::GCS_BUCKET)
        .ok_or_else(|| {
            format!(
                "--{} is required when using storage type gcs",
                args::GCS_BUCKET
            )
        })?
        .into();
    let base_url: String = m
        .value_of(args::GCS_BASE_URL)
        .ok_or_else(|| {
            format!(
                "--{} is required when using storage type gcs",
                args::GCS_BUCKET
            )
        })?
        .into();
    let root_dir: PathBuf = m
        .value_of(args::GCS_ROOT)
        .ok_or_else(|| {
            format!(
                "--{} is required when using storage type gcs",
                args::GCS_ROOT
            )
        })?
        .into();
    let auth_method: AuthMethod = match (
        m.value_of(args::GCS_SERVICE_ACCOUNT),
        m.value_of(args::GCS_KEY_FILE),
    ) {
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
            let service_account_key = std::fs::read(key_file).map_err(|e| {
                format!(
                    "could not load GCS back-end service account key from file: {}",
                    e
                )
            })?;
            AuthMethod::ServiceAccountKey(service_account_key)
        }
    };

    slog::info!(log, "GCS back-end auth method: {}", auth_method);

    let sub_log = Arc::new(log.new(o!("module" => "storage")));
    Ok(Box::new(move || {
        RooterVfs::new(RestrictingVfs::new(storage::ChoosingVfs {
            inner: storage::InnerVfs::Cloud(unftp_sbe_gcs::CloudStorage::with_api_base(
                base_url.clone(),
                bucket.clone(),
                root_dir.clone(),
                auth_method.clone(),
            )),
            log: sub_log.clone(),
        }))
    }))
}

#[cfg(feature = "azblob")]
pub fn azblob_storage_backend(log: &Logger, m: &clap::ArgMatches) -> Result<VfsProducer, String> {
    let mut b = opendal::services::Azblob::default();
    if let Some(val) = m.value_of(args::AZBLOB_ROOT) {
        b.root(val);
    }
    if let Some(val) = m.value_of(args::AZBLOB_CONTAINER) {
        b.container(val);
    }
    if let Some(val) = m.value_of(args::AZBLOB_ENDPOINT) {
        b.endpoint(val);
    }
    if let Some(val) = m.value_of(args::AZBLOB_ACCOUNT_NAME) {
        b.account_name(val);
    }
    if let Some(val) = m.value_of(args::AZBLOB_ACCOUNT_KEY) {
        b.account_key(val);
    }
    if let Some(val) = m.value_of(args::AZBLOB_SAS_TOKEN) {
        b.sas_token(val);
    }
    if let Some(val) = m.value_of(args::AZBLOB_BATCH_MAX_OPERATIONS) {
        b.batch_max_operations(
            val.parse::<usize>().map_err(|e| {
                format!("could not parse AZBLOB_BATCH_MAX_OPERATIONS to usize: {e}")
            })?,
        );
    }
    let op = opendal::Operator::new(b)
        .map_err(|e| format!("could not build Azblob: {e}"))?
        .finish();
    let sbe = unftp_sbe_opendal::OpendalStorage::new(op);
    let sub_log = Arc::new(log.new(o!("module" => "storage")));

    Ok(Box::new(move || {
        RooterVfs::new(RestrictingVfs::new(storage::ChoosingVfs {
            inner: storage::InnerVfs::OpenDAL(sbe.clone()),
            log: sub_log.clone(),
        }))
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
    let event_dispatcher =
        notify::create_event_dispatcher(Arc::new(log.new(o!("module" => "storage"))), m)?;
    let svc = |prod: VfsProducer| {
        start_ftp_with_storage(log, root_log, m, prod, event_dispatcher, shutdown, done)
    };

    match m.value_of(args::STORAGE_BACKEND_TYPE) {
        None | Some("filesystem") => svc(fs_storage_backend(root_log, m)),
        Some("gcs") => svc(gcs_storage_backend(root_log, m)?),
        #[cfg(feature = "azblob")]
        Some("azblob") => svc(azblob_storage_backend(root_log, m)?),
        Some(x) => Err(format!("unknown storage back-end type {}", x)),
    }
}

fn resolve_dns(log: &Logger, dns_name: &str) -> Result<Ipv4Addr, String> {
    slog::info!(log, "Resolving domain name '{}'", dns_name);
    // Normalize the address. If lookup_host (https://doc.rust-lang.org/1.6.0/std/net/fn.lookup_host.html)
    // is stable we won't need this.
    let host_port = dns_name
        .split(':')
        .take(1)
        .map(|s| format!("{}:21", s))
        .next()
        .unwrap();
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

fn get_passive_host_option(
    log: &Logger,
    arg_matches: &ArgMatches,
) -> Result<options::PassiveHost, String> {
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
    let end_port: u16 = ports[1]
        .parse()
        .map_err(|_| "end of port range needs to be numeric")?;

    info!(log, "Using passive port range {}..{}", start_port, end_port);

    let passive_host = get_passive_host_option(log, arg_matches)?;
    info!(log, "Using passive host option '{:?}'", passive_host);

    let idle_timeout_str = arg_matches.value_of(args::IDLE_SESSION_TIMEOUT).unwrap();
    let idle_timeout = String::from(idle_timeout_str)
        .parse::<u64>()
        .map_err(move |e| {
            format!(
                "unable to parse given value '{}' for --{}: {}. Please use a numeric value",
                idle_timeout_str,
                args::IDLE_SESSION_TIMEOUT,
                e
            )
        })?;

    info!(
        log,
        "Idle session timeout is set to {} seconds", idle_timeout
    );

    let md5_setting = match (
        arg_matches.value_of(args::STORAGE_BACKEND_TYPE),
        arg_matches.is_present(args::ENABLE_SITEMD5),
    ) {
        (Some("gcs"), _) => SiteMd5::All,
        (_, true) => SiteMd5::Accounts,
        (_, false) => SiteMd5::None,
    };

    let hostname = get_host_name();
    let instance_name = arg_matches
        .value_of(args::INSTANCE_NAME)
        .unwrap()
        .to_owned();

    let authenticator = make_auth(arg_matches)?;

    let l = log.clone();

    let listener = Arc::new(FTPListener {
        event_dispatcher: event_dispatcher.clone(),
        instance_name: instance_name.clone(),
        hostname: hostname.clone(),
    });

    let mut server = ServerBuilder::with_authenticator(storage_backend, authenticator)
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
        let port_num = String::from(port).parse::<u16>().map_err(|e| {
            format!(
                "unable to parse proxy protocol external control port {}: {}",
                port, e
            )
        })?;
        server = server.proxy_protocol_mode(port_num);
    }

    // Set up failed logins policy (anti-bruteforce)
    if let Some(arg) = arg_matches.value_of(args::FAILED_LOGINS_POLICY) {
        let max_attempts_str = arg_matches.value_of(args::FAILED_MAX_ATTEMPTS).unwrap();
        let max_attempts = String::from(max_attempts_str)
            .parse::<u32>()
            .map_err(move |e| {
                format!(
                    "unable to parse given value '{}' for --{}: {}. Please use a numeric value",
                    max_attempts_str,
                    args::FAILED_MAX_ATTEMPTS,
                    e
                )
            })?;

        let expires_after_str = arg_matches.value_of(args::FAILED_EXPIRE_AFTER).unwrap();
        let expires_after = String::from(expires_after_str)
            .parse::<u32>()
            .map_err(move |e| {
                format!(
                    "unable to parse given value '{}' for --{}: {}. Please use a numeric value",
                    expires_after_str,
                    args::FAILED_EXPIRE_AFTER,
                    e
                )
            })?;

        let policy = match arg.parse::<args::FailedLoginsPolicyType>()? {
            args::FailedLoginsPolicyType::ip => {
                info!(
                    log,
                    "Using failed logins policy to block by IP after {} attempts and expires after {} seconds",
                    max_attempts,
                    expires_after
                );
                FailedLoginsPolicy::new(
                    max_attempts,
                    Duration::from_secs(expires_after.into()),
                    FailedLoginsBlock::IP,
                )
            }
            args::FailedLoginsPolicyType::user => {
                info!(
                    log,
                    "Using failed logins policy to block by username after {} attempts and expires after {} seconds",
                    max_attempts,
                    expires_after
                );
                FailedLoginsPolicy::new(
                    max_attempts,
                    Duration::from_secs(expires_after.into()),
                    FailedLoginsBlock::User,
                )
            }
            args::FailedLoginsPolicyType::combination => {
                info!(log, "Using failed logins policy to block by username and IP after {} attempts and expires after {} seconds", max_attempts, expires_after);
                FailedLoginsPolicy::new(
                    max_attempts,
                    Duration::from_secs(expires_after.into()),
                    FailedLoginsBlock::UserAndIP,
                )
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
                        args::FtpsRequiredType::accounts => {
                            libunftp::options::FtpsRequired::Accounts
                        }
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
                .ftps_tls_flags(
                    TlsFlags::V1_2 | TlsFlags::RESUMPTION_SESS_ID | TlsFlags::RESUMPTION_TICKETS,
                )
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
                return Err(format!(
                    "file specified for --{} not found",
                    args::FTPS_TRUST_STORE
                ));
            }
            server
                .ftps_client_auth(FtpsClientAuth::Request)
                .ftps_trust_store(file)
        }
        (FtpsClientAuthType::require, Some(file)) => {
            if !PathBuf::from(file).exists() {
                return Err(format!(
                    "file specified for --{} not found",
                    args::FTPS_TRUST_STORE
                ));
            }
            server
                .ftps_client_auth(FtpsClientAuth::Require)
                .ftps_trust_store(file)
        }
    };

    let server = server
        .build()
        .map_err(|e| format!("Could not build server: {}", e))?;

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

#[derive(PartialEq)]
struct ExitSignal(pub &'static str);

async fn listen_for_signals() -> Result<ExitSignal, String> {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};

        let mut term_sig = signal(SignalKind::terminate())
            .map_err(|e| format!("could not listen for TERM signals: {}", e))?;
        let mut int_sig = signal(SignalKind::interrupt())
            .map_err(|e| format!("Could not listen for INT signal: {}", e))?;
        let mut hup_sig = signal(SignalKind::hangup())
            .map_err(|e| format!("Could not listen for HUP signal: {}", e))?;

        let sig_name = tokio::select! {
            Some(_signal) = term_sig.recv() => {
                "SIG_TERM"
            },
            Some(_signal) = int_sig.recv() => {
                "SIG_INT"
            },
            Some(_signal) = hup_sig.recv() => {
                "SIG_HUP"
            },
        };
        Ok(ExitSignal(sig_name))
    }

    #[cfg(windows)]
    {
        use tokio::signal;
        signal::ctrl_c()
            .await
            .map_err(|e| format!("could not listen for ctrl-c: {}", e))?;
        Ok(ExitSignal("CTRL-C"))
    }
}

async fn main_task(
    arg_matches: ArgMatches,
    log: &Logger,
    root_log: &Logger,
) -> Result<ExitSignal, String> {
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
            if let Err(e) =
                http::start(&log, &addr, ftp_addr, http_receiver, http_done_sender).await
            {
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

async fn run(arg_matches: ArgMatches) -> Result<(), String> {
    let (root_logger, google_shipper) = logging::create_logger(&arg_matches)?;
    let log = root_logger.new(o!("module" => "main"));

    let addr = arg_matches.value_of(args::BIND_ADDRESS).unwrap();
    let http_addr = arg_matches.value_of(args::HTTP_BIND_ADDRESS).unwrap();
    let auth_type = arg_matches.value_of(args::AUTH_TYPE).unwrap_or_else(|| {
        eprintln!("Required option --auth-type is missing. To disable authentication, use: `--auth-type anonymous` or set the `UNFTP_AUTH_TYPE=anonymous` environment variable.");
        ::std::process::exit(1)
    });
    let sbe_type = arg_matches.value_of(args::STORAGE_BACKEND_TYPE).unwrap();

    let home_dir = match sbe_type {
        "gcs" => arg_matches.value_of(args::GCS_ROOT).unwrap(),
        _ => arg_matches.value_of(args::ROOT_DIR).unwrap(),
    };

    info!(log, "Starting {} server.", app::NAME;
    "version" => app::VERSION,
    "libunftp-version" => app::libunftp_version(),
    "ftp-address" => &addr,
    "http-address" => &http_addr,
    "home" => home_dir,
    "auth-type" => auth_type,
    "sbe-type" => sbe_type,
    );

    // If logging needs to be sent to Google, we need to start tasks
    // to bridge between the sync and async channels, as well as start
    // the log shipper. For now this is the only clean way I could
    // find to ship from the slog Drain to the Google API
    if let Some(mut shipper) = google_shipper {
        // This is an sync to async bridge: The drain creates the
        // Google LogEntry's, and sends them over the sync
        // channel. The bridge receives it and forwards it over the
        // async bridge to the shipper.
        let bridge = shipper.yield_bridge();
        tokio::task::spawn_blocking(move || {
            bridge.run_sync_to_async_bridge();
        });

        // The shipper does the calls to Google Logging API
        tokio::task::spawn(async move {
            shipper.run_log_shipper().await;
        });

        info!(log, "Started Google Logger");
    }

    // We wait for a signal (HUP, INT, TERM). If the signal is a HUP,
    // we restart, otherwise we exit the loop and the program ends.
    while main_task(arg_matches.clone(), &log, &root_logger).await? == ExitSignal("SIG_HUP") {
        info!(log, "Received SIG_HUP, restarting");
    }
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
        Ok(output) => String::from_utf8_lossy(&output.stdout).replace('\n', ""),
        Err(_) => "unknown".to_string(),
    }
}

#[tokio::main]
async fn main() {
    #[cfg(feature = "tokio_console")]
    {
        console_subscriber::ConsoleLayer::builder()
            // set the address the server is bound to
            .server_addr(([127, 0, 0, 1], 6669))
            // ... other configurations ...
            .init();
    }

    let tmp_dir = env::temp_dir();
    let tmp_dir = tmp_dir.as_path().to_str().unwrap();
    let arg_matches = args::clap_app(tmp_dir).get_matches();
    if let Err(e) = run(arg_matches).await {
        eprintln!("\nError: {}", e);
        process::exit(1);
    };
}
