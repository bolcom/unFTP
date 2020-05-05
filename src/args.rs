use crate::app;
use clap::{App, Arg};

pub const AUTH_PAM_SERVICE: &str = "auth-pam-service";
pub const AUTH_REST_BODY: &str = "auth-rest-body";
pub const AUTH_REST_METHOD: &str = "auth-rest-method";
pub const AUTH_REST_REGEX: &str = "auth-rest-regex";
pub const AUTH_REST_SELECTOR: &str = "auth-rest-selector";
pub const AUTH_REST_URL: &str = "auth-rest-url";
pub const AUTH_JSON_PATH: &str = "auth-json-path";
pub const AUTH_TYPE: &str = "auth-type";
pub const BIND_ADDRESS: &str = "bind-address";
pub const FTPS_CERTS_FILE: &str = "ftps-certs-file";
pub const FTPS_KEY_FILE: &str = "ftps-key-file";
pub const GCS_BUCKET: &str = "sbe-gcs-bucket";
pub const GCS_KEY_FILE: &str = "sbe-gcs-key-file";
pub const HTTP_BIND_ADDRESS: &str = "bind-address-http";
pub const IDLE_SESSION_TIMEOUT: &str = "idle-session-timeout";
pub const PASSIVE_PORTS: &str = "passive-ports";
pub const PROXY_EXTERNAL_CONTROL_PORT: &str = "proxy-external-control-port";
pub const REDIS_HOST: &str = "log-redis-host";
pub const REDIS_KEY: &str = "log-redis-key";
pub const REDIS_PORT: &str = "log-redis-port";
pub const ROOT_DIR: &str = "root-dir";
pub const STORAGE_BACKEND_TYPE: &str = "sbe-type";
pub const VERBOSITY: &str = "verbosity";

arg_enum! {
    #[derive(Debug)]
    #[allow(non_camel_case_types)]
    enum AuthType {
        anonymous,
        pam,
        rest,
        json,
    }
}

arg_enum! {
    #[derive(Debug)]
    #[allow(non_camel_case_types)]
    enum StorageBackendType {
        filesystem,
        gcs,
    }
}

