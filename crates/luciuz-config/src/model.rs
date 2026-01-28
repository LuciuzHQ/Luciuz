use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server: Server,
    #[serde(default)]
    pub telemetry: Telemetry,
    #[serde(default)]
    pub acme: Acme,
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
}

impl Default for Acme {
    fn default() -> Self {
        Self {
            enabled: false,
            prod: false,
            email: String::new(),
            domains: Vec::new(),
            cache_dir: default_acme_cache_dir(),
        }
    }
}

fn default_acme_cache_dir() -> String {
    "./acme-cache".to_string()
}
