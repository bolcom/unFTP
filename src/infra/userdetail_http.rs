//! A libunftp [`UserDetail`](libunftp::auth::UserDetail) provider that obtains user detail
//! over HTTP.

use crate::domain::user::User;
use crate::infra::userdetail_json::JsonUserProvider;
use async_trait::async_trait;
use http::{Method, Request};
use http_body_util::{BodyExt, Empty};
use hyper::body::Bytes;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use libunftp::auth::{Principal, UserDetailError, UserDetailProvider};
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
    type User = User;

    async fn provide_user_detail(&self, principal: &Principal) -> Result<User, UserDetailError> {
        let _url_suffix: String =
            form_urlencoded::byte_serialize(principal.username.as_bytes()).collect();
        let req = Request::builder()
            .method(Method::GET)
            .header("Content-type", "application/json")
            .uri(format!("{}{}", self.url, principal.username))
            .body(Empty::<Bytes>::new())
            .map_err(|e| UserDetailError::with_source("error creating request", e))?;

        let https = hyper_rustls::HttpsConnectorBuilder::new()
            .with_native_roots()
            .expect("no native root CA certificates found")
            .https_or_http()
            .enable_http1()
            .build();

        let client = Client::builder(TokioExecutor::new()).build(https);

        let resp = client
            .request(req)
            .await
            .map_err(|e| UserDetailError::with_source("error doing HTTP request", e))?;

        let body_bytes = BodyExt::collect(resp.into_body())
            .await
            .map_err(|e| UserDetailError::with_source("error parsing body", e))?
            .to_bytes();

        let json_str = std::str::from_utf8(body_bytes.as_ref())
            .map_err(|e| UserDetailError::with_source("body is not a valid UTF string", e))?;

        let json_usr_provider =
            JsonUserProvider::from_json(json_str).map_err(UserDetailError::Generic)?;

        json_usr_provider.provide_user_detail(principal).await
    }
}
