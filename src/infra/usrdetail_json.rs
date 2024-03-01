use crate::domain::user::{User, UserDetailError, UserDetailProvider};
use async_trait::async_trait;
use serde::Deserialize;
use std::path::PathBuf;
use unftp_sbe_restrict::VfsOperations;

/// A [`UserDetailProvider`] that gets user details from a JSON file.
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
    #[allow(dead_code)]
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

#[async_trait]
impl UserDetailProvider for JsonUserProvider {
    async fn provide_user_detail(&self, username: &str) -> Result<User, UserDetailError> {
        self.users
            .iter()
            .find(|u| u.username == username)
            .ok_or(UserDetailError::UserNotFound {
                username: String::from(username),
            })
            .map(|u| {
                let u = u.clone();
                User {
                    username: u.username,
                    name: u.name,
                    surname: u.surname,
                    account_enabled: u.account_enabled.unwrap_or(true),
                    vfs_permissions: u.vfs_perms.map_or(VfsOperations::all(), |p| {
                        p.iter()
                            .fold(VfsOperations::all(), |ops, s| match s.as_str() {
                                "none" => VfsOperations::empty(),
                                "all" => VfsOperations::all(),
                                "-mkdir" => ops - VfsOperations::MK_DIR,
                                "-rmdir" => ops - VfsOperations::RM_DIR,
                                "-del" => ops - VfsOperations::DEL,
                                "-ren" => ops - VfsOperations::RENAME,
                                "-md5" => ops - VfsOperations::MD5,
                                "-get" => ops - VfsOperations::GET,
                                "-put" => ops - VfsOperations::PUT,
                                "-list" => ops - VfsOperations::LIST,
                                "+mkdir" => ops | VfsOperations::MK_DIR,
                                "+rmdir" => ops | VfsOperations::RM_DIR,
                                "+del" => ops | VfsOperations::DEL,
                                "+ren" => ops | VfsOperations::RENAME,
                                "+md5" => ops | VfsOperations::MD5,
                                "+get" => ops | VfsOperations::GET,
                                "+put" => ops | VfsOperations::PUT,
                                "+list" => ops | VfsOperations::LIST,
                                _ => ops,
                            })
                    }),
                    allowed_mime_types: None,
                    root: u.root.map(PathBuf::from),
                }
            })
    }
}
