//! Contains code pertaining to unFTPs HTTP service it exposes, including prometheus metrics.
use crate::{app, metrics};

use hyper::{
    service::{make_service_fn, service_fn},
    Body, Method, Request, Response, StatusCode,
};
use slog::*;
use std::{net::SocketAddr, result::Result};

// starts an HTTP server and exports Prometheus metrics.
pub async fn start(log: &Logger, bind_addr: &str) -> Result<(), String> {
    let http_addr: SocketAddr = bind_addr
        .parse()
        .map_err(|e| format!("unable to parse HTTP address {}: {}", bind_addr, e))?;

    let make_svc = make_service_fn(|_conn| {
        async {
            // service_fn converts our function into a `Service`
            Ok::<_, hyper::Error>(service_fn(metrics_service))
        }
    });

    let http_server = hyper::Server::bind(&http_addr).serve(make_svc);

    info!(log, "Starting Prometheus {} exporter.", app::NAME; "address" => &http_addr);

    if let Err(e) = http_server.await {
        error!(log, "HTTP Server error: {}", e)
    }
    Ok(())
}

async fn metrics_service(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let mut response = Response::new(Body::empty());
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/metrics") => {
            *response.body_mut() = Body::from(metrics::gather());
        }
        _ => {
            *response.status_mut() = StatusCode::NOT_FOUND;
        }
    }

    Ok(response)
}
