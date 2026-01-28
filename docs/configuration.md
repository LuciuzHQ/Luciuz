# Configuration

Luciuz uses a TOML configuration file (default: `luciuz.toml`).

## Minimal config (current)
```toml
[server]
http_listen = "127.0.0.1:8080"
https_listen = "127.0.0.1:8443"
profile = "public_api"


[acme]
enabled = true
prod = false
email = "admin@example.com"
domains = ["example.com", "www.example.com"]
cache_dir = "./acme-cache"

[telemetry]
json_logs = true
log_level = "info"
```

## Validation
Use:
```bash
cargo run -p luciuz -- check -c luciuz.toml
```

## Profiles (planned)
`server.profile` will apply secure defaults depending on your use case:
- `static_site`
- `public_api`
- `admin_panel`
