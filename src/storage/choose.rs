use std::fmt::Debug;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use async_trait::async_trait;
use libunftp::storage;
use libunftp::storage::{Fileinfo, StorageBackend};

use crate::domain::user::User;

/**
 * A virtual file system that represents either a Cloud or file system back-end.
 */
#[derive(Debug)]
pub struct ChoosingVfs {
    pub inner: InnerVfs,
    #[allow(dead_code)]
    pub log: Arc<slog::Logger>,
}

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum InnerVfs {
    #[cfg(feature = "sbe_azblob")]
    OpenDAL(unftp_sbe_opendal::OpendalStorage),
    #[cfg(feature = "sbe_gcs")]
    Cloud(unftp_sbe_gcs::CloudStorage),
    File(unftp_sbe_fs::Filesystem),
    #[cfg(feature = "sbe_iso")]
    Iso(unftp_sbe_iso::Storage),
}

#[derive(Debug)]
pub enum SbeMeta {
    #[cfg(feature = "sbe_azblob")]
    OpenDAL(unftp_sbe_opendal::OpendalMetadata),
    #[cfg(feature = "sbe_gcs")]
    Cloud(unftp_sbe_gcs::object_metadata::ObjectMetadata),
    File(unftp_sbe_fs::Meta),
    #[cfg(feature = "sbe_iso")]
    Iso(unftp_sbe_iso::IsoMeta),
}

impl libunftp::storage::Metadata for SbeMeta {
    fn len(&self) -> u64 {
        match self {
            #[cfg(feature = "sbe_azblob")]
            SbeMeta::OpenDAL(m) => m.len(),
            #[cfg(feature = "sbe_gcs")]
            SbeMeta::Cloud(m) => m.len(),
            SbeMeta::File(m) => m.len(),
            #[cfg(feature = "sbe_iso")]
            SbeMeta::Iso(m) => m.len(),
        }
    }

    fn is_dir(&self) -> bool {
        match self {
            #[cfg(feature = "sbe_azblob")]
            SbeMeta::OpenDAL(m) => m.is_dir(),
            #[cfg(feature = "sbe_gcs")]
            SbeMeta::Cloud(m) => m.is_dir(),
            SbeMeta::File(m) => m.is_dir(),
            #[cfg(feature = "sbe_iso")]
            SbeMeta::Iso(m) => m.is_dir(),
        }
    }

    fn is_file(&self) -> bool {
        match self {
            #[cfg(feature = "sbe_azblob")]
            SbeMeta::OpenDAL(m) => m.is_file(),
            #[cfg(feature = "sbe_gcs")]
            SbeMeta::Cloud(m) => m.is_file(),
            SbeMeta::File(m) => m.is_file(),
            #[cfg(feature = "sbe_iso")]
            SbeMeta::Iso(m) => m.is_file(),
        }
    }

    fn is_symlink(&self) -> bool {
        match self {
            #[cfg(feature = "sbe_azblob")]
            SbeMeta::OpenDAL(m) => m.is_symlink(),
            #[cfg(feature = "sbe_gcs")]
            SbeMeta::Cloud(m) => m.is_symlink(),
            SbeMeta::File(m) => m.is_symlink(),
            #[cfg(feature = "sbe_iso")]
            SbeMeta::Iso(m) => m.is_symlink(),
        }
    }

    fn modified(&self) -> storage::Result<SystemTime> {
        match self {
            #[cfg(feature = "sbe_azblob")]
            SbeMeta::OpenDAL(m) => m.modified(),
            #[cfg(feature = "sbe_gcs")]
            SbeMeta::Cloud(m) => m.modified(),
            SbeMeta::File(m) => m.modified(),
            #[cfg(feature = "sbe_iso")]
            SbeMeta::Iso(m) => m.modified(),
        }
    }

