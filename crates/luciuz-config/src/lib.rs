mod model;
pub use model::Config;

use luciuz_core::{error::LuciuzError, Result};

pub fn load_from_path(path: &str) -> Result<Config> {
    let raw = std::fs::read_to_string(path).map_err(|e| LuciuzError::Io(e.to_string()))?;

    let cfg: Config = toml::from_str(&raw).map_err(|e| LuciuzError::Config(e.to_string()))?;

    validate(&cfg)?;
    Ok(cfg)
}

fn validate(cfg: &Config) -> Result<()> {
    if cfg.server.http_listen.trim().is_empty() {
        return Err(LuciuzError::Config("server.http_listen is empty".into()));
    }
    if cfg.server.https_listen.trim().is_empty() {
        return Err(LuciuzError::Config("server.https_listen is empty".into()));
    }

    match cfg.server.profile.as_str() {
        "static_site" | "public_api" | "admin_panel" => {}
        other => {
            return Err(LuciuzError::Config(format!(
                "server.profile invalid: {other} (allowed: static_site|public_api|admin_panel)"
            )))
        }
    }

    if cfg.server.hsts && cfg.server.hsts_max_age == 0 {
        return Err(LuciuzError::Config(
            "server.hsts_max_age must be > 0 when hsts=true".into(),
        ));
    }

    if cfg.acme.enabled {
        if cfg.acme.domains.is_empty() {
            return Err(LuciuzError::Config(
                "acme.enabled=true but acme.domains is empty".into(),
            ));
        }
        if cfg.acme.email.trim().is_empty() {
            return Err(LuciuzError::Config(
                "acme.enabled=true but acme.email is empty".into(),
            ));
        }
    }

    if let Some(host) = &cfg.server.canonical_host {
        if host.trim().is_empty() {
            return Err(LuciuzError::Config("server.canonical_host is empty".into()));
        }
        if host.contains(' ') {
            return Err(LuciuzError::Config(
                "server.canonical_host must not contain spaces".into(),
            ));
        }
    }

    Ok(())
}
