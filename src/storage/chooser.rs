use std::fmt::Debug;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use async_trait::async_trait;
use libunftp::storage;
use libunftp::storage::{Fileinfo, StorageBackend};

use crate::auth::User;

#[derive(Debug)]
pub enum InnerStorage {
    Cloud(unftp_sbe_gcs::CloudStorage),
    File(unftp_sbe_fs::Filesystem),
}

#[derive(Debug)]
pub struct StorageBe {
    pub inner: InnerStorage,
    pub log: Arc<slog::Logger>,
}

#[derive(Debug)]
pub enum SbeMeta {
    Cloud(unftp_sbe_gcs::object_metadata::ObjectMetadata),
    File(unftp_sbe_fs::Meta),
}

impl libunftp::storage::Metadata for SbeMeta {
    fn len(&self) -> u64 {
        match self {
            SbeMeta::Cloud(m) => m.len(),
            SbeMeta::File(m) => m.len(),
        }
    }

    fn is_dir(&self) -> bool {
        match self {
            SbeMeta::Cloud(m) => m.is_dir(),
            SbeMeta::File(m) => m.is_dir(),
        }
    }

    fn is_file(&self) -> bool {
        match self {
            SbeMeta::Cloud(m) => m.is_file(),
            SbeMeta::File(m) => m.is_file(),
        }
    }

    fn is_symlink(&self) -> bool {
        match self {
            SbeMeta::Cloud(m) => m.is_symlink(),
            SbeMeta::File(m) => m.is_symlink(),
        }
    }

    fn modified(&self) -> storage::Result<SystemTime> {
        match self {
            SbeMeta::Cloud(m) => m.modified(),
            SbeMeta::File(m) => m.modified().map_err(|e| e),
        }
    }

    fn gid(&self) -> u32 {
        match self {
            SbeMeta::Cloud(m) => m.gid(),
            SbeMeta::File(m) => m.gid(),
        }
    }

    fn uid(&self) -> u32 {
        match self {
            SbeMeta::Cloud(m) => m.uid(),
            SbeMeta::File(m) => m.uid(),
        }
    }
}

#[async_trait]
impl StorageBackend<User> for StorageBe {
    type Metadata = SbeMeta;

    fn name(&self) -> &str {
        match &self.inner {
            InnerStorage::Cloud(i) => StorageBackend::<User>::name(i),
            InnerStorage::File(i) => StorageBackend::<User>::name(i),
        }
    }

    fn supported_features(&self) -> u32 {
        match &self.inner {
            InnerStorage::Cloud(i) => StorageBackend::<User>::supported_features(i),
            InnerStorage::File(i) => StorageBackend::<User>::supported_features(i),
        }
    }

    async fn metadata<P: AsRef<Path> + Send + Debug>(
        &self,
        user: &Option<User>,
        path: P,
    ) -> storage::Result<Self::Metadata> {
        match &self.inner {
            InnerStorage::Cloud(i) => i.metadata(user, path).await.map(SbeMeta::Cloud),
            InnerStorage::File(i) => i.metadata(user, path).await.map(SbeMeta::File),
        }
    }

    async fn list<P: AsRef<Path> + Send + Debug>(
        &self,
        user: &Option<User>,
        path: P,
    ) -> storage::Result<Vec<Fileinfo<PathBuf, Self::Metadata>>>
    where
        <Self as StorageBackend<User>>::Metadata: libunftp::storage::Metadata,
    {
        match &self.inner {
            InnerStorage::Cloud(i) => i.list(user, path).await.map(|v| {
                v.into_iter()
                    .map(|fi| Fileinfo {
                        path: fi.path,
                        metadata: SbeMeta::Cloud(fi.metadata),
                    })
                    .collect()
            }),
            InnerStorage::File(i) => i.list(user, path).await.map(|v| {
                v.into_iter()
                    .map(|fi| Fileinfo {
                        path: fi.path,
                        metadata: SbeMeta::File(fi.metadata),
                    })
                    .collect()
            }),
        }
    }

    async fn list_fmt<P>(&self, user: &Option<User>, path: P) -> storage::Result<Cursor<Vec<u8>>>
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
    ) -> storage::Result<u64>
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
    ) -> storage::Result<Box<dyn tokio::io::AsyncRead + Send + Sync + Unpin>> {
        match &self.inner {
            InnerStorage::Cloud(i) => i.get(user, path, start_pos).await,
            InnerStorage::File(i) => i.get(user, path, start_pos).await,
        }
    }

    // async fn put<'a, P, R: ?Sized>(&self, user: &Option<User>, input: &'a mut R, path: P, start_pos: u64) -> Result<u64>
    //     where
    //         R: tokio::io::AsyncRead + Unpin + Sync + Send,
    //         P: AsRef<Path> + Send + Debug,
    // {
    //     slog::info!(self.log(user, &path), "Client requested to store a file");
    //     match &self.inner {
    //         InnerStorage::Cloud(i) => i.put(user, input, path, start_pos).await,
    //         InnerStorage::File(i) => i.put(user, input, path, start_pos).await,
    //     }
    // }

    async fn put<P: AsRef<Path> + Send + Debug, R: tokio::io::AsyncRead + Send + Sync + Unpin + 'static>(
        &self,
        user: &Option<User>,
        input: R,
        path: P,
        start_pos: u64,
    ) -> storage::Result<u64> {
        match &self.inner {
            InnerStorage::Cloud(i) => i.put(user, input, path, start_pos).await,
            InnerStorage::File(i) => i.put(user, input, path, start_pos).await,
        }
    }

    async fn del<P: AsRef<Path> + Send + Debug>(&self, user: &Option<User>, path: P) -> storage::Result<()> {
        match &self.inner {
            InnerStorage::Cloud(i) => i.del(user, path).await,
            InnerStorage::File(i) => i.del(user, path).await,
        }
    }

    async fn mkd<P: AsRef<Path> + Send + Debug>(&self, user: &Option<User>, path: P) -> storage::Result<()> {
        match &self.inner {
            InnerStorage::Cloud(i) => i.mkd(user, path).await,
            InnerStorage::File(i) => i.mkd(user, path).await,
        }
    }

    async fn rename<P: AsRef<Path> + Send + Debug>(&self, user: &Option<User>, from: P, to: P) -> storage::Result<()> {
        match &self.inner {
            InnerStorage::Cloud(i) => i.rename(user, from, to).await,
            InnerStorage::File(i) => i.rename(user, from, to).await,
        }
    }

    async fn rmd<P: AsRef<Path> + Send + Debug>(&self, user: &Option<User>, path: P) -> storage::Result<()> {
        match &self.inner {
            InnerStorage::Cloud(i) => i.rmd(user, path).await,
            InnerStorage::File(i) => i.rmd(user, path).await,
        }
    }

    async fn cwd<P: AsRef<Path> + Send + Debug>(&self, user: &Option<User>, path: P) -> storage::Result<()> {
        match &self.inner {
            InnerStorage::Cloud(i) => i.cwd(user, path).await,
            InnerStorage::File(i) => i.cwd(user, path).await,
        }
    }
}