    fn gid(&self) -> u32 {
        match self {
            #[cfg(feature = "sbe_azblob")]
            SbeMeta::OpenDAL(m) => m.gid(),
            #[cfg(feature = "sbe_gcs")]
            SbeMeta::Cloud(m) => m.gid(),
            SbeMeta::File(m) => m.gid(),
            #[cfg(feature = "sbe_iso")]
            SbeMeta::Iso(m) => m.gid(),
        }
    }

    fn uid(&self) -> u32 {
        match self {
            #[cfg(feature = "sbe_azblob")]
            SbeMeta::OpenDAL(m) => m.uid(),
            #[cfg(feature = "sbe_gcs")]
            SbeMeta::Cloud(m) => m.uid(),
            SbeMeta::File(m) => m.uid(),
            #[cfg(feature = "sbe_iso")]
            SbeMeta::Iso(m) => m.uid(),
        }
    }
}

#[async_trait]
impl StorageBackend<User> for ChoosingVfs {
    type Metadata = SbeMeta;

    fn name(&self) -> &str {
        match &self.inner {
            #[cfg(feature = "sbe_azblob")]
            InnerVfs::OpenDAL(i) => StorageBackend::<User>::name(i),
            #[cfg(feature = "sbe_gcs")]
            InnerVfs::Cloud(i) => StorageBackend::<User>::name(i),
            InnerVfs::File(i) => StorageBackend::<User>::name(i),
            #[cfg(feature = "sbe_iso")]
            InnerVfs::Iso(i) => StorageBackend::<User>::name(i),
        }
    }

    fn supported_features(&self) -> u32 {
        match &self.inner {
            #[cfg(feature = "sbe_azblob")]
            InnerVfs::OpenDAL(i) => StorageBackend::<User>::supported_features(i),
            #[cfg(feature = "sbe_gcs")]
            InnerVfs::Cloud(i) => StorageBackend::<User>::supported_features(i),
            InnerVfs::File(i) => StorageBackend::<User>::supported_features(i),
            #[cfg(feature = "sbe_iso")]
            InnerVfs::Iso(i) => StorageBackend::<User>::supported_features(i),
        }
    }

    async fn metadata<P: AsRef<Path> + Send + Debug>(
        &self,
        user: &User,
        path: P,
    ) -> storage::Result<Self::Metadata> {
        match &self.inner {
            #[cfg(feature = "sbe_azblob")]
            InnerVfs::OpenDAL(i) => i.metadata(user, path).await.map(SbeMeta::OpenDAL),
            #[cfg(feature = "sbe_gcs")]
            InnerVfs::Cloud(i) => i.metadata(user, path).await.map(SbeMeta::Cloud),
            InnerVfs::File(i) => i.metadata(user, path).await.map(SbeMeta::File),
            #[cfg(feature = "sbe_iso")]
            InnerVfs::Iso(i) => i.metadata(user, path).await.map(SbeMeta::Iso),
        }
    }

    async fn list<P: AsRef<Path> + Send + Debug>(
        &self,
        user: &User,
        path: P,
    ) -> storage::Result<Vec<Fileinfo<PathBuf, Self::Metadata>>>
    where
        <Self as StorageBackend<User>>::Metadata: libunftp::storage::Metadata,
    {
        match &self.inner {
            #[cfg(feature = "sbe_azblob")]
            InnerVfs::OpenDAL(i) => i.list(user, path).await.map(|v| {
                v.into_iter()
                    .map(|fi| Fileinfo {
                        path: fi.path,
                        metadata: SbeMeta::OpenDAL(fi.metadata),
                    })
                    .collect()
            }),
            #[cfg(feature = "sbe_gcs")]
            InnerVfs::Cloud(i) => i.list(user, path).await.map(|v| {
                v.into_iter()
                    .map(|fi| Fileinfo {
                        path: fi.path,
                        metadata: SbeMeta::Cloud(fi.metadata),
                    })
                    .collect()
            }),
            InnerVfs::File(i) => i.list(user, path).await.map(|v| {
                v.into_iter()
                    .map(|fi| Fileinfo {
                        path: fi.path,
                        metadata: SbeMeta::File(fi.metadata),
                    })
                    .collect()
            }),
            #[cfg(feature = "sbe_iso")]
            InnerVfs::Iso(i) => i.list(user, path).await.map(|v| {
                v.into_iter()
                    .map(|fi| Fileinfo {
                        path: fi.path,
                        metadata: SbeMeta::Iso(fi.metadata),
                    })
                    .collect()
            }),
        }
    }

