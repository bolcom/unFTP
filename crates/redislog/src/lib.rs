//! This crate implements a [slog](https://crates.io/crates/slog) drain that outputs JSON formatted
//! logs to a Redis list
//!
//! Useful for structured, **centralized logging** using a RELK stack (Redis, Elasticsearch,
//! Logstash and Kibana). All log messages are sent to a Redis server, in **Logstash message V0 format**,
//! ready to be parsed/processed by Logstash.
//!
//! The format looks like this:
//!
//! ```json
//!  {
//!     "@timestamp": ${timeRFC3339},
//!     "@source_host": ${hostname},
//!     "@message": ${message},
//!     "@fields": {
//!        "level": ${levelLowercase},
//!        "application": ${appName}
//!        ... // logged field 1
//!        ... // logged field 2
//!        ...
//!    }
//! ```
//!
//! Example usage:
//!
//! ```no_run
//!  use slog::*;
//!  use slog_redis::Builder;
//!
//!  let redis_drain = Builder::new("my-app-name")
//!    .redis_host("localhost")
//!    .redis_key("my_redis_list_key")
//!    .build()
//!    .unwrap();
//!
//!  let drain = slog_async::Async::new(redis_drain.fuse()).build().fuse();
//!
//!  let log = Logger::root(drain, o!());
//!  info!(log, "Send me to {}!", "Redis"; "msg" => "Hello World!");
//! ```
//!

use std::cell::RefCell;
use std::fmt;
use std::process::Command;
use std::time::Duration;

use chrono::{DateTime, SecondsFormat, Utc};
use core::fmt::Write;
use r2d2_redis::RedisConnectionManager;
use serde_json::json;
use slog::Key;
use slog::{OwnedKVList, Record, KV};

/// A logger that sends JSON formatted logs to a list in a Redis instance. It uses this format
///
/// ```json
///   {
///     "@timestamp": ${timeRFC3339},
///     "@source_host": ${hostname},
///     "@message": ${message},
///     "@fields": {
///        "level": ${levelLowercase},
///        "application": ${appName}
///        ... // logged field 1
///        ... // logged field 2
///        ...
///    }
/// ```
///
/// It supports structured logging via [`slog`][slog-url]. You can use the [`Builder`] to
/// construct it and then use it as an slog drain.
///
/// [`Builder`]: struct.Builder.html
/// [slog-url]: https://github.com/slog-rs/slog
#[derive(Debug)]
pub struct Logger {
    redis_key: String,
    app_name: String,
    hostname: String,
    ttl_seconds: Option<u64>,
    pool: r2d2::Pool<RedisConnectionManager>,
}

/// Builds the Redis logger.
#[derive(Default, Debug)]
pub struct Builder {
    redis_host: String,
    redis_port: u32,
    redis_key: String,
    app_name: String,
    hostname: Option<String>,
    ttl_seconds: Option<u64>,
    connection_pool_size: u32,
}

/// Errors returned by the [`Builder`](crate::Builder) and the [`Logger`](crate::Logger)
#[derive(Debug)]
pub enum Error {
    ConnectionPoolErr(r2d2::Error),
    RedisErr(redis::RedisError),
    LogErr(slog::Error),
}

// A Key/Value pair used when constructing the JSON message.
type KeyVals = std::vec::Vec<(String, serde_json::Value)>;

// Serializes to KeyVals and implements slog::Serializer
struct Serializer {
    vec: KeyVals,
}

#[allow(dead_code)]
impl Builder {
    /// Creates the builder taking an application name that will end up in the `@fields.application`
    /// JSON field of the structured log message.
    pub fn new(app_name: &str) -> Builder {
        Builder {
            app_name: app_name.to_string(),
            redis_host: "localhost".to_string(),
            redis_port: 6379,
            connection_pool_size: 10,
            ..Default::default()
        }
    }

    /// Sets the redis details all at once.
    pub fn redis(self, host: String, port: u32, key: impl Into<String>) -> Builder {
        Builder {
            redis_host: host,
            redis_port: port,
            redis_key: key.into(),
            ..self
        }
    }

    /// Sets the name of the key for the list where log messages will be added.
    pub fn redis_key(self, key: impl Into<String>) -> Builder {
        Builder {
            redis_key: key.into(),
            ..self
        }
    }

    /// Sets the name of the redis host. Defaults to 'localhost'.
    pub fn redis_host(self, host: impl Into<String>) -> Builder {
        Builder {
            redis_host: host.into(),
            ..self
        }
    }

    /// Sets the name of the redis port. Defaults to 6379.
    pub fn redis_port(self, val: u32) -> Builder {
        Builder {
            redis_port: val,
            ..self
        }
    }

    /// Sets the time to live for messages in the redis list. Defaults to no timeout
    pub fn ttl(self, duration: Duration) -> Builder {
        Builder {
            ttl_seconds: Some(duration.as_secs()),
            ..self
        }
    }

