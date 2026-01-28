# Development

## Rust version (MSRV)
Luciuz targets **Rust 1.90** as its Minimum Supported Rust Version (MSRV).

## Workspace layout
Luciuz is a Cargo workspace with multiple crates under `crates/` and the main binary under `bin/luciuz`.

## Build
```bash
cargo build
```

## Run
```bash
cargo run -p luciuz -- run -c luciuz.toml
```

## Check config
```bash
cargo run -p luciuz -- check -c luciuz.toml
```
