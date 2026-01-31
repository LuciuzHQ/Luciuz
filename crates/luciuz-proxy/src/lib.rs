use axum::{
    body::{to_bytes, Body},
    http::{header, HeaderMap, HeaderName, HeaderValue, Request, Response, StatusCode},
    routing::any,
    Router,
};
use luciuz_config::Config;
use reqwest::Client;
use std::{collections::HashSet, time::Duration};
use tracing::{info, warn};

/// Build the proxy router from config.
/// It creates explicit routes for:
/// - /api
/// - /api/
/// - /api/{*path}
pub fn router(cfg: &Config) -> anyhow::Result<Router<()>> {
    let proxy_cfg = cfg
        .proxy
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("missing [proxy] config"))?;

    let routes = proxy_cfg.routes.clone();
    info!(proxy_routes = ?routes, "proxy routes");

    let max_body: usize = if proxy_cfg.max_body_bytes == 0 {
        10 * 1024 * 1024 // 10 MB
    } else {
        proxy_cfg.max_body_bytes as usize
    };

    // A simple reqwest client for upstream calls
    let client = Client::builder().timeout(Duration::from_secs(30)).build()?;

    let mut rtr = Router::new();

    for route in routes {
        let prefix = route.prefix.trim_end_matches('/').to_string(); // "/api"
        let upstream = route.upstream.trim_end_matches('/').to_string();

        if prefix.is_empty() {
            continue;
        }

        let prefix_slash = format!("{}/", prefix.trim_end_matches('/')); // "/api/"
        let pattern = format!("{}/{{*path}}", prefix.trim_end_matches('/')); // "/api/{*path}"

        // /api
        {
            let client = client.clone();
            let upstream = upstream.clone();
            let prefix_for_strip = prefix.clone();
            let max_body = max_body;

            rtr = rtr.route(
                &prefix,
                any(move |req: Request<Body>| {
                    let client = client.clone();
                    let upstream = upstream.clone();
                    let prefix_for_strip = prefix_for_strip.clone();
                    async move {
                        proxy_one(req, client, upstream, prefix_for_strip, max_body).await
                    }
                }),
            );
        }

        // /api/
        {
            let client = client.clone();
            let upstream = upstream.clone();
            let prefix_for_strip = prefix.clone();
            let max_body = max_body;

            rtr = rtr.route(
                &prefix_slash,
                any(move |req: Request<Body>| {
                    let client = client.clone();
                    let upstream = upstream.clone();
                    let prefix_for_strip = prefix_for_strip.clone();
                    async move {
                        proxy_one(req, client, upstream, prefix_for_strip, max_body).await
                    }
                }),
            );
        }

        // /api/{*path}
        {
            let client = client.clone();
            let upstream = upstream.clone();
            let prefix_for_strip = prefix.clone();
            let max_body = max_body;

            rtr = rtr.route(
                &pattern,
                any(move |req: Request<Body>| {
                    let client = client.clone();
                    let upstream = upstream.clone();
                    let prefix_for_strip = prefix_for_strip.clone();
                    async move {
                        proxy_one(req, client, upstream, prefix_for_strip, max_body).await
                    }
                }),
            );
        }
    }

    Ok(rtr)
}

async fn proxy_one(
    req: Request<Body>,
    client: Client,
    upstream: String,
    prefix: String,
    max_body_bytes: usize,
) -> Response<Body> {
    let (parts, body) = req.into_parts();

    // IMPORTANT: we want /api => / and /api/ => /
    let orig_path = parts.uri.path();
    let rest = if prefix == "/" {
        orig_path
    } else {
        orig_path.strip_prefix(prefix.as_str()).unwrap_or(orig_path)
    };

    let rest = if rest.is_empty() { "/" } else { rest };
    let base = upstream.trim_end_matches('/');

    let mut target = format!("{base}{rest}");
    if let Some(q) = parts.uri.query() {
        target.push('?');
        target.push_str(q);
    }

    // Body (with limit)
    let bytes = match to_bytes(body, max_body_bytes).await {
        Ok(b) => b,
        Err(_) => {
            return Response::builder()
                .status(StatusCode::PAYLOAD_TOO_LARGE)
                .body(Body::from("payload too large"))
                .unwrap();
        }
    };

    // Build upstream request
    let mut rb = client.request(parts.method.clone(), target);

    let mut out_headers = filter_hop_by_hop(parts.headers);

    if let Some(host) = out_headers.get(header::HOST).and_then(|v| v.to_str().ok()) {
        out_headers.insert(
            HeaderName::from_static("x-forwarded-host"),
            HeaderValue::from_str(host).unwrap_or_else(|_| HeaderValue::from_static("invalid")),
        );
    }

    out_headers.insert(
        HeaderName::from_static("x-forwarded-proto"),
        HeaderValue::from_static("https"),
    );

    rb = rb.headers(out_headers);

    // Send
    let upstream_resp = match rb.body(bytes).send().await {
        Ok(r) => r,
        Err(err) => {
            warn!(?err, "upstream request failed");
            return Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(Body::from("bad gateway"))
                .unwrap();
        }
    };

    let status =
        StatusCode::from_u16(upstream_resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);

    let mut out = Response::builder().status(status);

    // Copy upstream response headers (filter hop-by-hop)
    if let Some(headers) = out.headers_mut() {
        for (k, v) in upstream_resp.headers().iter() {
            if is_hop_by_hop_header(k) {
                continue;
            }
            headers.append(k.clone(), v.clone());
        }

        // safety: avoid letting upstream set our security headers policy
        headers.remove(header::STRICT_TRANSPORT_SECURITY);
    }

    let body_bytes = match upstream_resp.bytes().await {
        Ok(b) => b,
        Err(_) => {
            return Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(Body::from("bad gateway"))
                .unwrap();
        }
    };

    out.body(Body::from(body_bytes)).unwrap()
}

fn filter_hop_by_hop(mut in_headers: HeaderMap) -> HeaderMap {
    // Remove hop-by-hop headers
    let hop = hop_by_hop_set();
    let keys: Vec<HeaderName> = in_headers.keys().cloned().collect();
    for k in keys {
        if hop.contains(k.as_str()) {
            in_headers.remove(&k);
        }
    }
    in_headers
}

fn hop_by_hop_set() -> HashSet<&'static str> {
    HashSet::from([
        "connection",
        "keep-alive",
        "proxy-authenticate",
        "proxy-authorization",
        "te",
        "trailer",
        "transfer-encoding",
        "upgrade",
    ])
}

fn is_hop_by_hop_header(name: &HeaderName) -> bool {
    matches!(
        name.as_str(),
        "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailer"
            | "transfer-encoding"
            | "upgrade"
    )
}
