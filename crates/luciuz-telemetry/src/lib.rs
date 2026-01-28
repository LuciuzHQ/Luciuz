use tracing_subscriber::{fmt, EnvFilter};
use tracing_subscriber::prelude::*; // SubscriberExt::with
use luciuz_config::Config;

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
