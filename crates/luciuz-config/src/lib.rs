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
    let http_listen_empty = cfg.server.http_listen.trim().is_empty();

    if http_listen_empty {
        let ok = cfg.acme.enabled && cfg.acme.challenge == "tls-alpn-01";
        if !ok {
            return Err(LuciuzError::Config(
                "server.http_listen is empty (required unless acme.challenge=tls-alpn-01)".into(),
            ));
        }
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

    if cfg.server.profile == "static_site" {
        let s = cfg.static_site.as_ref().ok_or_else(|| {
            LuciuzError::Config(
                "server.profile=static_site but [static_site] section is missing".into(),
            )
        })?;

        if s.root.trim().is_empty() {
            return Err(LuciuzError::Config("static_site.root is empty".into()));
        }

        if s.index.trim().is_empty() {
            return Err(LuciuzError::Config("static_site.index is empty".into()));
        }

        if let Some(cc) = &s.cache_control {
            if cc.trim().is_empty() {
                return Err(LuciuzError::Config(
                    "static_site.cache_control is empty".into(),
                ));
            }
        }
    }

    if cfg.server.profile == "public_api" {
        let p = cfg.proxy.as_ref().ok_or_else(|| {
            LuciuzError::Config("server.profile=public_api but [proxy] section is missing".into())
        })?;

        if p.routes.is_empty() {
            return Err(LuciuzError::Config("proxy.routes is empty".into()));
        }

        for r in &p.routes {
            if r.prefix.trim().is_empty() {
                return Err(LuciuzError::Config("proxy.routes[].prefix is empty".into()));
            }
            if !r.prefix.starts_with('/') {
                return Err(LuciuzError::Config(
                    "proxy.routes[].prefix must start with '/'".into(),
                ));
            }
            if r.upstream.trim().is_empty() {
                return Err(LuciuzError::Config(
                    "proxy.routes[].upstream is empty".into(),
                ));
            }
            if !(r.upstream.starts_with("http://") || r.upstream.starts_with("https://")) {
                return Err(LuciuzError::Config(
                    "proxy.routes[].upstream must start with http:// or https://".into(),
                ));
            }
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

        match cfg.acme.challenge.as_str() {
            "http-01" | "tls-alpn-01" => {}
            other => {
                return Err(LuciuzError::Config(format!(
                    "acme.challenge invalid: {other} (allowed: http-01|tls-alpn-01)"
                )));
            }
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
