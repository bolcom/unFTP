//! A libunftp [`UserDetail`](libunftp::auth::UserDetail) provider that obtains user detail
//! over HTTP.

use crate::domain::user::{User, UserDetailError, UserDetailProvider};
use crate::infra::usrdetail_json::JsonUserProvider;
use async_trait::async_trait;
use http::{Method, Request};
use hyper::{Body, Client};
use url::form_urlencoded;

/// A libunftp [`UserDetail`](libunftp::auth::UserDetail) provider that obtains user detail
/// over HTTP.
#[derive(Debug)]
pub struct HTTPUserDetailProvider {
    url: String,
    #[allow(dead_code)]
    header_name: Option<String>,
}

impl HTTPUserDetailProvider {
    /// Creates a provider that will obtain user detail from the specified URL.
    pub fn new(url: impl Into<String>) -> HTTPUserDetailProvider {
        HTTPUserDetailProvider {
            url: url.into(),
            header_name: None,
        }
    }
}

impl Default for HTTPUserDetailProvider {
    fn default() -> Self {
        HTTPUserDetailProvider {
            url: "http://localhost:8080/users/".to_string(),
            header_name: None,
        }
    }
}

#[async_trait]
impl UserDetailProvider for HTTPUserDetailProvider {
    async fn provide_user_detail(&self, username: &str) -> Result<User, UserDetailError> {
        let _url_suffix: String = form_urlencoded::byte_serialize(username.as_bytes()).collect();
        let req = Request::builder()
            .method(Method::GET)
            .header("Content-type", "application/json")
            .uri(format!("{}{}", self.url, username))
            .body(Body::empty())
            .map_err(|e| UserDetailError::with_source("error creating request", e))?;

        let https = hyper_rustls::HttpsConnectorBuilder::new()
            .with_native_roots()
            .https_or_http()
            .enable_http1()
            .build();

        let client = Client::builder().build(https);

        let resp = client
            .request(req)
            .await
            .map_err(|e| UserDetailError::with_source("error doing HTTP request", e))?;

        let body_bytes = hyper::body::to_bytes(resp.into_body())
            .await
            .map_err(|e| UserDetailError::with_source("error parsing body", e))?;

        let json_str = std::str::from_utf8(body_bytes.as_ref())
            .map_err(|e| UserDetailError::with_source("body is not a valid UTF string", e))?;

        let json_usr_provider =
            JsonUserProvider::from_json(json_str).map_err(UserDetailError::Generic)?;

        json_usr_provider.provide_user_detail(username).await
    }
}
