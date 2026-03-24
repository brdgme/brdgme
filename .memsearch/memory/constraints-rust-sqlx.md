# Rust and SQLx Constraints

## Rust Edition and Crate Versions

All crates use edition `2024`.

New operator-style crates need these specific versions (kube 3.x is incompatible with schemars 0.8):
- `kube = "3"`
- `k8s-openapi = { version = "0.27", features = ["latest"] }`
- `schemars = "1"`

## SQLx Workflow

Queries need cached metadata in `rust/web/.sqlx/`. After any query change:

1. Run `sqlx migrate run`
2. Run `cargo sqlx prepare -- --features ssr` from `rust/web/`

The operator uses dynamic queries (no metadata cache needed).

## GameVersion CRDs

One CR per deployed game service version. `is_deprecated: true` keeps the service running for in-progress games but blocks new game creation. `lost-cities-1` is deprecated; `lost-cities-2` is current.

## Sessions

`tower-sessions-sqlx-store` with PostgreSQL. Sessions are persistent across restarts. `SECURE_COOKIE=true` must be set in production.
