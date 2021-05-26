use crate::app;
use crate::args;

use app::NAME;
use args::{INSTANCE_NAME, LOG_LEVEL, REDIS_HOST, REDIS_KEY, REDIS_PORT, VERBOSITY};
use clap::ArgMatches;
use redislog::Builder;
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

pub fn create_logger(arg_matches: &ArgMatches) -> Result<slog::Logger, String> {
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
    let drain = match redis_logger(&arg_matches) {
        Ok(Some(redis_logger)) => {
            let both = Duplicate::new(redis_logger, term_drain).fuse();
            Async::new(both.filter_level(min_log_level).fuse()).build().fuse()
        }
        Ok(None) => Async::new(term_drain).build().fuse(),
        Err(e) => {
            err = e.into();
            Async::new(term_drain).build().fuse()
        }
    };
    let root = Logger::root(drain, o!());
    let log = root.new(o!());
    if let Some(err_str) = err {
        error!(log, "Continuing only with terminal logger: {}", err_str)
    }
    Ok(log)
}

fn redis_logger(m: &ArgMatches) -> Result<Option<FallbackToStderr<redislog::Logger>>, String> {
    match (m.value_of(REDIS_KEY), m.value_of(REDIS_HOST), m.value_of(REDIS_PORT)) {
        (Some(key), Some(host), Some(port)) => {
            let instance_name = m.value_of(INSTANCE_NAME).unwrap();
            let app_name = if instance_name == NAME {
                String::from(NAME)
            } else {
                format!("{}-{}", NAME, instance_name)
            };
            let logger = Builder::new(&*app_name)
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
