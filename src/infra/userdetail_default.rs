use crate::domain::user::User;
use async_trait::async_trait;
use libunftp::auth::{Principal, UserDetailError, UserDetailProvider};

/// A default [`UserDetailProvider`] that creates a [`User`] with default settings.
#[derive(Debug)]
pub struct DefaultUserProvider {}

#[async_trait]
impl UserDetailProvider for DefaultUserProvider {
    type User = User;

    async fn provide_user_detail(&self, principal: &Principal) -> Result<User, UserDetailError> {
        Ok(User::with_defaults(&principal.username))
    }
}
