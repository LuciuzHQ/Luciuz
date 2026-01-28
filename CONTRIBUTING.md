# Contributing to Luciuz

Thank you for considering contributing.

## Development setup
1. Install Rust (stable) and Cargo.
2. Build:
   ```bash
   cargo build
   ```
3. Run:
   ```bash
   cargo run -p luciuz -- run -c luciuz.toml
   ```

## Commit convention
We use Conventional Commits:
- `feat:` new functionality
- `fix:` bug fixes
- `docs:` documentation
- `refactor:` refactoring without behavior change
- `perf:` performance improvements
- `test:` tests
- `chore:` maintenance tasks

Examples:
- `feat(proxy): add basic reverse proxy`
- `docs(security): document default hardening profiles`

## Coding guidelines
- Prefer safe Rust; avoid `unsafe` unless strictly necessary and documented.
- Keep public APIs minimal and well-documented.
- Add tests for parsing/validation, limits, and regressions where feasible.

## Documentation
Documentation lives in `docs/`. Update docs alongside code changes.

## Pull request checklist
- [ ] Code builds (`cargo build`)
- [ ] Documentation updated (`README.md` and/or `docs/`)
- [ ] Changelog updated for user-facing changes
