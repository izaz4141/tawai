use std::convert::Infallible;
use std::io;

use bytes::Bytes;
use futures::{StreamExt, TryStreamExt};
use http_body_util::combinators::UnsyncBoxBody;
use http_body_util::{BodyExt, Full, StreamBody};
use hyper::body::{Frame, Incoming};
use hyper::header::{
    ACCEPT_ENCODING, ACCEPT_RANGES, CONTENT_ENCODING, CONTENT_LENGTH, CONTENT_RANGE, CONTENT_TYPE,
    HOST, HeaderName, HeaderValue,
};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

/// Ephemeral loopback HTTP server that forwards requests to a fixed upstream
/// URL while injecting credentials. `ffprobe` only ever sees the loopback URL,
/// so headers and signed query strings stay out of its process arguments.
pub(super) struct ProbeProxy {
    url: String,
    task: JoinHandle<()>,
}

impl ProbeProxy {
    pub(super) async fn start(
        target: &str,
        headers: &[(String, String)],
        client: reqwest::Client,
    ) -> anyhow::Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
        let address = listener.local_addr()?;
        let token = uuid::Uuid::new_v4().simple().to_string();
        let target = target.to_string();
        let headers = headers
            .iter()
            .filter_map(|(name, value)| {
                Some((
                    HeaderName::from_bytes(name.as_bytes()).ok()?,
                    HeaderValue::from_str(value).ok()?,
                ))
            })
            .collect::<Vec<_>>();

        let proxy_token = token.clone();
        let task = tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    break;
                };
                let target = target.clone();
                let headers = headers.clone();
                let token = token.clone();
                let client = client.clone();
                tokio::spawn(async move {
                    let service = service_fn(move |request| {
                        forward(
                            request,
                            target.clone(),
                            headers.clone(),
                            token.clone(),
                            client.clone(),
                        )
                    });
                    let _ = http1::Builder::new()
                        .serve_connection(TokioIo::new(stream), service)
                        .await;
                });
            }
        });

        Ok(Self {
            url: format!("http://{address}/{proxy_token}"),
            task,
        })
    }

    pub(super) fn url(&self) -> &str {
        &self.url
    }
}

impl Drop for ProbeProxy {
    fn drop(&mut self) {
        self.task.abort();
    }
}

type ProxyBody = UnsyncBoxBody<Bytes, io::Error>;

async fn forward(
    request: Request<Incoming>,
    target: String,
    headers: Vec<(HeaderName, HeaderValue)>,
    token: String,
    client: reqwest::Client,
) -> Result<Response<ProxyBody>, Infallible> {
    if request.uri().path() != format!("/{token}") {
        return Ok(text_response(StatusCode::NOT_FOUND, "not found"));
    }

    let mut upstream = client
        .request(request.method().clone(), target.as_str())
        .header(ACCEPT_ENCODING, "identity");
    for (name, value) in request.headers() {
        if is_hop_by_hop(name)
            || *name == HOST
            || *name == ACCEPT_ENCODING
            || headers.iter().any(|(configured, _)| configured == name)
        {
            continue;
        }
        upstream = upstream.header(name.clone(), value.clone());
    }
    for (name, value) in &headers {
        upstream = upstream.header(name.clone(), value.clone());
    }

    let response = match upstream.send().await {
        Ok(response) => response,
        Err(error) => {
            return Ok(text_response(
                StatusCode::BAD_GATEWAY,
                &error.without_url().to_string(),
            ));
        }
    };

    let mut builder = Response::builder().status(response.status());
    for name in [
        CONTENT_TYPE,
        CONTENT_LENGTH,
        CONTENT_RANGE,
        ACCEPT_RANGES,
        CONTENT_ENCODING,
    ] {
        if let Some(value) = response.headers().get(&name) {
            builder = builder.header(name, value.clone());
        }
    }

    let body = StreamBody::new(
        response
            .bytes_stream()
            .map_ok(Frame::data)
            .map_err(|error| io::Error::other(error.without_url().to_string()))
            .boxed(),
    );
    Ok(builder
        .body(body.boxed_unsync())
        .unwrap_or_else(|_| text_response(StatusCode::BAD_GATEWAY, "invalid response")))
}

fn is_hop_by_hop(name: &HeaderName) -> bool {
    matches!(
        name.as_str(),
        "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "proxy-connection"
            | "te"
            | "trailer"
            | "transfer-encoding"
            | "upgrade"
    )
}

fn text_response(status: StatusCode, text: &str) -> Response<ProxyBody> {
    let body = Full::new(Bytes::from(text.to_owned()))
        .map_err(|never| match never {})
        .boxed_unsync();
    Response::builder()
        .status(status)
        .header(CONTENT_TYPE, "text/plain")
        .body(body)
        .unwrap_or_else(|_| {
            Response::new(
                Full::new(Bytes::new())
                    .map_err(|never| match never {})
                    .boxed_unsync(),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn forwards_headers_and_hides_target() {
        let upstream = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let address = upstream.local_addr().unwrap();
        let target = format!("http://{address}/media?token=secret");
        let (sender, receiver) = tokio::sync::oneshot::channel();

        tokio::spawn(async move {
            let (mut socket, _) = upstream.accept().await.unwrap();
            let mut buffer = vec![0u8; 4096];
            let read = socket.read(&mut buffer).await.unwrap();
            sender
                .send(String::from_utf8_lossy(&buffer[..read]).to_lowercase())
                .unwrap();
            socket
                .write_all(
                    b"HTTP/1.1 206 Partial Content\r\nContent-Type: audio/mpeg\r\nContent-Length: 4\r\nContent-Range: bytes 0-3/100\r\nAccept-Ranges: bytes\r\n\r\nABCD",
                )
                .await
                .unwrap();
            let _ = socket.shutdown().await;
        });

        let proxy = ProbeProxy::start(
            &target,
            &[("Authorization".to_string(), "Bearer secret".to_string())],
            reqwest::Client::new(),
        )
        .await
        .unwrap();
        assert!(proxy.url().starts_with("http://127.0.0.1:"));
        assert!(!proxy.url().contains("secret"));
        assert!(!proxy.url().contains("token"));

        let response = reqwest::Client::new()
            .get(proxy.url())
            .header("Range", "bytes=0-3")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::PARTIAL_CONTENT);
        assert_eq!(response.bytes().await.unwrap().as_ref(), b"ABCD");

        let request = tokio::time::timeout(Duration::from_secs(5), receiver)
            .await
            .unwrap()
            .unwrap();
        assert!(request.starts_with("get /media?token=secret"));
        assert!(request.contains("authorization: bearer secret"));
        assert!(request.contains("range: bytes=0-3"));
    }

    #[tokio::test]
    async fn rejects_unknown_paths() {
        let proxy = ProbeProxy::start("http://127.0.0.1:1/never", &[], reqwest::Client::new())
            .await
            .unwrap();
        let address = proxy.url().split('/').nth(2).unwrap().to_string();
        let response = reqwest::Client::new()
            .get(format!("http://{address}/wrong"))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);
    }
}
