use crate::user::User;
use async_trait::async_trait;
use libunftp::storage::Result;
use libunftp::storage::{Fileinfo, StorageBackend};
use std::fmt::Debug;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

#[derive(Debug)]
pub enum InnerStorage {
    Cloud(libunftp::storage::cloud_storage::CloudStorage),
    File(libunftp::storage::filesystem::Filesystem),
}

#[derive(Debug)]
pub struct StorageBE {
    pub inner: InnerStorage,
    pub log: Arc<slog::Logger>,
}

#[derive(Debug)]
pub enum SBEMeta {
    Cloud(libunftp::storage::cloud_storage::object_metadata::ObjectMetadata),
    File(std::fs::Metadata),
}

impl libunftp::storage::Metadata for SBEMeta {
    fn len(&self) -> u64 {
        match self {
            SBEMeta::Cloud(m) => m.len(),
            SBEMeta::File(m) => m.len(),
        }
    }

    fn is_dir(&self) -> bool {
        match self {
            SBEMeta::Cloud(m) => m.is_dir(),
            SBEMeta::File(m) => m.is_dir(),
        }
    }

    fn is_file(&self) -> bool {
        match self {
            SBEMeta::Cloud(m) => m.is_file(),
            SBEMeta::File(m) => m.is_file(),
        }
    }

    fn is_symlink(&self) -> bool {
        match self {
            SBEMeta::Cloud(m) => m.is_symlink(),
            SBEMeta::File(m) => m.is_symlink(),
        }
    }

    fn modified(&self) -> Result<SystemTime> {
        match self {
            SBEMeta::Cloud(m) => m.modified(),
            SBEMeta::File(m) => m.modified().map_err(|e| e.into()),
        }
    }

    fn gid(&self) -> u32 {
        match self {
            SBEMeta::Cloud(m) => m.gid(),
            SBEMeta::File(m) => m.gid(),
        }
    }

    fn uid(&self) -> u32 {
        match self {
            SBEMeta::Cloud(m) => m.uid(),
            SBEMeta::File(m) => m.uid(),
        }
    }
}

impl StorageBE {
    fn log<P: AsRef<Path> + Send + Debug>(&self, user: &Option<User>, path: &P) -> slog::Logger {
        let username = user.as_ref().map_or("unknown".to_string(), |u| u.username.to_string());
        let path_str = path.as_ref().to_string_lossy().to_string();
        self.log.new(slog::o!(
        "username" => username,
        "path" => path_str
        ))
    }
}

#[async_trait]
impl StorageBackend<User> for StorageBE {
    type Metadata = SBEMeta;

    fn supported_features(&self) -> u32 {
        match &self.inner {
            InnerStorage::Cloud(i) => StorageBackend::<User>::supported_features(i),
            InnerStorage::File(i) => StorageBackend::<User>::supported_features(i),
        }
    }

    async fn metadata<P: AsRef<Path> + Send + Debug>(&self, user: &Option<User>, path: P) -> Result<Self::Metadata> {
        match &self.inner {
            InnerStorage::Cloud(i) => i.metadata(user, path).await.map(SBEMeta::Cloud),
            InnerStorage::File(i) => i.metadata(user, path).await.map(SBEMeta::File),
        }
    }

    async fn list<P: AsRef<Path> + Send + Debug>(
        &self,
        user: &Option<User>,
        path: P,
    ) -> Result<Vec<Fileinfo<PathBuf, Self::Metadata>>>
    where
        <Self as StorageBackend<User>>::Metadata: libunftp::storage::Metadata,
    {
        slog::info!(self.log(user, &path), "Client requested to list a directory");
        match &self.inner {
            InnerStorage::Cloud(i) => i.list(user, path).await.map(|v| {
                v.into_iter()
                    .map(|fi| Fileinfo {
                        path: fi.path,
                        metadata: SBEMeta::Cloud(fi.metadata),
                    })
                    .collect()
            }),
            InnerStorage::File(i) => i.list(user, path).await.map(|v| {
                v.into_iter()
                    .map(|fi| Fileinfo {
                        path: fi.path,
                        metadata: SBEMeta::File(fi.metadata),
                    })
                    .collect()
            }),
        }
    }

