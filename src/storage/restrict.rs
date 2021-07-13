use std::fmt::Debug;
use std::io::{Cursor, Error, ErrorKind};
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use libunftp::storage;
use libunftp::storage::{Fileinfo, Metadata, StorageBackend};
use tokio::io::AsyncRead;

use crate::auth::{User, VfsOperations};
use crate::storage::choose::{ChoosingVfs, SbeMeta};

/// A virtual filesystem that checks if the user has permissions to do its operations before it
/// delegates to another storage back-end.
#[derive(Debug)]
pub struct RestrictingVfs {
    pub delegate: ChoosingVfs,
}

#[async_trait]
impl StorageBackend<User> for RestrictingVfs {
    type Metadata = SbeMeta;

    fn name(&self) -> &str {
        self.delegate.name()
    }

    fn supported_features(&self) -> u32 {
        self.delegate.supported_features()
    }

    async fn metadata<P: AsRef<Path> + Send + Debug>(&self, user: &User, path: P) -> storage::Result<Self::Metadata> {
        self.delegate.metadata(user, path).await
    }

    async fn md5<P: AsRef<Path> + Send + Debug>(&self, user: &User, path: P) -> storage::Result<String>
    where
        P: AsRef<Path> + Send + Debug,
    {
        if user.vfs_permissions.contains(VfsOperations::MD5) {
            self.delegate.md5(user, path).await
        } else {
            Err(libunftp::storage::ErrorKind::PermissionDenied.into())
        }
    }

    async fn list<P: AsRef<Path> + Send + Debug>(
        &self,
        user: &User,
        path: P,
    ) -> storage::Result<Vec<Fileinfo<PathBuf, Self::Metadata>>>
    where
        <Self as StorageBackend<User>>::Metadata: Metadata,
    {
        if user.vfs_permissions.contains(VfsOperations::LIST) {
            self.delegate.list(user, path).await
        } else {
            Err(libunftp::storage::ErrorKind::PermissionDenied.into())
        }
    }

    async fn list_fmt<P>(&self, user: &User, path: P) -> storage::Result<Cursor<Vec<u8>>>
    where
        P: AsRef<Path> + Send + Debug,
        Self::Metadata: Metadata + 'static,
    {
        if user.vfs_permissions.contains(VfsOperations::LIST) {
            self.delegate.list_fmt(user, path).await
        } else {
            Err(libunftp::storage::ErrorKind::PermissionDenied.into())
        }
    }

    async fn nlst<P>(&self, user: &User, path: P) -> std::result::Result<Cursor<Vec<u8>>, Error>
    where
        P: AsRef<Path> + Send + Debug,
        Self::Metadata: Metadata + 'static,
    {
        if user.vfs_permissions.contains(VfsOperations::LIST) {
            self.delegate.nlst(user, path).await
        } else {
            Err(ErrorKind::PermissionDenied.into())
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
        if user.vfs_permissions.contains(VfsOperations::GET) {
            self.delegate.get_into(user, path, start_pos, output).await
        } else {
            Err(libunftp::storage::ErrorKind::PermissionDenied.into())
        }
    }

    async fn get<P: AsRef<Path> + Send + Debug>(
        &self,
        user: &User,
        path: P,
        start_pos: u64,
    ) -> storage::Result<Box<dyn AsyncRead + Send + Sync + Unpin>> {
        if user.vfs_permissions.contains(VfsOperations::GET) {
            self.delegate.get(user, path, start_pos).await
        } else {
            Err(libunftp::storage::ErrorKind::PermissionDenied.into())
        }
    }

    async fn put<P: AsRef<Path> + Send + Debug, R: tokio::io::AsyncRead + Send + Sync + Unpin + 'static>(
        &self,
        user: &User,
        input: R,
        path: P,
        start_pos: u64,
    ) -> storage::Result<u64> {
        if user.vfs_permissions.contains(VfsOperations::PUT) {
            self.delegate.put(user, input, path, start_pos).await
        } else {
            Err(libunftp::storage::ErrorKind::PermissionDenied.into())
        }
    }

    async fn del<P: AsRef<Path> + Send + Debug>(&self, user: &User, path: P) -> storage::Result<()> {
        if user.vfs_permissions.contains(VfsOperations::DEL) {
            self.delegate.del(user, path).await
        } else {
            Err(libunftp::storage::ErrorKind::PermissionDenied.into())
        }
    }

    async fn mkd<P: AsRef<Path> + Send + Debug>(&self, user: &User, path: P) -> storage::Result<()> {
        if user.vfs_permissions.contains(VfsOperations::MK_DIR) {
            self.delegate.mkd(user, path).await
        } else {
            Err(libunftp::storage::ErrorKind::PermissionDenied.into())
        }
    }

    async fn rename<P: AsRef<Path> + Send + Debug>(&self, user: &User, from: P, to: P) -> storage::Result<()> {
        if user.vfs_permissions.contains(VfsOperations::RENAME) {
            self.delegate.rename(user, from, to).await
        } else {
            Err(libunftp::storage::ErrorKind::PermissionDenied.into())
        }
    }

    async fn rmd<P: AsRef<Path> + Send + Debug>(&self, user: &User, path: P) -> storage::Result<()> {
        if user.vfs_permissions.contains(VfsOperations::RM_DIR) {
            self.delegate.rmd(user, path).await
        } else {
            Err(libunftp::storage::ErrorKind::PermissionDenied.into())
        }
    }

    async fn cwd<P: AsRef<Path> + Send + Debug>(&self, user: &User, path: P) -> storage::Result<()> {
        self.delegate.cwd(user, path).await
    }
}
