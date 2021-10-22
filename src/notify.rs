use crate::auth::User;
use crate::domain::{EventDispatcher, FTPEvent, FTPEventPayload, NullEventDispatcher};
use crate::infra::PubsubEventDispatcher;
use crate::{args, auth};
use async_trait::async_trait;
use clap::ArgMatches;
use libunftp::auth::{AuthenticationError, Credentials, UserDetail};
use libunftp::storage::{Fileinfo, Metadata, StorageBackend};
use std::fmt::Debug;
use std::io::Cursor;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite};

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

/// A libunftp authenticator that will send pub/sub notifications if someone logs in.
#[derive(Debug)]
pub struct NotifyingAuthenticator {
    inner: Box<dyn libunftp::auth::Authenticator<auth::User>>,
    event_dispatcher: Arc<dyn EventDispatcher<FTPEvent>>,
    instance_name: String,
    hostname: String,
}

impl NotifyingAuthenticator {
    pub fn new<Str>(
        inner: Box<dyn libunftp::auth::Authenticator<auth::User>>,
        event_dispatcher: Arc<dyn EventDispatcher<FTPEvent>>,
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

#[derive(Debug)]
pub struct NotifyingStorageBackend<Delegate, User>
where
    Delegate: StorageBackend<User>,
    User: UserDetail,
{
    inner: Delegate,
    event_dispatcher: Arc<dyn EventDispatcher<FTPEvent>>,
    instance_name: String,
    hostname: String,
    x: PhantomData<User>,
}

impl<Delegate, User> NotifyingStorageBackend<Delegate, User>
where
    Delegate: StorageBackend<User>,
    User: UserDetail,
{
    pub fn new<Str>(
        inner: Delegate,
        event_dispatcher: Arc<dyn EventDispatcher<FTPEvent>>,
        instance_name: Str,
        hostname: Str,
    ) -> Self
    where
        Str: Into<String>,
    {
        NotifyingStorageBackend {
            inner,
            event_dispatcher,
            instance_name: instance_name.into(),
            hostname: hostname.into(),
            x: PhantomData,
        }
    }

    async fn dispatch(&self, payload: FTPEventPayload) {
        self.event_dispatcher
            .dispatch(FTPEvent {
                source_instance: self.instance_name.clone(),
                hostname: self.hostname.clone(),
                payload,
            })
            .await
    }
}

#[async_trait]
impl<Delegate, User> StorageBackend<User> for NotifyingStorageBackend<Delegate, User>
where
    Delegate: StorageBackend<User>,
    Delegate::Metadata: Send + Sync + Debug,
    User: UserDetail,
{
    type Metadata = Delegate::Metadata;

    fn name(&self) -> &str {
        self.inner.name()
    }

    fn supported_features(&self) -> u32 {
        self.inner.supported_features()
    }

    async fn metadata<P: AsRef<Path> + Send + Debug>(
        &self,
        user: &User,
        path: P,
    ) -> libunftp::storage::Result<Self::Metadata> {
        self.inner.metadata(user, path).await
    }

    async fn md5<P: AsRef<Path> + Send + Debug>(&self, user: &User, path: P) -> libunftp::storage::Result<String>
    where
        P: AsRef<Path> + Send + Debug,
    {
        self.inner.md5(user, path).await
    }

    async fn list<P: AsRef<Path> + Send + Debug>(
        &self,
        user: &User,
        path: P,
    ) -> libunftp::storage::Result<Vec<Fileinfo<PathBuf, Self::Metadata>>>
    where
        <Self as StorageBackend<User>>::Metadata: Metadata,
    {
        self.inner.list(user, path).await
    }

    async fn list_fmt<P>(&self, user: &User, path: P) -> libunftp::storage::Result<Cursor<Vec<u8>>>
    where
        P: AsRef<Path> + Send + Debug,
        Self::Metadata: Metadata + 'static,
    {
        self.inner.list_fmt(user, path).await
    }

    async fn list_vec<P>(&self, user: &User, path: P) -> libunftp::storage::Result<Vec<String>>
    where
        P: AsRef<Path> + Send + Debug,
        Self::Metadata: Metadata + 'static,
    {
        self.inner.list_vec(user, path).await
    }

    async fn nlst<P>(&self, user: &User, path: P) -> Result<Cursor<Vec<u8>>, std::io::Error>
    where
        P: AsRef<Path> + Send + Debug,
        Self::Metadata: Metadata + 'static,
    {
        self.inner.nlst(user, path).await
    }

    async fn get_into<'a, P, W: ?Sized>(
        &self,
        user: &User,
        path: P,
        start_pos: u64,
        output: &'a mut W,
    ) -> libunftp::storage::Result<u64>
    where
        W: AsyncWrite + Unpin + Sync + Send,
        P: AsRef<Path> + Send + Debug,
    {
        let path_str = path.as_ref().to_string_lossy().to_string();
        let result = self.inner.get_into(user, path, start_pos, output).await;
        if result.is_ok() {
            self.dispatch(FTPEventPayload::Get { path: path_str }).await;
        }
        result
    }

    async fn get<P: AsRef<Path> + Send + Debug>(
        &self,
        user: &User,
        path: P,
        start_pos: u64,
    ) -> libunftp::storage::Result<Box<dyn AsyncRead + Send + Sync + Unpin>> {
        self.inner.get(user, path, start_pos).await
    }

    async fn put<P: AsRef<Path> + Send + Debug, R: AsyncRead + Send + Sync + Unpin + 'static>(
        &self,
        user: &User,
        input: R,
        path: P,
        start_pos: u64,
    ) -> libunftp::storage::Result<u64> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        let result = self.inner.put(user, input, path, start_pos).await;
        if result.is_ok() {
            self.dispatch(FTPEventPayload::Put { path: path_str }).await;
        }
        result
    }

    async fn del<P: AsRef<Path> + Send + Debug>(&self, user: &User, path: P) -> libunftp::storage::Result<()> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        let result = self.inner.del(user, path).await;
        if result.is_ok() {
            self.dispatch(FTPEventPayload::Delete { path: path_str }).await;
        }
        result
    }

    async fn mkd<P: AsRef<Path> + Send + Debug>(&self, user: &User, path: P) -> libunftp::storage::Result<()> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        let result = self.inner.mkd(user, path).await;
        if result.is_ok() {
            self.dispatch(FTPEventPayload::MakeDir { path: path_str }).await;
        }
        result
    }

    async fn rename<P: AsRef<Path> + Send + Debug>(
        &self,
        user: &User,
        from: P,
        to: P,
    ) -> libunftp::storage::Result<()> {
        let from_str = from.as_ref().to_string_lossy().to_string();
        let to_str = from.as_ref().to_string_lossy().to_string();
        let result = self.inner.rename(user, from, to).await;
        if result.is_ok() {
            self.dispatch(FTPEventPayload::Rename {
                from: from_str,
                to: to_str,
            })
            .await;
        }
        result
    }

    async fn rmd<P: AsRef<Path> + Send + Debug>(&self, user: &User, path: P) -> libunftp::storage::Result<()> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        let result = self.inner.rmd(user, path).await;
        if result.is_ok() {
            self.dispatch(FTPEventPayload::RemoveDir { path: path_str }).await;
        }
        result
    }

    async fn cwd<P: AsRef<Path> + Send + Debug>(&self, user: &User, path: P) -> libunftp::storage::Result<()> {
        self.inner.cwd(user, path).await
    }
}
