//! Event definitions
//!
//! Packages elsewhere e.g. in the [`infra`](crate::infra) module implements these traits defined here.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

// EventDispatcher can send events to the outside world.
#[async_trait]
pub trait EventDispatcher<T>: Send + Sync + Debug {
    async fn dispatch(&self, event: T);
}

#[async_trait]
impl EventDispatcher<FTPEvent> for NullEventDispatcher {
    async fn dispatch(&self, _event: FTPEvent) {
        // Do Nothing
    }
}

// An EventDispatcher that dispatches to the void of nothingness.
#[derive(Debug)]
pub struct NullEventDispatcher {}

// The event that will be sent
#[derive(Serialize, Deserialize, Debug)]
pub struct FTPEvent {
    pub source_instance: String,
    pub hostname: String,
    pub payload: FTPEventPayload,
    /// The user this event pertains to. A user may have more than one connection or session.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    /// Identifies a single session pertaining to a connected client.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    /// The event sequence number as incremented per session.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sequence_number: Option<u64>,
}

// The event variant
#[derive(Serialize, Deserialize, Debug)]
pub enum FTPEventPayload {
    Startup {
        libunftp_version: String,
        unftp_version: String,
    },
    Login {},
    Logout {},
    Get {
        path: String,
    },
    Put {
        path: String,
    },
    Delete {
        path: String,
    },
    MakeDir {
        path: String,
    },
    Rename {
        from: String,
        to: String,
    },
    RemoveDir {
        path: String,
    },
}
