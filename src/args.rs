use crate::app;
use clap::{Arg, ArgEnum, Command};
use std::str::FromStr;

pub const AUTH_JSON_PATH: &str = "auth-json-path";
pub const AUTH_PAM_SERVICE: &str = "auth-pam-service";
pub const AUTH_REST_BODY: &str = "auth-rest-body";
pub const AUTH_REST_METHOD: &str = "auth-rest-method";
pub const AUTH_REST_REGEX: &str = "auth-rest-regex";
pub const AUTH_REST_SELECTOR: &str = "auth-rest-selector";
pub const AUTH_REST_URL: &str = "auth-rest-url";
pub const AUTH_TYPE: &str = "auth-type";
pub const BIND_ADDRESS: &str = "bind-address";
pub const ENABLE_SITEMD5: &str = "enable-sitemd5";
pub const FAILED_LOGINS_POLICY: &str = "failed-logins-policy";
pub const FTPS_CERTS_FILE: &str = "ftps-certs-file";
pub const FTPS_CLIENT_AUTH: &str = "ftps-client-auth";
pub const FTPS_KEY_FILE: &str = "ftps-key-file";
pub const FTPS_REQUIRED_ON_CONTROL_CHANNEL: &str = "ftps-required-on-control-channel";
pub const FTPS_REQUIRED_ON_DATA_CHANNEL: &str = "ftps-required-on-data-channel";
pub const FTPS_TRUST_STORE: &str = "ftps-trust-store";
pub const GCS_BASE_URL: &str = "sbe-gcs-base-url";
pub const GCS_BUCKET: &str = "sbe-gcs-bucket";
pub const GCS_KEY_FILE: &str = "sbe-gcs-key-file";
pub const GCS_ROOT: &str = "sbe-gcs-root";
pub const GCS_SERVICE_ACCOUNT: &str = "sbe-gcs-service-account";
pub const HTTP_BIND_ADDRESS: &str = "bind-address-http";
pub const IDLE_SESSION_TIMEOUT: &str = "idle-session-timeout";
pub const INSTANCE_NAME: &str = "instance-name";
pub const LOG_LEVEL: &str = "log-level";
pub const PASSIVE_HOST: &str = "passive-host";
pub const PASSIVE_PORTS: &str = "passive-ports";
pub const PROXY_EXTERNAL_CONTROL_PORT: &str = "proxy-external-control-port";
pub const PUBSUB_BASE_URL: &str = "ntf-pubsub-base-url";
pub const PUBSUB_TOPIC: &str = "ntf-pubsub-topic";
pub const PUBSUB_PROJECT: &str = "ntf-pubsub-project";
pub const REDIS_HOST: &str = "log-redis-host";
pub const REDIS_KEY: &str = "log-redis-key";
pub const REDIS_PORT: &str = "log-redis-port";
pub const ROOT_DIR: &str = "root-dir";
pub const STORAGE_BACKEND_TYPE: &str = "sbe-type";
pub const USR_JSON_PATH: &str = "usr-json-path";
pub const VERBOSITY: &str = "verbosity";

#[derive(ArgEnum, Clone, Debug)]
#[allow(non_camel_case_types)]
pub enum AuthType {
    anonymous,
    pam,
    rest,
    json,
}

#[derive(ArgEnum, Clone, Debug)]
#[allow(non_camel_case_types)]
pub enum StorageBackendType {
    filesystem,
    gcs,
}

#[derive(ArgEnum, Clone, Debug)]
#[allow(non_camel_case_types)]
pub enum FailedLoginsPolicyType {
    ip,
    user,
    combination,
}

impl FromStr for FailedLoginsPolicyType {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ip" => Ok(FailedLoginsPolicyType::ip),
            "user" => Ok(FailedLoginsPolicyType::user),
            "combination" => Ok(FailedLoginsPolicyType::combination),
            _ => Err("no match"),
        }
    }
}

#[derive(ArgEnum, Clone, Debug)]
#[allow(non_camel_case_types)]
pub enum FtpsRequiredType {
    all,
    accounts,
    none,
}

impl FromStr for FtpsRequiredType {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "all" => Ok(FtpsRequiredType::all),
            "accounts" => Ok(FtpsRequiredType::accounts),
            "none" => Ok(FtpsRequiredType::none),
            _ => Err("no match"),
        }
    }
}

#[derive(ArgEnum, Clone, Debug)]
#[allow(non_camel_case_types)]
pub enum FtpsClientAuthType {
    off,
    request,
    require,
}

impl FromStr for FtpsClientAuthType {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "off" => Ok(FtpsClientAuthType::off),
            "request" => Ok(FtpsClientAuthType::request),
            "require" => Ok(FtpsClientAuthType::require),
            _ => Err("no match"),
        }
    }
}

