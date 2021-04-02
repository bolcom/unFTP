# slog-redis

This crate implements a [slog](https://crates.io/crates/slog) drain that outputs to a [Redis](https://redis.io/) list.

Useful for **centralized logging** using a RELK stack (Redis, Elasticsearch, Logstash and Kibana). All log messages are 
sent to a Redis server, in **Logstash message V0 format**, ready to be parsed/processed by Logstash.

The format looks like this:

 ```json
  {
     "@timestamp": ${timeRFC3339},
     "@source_host": ${hostname},
     "@message": ${message},
     "@fields": {
        "level": ${levelLowercase},
        "application": ${appName}
        ... // logged field 1
        ... // logged field 2
        ...
    }
```

Example Usage:

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
