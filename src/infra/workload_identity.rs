//! This module implements authentication through Google's [workload identity](https://cloud.google.com/kubernetes-engine/docs/how-to/workload-identity)
//!
//! Call the request_token method to obtain the token.
//!
use http::StatusCode;
use hyper::client::connect::dns::GaiResolver;
use hyper::client::HttpConnector;
use hyper::http::header;
use hyper::{Body, Client, Method, Request, Response};
use hyper_rustls::HttpsConnector;
use thiserror::Error;

// See https://github.com/mechiru/gcemeta/blob/master/src/metadata.rs
// See https://github.com/mechiru/gouth/blob/master/gouth/src/source/metadata.rs

// Environment variable specifying the GCE metadata hostname.
// If empty, the default value of `METADATA_IP` is used instead.
// const METADATA_HOST_VAR: &str = "GCE_METADATA_HOST";

// Documented metadata server IP address.
// const METADATA_IP: &str = "169.254.169.254";

// When is using the IP better?
const METADATA_HOST: &str = "metadata.google.internal";

// `github.com/bolcom/libunftp v{package_version}`
const USER_AGENT: &str = concat!("github.com/bolcom/unFTP v", env!("BUILD_VERSION"));

/// Defines the errors that can be encountered when `request_token` is called.
#[derive(Error, Debug)]
pub enum Error {
    /// Something went wrong with the HTTP request
    #[error("request error: {0}: {1:?}")]
    Request(
        String,
        #[source] Box<dyn std::error::Error + Send + Sync + 'static>,
    ),

    /// Access was denied when trying to retrieve the token
    #[error("access denied")]
    AccessDenied,

    /// An unexpected HTTP status code was returned when trying to retrieve the token
    #[error("unexpected HTTP result code {0}")]
    UnexpectedHttpResult(http::StatusCode),
}

// TODO: Cache the token.
pub(super) async fn request_token(
    service: Option<String>,
    client: Client<HttpsConnector<HttpConnector<GaiResolver>>>,
) -> Result<TokenResponse, Error> {
    // Does same as curl -s -HMetadata-Flavor:Google http://metadata.google.internal/computeMetadata/v1/instance/service-accounts/default/token
    let suffix = format!(
        "instance/service-accounts/{}/token",
        service.unwrap_or_else(|| "default".to_string())
    );
    //let host = env::var(METADATA_HOST_VAR).unwrap_or_else(|_| METADATA_IP.into());
    let host = METADATA_HOST;
    let uri = format!("http://{}/computeMetadata/v1/{}", host, suffix);

    let request = Request::builder()
        .uri(uri)
        .header("Metadata-Flavor", "Google")
        .header(header::USER_AGENT, USER_AGENT)
        .method(Method::GET)
        .body(Body::empty())
        .map_err(|e| Error::Request("error building request".to_owned(), Box::new(e)))?;

    let response: Response<Body> = client
        .request(request)
        .await
        .map_err(|e| Error::Request("error sending request".to_owned(), Box::new(e)))?;

    match response.status() {
        StatusCode::OK => {}
        StatusCode::FORBIDDEN | StatusCode::UNAUTHORIZED => return Err(Error::AccessDenied),
        code => return Err(Error::UnexpectedHttpResult(code)),
    }

    let body_bytes = hyper::body::to_bytes(response.into_body())
        .await
        .map_err(|e| Error::Request("error getting response body".to_owned(), Box::new(e)))?;

    let unmarshall_result: serde_json::Result<TokenResponse> =
        serde_json::from_slice(body_bytes.to_vec().as_slice());
    unmarshall_result
        .map_err(|e| Error::Request("error unmarshalling response body".to_owned(), Box::new(e)))
}

// Example:
// ```
// {
//   "access_token": "ya29.c.Ks0Cywchw6EJei_7ifQZKV....oRZy70M2ahRMfHY1qzUxGfxQcQ1cQ",
//   "expires_in": 3166,
//   "token_type": "Bearer"
// }
// ```
//
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub(super) struct TokenResponse {
    pub(super) token_type: String,
    pub(super) access_token: String,
    pub(super) expires_in: u64,
}
