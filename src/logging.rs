use crate::app;
use crate::args;

use clap::ArgMatches;
use slog::*;
use std::result::Result;

pub fn create_logger(arg_matches: &ArgMatches) -> Result<slog::Logger, String> {
    let min_log_level = match arg_matches.occurrences_of(args::VERBOSITY) {
        0 => slog::Level::Warning,
        1 => slog::Level::Info,
        2 => slog::Level::Debug,
        _ => slog::Level::Trace,
    };

    let min_log_level = match arg_matches.value_of(args::LOG_LEVEL) {
        Some(level) => match level.parse::<args::LogLevelType>()? {
            args::LogLevelType::error => slog::Level::Error,
            args::LogLevelType::warn => slog::Level::Warning,
            args::LogLevelType::info => slog::Level::Info,
            args::LogLevelType::debug => slog::Level::Debug,
            args::LogLevelType::trace => slog::Level::Trace,
        },
        None => min_log_level,
    };

    let decorator = slog_term::TermDecorator::new().force_color().build();
    let term_drain = slog_term::FullFormat::new(decorator)
        .build()
        .filter_level(min_log_level)
        .fuse();

    let mut err: Option<String> = None;
    let drain = match redis_logger(&arg_matches) {
        Ok(Some(redis_logger)) => {
            let both = slog::Duplicate::new(redis_logger, term_drain).fuse();
            slog_async::Async::new(both.filter_level(min_log_level).fuse())
                .build()
                .fuse()
        }
        Ok(None) => slog_async::Async::new(term_drain).build().fuse(),
        Err(e) => {
            err = e.into();
            slog_async::Async::new(term_drain).build().fuse()
        }
    };
    let root = Logger::root(drain, o!());
    let log = root.new(o!());
    if let Some(err_str) = err {
        error!(log, "Continuing only with terminal logger: {}", err_str)
    }
    Ok(log)
}

fn redis_logger(m: &clap::ArgMatches) -> Result<Option<redislog::Logger>, String> {
    match (
        m.value_of(args::REDIS_KEY),
        m.value_of(args::REDIS_HOST),
        m.value_of(args::REDIS_PORT),
    ) {
        (Some(key), Some(host), Some(port)) => {
            let instance_name = m.value_of(args::INSTANCE_NAME).unwrap();
            let app_name = if instance_name == app::NAME {
                String::from(app::NAME)
            } else {
                format!("{}-{}", app::NAME, instance_name)
            };
            let logger = redislog::Builder::new(&*app_name)
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
