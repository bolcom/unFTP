//! Contains definitions pertaining to FTP User Detail
use async_trait::async_trait;
use libunftp::auth::UserDetail;
use slog::error;
use std::{
    fmt::{Debug, Display, Formatter},
    path::PathBuf,
};
use thiserror::Error;
use unftp_sbe_restrict::{UserWithPermissions, VfsOperations};
use unftp_sbe_rooter::UserWithRoot;

/// The unFTP user details
#[derive(Debug, PartialEq, Eq)]
pub struct User {
    pub username: String,
    pub name: Option<String>,
    pub surname: Option<String>,
    /// Tells whether this user can log in or not.
    pub account_enabled: bool,
    /// What FTP commands can the user perform
    pub vfs_permissions: VfsOperations,
    /// For some users we know they will only upload a certain type of file
    pub allowed_mime_types: Option<Vec<String>>, // TODO: Look at https://crates.io/crates/infer to do this
    /// The user's home directory relative to the storage back-end root
    pub root: Option<PathBuf>,
}

impl User {
    pub fn with_defaults(username: impl Into<String>) -> Self {
        User {
            username: username.into(),
            name: None,
            surname: None,
            account_enabled: true,
            vfs_permissions: VfsOperations::all(),
            allowed_mime_types: None,
            root: None,
        }
    }
}

impl UserDetail for User {
    fn account_enabled(&self) -> bool {
        self.account_enabled
    }
}

impl Display for User {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "User(username: {:?}, name: {:?}, surname: {:?})",
            self.username, self.name, self.surname
        )
    }
}

impl UserWithRoot for User {
    fn user_root(&self) -> Option<PathBuf> {
        self.root.clone()
    }
}

impl UserWithPermissions for User {
    fn permissions(&self) -> VfsOperations {
        self.vfs_permissions
    }
}

/// Implementation of UserDetailProvider can look up and provide FTP user account details from
/// a source.
#[async_trait]
pub trait UserDetailProvider: Debug {
    /// This will do the lookup. An error is returned if the user was not found or something else
    /// went wrong.
    async fn provide_user_detail(&self, username: &str) -> Result<User, UserDetailError>;
}

/// The error type returned by [`UserDetailProvider`]
#[derive(Debug, Error)]
pub enum UserDetailError {
    #[error("{0}")]
    Generic(String),
    #[error("user '{username:?}' not found")]
    UserNotFound { username: String },
    #[error("error getting user details: {0}: {1:?}")]
    ImplPropagated(
        String,
        #[source] Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
    ),
}

impl UserDetailError {
    /// Creates a new domain specific error
    #[allow(dead_code)]
    pub fn new(s: impl Into<String>) -> Self {
        UserDetailError::ImplPropagated(s.into(), None)
    }

    /// Creates a new domain specific error with the given source error.
    pub fn with_source<E>(s: impl Into<String>, source: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        UserDetailError::ImplPropagated(s.into(), Some(Box::new(source)))
    }
}
