use crate::{
    args,
    domain::{EventDispatcher, FTPEvent, FTPEventPayload, NullEventDispatcher},
    infra::PubsubEventDispatcher,
};

use async_trait::async_trait;
use clap::ArgMatches;
use libunftp::notification::{DataEvent, EventMeta, PresenceEvent};
use std::{fmt::Debug, sync::Arc};

pub fn create_event_dispatcher(
    log: Arc<slog::Logger>,
    m: &ArgMatches,
) -> Result<Arc<dyn EventDispatcher<FTPEvent>>, String> {
    match (
        m.value_of(args::PUBSUB_TOPIC),
        m.value_of(args::PUBSUB_BASE_URL),
        m.value_of(args::PUBSUB_PROJECT),
    ) {
        (Some(topic), Some(base_url), Some(project_name)) => Ok(Arc::new(PubsubEventDispatcher::with_api_base(
            log,
            project_name,
            topic,
            base_url,
        ))),
        (Some(_topic), _, None) => Err(format!(
            "--{} is required when specifying --{}",
            args::PUBSUB_PROJECT,
            args::PUBSUB_TOPIC
        )),
        (None, _, Some(_project)) => Err(format!(
            "--{} is required when specifying --{}",
            args::PUBSUB_TOPIC,
            args::PUBSUB_PROJECT
        )),
        _ => Ok(Arc::new(NullEventDispatcher {})),
    }
}

#[derive(Debug)]
pub struct FTPListener {
    pub event_dispatcher: Arc<dyn EventDispatcher<FTPEvent>>,
    pub instance_name: String,
    pub hostname: String,
}

impl FTPListener {
    async fn dispatch(&self, payload: FTPEventPayload, m: EventMeta) {
        self.event_dispatcher
            .dispatch(FTPEvent {
                source_instance: self.instance_name.clone(),
                hostname: self.hostname.clone(),
                payload,
                username: Some(m.username),
                trace_id: Some(m.trace_id),
                sequence_number: Some(m.sequence_number),
            })
            .await
    }
}

#[async_trait]
impl libunftp::notification::DataListener for FTPListener {
    async fn receive_data_event(&self, e: DataEvent, m: EventMeta) {
        let payload = match e {
            DataEvent::Got { path, .. } => FTPEventPayload::Get { path },
            DataEvent::Put { path, .. } => FTPEventPayload::Put { path },
            DataEvent::Deleted { path } => FTPEventPayload::Delete { path },
            DataEvent::MadeDir { path } => FTPEventPayload::MakeDir { path },
            DataEvent::Renamed { from, to } => FTPEventPayload::Rename { from, to },
            DataEvent::RemovedDir { path } => FTPEventPayload::RemoveDir { path },
        };
        self.dispatch(payload, m).await;
    }
}

#[async_trait]
impl libunftp::notification::PresenceListener for FTPListener {
    async fn receive_presence_event(&self, e: PresenceEvent, m: EventMeta) {
        if m.username.eq("unknown") {
            // This is to prevent lots of LoggedOut events due to the unFTP health check
            // that does a NgsOP and then a Quit but never logs in. In this case libunftp sets the
            // username to unknown.
            return;
        }
        let payload = match e {
            PresenceEvent::LoggedIn => FTPEventPayload::Login,
            PresenceEvent::LoggedOut => FTPEventPayload::Logout,
        };
        self.dispatch(payload, m).await;
    }
}
