//! Contains code pertaining to unFTPs HTTP service it exposes, including prometheus metrics.
use crate::{app, metrics};

use http_body_util::combinators::UnsyncBoxBody;
use http_body_util::{Empty, Full};
use hyper::body::{Bytes, Incoming};
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use slog::*;
use std::convert::Infallible;
use std::net::{IpAddr, Ipv4Addr};
use std::{net::SocketAddr, result::Result};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt},
    net::TcpListener,
};

const PATH_HOME: &str = "/";
const PATH_METRICS: &str = "/metrics";
const PATH_HEALTH: &str = "/health";
const PATH_READINESS: &str = "/ready";

// starts an HTTP server and exports Prometheus metrics.
pub async fn start(
    log: &Logger,
    bind_addr: &str,
    ftp_addr: SocketAddr,
    mut shutdown: tokio::sync::broadcast::Receiver<()>,
    done: tokio::sync::mpsc::Sender<()>,
) -> Result<(), String> {
    let http_addr: SocketAddr = bind_addr
        .parse()
        .map_err(|e| format!("unable to parse HTTP address {}: {}", bind_addr, e))?;

    let listener = TcpListener::bind(http_addr)
        .await
        .map_err(|e| format!("unable to parse HTTP address {}: {}", bind_addr, e))?;
    let http_server =
        hyper_util::server::conn::auto::Builder::new(hyper_util::rt::TokioExecutor::new());
    let graceful = hyper_util::server::graceful::GracefulShutdown::new();

    info!(log, "Starting HTTP service."; "address" => &http_addr);
    info!(log, "Exposing {} service home.", app::NAME; "path" => PATH_HOME);
    info!(log, "Exposing Prometheus {} exporter endpoint.", app::NAME; "path" => PATH_METRICS);
    info!(log, "Exposing readiness endpoint."; "path" => PATH_READINESS);
    info!(log, "Exposing liveness endpoint."; "path" => PATH_HEALTH);

    loop {
        tokio::select! {
            conn = listener.accept() => {
                let (stream, peer_addr) = match conn {
                    Ok(conn) => conn,
                    Err(e) => {
                        error!(log, "Accept error: {}", e);
                        continue;
                    }
                };
                info!(log, "Incoming connection accepted: {}", peer_addr);

                let stream = hyper_util::rt::TokioIo::new(stream);

                let conn = http_server.serve_connection_with_upgrades(stream, service_fn(move |req: Request<Incoming>| async move {
                    let handler = HttpHandler { ftp_addr };
                    handler.router(req).await
                }));

                let conn = graceful.watch(conn.into_owned());

                let log_clone = log.clone();
                tokio::spawn(async move {
                    if let Err(err) = conn.await {
                        error!(log_clone, "connection error: {}", err);
                    }
                    debug!(log_clone, "connection dropped: {}", peer_addr);
                });
            },
            _ = shutdown.recv() => {
                drop(listener);
                info!(log, "Shutting down HTTP server");
                break;
            }
        }
    }

    info!(log, "HTTP shutdown OK");
    drop(done);
    Ok(())
}

struct HttpHandler {
    pub ftp_addr: SocketAddr,
}

impl HttpHandler {
    async fn router(
        &self,
        req: Request<Incoming>,
    ) -> Result<Response<UnsyncBoxBody<Bytes, Infallible>>, http::Error> {
        let (parts, _) = req.into_parts();

        let response = match (parts.method, parts.uri.path()) {
            (Method::GET, PATH_HOME) | (Method::GET, "/index.html") => Ok(Response::new(
                UnsyncBoxBody::new(Full::new(self.service_home())),
            )),
            (Method::GET, PATH_METRICS) => Ok(Response::new(UnsyncBoxBody::new(Full::new(
                metrics::gather().into(),
            )))),
            (Method::GET, PATH_HEALTH) => self.health().await,
            (Method::GET, PATH_READINESS) => Response::builder()
                .status(StatusCode::OK)
                .body(UnsyncBoxBody::new(Empty::<Bytes>::new())),
            _ => Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(UnsyncBoxBody::new(Empty::<Bytes>::new())),
        };

        response
    }

    fn service_home(&self) -> Bytes {
        let index_html = include_str!(concat!(env!("PROJ_WEB_DIR"), "/index.html"));
        Bytes::from(index_html.replace("{{ .AppVersion }}", app::VERSION))
    }

    async fn health(&self) -> Result<Response<UnsyncBoxBody<Bytes, Infallible>>, http::Error> {
        match self.ftp_probe().await {
            Ok(_) => Response::builder()
                .status(StatusCode::OK)
                .body(UnsyncBoxBody::new(Full::<Bytes>::from("<html>OK!</html>"))),
            Err(_e) => Response::builder()
                .status(StatusCode::SERVICE_UNAVAILABLE)
                .body(UnsyncBoxBody::new(Full::<Bytes>::from(
                    "<html>Service unavailable!</html>",
                ))),
        }
    }

    async fn ftp_probe(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let connect_to_addr = if self.ftp_addr.ip().is_unspecified() {
            SocketAddr::new(
                IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                self.ftp_addr.port(),
            )
        } else {
            self.ftp_addr
        };

        let connection = tokio::net::TcpStream::connect(connect_to_addr).await?;
        let (rx, mut tx) = tokio::io::split(connection);
        let mut reader = tokio::io::BufReader::new(rx);

        // Consume welcome message
        let mut line_buf = String::new();
        reader.read_line(&mut line_buf).await?;

        tx.write_all(b"NOOP\r\n").await?;
        line_buf.clear();
        reader.read_line(&mut line_buf).await?;

        tx.write_all(b"QUIT\r\n").await?;
        line_buf.clear();
        reader.read_line(&mut line_buf).await?;

        Ok(())
    }
}
