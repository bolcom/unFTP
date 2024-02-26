//! Infra contains infrastructure specific implementations of things in the [`domain`](crate::domain)
//! module.
mod pubsub;
mod workload_identity;

pub mod usrdetail_json;

pub use pubsub::PubsubEventDispatcher;
