# slog-redis

A [slog](https://crates.io/crates/slog) drain that writes JSON log messages to a [Redis](https://redis.io/) list.

Useful for **centralized logging** with a RELK stack (Redis, Elasticsearch, Logstash, and Kibana). Messages are stored in **Logstash message V0 format**, ready to be processed by Logstash.

The payload looks like this:

```json
{
   "@timestamp": ${timeRFC3339},
   "@source_host": ${hostname},
   "@message": ${message},
   "@fields": {
      "level": ${levelLowercase},
      "application": ${appName}
   }
}
```

```toml
[dependencies]
slog-redis = "0.2.0"
```

```rust
use slog::*;
use slog_redis::Builder;

let redis_drain = Builder::new("my-app-name")
    .redis_host("localhost")
    .redis_key("my_redis_list_key")
    .build()
    .unwrap();

let drain = slog_async::Async::new(redis_drain.fuse()).build().fuse();

let log = Logger::root(drain, o!());
info!(log, "Send me to {}!", "Redis"; "msg" => "Hello World!");
```

## Example and tests

Against a local Redis:

```sh
docker run --rm -p 6379:6379 redis:7-alpine
cargo run -p slog-redis --example log_to_redis
cargo test -p slog-redis
```

The integration tests skip if Redis is not running, except in CI where Redis is required.
