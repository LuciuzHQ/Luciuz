use std::net::SocketAddr;

use axum::error_handling::HandleErrorLayer;
use axum::{
    body::Body,
    extract::State,
    http::{
        header::{
            HeaderName, ALLOW, HOST, REFERRER_POLICY, STRICT_TRANSPORT_SECURITY,
            X_CONTENT_TYPE_OPTIONS, X_FRAME_OPTIONS,
        },
        HeaderMap, HeaderValue, Method, Request, StatusCode, Uri,
    },
    middleware::{from_fn_with_state, Next},
    response::{IntoResponse, Redirect, Response},
    routing::get,
    Router,
};
use clap::{Parser, Subcommand};
use std::time::Duration;
use tower::timeout::TimeoutLayer;
use tower::{BoxError, ServiceBuilder};
use tracing::{info, warn};

static COOP: HeaderName = HeaderName::from_static("cross-origin-opener-policy");
static CORP: HeaderName = HeaderName::from_static("cross-origin-resource-policy");

#[derive(Parser, Debug)]
#[command(name = "luciuz", version, about = "Luciuz web server (next-gen)")]
struct Cli {
    #[command(subcommand)]
    cmd: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Validate config and print key effective values
    Check {
        #[arg(short, long, default_value = "luciuz.toml")]
        config: String,
    },
    /// Run server
    Run {
        #[arg(short, long, default_value = "luciuz.toml")]
        config: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let cli = Cli::parse();

    match cli.cmd {
        Command::Check { config } => {
            let cfg = luciuz_config::load_from_path(&config).map_err(|e| anyhow::anyhow!(e))?;
            luciuz_telemetry::init(&cfg);

            info!("config ok");
            if let Some(p) = &cfg.proxy {
                info!(proxy_routes = ?p.routes, "proxy routes");
            }
            info!(
                http_listen = %cfg.server.http_listen,
                https_listen = %cfg.server.https_listen,
                profile = %cfg.server.profile,
                acme_enabled = cfg.acme.enabled,
                acme_prod = cfg.acme.prod,
                acme_domains = ?cfg.acme.domains,
                acme_cache_dir = %cfg.acme.cache_dir,
                "effective config"
            );
            Ok(())
        }
        Command::Run { config } => {
            let cfg = luciuz_config::load_from_path(&config).map_err(|e| anyhow::anyhow!(e))?;
            luciuz_telemetry::init(&cfg);
            if cfg.acme.enabled
                && cfg.acme.challenge == "http-01"
                && cfg.server.http_listen.trim().is_empty()
            {
                return Err(anyhow::anyhow!(
                    "server.http_listen is empty but acme.challenge=http-01 requires port 80"
                ));
            }
            let http_addr: Option<SocketAddr> = if cfg.server.http_listen.trim().is_empty() {
                None
            } else {
                Some(cfg.server.http_listen.parse()?)
            };
            let https_addr: SocketAddr = cfg.server.https_listen.parse()?;

            let app: axum::Router<()> = if cfg.server.profile == "public_api" {
                let proxy_router = luciuz_proxy::router(&cfg)?;
                Router::new()
                    .route("/healthz", get(|| async { "ok" }))
                    .route(
                        "/",
                        get(|| async {
                            axum::response::Html(
                                r#"<!doctype html>
            <html lang="en">
            <head>
              <meta charset="utf-8">
              <meta name="viewport" content="width=device-width, initial-scale=1">
              <title>Luciuz</title>
            </head>
            <body>
              <h1>Luciuz is running</h1>
              <ul>
                <li><a href="/healthz">/healthz</a></li>
                <li><a href="/api/">/api/</a></li>
              </ul>
            </body>
            </html>"#,
                            )
                        }),
                    )
                    .route(
                        "/version",
                        get(|| async {
                            (
                                StatusCode::OK,
                                format!("luciuz {}\n", env!("CARGO_PKG_VERSION")),
                            )
                        }),
                    )
                    .merge(proxy_router)
            } else {
                Router::new().route("/healthz", get(|| async { "ok" }))
            };
            if let Some(p) = &cfg.proxy {
                info!(proxy_routes = ?p.routes, "proxy routes");
            }

            info!(
                http_listen = %cfg.server.http_listen,
                https_listen = %cfg.server.https_listen,
                profile = %cfg.server.profile,
                acme_enabled = cfg.acme.enabled,
                "starting luciuz"
            );

            if cfg.acme.enabled {
                run_https_with_acme_http01(cfg, http_addr, https_addr, app).await?;
            } else {
                // Plain HTTP only (dev mode / debugging).
                if let Some(http_addr) = http_addr {
                    let listener = tokio::net::TcpListener::bind(http_addr).await?;
                    axum::serve(listener, app).await?;
                } else {
                    tracing::info!("HTTP listener disabled (server.http_listen is empty)");
                }
            }

            warn!("server stopped");
            Ok(())
        }
    }
}

#[derive(Clone)]
struct CanonicalHost {
    canonical: String,
    www: String,
}

async fn canonical_host_mw(
    State(state): State<CanonicalHost>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let canonical = state.canonical.as_str();
    let www = state.www.as_str();

    // Host header (strip optional port)
    let host = req
        .headers()
        .get(HOST)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(':').next().unwrap_or(s));

