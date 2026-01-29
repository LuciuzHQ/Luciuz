use luciuz_config::Config;
use tracing_subscriber::prelude::*; // SubscriberExt::with
use tracing_subscriber::{fmt, EnvFilter};

pub fn init(cfg: &Config) {
    let level = cfg.telemetry.log_level.as_str();
    let filter = EnvFilter::try_new(level).unwrap_or_else(|_| EnvFilter::new("info"));

    if cfg.telemetry.json_logs {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().json())
            .init();
    } else {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer())
            .init();
    }
}
