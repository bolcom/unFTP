//! A [slog](https://crates.io/crates/slog) drain that writes JSON log messages to a Redis list.
//!
//! Useful for structured, centralized logging with a RELK stack (Redis, Elasticsearch,
//! Logstash, and Kibana). Messages are stored in **Logstash message V0 format**, ready to be
//! processed by Logstash.
//!
//! The payload looks like this:
//!
//! ```json
//! {
//!    "@timestamp": ${timeRFC3339},
//!    "@source_host": ${hostname},
//!    "@message": ${message},
//!    "@fields": {
//!       "level": ${levelLowercase},
//!       "application": ${appName}
//!    }
//! }
//! ```
//!
//! # Example
//!
//! ```no_run
//! use slog::*;
//! use slog_redis::Builder;
//!
//! let redis_drain = Builder::new("my-app-name")
//!     .redis_host("localhost")
//!     .redis_key("my_redis_list_key")
//!     .build()
//!     .unwrap();
//!
//! let drain = slog_async::Async::new(redis_drain.fuse()).build().fuse();
//!
//! let log = Logger::root(drain, o!());
//! info!(log, "Send me to {}!", "Redis"; "msg" => "Hello World!");
//! ```

use std::cell::RefCell;
use std::fmt;
use std::process::Command;
use std::time::Duration;

use chrono::{DateTime, SecondsFormat, Utc};
use core::fmt::Write;
use serde_json::json;
use slog::Key;
use slog::{OwnedKVList, Record, KV};

/// A logger that sends JSON formatted logs to a list in a Redis instance. It uses this format
///
/// ```json
/// {
///    "@timestamp": ${timeRFC3339},
///    "@source_host": ${hostname},
///    "@message": ${message},
///    "@fields": {
///       "level": ${levelLowercase},
///       "application": ${appName}
///    }
/// }
/// ```
///
/// It supports structured logging via [`slog`](https://github.com/slog-rs/slog). You can use the
/// [`Builder`] to construct it and then use it as an slog drain.
#[derive(Debug)]
pub struct Logger {
    redis_key: String,
    app_name: String,
    hostname: String,
    ttl_seconds: Option<u64>,
    pool: r2d2::Pool<redis::Client>,
}

/// Builds the Redis logger.
#[derive(Debug)]
pub struct Builder {
    redis_host: String,
    redis_port: u32,
    redis_key: String,
    app_name: String,
    hostname: Option<String>,
    ttl_seconds: Option<u64>,
    connection_pool_size: u32,
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            redis_host: "localhost".to_string(),
            redis_port: 6379,
            redis_key: String::new(),
            app_name: String::new(),
            hostname: None,
            ttl_seconds: None,
            connection_pool_size: 10,
        }
    }
}