    match host {
        Some(h) if h == canonical => {
            // ok
        }
        Some(h) if h == www => {
            // www -> apex
            let path = req
                .uri()
                .path_and_query()
                .map(|pq| pq.as_str())
                .unwrap_or(req.uri().path());

            let target = format!("https://{canonical}{path}");
            return Redirect::permanent(&target).into_response();
        }
        Some(h) => {
            // Secure-by-default: refuse unknown hosts on the HTTPS listener.
            warn!(host = %h, uri = %req.uri(), "rejecting request for unknown host");
            return axum::http::StatusCode::MISDIRECTED_REQUEST.into_response();
        }
        None => {
            warn!(uri = %req.uri(), "rejecting request without Host header");
            return axum::http::StatusCode::BAD_REQUEST.into_response();
        }
    }

    next.run(req).await
}

#[derive(Clone)]
struct HstsState {
    value: HeaderValue,
}

async fn hsts_mw(State(state): State<HstsState>, req: Request<Body>, next: Next) -> Response {
    let mut res = next.run(req).await;
    res.headers_mut()
        .insert(STRICT_TRANSPORT_SECURITY, state.value.clone());
    res
}

#[derive(Clone)]
struct SecurityHeadersState {
    x_content_type_options: HeaderValue,
    referrer_policy: HeaderValue,
    x_frame_options: HeaderValue,
    coop: HeaderValue,
    corp: HeaderValue,
}

async fn security_headers_mw(
    State(state): State<SecurityHeadersState>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let mut res = next.run(req).await;

    // Only set if not already present (allows app/routes to override).
    let h = res.headers_mut();

    if !h.contains_key(X_CONTENT_TYPE_OPTIONS) {
        h.insert(X_CONTENT_TYPE_OPTIONS, state.x_content_type_options.clone());
    }
    if !h.contains_key(REFERRER_POLICY) {
        h.insert(REFERRER_POLICY, state.referrer_policy.clone());
    }
    if !h.contains_key(X_FRAME_OPTIONS) {
        h.insert(X_FRAME_OPTIONS, state.x_frame_options.clone());
    }
    if !h.contains_key(&COOP) {
        h.insert(COOP.clone(), state.coop.clone());
    }
    if !h.contains_key(&CORP) {
        h.insert(CORP.clone(), state.corp.clone());
    }

    res
}

#[derive(Clone)]
struct RedirectState {
    canonical_host: Option<String>,
}

#[derive(Clone)]
struct HttpGuardState {
    canonical: String,
    www: String,
}

fn method_not_allowed() -> Response {
    Response::builder()
        .status(StatusCode::METHOD_NOT_ALLOWED)
        .header(ALLOW, "GET, HEAD")
        .body(Body::empty())
        .unwrap()
}

async fn http_guard_mw(
    State(state): State<HttpGuardState>,
    req: Request<Body>,
    next: Next,
) -> Response {
    // Only allow GET/HEAD on port 80 (ACME + redirect only).
    let m = req.method();
    if *m != Method::GET && *m != Method::HEAD {
        warn!(method = %m, uri = %req.uri(), "rejecting non-GET/HEAD on http");
        return method_not_allowed();
    }

    // Host allowlist (strip optional port)
    let host = req
        .headers()
        .get(HOST)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(':').next().unwrap_or(s));

    match host {
        Some(h) if h == state.canonical || h == state.www => {
            // ok
        }
        Some(h) => {
            warn!(host = %h, uri = %req.uri(), "rejecting request for unknown host on http");
            return StatusCode::MISDIRECTED_REQUEST.into_response(); // 421
        }
        None => {
            warn!(uri = %req.uri(), "rejecting request without Host header on http");
            return StatusCode::BAD_REQUEST.into_response(); // 400
        }
    }

    next.run(req).await
}