    async fn list_fmt<P>(&self, user: &Option<User>, path: P) -> Result<Cursor<Vec<u8>>>
    where
        P: AsRef<Path> + Send + Debug,
        Self::Metadata: libunftp::storage::Metadata + 'static,
    {
        match &self.inner {
            InnerStorage::Cloud(i) => i.list_fmt(user, path).await,
            InnerStorage::File(i) => i.list_fmt(user, path).await,
        }
    }

    async fn nlst<P>(&self, user: &Option<User>, path: P) -> std::io::Result<Cursor<Vec<u8>>>
    where
        P: AsRef<Path> + Send + Debug,
        Self::Metadata: libunftp::storage::Metadata + 'static,
    {
        match &self.inner {
            InnerStorage::Cloud(i) => i.nlst(user, path).await,
            InnerStorage::File(i) => i.nlst(user, path).await,
        }
    }

    async fn get_into<'a, P, W: ?Sized>(
        &self,
        user: &Option<User>,
        path: P,
        start_pos: u64,
        output: &'a mut W,
    ) -> Result<u64>
    where
        W: tokio::io::AsyncWrite + Unpin + Sync + Send,
        P: AsRef<Path> + Send + Debug,
    {
        match &self.inner {
            InnerStorage::Cloud(i) => i.get_into(user, path, start_pos, output).await,
            InnerStorage::File(i) => i.get_into(user, path, start_pos, output).await,
        }
    }

    async fn get<P: AsRef<Path> + Send + Debug>(
        &self,
        user: &Option<User>,
        path: P,
        start_pos: u64,
    ) -> Result<Box<dyn tokio::io::AsyncRead + Send + Sync + Unpin>> {
        slog::info!(self.log(user, &path), "Client requested to retrieve a file");
        match &self.inner {
            InnerStorage::Cloud(i) => i.get(user, path, start_pos).await,
            InnerStorage::File(i) => i.get(user, path, start_pos).await,
        }
    }

    async fn put<P: AsRef<Path> + Send + Debug, R: tokio::io::AsyncRead + Send + Sync + Unpin + 'static>(
        &self,
        user: &Option<User>,
        input: R,
        path: P,
        start_pos: u64,
    ) -> Result<u64> {
        slog::info!(self.log(user, &path), "Client requested to store a file");
        match &self.inner {
            InnerStorage::Cloud(i) => i.put(user, input, path, start_pos).await,
            InnerStorage::File(i) => i.put(user, input, path, start_pos).await,
        }
    }

    async fn del<P: AsRef<Path> + Send + Debug>(&self, user: &Option<User>, path: P) -> Result<()> {
        slog::info!(self.log(user, &path), "Client requested to delete a file");
        match &self.inner {
            InnerStorage::Cloud(i) => i.del(user, path).await,
            InnerStorage::File(i) => i.del(user, path).await,
        }
    }

    async fn mkd<P: AsRef<Path> + Send + Debug>(&self, user: &Option<User>, path: P) -> Result<()> {
        slog::info!(self.log(user, &path), "Client requested to create a directory");
        match &self.inner {
            InnerStorage::Cloud(i) => i.mkd(user, path).await,
            InnerStorage::File(i) => i.mkd(user, path).await,
        }
    }

    async fn rename<P: AsRef<Path> + Send + Debug>(&self, user: &Option<User>, from: P, to: P) -> Result<()> {
        let path_str = &*to.as_ref().to_string_lossy();
        slog::info!(self.log(user, &from), "Client requested to rename a path"; "new-path" => path_str);
        match &self.inner {
            InnerStorage::Cloud(i) => i.rename(user, from, to).await,
            InnerStorage::File(i) => i.rename(user, from, to).await,
        }
    }

    async fn rmd<P: AsRef<Path> + Send + Debug>(&self, user: &Option<User>, path: P) -> Result<()> {
        slog::info!(self.log(user, &path), "Client requested to remove a directory");
        match &self.inner {
            InnerStorage::Cloud(i) => i.rmd(user, path).await,
            InnerStorage::File(i) => i.rmd(user, path).await,
        }
    }

    async fn cwd<P: AsRef<Path> + Send + Debug>(&self, user: &Option<User>, path: P) -> Result<()> {
        slog::info!(self.log(user, &path), "Client requested to change into a directory");
        match &self.inner {
            InnerStorage::Cloud(i) => i.cwd(user, path).await,
            InnerStorage::File(i) => i.cwd(user, path).await,
        }
    }
}
