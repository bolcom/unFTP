use crate::domain::user::{User, UserDetailProvider};
use async_trait::async_trait;
use libunftp::auth::{AuthenticationError, Credentials, DefaultUser};

#[derive(Debug)]
pub struct LookupAuthenticator {
    inner: Box<dyn libunftp::auth::Authenticator<DefaultUser>>,
    usr_detail: Option<Box<dyn UserDetailProvider + Send + Sync>>,
}

impl LookupAuthenticator {
    pub fn new<A: libunftp::auth::Authenticator<DefaultUser> + Send + Sync + 'static>(
        inner: A,
    ) -> Self {
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
    async fn authenticate(
        &self,
        username: &str,
        creds: &Credentials,
    ) -> Result<User, AuthenticationError> {
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
