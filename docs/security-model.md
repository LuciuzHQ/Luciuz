# Security Model

Luciuz prioritizes security defaults to reduce operator error ("footguns").

## Core principles
1. **Secure defaults**: safe baseline configuration out-of-the-box.
2. **Explicit weakening**: any reduction in security should be intentional and visible.
3. **Least privilege**: extensions (Wasm plugins) run with minimal permissions.
4. **Resource limits**: time/memory bounds to mitigate abuse and runaway plugins.

## Default protections (planned)
- Strict timeouts (request, upstream connect/read)
- Header/body size limits
- Rate limiting (IP-based baseline)
- Safer TLS defaults (rustls, modern ciphers)
- Baseline security headers (profile-dependent)
