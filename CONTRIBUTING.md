# Contributing to UltraAPI

Thanks for considering a contribution 🚀

## Quick contribution flow

1. Fork and clone this repository
2. Create a branch: `feat/<topic>` or `fix/<topic>`
3. Make changes with tests
4. Run checks locally:
   - `cargo fmt --all`
   - `cargo clippy --all-targets --all-features -- -D warnings`
   - `cargo test`
5. Open a PR with a clear summary and impact

## What we value in PRs

- Minimal, focused diffs
- Reproducible tests (unit/integration/UI where relevant)
- Updated docs when behavior changes
- Backward compatibility considerations

## Good first contribution areas

If you want to start small, these are high-signal areas:

- Expand OpenAPI compatibility tests (golden parity additions)
- Improve docs/examples for DI and response_model behavior
- Add missing validation keyword support in schema generation
- Improve error messages in macros and extractor paths

Look for issues labeled:

- `good first issue`
- `help wanted`
- `compatibility`

## Reporting compatibility gaps

Please include:

1. Minimal FastAPI reference snippet
2. Expected OpenAPI/runtime behavior
3. Actual UltraAPI behavior
4. Full error output (if any)

## Commit / PR style

- Use imperative commit messages (e.g., `add nested include parity test`)
- Mention affected modules (`ultraapi`, `ultraapi-macros`, tests)
- In PR description, include:
  - **What changed**
  - **Why**
  - **How verified**

## Release hygiene

Before release, ensure:

- `cargo test` passes on main
- docs/compatibility-matrix.md reflects current status
- notable user-facing changes are summarized in release notes
