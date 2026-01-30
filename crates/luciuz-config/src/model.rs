use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server: Server,
    #[serde(default)]
    pub telemetry: Telemetry,
    #[serde(default)]
    pub acme: Acme,
    pub timeouts: Option<TimeoutsConfig>,
    pub static_site: Option<StaticSite>,
    pub proxy: Option<Proxy>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Server {
    /// HTTP listen address (used for ACME HTTP-01 + redirect only in the MVP).
    #[serde(default = "default_http_listen")]
    pub http_listen: String,

    /// HTTPS listen address (the real service).
    #[serde(default = "default_https_listen")]
    pub https_listen: String,

    #[serde(default = "default_profile")]
    pub profile: String,

    /// Canonical host (e.g. "luciuz.com"). If set, any other Host redirects to it.
    #[serde(default)]
    pub canonical_host: Option<String>,

    /// Enable HSTS header on HTTPS responses only.
    #[serde(default)]
    pub hsts: bool,

    /// HSTS max-age in seconds.
    #[serde(default = "default_hsts_max_age")]
    pub hsts_max_age: u64,

    /// Add includeSubDomains directive.
    #[serde(default)]
    pub hsts_include_subdomains: bool,

    /// Add preload directive.
    #[serde(default)]
    pub hsts_preload: bool,

    /// Enable baseline security headers on HTTPS responses.
    #[serde(default = "default_security_headers")]
    pub security_headers: bool,
}

fn default_http_listen() -> String {
    "127.0.0.1:8080".to_string()
}

fn default_https_listen() -> String {
    "127.0.0.1:8443".to_string()
}

fn default_profile() -> String {
    "public_api".to_string()
}

fn default_hsts_max_age() -> u64 {
    86400
}

fn default_security_headers() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Telemetry {
    #[serde(default)]
    pub json_logs: bool,
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

fn default_log_level() -> String {
    "info".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct Acme {
    /// Enable integrated ACME.
    #[serde(default)]
    pub enabled: bool,

    /// Use Let's Encrypt production directory (staging is used when false).
    #[serde(default)]
    pub prod: bool,

    /// Contact email (used for Let's Encrypt account; must be a valid email, without `mailto:` prefix).
    #[serde(default)]
    pub email: String,

    /// Domains to request certificates for.
    #[serde(default)]
    pub domains: Vec<String>,

    /// Directory for account/cert cache.
    #[serde(default = "default_acme_cache_dir")]
    pub cache_dir: String,

    #[serde(default = "default_acme_challenge")]
    pub challenge: String,
}

impl Default for Acme {
    fn default() -> Self {
        Self {
            enabled: false,
            prod: false,
            email: String::new(),
            domains: Vec::new(),
            cache_dir: default_acme_cache_dir(),
            challenge: default_acme_challenge(),
        }
    }
}

fn default_acme_cache_dir() -> String {
    "./acme-cache".to_string()
}

fn default_acme_challenge() -> String {
    "http-01".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct TimeoutsConfig {
    #[serde(default = "default_handler_secs")]
    pub handler_secs: u64,
}

fn default_handler_secs() -> u64 {
    30
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct StaticSite {
    pub root: String,

    #[serde(default = "default_index")]
    pub index: String,

    /// Optional Cache-Control header value applied on static responses.
    /// Example: "public, max-age=3600"
    pub cache_control: Option<String>,
}

fn default_index() -> String {
    "index.html".to_string()
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Proxy {
    #[serde(default = "default_max_body_bytes")]
    pub max_body_bytes: usize,

    #[serde(default)]
    pub routes: Vec<ProxyRoute>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ProxyRoute {
    pub prefix: String,
    pub upstream: String,
}

fn default_max_body_bytes() -> usize {
    50 * 1024 * 1024 // 50 MB
}
