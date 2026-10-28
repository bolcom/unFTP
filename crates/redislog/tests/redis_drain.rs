//! Integration tests against a running Redis instance.
//!
//! They skip when Redis is not reachable, unless `CI` is set, in which case
//! missing Redis is a failure. Start one locally with:
//!
//! ```sh
//! docker run --rm -p 6379:6379 redis:7-alpine
//! ```

use std::time::{SystemTime, UNIX_EPOCH};

use slog::{info, o, Drain, Logger};
use slog_async::Async;
use slog_redis::Builder;

fn redis_host() -> String {
    std::env::var("REDIS_HOST").unwrap_or_else(|_| "127.0.0.1".to_string())
}

fn redis_port() -> u32 {
    std::env::var("REDIS_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(6379)
}

fn unique_key(label: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    format!("slog-redis-it-{}-{}-{}", std::process::id(), nanos, label)
}

fn open_client() -> redis::Client {
    let url = format!("redis://{}:{}", redis_host(), redis_port());
    redis::Client::open(url).expect("valid redis URL")
}

fn require_redis() -> Option<()> {
    match open_client().get_connection() {
        Ok(mut con) => redis::cmd("PING")
            .query::<String>(&mut con)
            .ok()
            .map(|_| ()),
        Err(_) => None,
    }
}

fn skip_or_fail_without_redis() -> bool {
    if require_redis().is_some() {
        return false;
    }
    if std::env::var_os("CI").is_some() {
        panic!(
            "Redis is required in CI but was not reachable at {}:{}",
            redis_host(),
            redis_port()
        );
    }
    eprintln!(
        "skipping slog-redis integration test; start Redis with:\n  docker run --rm -p 6379:6379 redis:7-alpine"
    );
    true
}

fn build_logger(key: &str) -> slog_redis::Logger {
    Builder::new("slog-redis-test")
        .redis_host(redis_host())
        .redis_port(redis_port())
        .redis_key(key)
        .source_host("test-host")
        .build()
        .unwrap_or_else(|e| panic!("failed to build redis logger: {e}"))
}

fn lrange(key: &str) -> Vec<String> {
    let mut con = open_client().get_connection().expect("redis connection");
    redis::cmd("LRANGE")
        .arg(key)
        .arg(0)
        .arg(-1)
        .query(&mut con)
        .expect("LRANGE")
}

fn del(key: &str) {
    let mut con = open_client().get_connection().expect("redis connection");
    let _: () = redis::cmd("DEL").arg(key).query(&mut con).expect("DEL");
}

#[test]
fn rpush_logstash_v0_entry() {
    if skip_or_fail_without_redis() {
        return;
    }

    let key = unique_key("rpush");
    {
        let drain = Async::new(build_logger(&key).fuse()).build().fuse();
        let log = Logger::root(drain, o!("build" => "test"));
        info!(log, "Send me to Redis"; "msg" => "Hello World!", "count" => 7_i64);
    }

    let entries = lrange(&key);
    del(&key);

    assert_eq!(entries.len(), 1, "expected one RPUSH'd log line");

    let v: serde_json::Value = serde_json::from_str(&entries[0]).expect("json");
    assert_eq!(v["@source_host"], "test-host");
    assert_eq!(v["@message"], "send me to redis");
    assert_eq!(v["@fields"]["application"], "slog-redis-test");
    assert_eq!(v["@fields"]["level"], "INFO");
    assert_eq!(v["@fields"]["msg"], "Hello World!");
    assert_eq!(v["@fields"]["count"], 7);
    assert_eq!(v["@fields"]["build"], "test");
    assert!(v["@timestamp"].as_str().is_some());
}

#[test]
fn expire_sets_ttl_on_the_list() {
    if skip_or_fail_without_redis() {
        return;
    }

    let key = unique_key("ttl");
    let drain = Builder::new("slog-redis-test")
        .redis_host(redis_host())
        .redis_port(redis_port())
        .redis_key(&key)
        .ttl(std::time::Duration::from_secs(60))
        .build()
        .unwrap_or_else(|e| panic!("failed to build redis logger: {e}"));
    {
        let drain = Async::new(drain.fuse()).build().fuse();
        let log = Logger::root(drain, o!());
        info!(log, "ttl check");
    }

    let mut con = open_client().get_connection().expect("redis connection");
    let ttl: i64 = redis::cmd("TTL").arg(&key).query(&mut con).expect("TTL");
    del(&key);

    assert!(
        (1..=60).contains(&ttl),
        "expected a TTL between 1 and 60 seconds, got {ttl}"
    );
}
