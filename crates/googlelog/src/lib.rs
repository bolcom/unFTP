//! An implemention of [`slog::Drain`](https://slog-rs.github.io/slog/slog/trait.Drain.html) for logging to [Google Cloud](https://cloud.google.com/logging).
//!
//! # Usage
//!
//! Warning: Currently, this library only works in the context of [workload identity](https://cloud.google.com/iam/docs/workload-identity-federation).
//!
//! The `googlelog` drain creates log entries compatible with [Google Cloud Logging](https://cloud.google.com/logging).
//! Depending on how you want to ship these logs to the Google Logging API, you can choose from one of the available build methods.
//!
//! Start by configuring the Logger with the builder ([`Builder`](logger::Builder::new)) and selecting the appropriate build method based on your shipping requirements:
//!
//! 1. [`build()`](logger::Builder::build): Receives [`WriteLogEntries`](https://cloud.google.com/logging/docs/reference/v2/rpc/google.logging.v2#google.logging.v2.LoggingServiceV2.WriteLogEntries) over a channel and allows you to handle the transportation manually.
//! 2. [`build_with_async_shipper()`](logger::Builder::build_with_async_shipper): Offloads transportation to the [`Shipper`](shipper::Shipper) and its sync-async Bridge in an async context. (Requires the `shipper` feature.)

//!
//! The [`builder`](struct@logger::Builder) supports several `with_*` methods to customize the log message format,
//! particularly the default labels attached to [log entries](https://cloud.google.com/logging/docs/reference/v2/rest/v2/LogEntry).
//!
/// Googlelog Error types
pub mod error;

/// The [`slog::Drain`](https://slog-rs.github.io/slog/slog/trait.Drain.html) Implementation of the slog Drain for [Google Cloud Logging](https://cloud.google.com/logging)
pub mod logger;

/// An optional async process to ship the log for you
#[cfg(feature = "shipper")]
pub mod shipper;