    async fn list_fmt<P>(&self, user: &User, path: P) -> storage::Result<Cursor<Vec<u8>>>
    where
        P: AsRef<Path> + Send + Debug,
        Self::Metadata: libunftp::storage::Metadata + 'static,
    {
        match &self.inner {
            #[cfg(feature = "sbe_azblob")]
            InnerVfs::OpenDAL(i) => i.list_fmt(user, path).await,
            #[cfg(feature = "sbe_gcs")]
            InnerVfs::Cloud(i) => i.list_fmt(user, path).await,
            InnerVfs::File(i) => i.list_fmt(user, path).await,
            #[cfg(feature = "sbe_iso")]
            InnerVfs::Iso(i) => i.list_fmt(user, path).await,
        }
    }

    async fn nlst<P>(&self, user: &User, path: P) -> std::io::Result<Cursor<Vec<u8>>>
    where
        P: AsRef<Path> + Send + Debug,
        Self::Metadata: libunftp::storage::Metadata + 'static,
    {
        match &self.inner {
            #[cfg(feature = "sbe_azblob")]
            InnerVfs::OpenDAL(i) => i.nlst(user, path).await,
            #[cfg(feature = "sbe_gcs")]
            InnerVfs::Cloud(i) => i.nlst(user, path).await,
            InnerVfs::File(i) => i.nlst(user, path).await,
            #[cfg(feature = "sbe_iso")]
            InnerVfs::Iso(i) => i.nlst(user, path).await,
        }
    }

