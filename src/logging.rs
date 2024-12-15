use crate::app;
use crate::args;
use crate::args::GLOG_LABELS_FILE;
use crate::args::GLOG_LEVEL_LABEL;
use crate::args::GLOG_LOGNAME;
use crate::args::GLOG_RESOURCE_TYPE;

use app::NAME;
use args::{INSTANCE_NAME, LOG_LEVEL, REDIS_HOST, REDIS_KEY, REDIS_PORT, VERBOSITY};
use clap::ArgMatches;
use slog::{error, o, Drain, Duplicate, Level, Logger, OwnedKVList, Record};
use slog_async::Async;
use slog_redis as redislog;
use slog_term::{CompactFormat, TermDecorator};
use std::{fmt::Display, result::Result};

#[derive(Clone)]
struct FallbackToStderr<D: Drain> {
    drain: D,
}

impl<D: Drain> Drain for FallbackToStderr<D>
where
    D::Err: Display,
{
    type Ok = ();
    type Err = ();
    fn log(&self, record: &Record, logger_values: &OwnedKVList) -> Result<(), ()> {
        if let Err(err) = self.drain.log(record, logger_values) {
            eprint!("A drain could not log to its destination: {}", err);
        }
        Ok(())
    }

    #[inline]
    fn is_enabled(&self, level: Level) -> bool {
        self.drain.is_enabled(level)
    }
}

pub fn create_logger(
    arg_matches: &ArgMatches,
) -> Result<(slog::Logger, Option<slog_google::shipper::Shipper>), String> {
    let min_log_level = match arg_matches.occurrences_of(VERBOSITY) {
        0 => Level::Warning,
        1 => Level::Info,
        2 => Level::Debug,
        _ => Level::Trace,
    };

    let min_log_level = match arg_matches.value_of(LOG_LEVEL) {
        Some(level) => match level.parse::<args::LogLevelType>()? {
            args::LogLevelType::error => Level::Error,
            args::LogLevelType::warn => Level::Warning,
            args::LogLevelType::info => Level::Info,
            args::LogLevelType::debug => Level::Debug,
            args::LogLevelType::trace => Level::Trace,
        },
        None => min_log_level,
    };

    let decorator = TermDecorator::new().force_color().build();
    let term_drain = CompactFormat::new(decorator)
        .build()
        .filter_level(min_log_level)
        .map(|drain| FallbackToStderr { drain })
        .fuse();

    let mut err: Option<String> = None;

    let redis_result = redis_logger(arg_matches);

    let google_result = google_logger(arg_matches);
    let mut google_shipper = None;

    let drain = match (redis_result, google_result) {
        (Ok(Some(redis_logger)), _) => {
            let both = Duplicate::new(redis_logger, term_drain).fuse();
            Async::new(both.filter_level(min_log_level).fuse())
                .build()
                .fuse()
        }
        (_, Ok(Some(google_logger))) => {
            let (drain, shipper) = google_logger;
            google_shipper = Some(shipper);
            let both = Duplicate::new(drain, term_drain).fuse();
            Async::new(both.filter_level(min_log_level).fuse())
                .build()
                .fuse()
        }
        (Ok(None), Ok(None)) => Async::new(term_drain).build().fuse(),
        (Err(e), _) | (_, Err(e)) => {
            err = e.into();
            Async::new(term_drain).build().fuse()
        }
    };
    let root = Logger::root(drain, o!());
    let log = root.new(o!());
    if let Some(err_str) = err {
        error!(log, "Continuing only with terminal logger: {}", err_str)
    }
    Ok((log, google_shipper))
}

fn redis_logger(m: &ArgMatches) -> Result<Option<FallbackToStderr<redislog::Logger>>, String> {
    match (
        m.value_of(REDIS_KEY),
        m.value_of(REDIS_HOST),
        m.value_of(REDIS_PORT),
    ) {
        (Some(key), Some(host), Some(port)) => {
            let instance_name = m.value_of(INSTANCE_NAME).unwrap();
            let app_name = if instance_name == NAME {
                String::from(NAME)
            } else {
                format!("{}-{}", NAME, instance_name)
            };
            let logger = redislog::Builder::new(&app_name)
                .redis(
                    String::from(host),
                    String::from(port).parse::<u32>().unwrap(),
                    String::from(key),
                )
                .build()
                .map_err(|e| format!("could not initialize Redis logger: {}", e))?;
            Ok(Some(logger.map(|drain| FallbackToStderr { drain })))
        }
        (None, None, None) => Ok(None),
        _ => Err("for the redis logger please specify all --log-redis-* options".to_string()),
    }
}

fn load_labels_file(file_path: &str, hostname: &str) -> Result<serde_json::Value, String> {
    let contents = std::fs::read_to_string(file_path)
        .map_err(|e| format!("could not read file '{}': {}", file_path, e))?;
    let input = contents.replace("{{hostname}}", hostname);
    serde_json::from_str(input.as_str())
        .map_err(|e| format!("could not parse file {} as json: {}", file_path, e))
}

fn google_logger(
    m: &ArgMatches,
) -> Result<Option<(slog_google::logger::Logger, slog_google::shipper::Shipper)>, String> {
    match (m.value_of(GLOG_LOGNAME), m.value_of(GLOG_RESOURCE_TYPE)) {

        (Some(logname), Some(resource_type)) => {
            let hostname = std::env::var("HOST")
                .or_else(|_| std::env::var("HOSTNAME"))
                .unwrap_or_default();

            let (labels_file, level_label) = (m.value_of(GLOG_LABELS_FILE), m.value_of(GLOG_LEVEL_LABEL));

            let mut builder = slog_google::logger::Builder::new(
                logname,
                resource_type,
            );

            if let Some(file) = labels_file {
                let data = load_labels_file(file, hostname.as_str()).map_err(|e| format!("error loading labels file: {}", e))?;
                let default_labels = data["default_labels"].clone();
                let resource_labels = data["resource_labels"].clone();

                builder = builder
                    .with_default_labels(default_labels).map_err(|e| format!("error using default labels: {}", e))?
                    .with_resource_labels(resource_labels).map_err(|e| format!("error using resource labels: {}", e))?;

            }

            if let Some(level_label) = level_label {
                builder = builder.with_log_level_label(level_label);
            }

            let (drain, shipper) = builder.build_with_async_shipper();

            Ok(Some((drain, shipper)))

        },
        (None, None) => Ok(None),
        _ => Err("To use the google logger please specify all required options (logname + resource type)".to_string()),
    }
}
