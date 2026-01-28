# Roadmap

This roadmap is milestone-driven to ensure each step produces a usable artifact.

## Milestone 1: Reverse proxy + routing + JSON logs
- host/path routing
- upstream pools (round robin)
- structured per-request logging (latency, status, upstream)

## Milestone 2: TLS + ACME + atomic reload
- rustls termination with SNI
- ACME support (automatic certificate management)
- atomic reload + graceful shutdown

## Milestone 3: Metrics/Traces + Diagnose v0
- Prometheus metrics endpoint
- OpenTelemetry tracing (OTLP export)
- `luciuz diagnose` report (latency hotspots + upstream issues)

## Milestone 4: Wasm runtime + ABI v0 + example plugins
- Wasmtime runtime integration
- capability enforcement, CPU/memory limits
- example plugins: JWT auth, header rewrite, custom metrics
