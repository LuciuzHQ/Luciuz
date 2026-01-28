# Architecture

Luciuz is structured as a Rust workspace with focused crates. The architecture separates:
- **Data plane**: high-performance request handling path.
- **Control plane**: configuration validation, reload orchestration, and diagnostics.

## High-level components
Planned crates:
- `luciuz-config`: TOML parsing + validation + profiles
- `luciuz-telemetry`: structured logging + metrics + tracing
- `luciuz-proxy`: routing + reverse proxy + upstream pools
- `luciuz-tls`: rustls + ACME + certificate storage + hot reload
- `luciuz-policy`: default limits, rate limiting, headers, timeouts
- `luciuz-wasm`: Wasmtime-based plugin runtime + ABI + capabilities
- `luciuz-control`: local control API (reload/rollback/diagnose)

## Request pipeline (target)
1. Accept connection
2. TLS termination (optional during early milestones)
3. HTTP parsing
4. Route match (host/path)
5. Apply security profile + policies
6. Wasm `on_request` hook(s)
7. Reverse proxy to upstream pool
8. Wasm `on_response` hook(s)
9. Emit telemetry (logs/metrics/traces)

## Configuration reload (target)
Reloads should be:
- **atomic**: new config becomes active at once
- **safe**: invalid config is rejected
- **non-disruptive**: existing connections finish gracefully
