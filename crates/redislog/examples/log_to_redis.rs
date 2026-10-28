//! Push a Logstash V0 JSON log line onto a Redis list, then print what was stored.
//!
//! ```sh
//! docker run --rm -p 6379:6379 redis:7-alpine
//! cargo run -p slog-redis --example log_to_redis
//! ```

use slog::{info, o, Drain, Logger};
use slog_async::Async;
use slog_redis::Builder;

fn main() {
    let host = std::env::var("REDIS_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("REDIS_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(6379);
    let key = std::env::var("REDIS_KEY").unwrap_or_else(|_| "slog-redis-example".to_string());

    let drain = Builder::new("slog-redis-example")
        .redis_host(&host)
        .redis_port(port)
        .redis_key(&key)
        .source_host("example-host")
        .build()
        .unwrap_or_else(|e| {
            eprintln!("could not connect to redis://{host}:{port}: {e}");
            eprintln!("start Redis with: docker run --rm -p 6379:6379 redis:7-alpine");
            std::process::exit(1);
        });

    {
        let drain = Async::new(drain.fuse()).build().fuse();
        let log = Logger::root(drain, o!("example" => true));
        info!(log, "Hello from slog-redis"; "msg" => "example");
    }

    let client = redis::Client::open(format!("redis://{host}:{port}")).expect("redis URL");
    let mut con = client.get_connection().expect("redis connection");
    let entries: Vec<String> = redis::cmd("LRANGE")
        .arg(&key)
        .arg(-1)
        .arg(-1)
        .query(&mut con)
        .expect("LRANGE");

    println!("RPUSH'd onto '{key}':");
    for entry in entries {
        println!("{entry}");
    }
}
