# Changelog

## 0.2.0

- Upgrade the Redis client from 0.20 to 1.8 and drop `r2d2_redis` in favor of
  the client's built-in r2d2 support. This crate no longer pulls in a Redis
  client that future Rust versions will reject.
- Move to the 2021 edition and let applications choose slog's compile-time log
  level filters.
- Stop panicking when the hostname cannot be determined, and add
  `Builder::connection_pool_size`.
- Add an example and Redis integration tests (skip locally when Redis is
  not running; CI starts Redis in Docker).

## 0.1.2

- Previous release.