    async fn get_into<'a, P, W: ?Sized>(
        &self,
        user: &User,
        path: P,
        start_pos: u64,
        output: &'a mut W,
    ) -> storage::Result<u64>
    where
        W: tokio::io::AsyncWrite + Unpin + Sync + Send,
        P: AsRef<Path> + Send + Debug,
    {
        match &self.inner {
            #[cfg(feature = "sbe_azblob")]
            InnerVfs::OpenDAL(i) => i.get_into(user, path, start_pos, output).await,
            #[cfg(feature = "sbe_gcs")]
            InnerVfs::Cloud(i) => i.get_into(user, path, start_pos, output).await,
            InnerVfs::File(i) => i.get_into(user, path, start_pos, output).await,
            #[cfg(feature = "sbe_iso")]
            InnerVfs::Iso(i) => i.get_into(user, path, start_pos, output).await,
        }
    }

    async fn get<P: AsRef<Path> + Send + Debug>(
        &self,
        user: &User,
        path: P,
        start_pos: u64,
    ) -> storage::Result<Box<dyn tokio::io::AsyncRead + Send + Sync + Unpin>> {
        match &self.inner {
            #[cfg(feature = "sbe_azblob")]
            InnerVfs::OpenDAL(i) => i.get(user, path, start_pos).await,
            #[cfg(feature = "sbe_gcs")]
            InnerVfs::Cloud(i) => i.get(user, path, start_pos).await,
            InnerVfs::File(i) => i.get(user, path, start_pos).await,
            #[cfg(feature = "sbe_iso")]
            InnerVfs::Iso(i) => i.get(user, path, start_pos).await,
        }
    }

    // async fn put<'a, P, R: ?Sized>(&self, user: &User, input: &'a mut R, path: P, start_pos: u64) -> Result<u64>
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

    async fn put<
        P: AsRef<Path> + Send + Debug,
        R: tokio::io::AsyncRead + Send + Sync + Unpin + 'static,
    >(
        &self,
        user: &User,
        input: R,
        path: P,
        start_pos: u64,
    ) -> storage::Result<u64> {
        match &self.inner {
            #[cfg(feature = "sbe_azblob")]
            InnerVfs::OpenDAL(i) => i.put(user, input, path, start_pos).await,
            #[cfg(feature = "sbe_gcs")]
            InnerVfs::Cloud(i) => i.put(user, input, path, start_pos).await,
            InnerVfs::File(i) => i.put(user, input, path, start_pos).await,
            #[cfg(feature = "sbe_iso")]
            InnerVfs::Iso(i) => i.put(user, input, path, start_pos).await,
        }
    }

    async fn del<P: AsRef<Path> + Send + Debug>(
        &self,
        user: &User,
        path: P,
    ) -> storage::Result<()> {
        match &self.inner {
            #[cfg(feature = "sbe_azblob")]
            InnerVfs::OpenDAL(i) => i.del(user, path).await,
            #[cfg(feature = "sbe_gcs")]
            InnerVfs::Cloud(i) => i.del(user, path).await,
            InnerVfs::File(i) => i.del(user, path).await,
            #[cfg(feature = "sbe_iso")]
            InnerVfs::Iso(i) => i.del(user, path).await,
        }
    }

    async fn mkd<P: AsRef<Path> + Send + Debug>(
        &self,
        user: &User,
        path: P,
    ) -> storage::Result<()> {
        match &self.inner {
            #[cfg(feature = "sbe_azblob")]
            InnerVfs::OpenDAL(i) => i.mkd(user, path).await,
            #[cfg(feature = "sbe_gcs")]
            InnerVfs::Cloud(i) => i.mkd(user, path).await,
            InnerVfs::File(i) => i.mkd(user, path).await,
            #[cfg(feature = "sbe_iso")]
            InnerVfs::Iso(i) => i.mkd(user, path).await,
        }
    }

    async fn rename<P: AsRef<Path> + Send + Debug>(
        &self,
        user: &User,
        from: P,
        to: P,
    ) -> storage::Result<()> {
        match &self.inner {
            #[cfg(feature = "sbe_azblob")]
            InnerVfs::OpenDAL(i) => i.rename(user, from, to).await,
            #[cfg(feature = "sbe_gcs")]
            InnerVfs::Cloud(i) => i.rename(user, from, to).await,
            InnerVfs::File(i) => i.rename(user, from, to).await,
            #[cfg(feature = "sbe_iso")]
            InnerVfs::Iso(i) => i.rename(user, from, to).await,
        }
    }

    async fn rmd<P: AsRef<Path> + Send + Debug>(
        &self,
        user: &User,
        path: P,
    ) -> storage::Result<()> {
        match &self.inner {
            #[cfg(feature = "sbe_azblob")]
            InnerVfs::OpenDAL(i) => i.rmd(user, path).await,
            #[cfg(feature = "sbe_gcs")]
            InnerVfs::Cloud(i) => i.rmd(user, path).await,
            InnerVfs::File(i) => i.rmd(user, path).await,
            #[cfg(feature = "sbe_iso")]
            InnerVfs::Iso(i) => i.rmd(user, path).await,
        }
    }

    async fn cwd<P: AsRef<Path> + Send + Debug>(
        &self,
        user: &User,
        path: P,
    ) -> storage::Result<()> {
        match &self.inner {
            #[cfg(feature = "sbe_azblob")]
            InnerVfs::OpenDAL(i) => i.cwd(user, path).await,
            #[cfg(feature = "sbe_gcs")]
            InnerVfs::Cloud(i) => i.cwd(user, path).await,
            InnerVfs::File(i) => i.cwd(user, path).await,
            #[cfg(feature = "sbe_iso")]
            InnerVfs::Iso(i) => i.cwd(user, path).await,
        }
    }
}
