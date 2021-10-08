use async_trait::async_trait;
use serde::__private::fmt::Debug;
use serde::{Deserialize, Serialize};

// EventDispatcher can send events to the outside world.
#[async_trait]
pub trait EventDispatcher<T>: Send + Sync + Debug {
    async fn dispatch(&self, event: T);
}

// An EventDispatcher that dispatches to the void of nothingness.
#[derive(Debug)]
pub struct NullEventDispatcher {}

#[async_trait]
impl EventDispatcher<FTPEvent> for NullEventDispatcher {
    async fn dispatch(&self, _event: FTPEvent) {
        // Do Nothing
    }
}

#[derive(Serialize, Deserialize)]
pub struct FTPEvent {
    pub source_instance: String,
    pub hostname: String,
    pub payload: FTPEventPayload,
}

#[derive(Serialize, Deserialize)]
pub enum FTPEventPayload {
    Startup {
        libunftp_version: String,
        unftp_version: String,
    },
    Login {
        username: String,
    },
    List {
        path: String,
    },
    Get {
        path: String,
    },
    Put {
        path: String,
        md5: String,
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
    RmdDir {
        path: String,
    },
}