    /// Sets the name noted down in logs indicating the source of the log entry i.e. the
    /// `@source_host` field in the JSON payload
    pub fn source_host(self, host: impl Into<String>) -> Builder {
        Builder {
            hostname: Some(host.into()),
            ..self
        }
    }

    /// Consumes the builder, returning the redis logger
    pub fn build(self) -> Result<Logger, Error> {
        // TODO: Get something that works on windows too
        fn get_host_name() -> String {
            let output = Command::new("hostname")
                .output()
                .expect("failed to execute process");
            String::from_utf8_lossy(&output.stdout).replace('\n', "")
        }

        let connection_str = format!("redis://{}:{}", self.redis_host, self.redis_port);
        let manager = RedisConnectionManager::new(connection_str.as_str())?;
        let pool = r2d2::Pool::builder()
            .max_size(self.connection_pool_size)
            .connection_timeout(Duration::new(1, 0))
            .build(manager)?;

        let mut con = pool.get()?;
        redis::cmd("PING").query(&mut *con)?;

        Ok(Logger {
            redis_key: self.redis_key,
            app_name: self.app_name,
            hostname: self.hostname.unwrap_or_else(get_host_name),
            ttl_seconds: self.ttl_seconds,
            pool,
        })
    }
}

impl Logger {
    fn v0_msg(&self, level: &str, msg: &str, key_vals: Option<KeyVals>) -> String {
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

        let fields = match json_val {
            serde_json::Value::Object(ref mut v) => match v.get_mut("@fields").unwrap() {
                serde_json::Value::Object(ref mut v) => Some(v),
                _ => None,
            },
            _ => None,
        }
        .unwrap();

        for key_val in &key_vals.unwrap() {
            fields.insert(key_val.0.clone(), key_val.1.clone());
        }

        json_val.to_string()
    }

    /// Sends a message constructed by v0_msg to the redis server
    fn send_to_redis(&self, msg: &str) -> Result<(), Error> {
        let mut con = self.pool.get()?;

        redis::cmd("RPUSH")
            .arg(self.redis_key.as_str())
            .arg(msg)
            .query(&mut *con)?;

        if let Some(t) = self.ttl_seconds {
            redis::cmd("EXPIRE")
                .arg(self.redis_key.as_str())
                .arg(t)
                .query(&mut *con)?
        }
        Ok(())
    }
}

impl slog::Drain for Logger {
    type Ok = ();
    type Err = self::Error;

    fn log(&self, record: &Record, values: &OwnedKVList) -> Result<Self::Ok, Self::Err> {
        let ser = &mut Serializer::new();
        record.kv().serialize(record, ser)?;
        values.serialize(record, ser)?;

        let level_str = record.level().as_str();
        let msg = format!("{}", record.msg());
        let log_entry = self.v0_msg(level_str, msg.as_str(), Some(ser.done()));
        self.send_to_redis(&log_entry)?;
        Ok(())
    }
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

impl From<slog::Error> for Error {
    fn from(error: slog::Error) -> Self {
        Error::LogErr(error)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::ConnectionPoolErr(e) => write!(f, "Redis logger connection pool error: {}", e),
            Error::RedisErr(e) => write!(f, "Redis logger Redis error: {}", e),
            Error::LogErr(e) => write!(f, "Redis logger slog error: {}", e),
        }
    }
}

impl Serializer {
    pub fn new() -> Serializer {
        Serializer { vec: Vec::new() }
    }

    pub fn emit_val(&mut self, key: slog::Key, val: serde_json::Value) -> slog::Result {
        self.vec.push((key.to_string(), val));
        Ok(())
    }

    fn emit_serde_json_number<V>(&mut self, key: Key, value: V) -> slog::Result
    where
        serde_json::Number: From<V>,
    {
        // convert a given number into serde_json::Number
        let num = serde_json::Number::from(value);
        self.emit_val(key, serde_json::Value::Number(num))
    }

    fn done(&mut self) -> KeyVals {
        self.vec.clone()
    }
}

// used by Serializer
thread_local! {
    static THREAD_LOCAL_BUF: RefCell<String> = RefCell::new(String::with_capacity(256))
}

#[allow(dead_code)]
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
        let n = serde_json::Number::from_f64(val);
        self.emit_val(key, serde_json::Value::Number(n.unwrap()))
    }

    fn emit_arguments(&mut self, key: Key, val: &fmt::Arguments) -> slog::Result {
        THREAD_LOCAL_BUF.with(|buf| {
            let mut buf = buf.borrow_mut();
            buf.write_fmt(*val).unwrap();
            let res = self.emit_val(key, serde_json::Value::String(buf.clone()));
            buf.clear();
            res
        })
    }
}
