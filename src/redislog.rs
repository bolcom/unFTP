extern crate chrono;
extern crate r2d2;
extern crate r2d2_redis;
extern crate redis;

use chrono::{DateTime, SecondsFormat, Utc};
use r2d2_redis::RedisConnectionManager;
use serde_json::json;
use std::fmt;
use std::process::Command;
use std::time::Duration;

/// A logger that sends JSON formatted logs to a key in a Redis instance.
///
/// This struct implements the `Log` trait from the [`log` crate][log-crate-url],
/// which allows it to act as a logger.
///
/// You can use the [`Builder`] to construct it and then install it with
/// the [`log` crate][log-crate-url] directly.
///
/// [log-crate-url]: https://docs.rs/log/
/// [`Builder`]: struct.Builder.html
#[derive(Debug)]
pub struct Logger {
    config: LoggerConfig,
    pool: r2d2::Pool<RedisConnectionManager>,
}

#[derive(Default, Debug)]
pub struct Builder {
    redis_host: String,
    redis_port: u32,
    redis_key: String,
    app_name: String,
    hostname: Option<String>,
    ttl_seconds: Option<u64>,
}

#[derive(Debug)]
pub enum Error {
    ConnectionPoolErr(r2d2::Error),
    RedisErr(redis::RedisError),
}

#[derive(Debug)]
struct LoggerConfig {
    pub redis_host: String,
    pub redis_port: u32,
    pub redis_key: String,
    pub app_name: String,
    pub hostname: String,
    pub ttl_seconds: Option<u64>,
}

#[allow(dead_code)]
impl Builder {
    pub fn new(app_name: &str) -> Builder {
        Builder {
            app_name: app_name.to_string(),
            redis_host: "localhost".to_string(),
            redis_port: 6379,
            ..Default::default()
        }
    }

    pub fn redis(self, host: String, port: u32, key: String) -> Builder {
        Builder {
            redis_host: host,
            redis_port: port,
            redis_key: key,
            ..self
        }
    }

    pub fn redis_key(self, key: &str) -> Builder {
        Builder {
            redis_key: key.to_string(),
            ..self
        }
    }

    pub fn redis_host(self, host: &str) -> Builder {
        Builder {
            redis_host: host.to_string(),
            ..self
        }
    }

    pub fn redis_port(self, val: u32) -> Builder {
        Builder {
            redis_port: val,
            ..self
        }
    }

    pub fn ttl(self, duration: Duration) -> Builder {
        Builder {
            ttl_seconds: Some(duration.as_secs()),
            ..self
        }
    }

    /// Builds the redis logger
    ///
    /// The returned logger implements the `Log` trait and can be installed manually
    /// or nested within another logger.
    pub fn build(self) -> Result<Logger, Error> {
        // TODO: Get something that works on windows too
        fn get_host_name() -> String {
            let output = Command::new("hostname").output().expect("failed to execute process");
            String::from_utf8_lossy(&output.stdout).replace("\n", "").to_string()
        }

        let con_str = format!("redis://{}:{}", self.redis_host, self.redis_port);
        let manager = RedisConnectionManager::new(con_str.as_str())?;
        let pool = r2d2::Pool::builder()
            .connection_timeout(Duration::new(1, 0))
            .build(manager)?;

        let con = pool.get()?;
        let _: () = redis::cmd("PING").query(&*con)?;

        Ok(Logger {
            config: LoggerConfig {
                redis_host: self.redis_host,
                redis_port: self.redis_port,
                redis_key: self.redis_key,
                app_name: self.app_name,
                hostname: self.hostname.unwrap_or_else(get_host_name),
                ttl_seconds: self.ttl_seconds,
            },
            pool,
        })
    }
}

impl Logger {
    fn v0_msg(&self, record: &log::Record) -> String {
        let now: DateTime<Utc> = Utc::now();
        let time = now.to_rfc3339_opts(SecondsFormat::AutoSi, true);
        let level = record.level().to_string();
        let application = self.config.app_name.clone();
        let json_val = json!({
            "@timestamp": time,
            "@source_host": self.config.hostname.clone(),
            "@message": format!("{}", record.args()),
            "@fields": {
                "level": level,
                "application": application
            }
        });
        json_val.to_string()
    }

    fn send_to_redis(&self, record: &log::Record) -> Result<(), Error> {
        let con = self.pool.get()?;

        redis::cmd("RPUSH")
            .arg(self.config.redis_key.clone())
            .arg(self.v0_msg(record))
            .query(&*con)?;

        if let Some(t) = self.config.ttl_seconds {
            redis::cmd("EXPIRE")
                .arg(self.config.redis_key.clone())
                .arg(t)
                .query(&*con)?
        }
        Ok(())
    }
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Info
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let msg = self.v0_msg(record);
            let mut prefix = String::from("");
            if let Err(e) = self.send_to_redis(record) {
                prefix = format!("fallback logger: [{}]", e);
            }
            println!("{}{}", prefix, msg);
        }
    }

    fn flush(&self) {}
}

impl From<r2d2::Error> for Error {
    fn from(error: r2d2::Error) -> Self {
        Error::ConnectionPoolErr(error)
    }
}

impl From<redis::RedisError> for Error {
    fn from(error: redis::RedisError) -> Self {
        Error::RedisErr(error)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::ConnectionPoolErr(_e) => write!(f, "Redis logger error!"),
            Error::RedisErr(_e) => write!(f, "Redis logger error!"),
        }
    }
}
