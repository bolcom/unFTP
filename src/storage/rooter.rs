use async_trait::async_trait;
use libunftp::{
    auth::UserDetail,
    storage::{Fileinfo, Metadata, Result, StorageBackend},
};
use std::borrow::Cow;
use std::ffi::{OsStr, OsString};
use std::fmt::Debug;
use std::io::{Cursor, Error};
use std::marker::PhantomData;
use std::path::{Component, Path, PathBuf};
use tokio::io::AsyncRead;

/// Used by [RooterVfs] to obtain the user's root path from a [UserDetail](libunftp::auth::UserDetail) implementation
pub trait UserWithRoot {
    /// Returns the relative path to the user's root if it exists otherwise null.
    fn user_root(&self) -> Option<PathBuf>;
}

/// A virtual file system for libunftp that wraps other file systems
#[derive(Debug)]
pub struct RooterVfs<Delegate, User, Meta>
where
    Delegate: StorageBackend<User>,
    User: UserDetail + UserWithRoot,
    Meta: Metadata + Debug + Sync + Send,
{
    inner: Delegate,
    x: PhantomData<Meta>,
    y: PhantomData<User>,
}

impl<Delegate, User, Meta> RooterVfs<Delegate, User, Meta>
where
    Delegate: StorageBackend<User>,
    User: UserDetail + UserWithRoot,
    Meta: Metadata + Debug + Sync + Send,
{
    pub fn new(inner: Delegate) -> Self {
        RooterVfs {
            inner,
            x: PhantomData,
            y: PhantomData,
        }
    }

    pub(super) fn new_path<'a>(user: &Option<User>, requested_path: &'a Path) -> Cow<'a, Path> {
        if let Some(u) = user {
            if let Some(user_root) = u.user_root() {
                Cow::Owned(Self::root_to(user_root.as_os_str(), requested_path).unwrap())
            } else {
                Cow::Borrowed(requested_path)
            }
        } else {
            Cow::Borrowed(requested_path)
        }
    }

    fn root_to(root: &OsStr, requested_path: &Path) -> std::result::Result<PathBuf, ()> {
        let mut iter = requested_path.components();

        if let Some(first_component) = iter.next() {
            let mut tokens = Vec::new();

            match first_component {
                Component::RootDir | Component::ParentDir => {
                    tokens.push(root);
                }
                Component::CurDir => {
                    return Err(()); // It should never start with .
                }
                _ => {
                    tokens.push(root);
                    tokens.push(first_component.as_os_str());
                }
            }

            for component in iter {
                match component {
                    Component::CurDir => {}
                    Component::ParentDir => {
                        let tokens_length = tokens.len();
                        if tokens_length > 1 {
                            tokens.remove(tokens_length - 1);
                        }
                    }
                    _ => {
                        tokens.push(component.as_os_str());
                    }
                }
            }

            let tokens_length = tokens.len();

            let size = tokens.iter().fold(tokens_length - 1, |acc, &x| acc + x.len()) - 1;

            let mut path_string = OsString::with_capacity(size);

            for token in tokens.iter().take(tokens_length - 1) {
                path_string.push(token);
                path_string.push("/");
            }

            path_string.push(tokens[tokens_length - 1]);

            let path_buf = PathBuf::from(path_string);

            Ok(path_buf)
        } else {
            Err(()) // There will always be a prefix
        }
    }
}

