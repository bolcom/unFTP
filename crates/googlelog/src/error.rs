use thiserror;

use reqwest::{self, StatusCode};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Failed to read the 'default_labels' object from the JSON file, does it exist by this name?. Parse error: {0}")]
    DefaultLabelsError(serde_json::Error),
    #[error("Failed to read the 'resource_labels' object from the JSON file, does it exist by this name?. Parse error: {0}")]
    ResourceLabelsError(serde_json::Error),
    #[error("Serde JSON serialization failed with context '{context}'. Error: {source}")]
    ShipperSerializeError {
        context: String,
        source: serde_json::Error,
    },
    #[error("Reqwest error with context '{context}'. Error: {source}")]
    ShipperReqwestError {
        context: String,
        source: reqwest::Error,
    },
    #[error("No 'access_token' found in the metadata server response body")]
    ShipperTokenNotFound,
    #[error("No 'expires_in' found in the metadata server response body")]
    ShipperTokenExpiryNotFound,
    #[error("unsuccessful HTTP response error with context '{context}'. HTTP status code: '{status}', body: '{body}'")]
    HttpResponseError {
        context: String,
        status: StatusCode,
        body: String,
    },
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Self::ShipperReqwestError {
            context: "Error sending HTTP request".to_string(),
            source: err,
        }
    }
}
