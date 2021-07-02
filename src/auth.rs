use async_trait::async_trait;
use bitflags::bitflags;
use libunftp::auth::{AuthenticationError, Credentials, DefaultUser, UserDetail};
use serde::Deserialize;
use std::fmt::Formatter;

/// The unFTP user details
#[derive(Debug, PartialEq)]
pub struct User {
    pub username: String,
    pub name: Option<String>,
    pub surname: Option<String>,
    /// Tells whether this user can log in or not.
    pub account_enabled: bool,
    /// What FTP commands can the user perform
    pub vfs_permissions: VfsOperations,
    /// For some users we know they will only upload a certain type of file
    pub allowed_mime_types: Option<Vec<String>>,
    // Example of things we can extend with:
    // Switch the on for users that we know can/will connect with FTPS
    //pub enforce_tls: bool,
}

impl User {
    fn with_defaults(username: impl Into<String>) -> Self {
        User {
            username: username.into(),
            name: None,
            surname: None,
            account_enabled: true,
            vfs_permissions: VfsOperations::all(),
            allowed_mime_types: None,
        }
    }
}

impl UserDetail for User {
    fn account_enabled(&self) -> bool {
        self.account_enabled
    }
}

impl std::fmt::Display for User {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "User(username: {:?}, name: {:?}, surname: {:?})",
            self.username, self.name, self.surname
        )
    }
}

bitflags! {
    pub struct VfsOperations: u32 {
        const MK_DIR = 0b00000001;
        const RM_DIR = 0b00000010;
        const GET    = 0b00000100;
        const PUT    = 0b00001000;
        const DEL    = 0b00010000;
        const RENAME = 0b00100000;
        const MD5    = 0b01000000;

        const WRITE_OPS = Self::MK_DIR.bits | Self::RM_DIR.bits | Self::PUT.bits | Self::DEL.bits | Self::RENAME.bits;
    }
}

#[derive(Debug)]
pub struct LookupAuthenticator {
    inner: Box<dyn libunftp::auth::Authenticator<DefaultUser>>,
    usr_detail: Option<Box<dyn UserDetailProvider + Send + Sync>>,
}

/// Implementation of UserDetailProvider can look up and provide FTP user account details from
/// a source.
pub trait UserDetailProvider: std::fmt::Debug {
    fn provide_user_detail(&self, username: &str) -> Option<User>;
}

impl LookupAuthenticator {
    pub fn new<A: libunftp::auth::Authenticator<DefaultUser> + Send + Sync + 'static>(inner: A) -> Self {
        LookupAuthenticator {
            inner: Box::new(inner),
            usr_detail: None,
        }
    }

    pub fn set_usr_detail(&mut self, provider: Box<dyn UserDetailProvider + Send + Sync>) {
        self.usr_detail = Some(provider);
    }
}

#[async_trait]
impl libunftp::auth::Authenticator<User> for LookupAuthenticator {
    async fn authenticate(&self, username: &str, creds: &Credentials) -> Result<User, AuthenticationError> {
        self.inner.authenticate(username, creds).await?;
        let user_provider = self.usr_detail.as_ref().unwrap();
        if let Some(user) = user_provider.provide_user_detail(username) {
            Ok(user)
        } else {
            Ok(User::with_defaults(username))
        }
    }

    async fn cert_auth_sufficient(&self, username: &str) -> bool {
        self.inner.cert_auth_sufficient(username).await
    }
}

#[derive(Debug)]
pub struct DefaultUserProvider {}

impl UserDetailProvider for DefaultUserProvider {
    fn provide_user_detail(&self, username: &str) -> Option<User> {
        Some(User::with_defaults(username))
    }
}

#[derive(Debug, Deserialize)]
pub struct JsonUserProvider {
    users: Vec<UserJsonObj>,
}

#[derive(Deserialize, Clone, Debug)]
struct UserJsonObj {
    username: String,
    name: Option<String>,
    surname: Option<String>,
    vfs_perms: Option<Vec<String>>,
    allowed_mime_types: Option<Vec<String>>,
    root: Option<String>,
    account_enabled: Option<bool>,
}

impl JsonUserProvider {
    pub fn from_json(json: &str) -> std::result::Result<JsonUserProvider, String> {
        let v: Vec<UserJsonObj> = serde_json::from_str(json).map_err(|e| format!("{:?}", e))?;
        Ok(JsonUserProvider { users: v })
    }
}

impl UserDetailProvider for JsonUserProvider {
    fn provide_user_detail(&self, username: &str) -> Option<User> {
        self.users.iter().find(|u| u.username == username).map(|u| {
            let u = u.clone();
            User {
                username: u.username,
                name: u.name,
                surname: u.surname,
                account_enabled: u.account_enabled.unwrap_or(true),
                vfs_permissions: u.vfs_perms.map_or(VfsOperations::all(), |p| {
                    p.iter().fold(VfsOperations::all(), |ops, s| match s.as_str() {
                        "none" => VfsOperations::empty(),
                        "all" => VfsOperations::all(),
                        "-mkdir" => ops - VfsOperations::MK_DIR,
                        "-rmdir" => ops - VfsOperations::RM_DIR,
                        "-del" => ops - VfsOperations::DEL,
                        "-ren" => ops - VfsOperations::RENAME,
                        "-md5" => ops - VfsOperations::MD5,
                        "-get" => ops - VfsOperations::GET,
                        "-put" => ops - VfsOperations::PUT,
                        "+mkdir" => ops | VfsOperations::MK_DIR,
                        "+rmdir" => ops | VfsOperations::RM_DIR,
                        "+del" => ops | VfsOperations::DEL,
                        "+ren" => ops | VfsOperations::RENAME,
                        "+md5" => ops | VfsOperations::MD5,
                        "+get" => ops | VfsOperations::GET,
                        "+put" => ops | VfsOperations::PUT,
                        _ => ops,
                    })
                }),
                allowed_mime_types: None,
            }
        })
    }
}
