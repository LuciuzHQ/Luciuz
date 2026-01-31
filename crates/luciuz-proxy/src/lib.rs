use axum::{
    body::{to_bytes, Body},
    http::{header, Request, Response, StatusCode},
    routing::any,
    Router,
};
use luciuz_config::Config;
use reqwest::Method;
use tracing::{error, warn};

const HOP_BY_HOP: &[header::HeaderName] = &[
    header::CONNECTION,
    header::PROXY_AUTHENTICATE,
    header::PROXY_AUTHORIZATION,
    header::TE,
    header::TRAILER,
    header::TRANSFER_ENCODING,
    header::UPGRADE,
];

pub fn router(cfg: &Config) -> anyhow::Result<Router<()>> {
    let p = cfg
        .proxy
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("missing [proxy] config"))?;

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;

    let max_body = p.max_body_bytes;

    // More specific prefixes should be added first (e.g. /api before /).
    let mut routes = p.routes.clone();
    routes.sort_by(|a, b| b.prefix.len().cmp(&a.prefix.len()));

    let mut rtr = Router::new();

    for route in routes {
        let prefix = route.prefix.clone();
        let upstream = route.upstream.clone();

        // One shared client for this route setup
        let client_base = client.clone();

        // Match both "/prefix" and "/prefix/*path"
        let pattern = if prefix == "/" {
            "/{*path}".to_string()
        } else {
            format!("{}/{{*path}}", prefix.trim_end_matches('/'))
        };

        // ---- closure for "/prefix/*path"
        let prefix_1 = prefix.clone();
        let upstream_1 = upstream.clone();
        let client_1 = client_base.clone();

        // ---- closure for "/prefix"
        let prefix_2 = prefix.clone();
        let upstream_2 = upstream.clone();
        let client_2 = client_base.clone();

        rtr = rtr
            .route(
                &pattern,
                any(move |req| {
                    proxy_one(
                        req,
                        client_1.clone(),
                        upstream_1.clone(),
                        prefix_1.clone(),
                        max_body,
                    )
                }),
            )
            .route(
                &prefix,
                any(move |req| {
                    proxy_one(
                        req,
                        client_2.clone(),
                        upstream_2.clone(),
                        prefix_2.clone(),
                        max_body,
                    )
                }),
            );
    }

    Ok(rtr)
}

async fn proxy_one(
    req: Request<Body>,
    client: reqwest::Client,
    upstream: String,
    prefix: String,
    max_body: usize,
) -> Response<Body> {
    let (parts, body) = req.into_parts();

    // Read body (MVP: buffered). If too big, return 413.
    let body_bytes = match to_bytes(body, max_body).await {
        Ok(b) => b,
        Err(_) => return (StatusCode::PAYLOAD_TOO_LARGE, "payload too large").into_response(),
    };

    let method = parts
        .method
        .as_str()
        .parse::<Method>()
        .unwrap_or(Method::GET);

    // Build target URL: upstream + (path without prefix) + query
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

    // Prepare headers (remove hop-by-hop + Host)
    let mut out_headers = reqwest::header::HeaderMap::new();
    for (k, v) in parts.headers.iter() {
        if k == header::HOST {
            continue;
        }
        if HOP_BY_HOP.contains(k) {
            continue;
        }
        if let (Ok(hname), Ok(hval)) = (
            reqwest::header::HeaderName::from_bytes(k.as_str().as_bytes()),
            reqwest::header::HeaderValue::from_bytes(v.as_bytes()),
        ) {
            out_headers.insert(hname, hval);
        }
    }

    // X-Forwarded-* (MVP)
    if let Some(host) = parts
        .headers
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
    {
        out_headers.insert("x-forwarded-host", host.parse().unwrap());
    }
    out_headers.insert("x-forwarded-proto", "https".parse().unwrap());

    let res = client
        .request(method, target)
        .headers(out_headers)
        .body(body_bytes.to_vec())
        .send()
        .await;

    let res = match res {
        Ok(r) => r,
        Err(err) => {
            warn!(?err, "upstream request failed");
            return (StatusCode::BAD_GATEWAY, "bad gateway").into_response();
        }
    };

    // Convert response back to axum Response
    let status = StatusCode::from_u16(res.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);

    let mut resp = Response::builder().status(status);

    // Copy headers (skip hop-by-hop)
    for (k, v) in res.headers().iter() {
        if HOP_BY_HOP.contains(k) {
            continue;
        }
        resp = resp.header(k, v);
    }

    let bytes = match res.bytes().await {
        Ok(b) => b,
        Err(err) => {
            error!(?err, "failed reading upstream body");
            return (StatusCode::BAD_GATEWAY, "bad gateway").into_response();
        }
    };

    resp.body(Body::from(bytes))
        .unwrap_or_else(|_| (StatusCode::BAD_GATEWAY, "bad gateway").into_response())
}

trait IntoAxumResponse {
    fn into_response(self) -> Response<Body>;
}

impl IntoAxumResponse for (StatusCode, &'static str) {
    fn into_response(self) -> Response<Body> {
        Response::builder()
            .status(self.0)
            .body(Body::from(self.1))
            .unwrap()
    }
}
