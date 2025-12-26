//! Infra contains infrastructure specific implementations of things in the [`domain`](crate::domain)
//! module.
mod pubsub;
pub mod userdetail_default;
pub mod userdetail_http;
pub mod userdetail_json;
mod workload_identity;

pub use pubsub::PubsubEventDispatcher;