#[derive(clap::ArgEnum, Clone, Debug)]
#[allow(non_camel_case_types)]
pub enum LogLevelType {
    error,
    warn,
    info,
    debug,
    trace,
}

impl FromStr for LogLevelType {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "error" => Ok(LogLevelType::error),
            "warn" => Ok(LogLevelType::warn),
            "info" => Ok(LogLevelType::info),
            "debug" => Ok(LogLevelType::debug),
            "trace" => Ok(LogLevelType::trace),
            _ => Err("no match"),
        }
    }
}

pub(crate) fn clap_app(tmp_dir: &str) -> clap::Command {
    Command::new(app::NAME)
        .version(app::VERSION)
        .long_version(app::long_version())
        .about("An FTP server for when you need to FTP but don't want to")
        .author("The bol.com unFTP team")
        .arg(
            Arg::new(VERBOSITY)
                .short('v')
                .multiple_occurrences(true)
                .help("verbosity level")
        )
        .arg(
            Arg::new(LOG_LEVEL)
                .long("log-level")
                .value_name("LEVEL")
                .help("Sets the logging level. This overrides the verbosity flag -v if it is \
                          also specified.")
                .env("UNFTP_LOG_LEVEL")
                .takes_value(true)
        )
        .arg(
            Arg::new(BIND_ADDRESS)
                .long("bind-address")
                .value_name("HOST_PORT")
                .help("Sets the host and port to listen on for FTP(S) connections.")
                .env("UNFTP_BIND_ADDRESS")
                .takes_value(true)
                .default_value("0.0.0.0:2121"),
        )
        .arg(
            Arg::new(ROOT_DIR)
                .long("root-dir")
                .value_name("PATH")
                .help("When the storage backend type is 'filesystem' this sets the path where \
                          files are stored.")
                .env("UNFTP_ROOT_DIR")
                .takes_value(true)
                .default_value(tmp_dir),
        )
        .arg(
            Arg::new(FAILED_LOGINS_POLICY)
                .long("failed-logins-policy")
                .value_name("POLICY")
                .help("Enable a policy for failed logins to deter \
                       password guessing (brute-force) attacks. When set to 'user', any login \
                       attempt for that user is temporary blocked if there were too many failed \
                       login attempts. When set to 'ip', any login attempt for any account is \
                       temporarily blocked from that client IP. When set to 'combination', only \
                       the specific combination of client IP and username will be blocked after \
                       too many failed login attempts.")
                .env("FAILED_LOGIN_POLICY")
                .takes_value(true)
                .default_missing_value("combination"),
        )

        .arg(
            Arg::new(FTPS_CERTS_FILE)
                .long("ftps-certs-file")
                .value_name("FILE")
                .help("Sets the path to the certificates (PEM format) used for TLS security")
                .env("UNFTP_FTPS_CERTS_FILE")
                .takes_value(true)
                .requires(FTPS_KEY_FILE),
        )
        .arg(
            Arg::new(FTPS_CLIENT_AUTH)
                .long("ftps-client-auth")
                .value_name("CLIENT_AUTH_SETTING")
                .help("Allows switching on Mutual TLS. The difference \
                          between 'request' and 'require' is that the former does not enforce the use \
                          of client side certificates although it still does validation if sent. \
                          The latter won't let TLS connections proceed unless the client sents a \
                          valid certificate")
                .env("UNFTP_FTPS_CLIENT_AUTH")
                .takes_value(true)
                .default_value("off")
        )
        .arg(
            Arg::new(FTPS_KEY_FILE)
                .long("ftps-key-file")
                .value_name("FILE")
                .help("Sets the path to the private key file (PEM format) used for TLS security")
                .env("UNFTP_FTPS_KEY_FILE")
                .takes_value(true)
                .requires(FTPS_CERTS_FILE),
        )
        .arg(
            Arg::new(FTPS_REQUIRED_ON_CONTROL_CHANNEL)
                .long("ftps-required-on-control-channel")
                .value_name("REQUIRE_SETTING")
                .help("Sets whether FTP clients are required to upgrade to FTPS on the control channel. The difference \
                          between 'all' and 'accounts' is that the latter does not enforce FTPS on \
                          anonymous logins i.e. it applies to accounts only")
                .env("UNFTP_FTPS_REQUIRED_ON_CONTROL_CHANNEL")
                .takes_value(true)
                .default_value("none")
        )
        .arg(
            Arg::new(FTPS_REQUIRED_ON_DATA_CHANNEL)
                .long("ftps-required-on-data-channel")
                .value_name("REQUIRE_SETTING")
                .help("Sets whether FTP clients are required to upgrade to FTPS on the data channel. The difference \
                          between 'all' and 'accounts' is that the latter does not enforce FTPS on \
                          anonymous logins i.e. it applies to accounts only")
                .env("UNFTP_FTPS_REQUIRED_ON_DATA_CHANNEL")
                .takes_value(true)
                .default_value("none")
        )
        .arg(
            Arg::new(FTPS_TRUST_STORE)
                .long("ftps-trust-store")
                .value_name("FILE")
                .help("Sets the path to a PEM file containing certificates to use when validating \
                          client certificates in MTLS mode")
                .env("UNFTP_FTPS_TRUST_STORE")
                .takes_value(true)
                .requires(FTPS_CLIENT_AUTH),
        )
        .arg(
            Arg::new(REDIS_KEY)
                .long("log-redis-key")
                .value_name("KEY")
                .help("Sets the key name for storage in Redis")
                .env("UNFTP_LOG_REDIS_KEY")
                .takes_value(true),
        )
        .arg(
            Arg::new(REDIS_HOST)
                .long("log-redis-host")
                .value_name("HOST")
                .help("Sets the hostname for the Redis server where logging should go")
                .env("UNFTP_LOG_REDIS_HOST")
                .takes_value(true),
        )
        .arg(
            Arg::new(REDIS_PORT)
                .long("log-redis-port")
                .value_name("PORT")
                .help("Sets the port for the Redis server where logging should go")
                .env("UNFTP_LOG_REDIS_PORT")
                .takes_value(true),
        )
        .arg(
            Arg::new(HTTP_BIND_ADDRESS)
                .long("bind-address-http")
                .value_name("HOST_PORT")
                .help("Sets the host and port for the HTTP server used by prometheus metrics collection")
                .env("UNFTP_BIND_ADDRESS_HTTP")
                .takes_value(true)
                .default_value("0.0.0.0:8080"),
        )
        .arg(
            Arg::new(INSTANCE_NAME)
                .long("instance-name")
                .value_name("NAME")
                .help("Gives a user friendly name to this instance. This is for used for example \
                          as part of the app name during logging.")
                .env("UNFTP_INSTANCE_NAME")
                .takes_value(true)
                .default_value("unFTP"),
        )
        .arg(
            Arg::new(PASSIVE_PORTS)
                .long("passive-ports")
                .value_name("PORT_RANGE")
                .help("Sets the port range for data connections. In proxy protocol mode this \
                          resembles ports on the proxy.")
                .env("UNFTP_PASSIVE_PORTS")
                .takes_value(true)
                .default_value("49152-65535"),
        )
        .arg(
            Arg::new(PASSIVE_HOST)
                .long("passive-host")
                .value_name("HOST")
                .help("Tells how unFTP determines the IP that is sent in response to PASV. \
                          Can be fixed, a DNS name or determined from the incoming control connection. \
                          Examples: 'from-connection', '127.0.0.1' or 'ftp.myhost.org'"
                )
                .env("UNFTP_PASSIVE_HOST")
                .takes_value(true)
                .default_value("from-connection"),
        )
        .arg(
            Arg::new(AUTH_TYPE)
                .long("auth-type")
                .value_name("NAME")
                .help("The type of authorization to use")
                //.case_insensitive(true)
                .env("UNFTP_AUTH_TYPE")
                .takes_value(true)
                .default_value("anonymous"),
        )
        .arg(
            Arg::new(AUTH_PAM_SERVICE)
                .long("auth-pam-service")
                .value_name("NAME")
                .help("The name of the PAM service")
                .env("UNFTP_AUTH_PAM_SERVICE")
                .takes_value(true),
        )
        .arg(
            Arg::new(AUTH_REST_URL)
                .long("auth-rest-url")
                .value_name("URL")
                .help("Define REST endpoint. {USER} and {PASS} are replaced by provided credentials.")
                .env("UNFTP_AUTH_REST_URL")
                .takes_value(true),
        )
        .arg(
            Arg::new(AUTH_REST_METHOD)
                .long("auth-rest-method")
                .value_name("URL")
                .help("HTTP method to access REST endpoint")
                .env("UNFTP_AUTH_REST_METHOD")
                .default_value("GET")
                .takes_value(true),
        )
        .arg(
            Arg::new(AUTH_REST_BODY)
                .long("auth-rest-body")
                .value_name("URL")
                .help("If HTTP method contains body, it can be specified here. {USER} and {PASS} \
                are replaced by provided credentials.")
                .env("UNFTP_AUTH_REST_BODY")
                .takes_value(true),
        )
        .arg(
            Arg::new(AUTH_REST_SELECTOR)
                .long("auth-rest-selector")
                .value_name("SELECTOR")
                .help("Define JSON pointer to fetch from REST response body (RFC6901)")
                .env("UNFTP_AUTH_REST_SELECTOR")
                .takes_value(true),
        )
        .arg(
            Arg::new(AUTH_REST_REGEX)
                .long("auth-rest-regex")
                .value_name("REGEX")
                .help("Regular expression to try match against value extracted via selector")
                .env("UNFTP_AUTH_REST_REGEX")
                .takes_value(true),
        )
        .arg(
            Arg::new(AUTH_JSON_PATH)
                .long("auth-json-path")
                .value_name("PATH")
                .help("The path to the json authentication file")
                .env("UNFTP_AUTH_JSON_PATH")
                .takes_value(true),
        )
        .arg(
            Arg::new(STORAGE_BACKEND_TYPE)
                .long("sbe-type")
                .value_name("NAME")
                .help("The type of storage backend to use.")
                .env("UNFTP_SBE_TYPE")
                .takes_value(true)
                .default_value("filesystem"),
        )
        .arg(
            Arg::new(GCS_BASE_URL)
                .long("sbe-gcs-base-url")
                .value_name("URL")
                .help("The base url of Google Cloud Storage API")
                .env("UNFTP_SBE_GCS_BASE_URL")
                .default_value("https://www.googleapis.com")
                .takes_value(true),
        )
        .arg(
            Arg::new(GCS_BUCKET)
                .long("sbe-gcs-bucket")
                .value_name("BUCKET")
                .help("The bucket to use for the Google Cloud Storage backend")
                .env("UNFTP_SBE_GCS_BUCKET")
                .takes_value(true),
        )
        .arg(
            Arg::new(GCS_KEY_FILE)
                .long("sbe-gcs-key-file")
                .value_name("KEY_FILE")
                .help("The JSON file that contains the service account key for access to Google Cloud Storage.")
                .env("UNFTP_SBE_GCS_KEY_FILE")
                .takes_value(true),
        )
        .arg(
            Arg::new(GCS_ROOT)
                .long("sbe-gcs-root")
                .value_name("PATH")
                .help("The root path in the bucket where unFTP will look for and store files.")
                .env("UNFTP_SBE_GCS_ROOT")
                .default_value("")
                .takes_value(true),
        )
        .arg(
            Arg::new(GCS_SERVICE_ACCOUNT)
                .long("sbe-gcs-service-account")
                .value_name("SERVICE_ACCOUNT_NAME")
                .help("The name of the service account to use when authenticating using GKE workload identity.")
                .env("UNFTP_SBE_GCS_SERVICE_ACCOUNT")
                .takes_value(true),
        )
        .arg(
            Arg::new(IDLE_SESSION_TIMEOUT)
                .long("idle-session-timeout")
                .value_name("TIMEOUT_SECONDS")
                .help("The timeout in seconds after which idle connections will be closed")
                .env("UNFTP_IDLE_SESSION_TIMEOUT")
                .takes_value(true)
                .default_value("600"),
        )
        .arg(
            Arg::new(PROXY_EXTERNAL_CONTROL_PORT)
                .long("proxy-external-control-port")
                .value_name("PORT")
                .help("This switches on proxy protocol mode and sets the external control port number exposed on the proxy.")
                .env("UNFTP_PROXY_EXTERNAL_CONTROL_PORT")
                .takes_value(true),
        )
        .arg(
            Arg::new(ENABLE_SITEMD5)
                .long("enable-sitemd5")
                .help("Enable the SITE MD5 command for authenticated users (not anonymous) (always enabled for GCS backend)")
                .env("UNFTP_ENABLE_SITEMD5")
                .takes_value(false)
        )
        .arg(
            Arg::new(USR_JSON_PATH)
                .long("usr-json-path")
                .value_name("PATH")
                .help("The path to a JSON user detail file")
                .env("UNFTP_USR_JSON_PATH")
                .takes_value(true),
        )
        .arg(
            Arg::new(PUBSUB_BASE_URL)
                .long("ntf-pubsub-base-url")
                .value_name("URL")
                .help("The base url of the Google Pub/Sub API")
                .env("UNFTP_NTF_PUBSUB_BASE_URL")
                .default_value("https://pubsub.googleapis.com")
                .takes_value(true),
        )
        .arg(
            Arg::new(PUBSUB_TOPIC)
                .long("ntf-pubsub-topic")
                .value_name("TOPIC_NAME")
                .help("The name of the Google Pub/Sub topic to publish to")
                .env("UNFTP_NTF_PUBSUB_TOPIC")
                .takes_value(true),
        )
        .arg(
            Arg::new(PUBSUB_PROJECT)
                .long("ntf-pubsub-project")
                .value_name("PROJECT_ID")
                .help("The ID of the GCP project where the Google Pub/Sub topic exists")
                .env("UNFTP_NTF_PUBSUB_PROJECT")
                .takes_value(true),
        )
}
