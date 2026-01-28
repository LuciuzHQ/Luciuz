# Observability

Luciuz treats observability as a first-class feature.

## Goals
- Make production issues diagnosable quickly.
- Provide correlation across logs, metrics, and traces.
- Offer operator-friendly reports (`diagnose`) instead of raw data only.

## Logs (current)
Luciuz supports structured logging via `tracing` + `tracing-subscriber`.

Implementation note: layer composition uses `SubscriberExt` via
`tracing_subscriber::prelude::*` to enable `.with()`.

Configuration:
```toml
[telemetry]
json_logs = true
log_level = "info"
```
