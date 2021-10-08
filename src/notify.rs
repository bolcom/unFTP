use crate::auth::User;
use crate::domain::{EventDispatcher, FTPEvent, FTPEventPayload, NullEventDispatcher};
use crate::infra::PubsubEventDispatcher;
use crate::{args, auth};
use async_trait::async_trait;
use clap::ArgMatches;
use libunftp::auth::{AuthenticationError, Credentials};
use std::sync::Arc;

pub fn create_event_dispatcher(
    log: Arc<slog::Logger>,
    m: &ArgMatches,
) -> Result<Box<dyn EventDispatcher<FTPEvent>>, String> {
    match (
        m.value_of(args::PUBSUB_TOPIC),
        m.value_of(args::PUBSUB_BASE_URL),
        m.value_of(args::PUBSUB_PROJECT),
    ) {
        (Some(topic), Some(base_url), Some(project_name)) => Ok(Box::new(PubsubEventDispatcher::with_api_base(
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
        _ => Ok(Box::new(NullEventDispatcher {})),
    }
}

/// A libunftp authenticator that will send pub/sub notifications if someone logs in.
#[derive(Debug)]
pub struct NotifyingAuthenticator {
    inner: Box<dyn libunftp::auth::Authenticator<auth::User>>,
    event_dispatcher: Box<dyn EventDispatcher<FTPEvent>>,
    instance_name: String,
    hostname: String,
}

impl NotifyingAuthenticator {
    pub fn new<Str>(
        inner: Box<dyn libunftp::auth::Authenticator<auth::User>>,
        event_dispatcher: Box<dyn EventDispatcher<FTPEvent>>,
        instance_name: Str,
        hostname: Str,
    ) -> Self
    where
        Str: Into<String>,
    {
        NotifyingAuthenticator {
            inner,
            event_dispatcher,
            instance_name: instance_name.into(),
            hostname: hostname.into(),
        }
    }
}

#[async_trait]
impl libunftp::auth::Authenticator<auth::User> for NotifyingAuthenticator {
    async fn authenticate(&self, username: &str, creds: &Credentials) -> Result<User, AuthenticationError> {
        let user = self.inner.authenticate(username, creds).await?;
        self.event_dispatcher
            .dispatch(FTPEvent {
                source_instance: self.instance_name.clone(),
                hostname: self.hostname.clone(),
                payload: FTPEventPayload::Login {
                    username: username.to_string(),
                },
            })
            .await;

        Ok(user)
    }

    async fn cert_auth_sufficient(&self, username: &str) -> bool {
        self.inner.cert_auth_sufficient(username).await
    }
}