#[async_trait]
impl<Delegate, User, Meta> StorageBackend<User> for RooterVfs<Delegate, User, Meta>
where
    Delegate: StorageBackend<User>,
    User: UserDetail + UserWithRoot,
    Meta: Metadata + Debug + Sync + Send,
{
    type Metadata = Delegate::Metadata;

    async fn metadata<P: AsRef<Path> + Send + Debug>(&self, user: &Option<User>, path: P) -> Result<Self::Metadata> {
        let path = Self::new_path(user, path.as_ref());
        self.inner.metadata(user, path).await
    }

    async fn md5<P: AsRef<Path> + Send + Debug>(&self, user: &Option<User>, path: P) -> Result<String>
    where
        P: AsRef<Path> + Send + Debug,
    {
        let path = Self::new_path(user, path.as_ref());
        self.inner.md5(user, path).await
    }

    async fn list<P: AsRef<Path> + Send + Debug>(
        &self,
        user: &Option<User>,
        path: P,
    ) -> Result<Vec<Fileinfo<PathBuf, Self::Metadata>>>
    where
        <Self as StorageBackend<User>>::Metadata: Metadata,
    {
        let path = Self::new_path(user, path.as_ref());
        self.inner.list(user, path).await
    }

    async fn list_fmt<P>(&self, user: &Option<User>, path: P) -> Result<Cursor<Vec<u8>>>
    where
        P: AsRef<Path> + Send + Debug,
        Self::Metadata: Metadata + 'static,
    {
        let path = Self::new_path(user, path.as_ref());
        self.inner.list_fmt(user, path).await
    }

    async fn nlst<P>(&self, user: &Option<User>, path: P) -> std::result::Result<Cursor<Vec<u8>>, Error>
    where
        P: AsRef<Path> + Send + Debug,
        Self::Metadata: Metadata + 'static,
    {
        let path = Self::new_path(user, path.as_ref());
        self.inner.nlst(user, path).await
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
        let path = Self::new_path(user, path.as_ref());
        self.inner.get_into(user, path, start_pos, output).await
    }

    async fn get<P: AsRef<Path> + Send + Debug>(
        &self,
        user: &Option<User>,
        path: P,
        start_pos: u64,
    ) -> Result<Box<dyn AsyncRead + Send + Sync + Unpin>> {
        let path = Self::new_path(user, path.as_ref());
        self.inner.get(user, path, start_pos).await
    }

    async fn put<P: AsRef<Path> + Send + Debug, R: tokio::io::AsyncRead + Send + Sync + Unpin + 'static>(
        &self,
        user: &Option<User>,
        input: R,
        path: P,
        start_pos: u64,
    ) -> Result<u64> {
        let path = Self::new_path(user, path.as_ref());
        self.inner.put(user, input, path, start_pos).await
    }

    async fn del<P: AsRef<Path> + Send + Debug>(&self, user: &Option<User>, path: P) -> Result<()> {
        let path = Self::new_path(user, path.as_ref());
        self.inner.del(user, path).await
    }

    async fn mkd<P: AsRef<Path> + Send + Debug>(&self, user: &Option<User>, path: P) -> Result<()> {
        let path = Self::new_path(user, path.as_ref());
        self.inner.mkd(user, path).await
    }

    async fn rename<P: AsRef<Path> + Send + Debug>(&self, user: &Option<User>, from: P, to: P) -> Result<()> {
        let from = Self::new_path(user, from.as_ref());
        let to = Self::new_path(user, to.as_ref());
        self.inner.rename(user, from, to).await
    }

    async fn rmd<P: AsRef<Path> + Send + Debug>(&self, user: &Option<User>, path: P) -> Result<()> {
        let path = Self::new_path(user, path.as_ref());
        self.inner.rmd(user, path).await
    }

    async fn cwd<P: AsRef<Path> + Send + Debug>(&self, user: &Option<User>, path: P) -> Result<()> {
        let path = Self::new_path(user, path.as_ref());
        self.inner.cwd(user, path).await
    }
}

#[cfg(test)]
mod tests {
    use crate::auth::VfsOperations;
    use pretty_assertions::assert_eq;
    use std::path::{Path, PathBuf};

    fn new_path(root: &str, requested: &str) -> PathBuf {
        super::RooterVfs::<unftp_sbe_fs::Filesystem, crate::auth::User, unftp_sbe_fs::Meta>::new_path(
            &Some(crate::auth::User {
                username: "test".to_string(),
                name: None,
                surname: None,
                account_enabled: false,
                vfs_permissions: VfsOperations::all(),
                allowed_mime_types: None,
                root: Some(PathBuf::from(root)),
            }),
            Path::new(requested),
        )
        .into()
    }

    fn new_path_no_root(requested: &str) -> PathBuf {
        super::RooterVfs::<unftp_sbe_fs::Filesystem, crate::auth::User, unftp_sbe_fs::Meta>::new_path(
            &Some(crate::auth::User {
                username: "test".to_string(),
                name: None,
                surname: None,
                account_enabled: false,
                vfs_permissions: VfsOperations::all(),
                allowed_mime_types: None,
                root: None,
            }),
            Path::new(requested),
        )
        .into()
    }

    #[test]
    fn no_user_root_case() {
        assert_eq!(
            PathBuf::from("/my/documents/test.txt"),
            new_path_no_root("/my/documents/test.txt")
        );
    }

    #[test]
    fn rooted_is_rerooted() {
        assert_eq!(
            PathBuf::from("alice/my/documents/test.txt"),
            new_path("alice", "/my/documents/test.txt")
        );
    }

    #[test]
    fn relative_is_rooted() {
        assert_eq!(
            PathBuf::from("alice/my/documents/test.txt"),
            new_path("alice", "my/documents/test.txt")
        );
    }

    #[test]
    fn cdups_is_ignored() {
        assert_eq!(
            PathBuf::from("alice/my/documents/test.txt"),
            new_path("alice", "../../my/documents/test.txt")
        );
    }

    #[test]
    fn dots_removed_and_applied() {
        assert_eq!(
            PathBuf::from("alice/documents/test.txt"),
            new_path("alice", "../../my/../.././documents/test.txt")
        );
    }

    #[test]
    fn user_root_with_trailing_slash() {
        assert_eq!(
            PathBuf::from("alice/documents/test.txt"),
            new_path("alice/", "../../my/../.././documents/test.txt")
        );
    }

    #[test]
    fn user_root_with_slashed_in_front() {
        assert_eq!(
            PathBuf::from("/alice/documents/test.txt"),
            new_path("/alice", "../../my/../.././documents/test.txt")
        );
    }
}
