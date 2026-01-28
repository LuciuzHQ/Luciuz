<div align="center">

# Luciuz
### Secure-by-default web server & reverse proxy with built-in observability and ACME HTTPS

</div>

## What is Luciuz?
Luciuz is a next-generation web server / reverse proxy focusing on:
- **Security by default**: hardened defaults and clear policy knobs.
- **Observability by default**: structured logs today; metrics/traces roadmap.
- **Built-in HTTPS**: ACME HTTP-01 support (no external certbot service).

## Status
Early development (v0.1). Current MVP:
- HTTP :80 minimal (ACME HTTP-01 challenge + redirect)
- HTTPS :443 service
- `/healthz` endpoint

## Quick start (local)
```bash
cargo build --release
./target/release/luciuz check -c luciuz.toml
./target/release/luciuz run -c luciuz.toml
