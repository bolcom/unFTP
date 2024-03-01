//! Infra contains infrastructure specific implementations of things in the [`domain`](crate::domain)
//! module.
mod pubsub;
pub mod userdetail_http;
pub mod usrdetail_json;
mod workload_identity;

pub use pubsub::PubsubEventDispatcher;