async fn run_https_with_acme_http01(
    cfg: luciuz_config::Config,
    http_addr: Option<SocketAddr>,
    https_addr: SocketAddr,
    https_app: Router,
) -> Result<(), anyhow::Error> {
    use axum_server::bind;
    use rustls_acme::caches::DirCache;
    use rustls_acme::tower::TowerHttp01ChallengeService;
    use rustls_acme::AcmeConfig;
    use rustls_acme::UseChallenge::{Http01, TlsAlpn01};
    use tokio_stream::StreamExt;

    // --- ACME state
    let challenge = match cfg.acme.challenge.as_str() {
        "http-01" => Http01,
        "tls-alpn-01" => TlsAlpn01,
        other => {
            return Err(anyhow::anyhow!(
                "acme.challenge invalid: {other} (allowed: http-01|tls-alpn-01)"
            ));
        }
    };

    let mut state = AcmeConfig::new(cfg.acme.domains.clone())
        .contact_push(format!("mailto:{}", cfg.acme.email))
        .cache(DirCache::new(cfg.acme.cache_dir))
        .directory_lets_encrypt(cfg.acme.prod)
        .challenge_type(challenge)
        .state();

    // Rustls acceptor for axum-server.
    let acceptor = state.axum_acceptor(state.default_rustls_config());

    let http01_service: Option<TowerHttp01ChallengeService> = if cfg.acme.challenge == "http-01" {
        Some(state.http01_challenge_tower_service())
    } else {
        None
    };

    // Log ACME events in the background.
    tokio::spawn(async move {
        loop {
            match state.next().await {
                Some(Ok(evt)) => tracing::info!(?evt, "acme event"),
                Some(Err(err)) => tracing::error!(?err, "acme error"),
                None => break,
            }
        }
    });

    let canonical = cfg.server.canonical_host.clone();

    // --- HTTPS: apply canonical host redirect (www -> apex)
    let https_app = if let Some(ch) = canonical.clone() {
        let state = CanonicalHost {
            www: format!("www.{ch}"),
            canonical: ch,
        };
        https_app.layer(from_fn_with_state(state, canonical_host_mw))
    } else {
        https_app
    };

    // --- HTTPS: HSTS (HTTPS only)
    let https_app = if cfg.server.hsts {
        let mut v = format!("max-age={}", cfg.server.hsts_max_age);
        if cfg.server.hsts_include_subdomains {
            v.push_str("; includeSubDomains");
        }
        if cfg.server.hsts_preload {
            v.push_str("; preload");
        }

        let hv =
            HeaderValue::from_str(&v).map_err(|_| anyhow::anyhow!("invalid HSTS header value"))?;

        https_app.layer(from_fn_with_state(HstsState { value: hv }, hsts_mw))
    } else {
        https_app
    };

    let https_app = if cfg.server.security_headers {
        let state = SecurityHeadersState {
            x_content_type_options: HeaderValue::from_static("nosniff"),
            referrer_policy: HeaderValue::from_static("strict-origin-when-cross-origin"),
            x_frame_options: HeaderValue::from_static("DENY"),
            coop: HeaderValue::from_static("same-origin"),
            corp: HeaderValue::from_static("same-site"),
        };

        https_app.layer(from_fn_with_state(state, security_headers_mw))
    } else {
        https_app
    };

    let http_app = if cfg.acme.challenge == "http-01" {
        let acme_challenge_service =
            http01_service.expect("http-01 selected but http01_service was not initialized");

        Router::new()
            .route_service(
                "/.well-known/acme-challenge/{challenge_token}",
                acme_challenge_service,
            )
            .fallback(get(http_to_https_redirect))
            .with_state(RedirectState {
                canonical_host: canonical.clone(),
            })
    } else {
        Router::new()
            .fallback(get(http_to_https_redirect))
            .with_state(RedirectState {
                canonical_host: canonical.clone(),
            })
    };

    // Apply HTTP guard only when canonical host is configured
    let http_app = if let Some(ch) = canonical.clone() {
        let state = HttpGuardState {
            www: format!("www.{ch}"),
            canonical: ch,
        };
        http_app.layer(from_fn_with_state(state, http_guard_mw))
    } else {
        http_app
    };

    let handler_timeout_secs = cfg.timeouts.as_ref().map(|t| t.handler_secs).unwrap_or(30);

    let https_app = https_app.layer(
        ServiceBuilder::new()
            .layer(HandleErrorLayer::new(|err: BoxError| async move {
                if err.is::<tower::timeout::error::Elapsed>() {
                    warn!("request timed out");
                    return (StatusCode::GATEWAY_TIMEOUT, "").into_response();
                }
                // fallback
                (StatusCode::INTERNAL_SERVER_ERROR, "").into_response()
            }))
            .layer(TimeoutLayer::new(Duration::from_secs(handler_timeout_secs))),
    );

    // --- Servers
    let https_future = bind(https_addr)
        .acceptor(acceptor)
        .serve(https_app.into_make_service_with_connect_info::<SocketAddr>());

    if let Some(http_addr) = http_addr {
        // Si on est en http-01, le port 80 doit exister (sinon on ne peut pas valider).
        if cfg.acme.challenge == "http-01" {
            // http_app déjà construit plus haut (ACME+redirect)
        }
        let http_future = bind(http_addr).serve(http_app.into_make_service());
        tokio::try_join!(https_future, http_future)?;
    } else {
        // 443-only: on ne lance que HTTPS
        if cfg.acme.challenge == "http-01" {
            return Err(anyhow::anyhow!(
                "server.http_listen is empty but acme.challenge=http-01 requires port 80"
            ));
        }
        https_future.await?;
    }

    Ok(())
}

async fn http_to_https_redirect(
    State(state): State<RedirectState>,
    uri: Uri,
    headers: HeaderMap,
) -> Redirect {
    let host = headers
        .get(HOST)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(':').next().unwrap_or(s))
        .unwrap_or("luciuz.com");

    let path = uri
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or(uri.path());

    let target_host = state.canonical_host.as_deref().unwrap_or(host);

    let target = format!("https://{target_host}{path}");
    Redirect::permanent(&target)
}
