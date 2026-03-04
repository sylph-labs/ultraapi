# UltraAPI ↔ FastAPI Compatibility Matrix

_Last updated: 2026-03-04_

This matrix tracks practical compatibility status for common FastAPI features.

Legend:

- ✅ Implemented and covered by tests
- 🟡 Partially implemented / semantics differ
- 🔧 In progress (active Todoist task)
- ❌ Not implemented yet

## Routing / Runtime

- ✅ HTTP route macros (`#[get]`, `#[post]`, `#[put]`, `#[delete]`)
- ✅ WebSocket route macro (`#[ws]`)
- ✅ SSE route macro (`#[sse]`) with `text/event-stream`
- ✅ Lifespan hooks (`on_startup`, `on_shutdown`)

## Validation / Modeling

- ✅ `#[api_model]` + serde-based schema generation
- ✅ Basic validation constraints and 422 responses
- 🔧 OpenAPI keyword parity expansion (`maxItems`, `uniqueItems`, `multipleOf`, ...)

## Response Modeling

- ✅ `exclude_none`
- 🟡 `exclude_unset` semantics (currently heuristic-based)
- 🟡 `exclude_defaults` semantics (currently heuristic-based)
- ✅ nested include/exclude basics and parser hardening

## Dependency Injection

- ✅ `Dep<T>`, `State<T>`, `Depends<T>`
- 🟡 `depends_with_deps` deep-chain auto-resolution parity
- 🔧 request-level dependency cache (`use_cache`-like behavior)

## OpenAPI / Docs

- ✅ `/openapi.json`, `/docs`, `/redoc`
- ✅ Header/Cookie parameter parity cases added to golden tests
- 🔧 parity coverage expansion for callbacks/webhooks/security

## Where gaps are tracked

- Primary backlog source: Todoist project `UltraAPI` (`6g3v42mhRcCq2mG7`)
- Periodic scanner job: `ultraapi-compat-gap-scan`
- Implementation runner: `ultraapi-development`

If you find a compatibility gap that is not listed, please open an issue with:

1. Minimal FastAPI example
2. Expected behavior
3. Actual UltraAPI behavior
4. Repro command/output
