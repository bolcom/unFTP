use crate::app;
use clap::{App, Arg};

pub(crate) fn clap_app(tmp_dir: &str) -> clap::App {
    App::new(app::NAME)
        .version(app::VERSION)
        .about("An FTP server for when you need to FTP but don't want to")
        .author("The bol.com unFTP team")
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .multiple(true)
                .help("verbosity level"),
        )
        .arg(
            Arg::with_name("bind-address")
                .long("bind-address")
                .value_name("HOST_PORT")
                .help("Sets the host and port to listen on for FTP control connections")
                .default_value("0.0.0.0:2121")
                .env("UNFTP_ADDRESS")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("home-dir")
                .long("home-dir")
                .value_name("PATH")
                .help("Sets the FTP home directory")
                .default_value(tmp_dir)
                .env("UNFTP_HOME")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("ftps-certs-file")
                .long("ftps-certs-file")
                .value_name("PEM_FILE")
                .help("Sets the path the the certificates used for TLS security")
                .env("UNFTP_CERTS_FILE")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("ftps-key-file")
                .long("ftps-key-file")
                .value_name("PEM_FILE")
                .help("Sets the path to the private key file used for TLS security")
                .env("UNFTP_CERTS_FILE")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("log-redis-key")
                .long("log-redis-key")
                .value_name("KEY")
                .help("Sets the key name for storage in Redis")
                .env("UNFTP_LOG_REDIS_KEY")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("log-redis-host")
                .long("log-redis-host")
                .value_name("HOST")
                .help("Sets the hostname for the Redis server where logging should go")
                .env("UNFTP_LOG_REDIS_HOST")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("log-redis-port")
                .long("log-redis-port")
                .value_name("PORT")
                .help("Sets the port for the Redis server where logging should go")
                .env("UNFTP_LOG_REDIS_PORT")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("bind-address-http")
                .long("bind-address-http")
                .value_name("HOST_PORT")
                .help("Sets the host and port for the HTTP server used by prometheus metrics collection")
                .env("UNFTP_METRICS_ADDRESS")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("auth-type")
                .long("auth-type")
                .value_name("NAME")
                .help("The type of authorization to use. One of 'anonymous', 'pam' or 'rest'")
                .default_value("anonymous")
                .env("UNFTP_AUTH_REST_URL")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("auth-pam-service")
                .long("auth-pam-service")
                .value_name("NAME")
                .help("The name of the PAM service")
                .env("UNFTP_PAM_SERVICE")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("auth-rest-url")
                .long("auth-rest-url")
                .value_name("URL")
                .help("-")
                .env("UNFTP_AUTH_REST_URL")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("auth-rest-method")
                .long("auth-rest-method")
                .value_name("URL")
                .help("-")
                .env("UNFTP_AUTH_REST_METHOD")
                .default_value("GET")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("auth-rest-body")
                .long("auth-rest-body")
                .value_name("URL")
                .help("-")
                .env("UNFTP_AUTH_REST_BODY")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("auth-rest-selector")
                .long("auth-rest-selector")
                .value_name("SELECTOR")
                .help("-")
                .env("UNFTP_AUTH_REST_SELECTOR")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("auth-rest-regex")
                .long("auth-rest-regex")
                .value_name("REGEX")
                .help("-")
                .env("UNFTP_AUTH_REST_REGEX")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("sbe-type")
                .long("sbe-type")
                .value_name("NAME")
                .help("The type of storage backend to use. Either 'filesystem' or 'gcs'")
                .default_value("filesystem")
                .env("UNFTP_SBE_TYPE")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("sbe-gcs-bucket")
                .long("sbe-gcs-bucket")
                .value_name("BUCKET")
                .help("The bucket to use for the Google Cloud Storage backend")
                .env("UNFTP_GCS_BUCKET")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("sbe-gcs-serv-acc-key")
                .long("sbe-gcs-serv-acc-key")
                .value_name("KEY")
                .help("The service account key for access to Google Cloud Storage.")
                .env("UNFTP_GCS_KEY")
                .takes_value(true),
        )
}