pub(crate) fn clap_app(tmp_dir: &str) -> clap::App {
    App::new(app::NAME)
        .version(app::VERSION)
        .long_version(app::long_version())
        .about("An FTP server for when you need to FTP but don't want to")
        .author("The bol.com unFTP team")
        .arg(
            Arg::with_name(VERBOSITY)
                .short("v")
                .multiple(true)
                .help("verbosity level"),
        )
        .arg(
            Arg::with_name(BIND_ADDRESS)
                .long("bind-address")
                .value_name("HOST_PORT")
                .help("Sets the host and port to listen on for FTP(S) connections.")
                .env("UNFTP_BIND_ADDRESS")
                .takes_value(true)
                .default_value("0.0.0.0:2121"),
        )
        .arg(
            Arg::with_name(ROOT_DIR)
                .long("root-dir")
                .value_name("PATH")
                .help("When the storage backend type is 'filesystem' this sets the path where files are stored.")
                .env("UNFTP_ROOT_DIR")
                .takes_value(true)
                .default_value(tmp_dir),
        )
        .arg(
            Arg::with_name(FTPS_CERTS_FILE)
                .long("ftps-certs-file")
                .value_name("FILE")
                .help("Sets the path the the certificates (PEM format) used for TLS security")
                .env("UNFTP_FTPS_CERTS_FILE")
                .takes_value(true)
                .requires(FTPS_KEY_FILE),
        )
        .arg(
            Arg::with_name(FTPS_KEY_FILE)
                .long("ftps-key-file")
                .value_name("FILE")
                .help("Sets the path to the private key file (PEM format) used for TLS security")
                .env("UNFTP_FTPS_KEY_FILE")
                .takes_value(true)
                .requires(FTPS_CERTS_FILE),
        )
        .arg(
            Arg::with_name(REDIS_KEY)
                .long("log-redis-key")
                .value_name("KEY")
                .help("Sets the key name for storage in Redis")
                .env("UNFTP_LOG_REDIS_KEY")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(REDIS_HOST)
                .long("log-redis-host")
                .value_name("HOST")
                .help("Sets the hostname for the Redis server where logging should go")
                .env("UNFTP_LOG_REDIS_HOST")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(REDIS_PORT)
                .long("log-redis-port")
                .value_name("PORT")
                .help("Sets the port for the Redis server where logging should go")
                .env("UNFTP_LOG_REDIS_PORT")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(HTTP_BIND_ADDRESS)
                .long("bind-address-http")
                .value_name("HOST_PORT")
                .help("Sets the host and port for the HTTP server used by prometheus metrics collection")
                .env("UNFTP_BIND_ADDRESS_HTTP")
                .takes_value(true)
                .default_value("0.0.0.0:8080"),
        )
        .arg(
            Arg::with_name(PASSIVE_PORTS)
                .long("passive-ports")
                .value_name("PORT_RANGE")
                .help("Sets the port range for data connections. In proxy protocol mode this resembles ports on the proxy.")
                .env("UNFTP_PASSIVE_PORTS")
                .takes_value(true)
                .default_value("49152-65535"),
        )
        .arg(
            Arg::with_name(AUTH_TYPE)
                .long("auth-type")
                .value_name("NAME")
                .help("The type of authorization to use")
                .possible_values(&AuthType::variants())
                //.case_insensitive(true)
                .env("UNFTP_AUTH_TYPE")
                .takes_value(true)
                .default_value("anonymous"),
        )
        .arg(
            Arg::with_name(AUTH_PAM_SERVICE)
                .long("auth-pam-service")
                .value_name("NAME")
                .help("The name of the PAM service")
                .env("UNFTP_AUTH_PAM_SERVICE")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(AUTH_REST_URL)
                .long("auth-rest-url")
                .value_name("URL")
                .help("Define REST endpoint. {USER} and {PASS} are replaced by provided credentials.")
                .env("UNFTP_AUTH_REST_URL")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(AUTH_REST_METHOD)
                .long("auth-rest-method")
                .value_name("URL")
                .help("HTTP method to access REST endpoint")
                .env("UNFTP_AUTH_REST_METHOD")
                .default_value("GET")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(AUTH_REST_BODY)
                .long("auth-rest-body")
                .value_name("URL")
                .help("If HTTP method contains body, it can be specified here. {USER} and {PASS} are replaced by provided credentials.")
                .env("UNFTP_AUTH_REST_BODY")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(AUTH_REST_SELECTOR)
                .long("auth-rest-selector")
                .value_name("SELECTOR")
                .help("Define JSON pointer to fetch from REST response body (RFC6901)")
                .env("UNFTP_AUTH_REST_SELECTOR")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(AUTH_REST_REGEX)
                .long("auth-rest-regex")
                .value_name("REGEX")
                .help("Regular expression to try match against value extracted via selector")
                .env("UNFTP_AUTH_REST_REGEX")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(AUTH_JSON_PATH)
                .long("auth-json-path")
                .value_name("PATH")
                .help("The path to the json authentication file")
                .env("UNFTP_AUTH_JSON_PATH")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(STORAGE_BACKEND_TYPE)
                .long("sbe-type")
                .value_name("NAME")
                .help("The type of storage backend to use.")
                .possible_values(&StorageBackendType::variants())
                .env("UNFTP_SBE_TYPE")
                .takes_value(true)
                .default_value("filesystem"),
        )
        .arg(
            Arg::with_name(GCS_BUCKET)
                .long("sbe-gcs-bucket")
                .value_name("BUCKET")
                .help("The bucket to use for the Google Cloud Storage backend")
                .env("UNFTP_SBE_GCS_BUCKET")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(GCS_KEY_FILE)
                .long("sbe-gcs-key-file")
                .value_name("KEY_FILE")
                .help("The JSON file that contains the service account key for access to Google Cloud Storage.")
                .env("UNFTP_SBE_GCS_KEY_FILE")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(IDLE_SESSION_TIMEOUT)
                .long("idle-session-timeout")
                .value_name("TIMEOUT_SECONDS")
                .help("The timeout in seconds after which idle connections will be closed")
                .env("UNFTP_IDLE_SESSION_TIMEOUT")
                .takes_value(true)
                .default_value("600"),
        )
        .arg(
            Arg::with_name(PROXY_EXTERNAL_CONTROL_PORT)
                .long("proxy-external-control-port")
                .value_name("PORT")
                .help("This switches on proxy protocol mode and sets the external control port number exposed on the proxy.")
                .env("UNFTP_PROXY_EXTERNAL_CONTROL_PORT")
                .takes_value(true),
        )
}