/// Errors returned by the [`Builder`] and the [`Logger`]
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// The Redis connection pool could not be created or a connection could not be obtained.
    #[error("Redis logger connection pool error: {0}")]
    ConnectionPoolErr(#[from] r2d2::Error),
    /// A Redis command failed.
    #[error("Redis logger Redis error: {0}")]
    RedisErr(#[from] redis::RedisError),
    /// Serializing slog key/value pairs failed.
    #[error("Redis logger slog error: {0}")]
    LogErr(#[from] slog::Error),
}

// A Key/Value pair used when constructing the JSON message.
type KeyVals = Vec<(String, serde_json::Value)>;

// Serializes to KeyVals and implements slog::Serializer
struct Serializer {
    vec: KeyVals,
}

impl Builder {
    /// Creates the builder taking an application name that will end up in the `@fields.application`
    /// JSON field of the structured log message.
    #[must_use]
    pub fn new(app_name: &str) -> Builder {
        Builder {
            app_name: app_name.to_string(),
            ..Default::default()
        }
    }

    /// Sets the redis details all at once.
    #[must_use]
    pub fn redis(self, host: String, port: u32, key: impl Into<String>) -> Builder {
        Builder {
            redis_host: host,
            redis_port: port,
            redis_key: key.into(),
            ..self
        }
    }

    /// Sets the name of the key for the list where log messages will be added.
    #[must_use]
    pub fn redis_key(self, key: impl Into<String>) -> Builder {
        Builder {
            redis_key: key.into(),
            ..self
        }
    }

    /// Sets the name of the redis host. Defaults to `localhost`.
    #[must_use]
    pub fn redis_host(self, host: impl Into<String>) -> Builder {
        Builder {
            redis_host: host.into(),
            ..self
        }
    }

    /// Sets the redis port. Defaults to 6379.
    #[must_use]
    pub fn redis_port(self, val: u32) -> Builder {
        Builder {
            redis_port: val,
            ..self
        }
    }

    /// Sets the time to live for messages in the redis list. Defaults to no timeout
    #[must_use]
    pub fn ttl(self, duration: Duration) -> Builder {
        Builder {
            ttl_seconds: Some(duration.as_secs()),
            ..self
        }
    }

    /// Sets the name noted down in logs indicating the source of the log entry i.e. the
    /// `@source_host` field in the JSON payload
    #[must_use]
    pub fn source_host(self, host: impl Into<String>) -> Builder {
        Builder {
            hostname: Some(host.into()),
            ..self
        }
    }

    /// Sets the r2d2 connection pool size. Defaults to 10.
    #[must_use]
    pub fn connection_pool_size(self, size: u32) -> Builder {
        Builder {
            connection_pool_size: size,
            ..self
        }
    }

    /// Consumes the builder, returning the redis logger
    pub fn build(self) -> Result<Logger, Error> {
        let connection_str = format!("redis://{}:{}", self.redis_host, self.redis_port);
        let client = redis::Client::open(connection_str.as_str())?;
        let pool = r2d2::Pool::builder()
            .max_size(self.connection_pool_size)
            .connection_timeout(Duration::from_secs(1))
            .build(client)?;

        let mut con = pool.get()?;
        redis::cmd("PING").query::<()>(&mut *con)?;

        Ok(Logger {
            redis_key: self.redis_key,
            app_name: self.app_name,
            hostname: self.hostname.unwrap_or_else(default_hostname),
            ttl_seconds: self.ttl_seconds,
            pool,
        })
    }
}

fn default_hostname() -> String {
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("HOST"))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            Command::new("hostname")
                .output()
                .ok()
                .and_then(|output| String::from_utf8(output.stdout).ok())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        })
        .unwrap_or_else(|| "unknown".to_string())
}

impl Logger {
    fn v0_msg(&self, level: &str, msg: &str, key_vals: KeyVals) -> String {
        let now: DateTime<Utc> = Utc::now();
        let time = now.to_rfc3339_opts(SecondsFormat::AutoSi, true);
        let mut json_val = json!({
            "@timestamp": time,
            "@source_host": &self.hostname,
            "@message": msg.to_lowercase(),
            "@fields": {
                "level": level,
                "application": &self.app_name
            }
        });

        if let Some(fields) = json_val.get_mut("@fields").and_then(|v| v.as_object_mut()) {
            for (key, val) in key_vals {
                fields.insert(key, val);
            }
        }

        json_val.to_string()
    }

    /// Sends a message constructed by v0_msg to the redis server
    fn send_to_redis(&self, msg: &str) -> Result<(), Error> {
        let mut con = self.pool.get()?;

        redis::cmd("RPUSH")
            .arg(self.redis_key.as_str())
            .arg(msg)
            .query::<()>(&mut *con)?;

        if let Some(t) = self.ttl_seconds {
            redis::cmd("EXPIRE")
                .arg(self.redis_key.as_str())
                .arg(t)
                .query::<()>(&mut *con)?;
        }
        Ok(())
    }
}

impl slog::Drain for Logger {
    type Ok = ();
    type Err = Error;

    fn log(&self, record: &Record, values: &OwnedKVList) -> Result<Self::Ok, Self::Err> {
        let mut ser = Serializer::new();
        record.kv().serialize(record, &mut ser)?;
        values.serialize(record, &mut ser)?;

        let level_str = record.level().as_str();
        let msg = format!("{}", record.msg());
        let log_entry = self.v0_msg(level_str, msg.as_str(), ser.done());
        self.send_to_redis(&log_entry)?;
        Ok(())
    }
}

