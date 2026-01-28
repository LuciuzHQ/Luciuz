# Wasm Plugins

Luciuz uses WebAssembly (Wasm) to enable safe extensibility.

## Design goals
- Plugins are sandboxed and resource-limited.
- Plugins have explicit, capability-based permissions.
- The host ABI is stable and versioned.

## Hooks (planned)
- `on_request`
- `on_response`
