use async_trait::async_trait;
use libunftp::auth::{DefaultUser, UserDetail, AuthenticationError};
use std::fmt::Formatter;

#[derive(Debug, PartialEq)]
pub struct User {
    pub username: String,
    pub name: Option<String>,
    pub surname: Option<String>,
    pub account_enabled: bool,
    // Example of things we can extend with:
    // Switch the on for users that we know can/will connect with FTPS
    //pub enforce_tls: bool,
    // pub company: Option<String>,
    // // Specify this if we know the IP the user will connect from.
    // pub expected_source_ip: Option<String>,
    // // Max allowed storage for the user
    // pub quota: Option<u64>,
    // // For some users we know they will only upload a certain type of file
    // pub allowed_mime_types: std::vec::Vec<String>,
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

#[derive(Debug)]
pub struct LookupAuthenticator {
    inner: Box<dyn libunftp::auth::Authenticator<DefaultUser>>,
}

impl LookupAuthenticator {
    pub fn new<A: libunftp::auth::Authenticator<DefaultUser> + Send + Sync + 'static>(inner: A) -> Self {
        LookupAuthenticator { inner: Box::new(inner) }
    }
}

#[async_trait]
impl libunftp::auth::Authenticator<User> for LookupAuthenticator {
    async fn authenticate(
        &self,
        username: &str,
        password: &str,
    ) -> Result<User, AuthenticationError> {
        self.inner.authenticate(username, password).await?;
        // TODO: User successfully authenticated, now lookup user details from repository e.g. PostgreSql
        Ok(User {
            username: username.into(),
            name: Some("unFTP".into()),
            surname: Some("User".into()),
            account_enabled: true,
        })
    }
}