impl Serializer {
    fn new() -> Serializer {
        Serializer { vec: Vec::new() }
    }

    fn emit_val(&mut self, key: slog::Key, val: serde_json::Value) -> slog::Result {
        self.vec.push((key.to_string(), val));
        Ok(())
    }

    fn emit_serde_json_number<V>(&mut self, key: Key, value: V) -> slog::Result
    where
        serde_json::Number: From<V>,
    {
        let num = serde_json::Number::from(value);
        self.emit_val(key, serde_json::Value::Number(num))
    }

    fn done(self) -> KeyVals {
        self.vec
    }
}

// used by Serializer
thread_local! {
    static THREAD_LOCAL_BUF: RefCell<String> = RefCell::new(String::with_capacity(256));
}

impl slog::Serializer for Serializer {
    fn emit_bool(&mut self, key: Key, val: bool) -> slog::Result {
        self.emit_val(key, serde_json::Value::Bool(val))
    }

    fn emit_unit(&mut self, key: Key) -> slog::Result {
        self.emit_val(key, serde_json::Value::Null)
    }

    fn emit_str(&mut self, key: Key, val: &str) -> slog::Result {
        self.emit_val(key, serde_json::Value::String(val.to_string()))
    }

    fn emit_char(&mut self, key: Key, val: char) -> slog::Result {
        self.emit_val(key, serde_json::Value::String(val.to_string()))
    }

    fn emit_none(&mut self, key: Key) -> slog::Result {
        self.emit_val(key, serde_json::Value::Null)
    }

    fn emit_u8(&mut self, key: Key, val: u8) -> slog::Result {
        self.emit_serde_json_number::<u8>(key, val)
    }

    fn emit_i8(&mut self, key: Key, val: i8) -> slog::Result {
        self.emit_serde_json_number::<i8>(key, val)
    }

    fn emit_u16(&mut self, key: Key, val: u16) -> slog::Result {
        self.emit_serde_json_number::<u16>(key, val)
    }

    fn emit_i16(&mut self, key: Key, val: i16) -> slog::Result {
        self.emit_serde_json_number::<i16>(key, val)
    }

    fn emit_usize(&mut self, key: Key, val: usize) -> slog::Result {
        self.emit_serde_json_number::<usize>(key, val)
    }

    fn emit_isize(&mut self, key: Key, val: isize) -> slog::Result {
        self.emit_serde_json_number::<isize>(key, val)
    }

    fn emit_u32(&mut self, key: Key, val: u32) -> slog::Result {
        self.emit_serde_json_number::<u32>(key, val)
    }

    fn emit_i32(&mut self, key: Key, val: i32) -> slog::Result {
        self.emit_serde_json_number::<i32>(key, val)
    }

    fn emit_f32(&mut self, key: Key, val: f32) -> slog::Result {
        self.emit_f64(key, f64::from(val))
    }

    fn emit_u64(&mut self, key: Key, val: u64) -> slog::Result {
        self.emit_serde_json_number::<u64>(key, val)
    }

    fn emit_i64(&mut self, key: Key, val: i64) -> slog::Result {
        self.emit_serde_json_number::<i64>(key, val)
    }

    fn emit_f64(&mut self, key: Key, val: f64) -> slog::Result {
        match serde_json::Number::from_f64(val) {
            Some(n) => self.emit_val(key, serde_json::Value::Number(n)),
            None => self.emit_val(key, serde_json::Value::Null),
        }
    }

    fn emit_arguments(&mut self, key: Key, val: &fmt::Arguments) -> slog::Result {
        THREAD_LOCAL_BUF.with(|buf| {
            let mut buf = buf.borrow_mut();
            buf.write_fmt(*val)?;
            let res = self.emit_val(key, serde_json::Value::String(buf.clone()));
            buf.clear();
            res
        })
    }
}
