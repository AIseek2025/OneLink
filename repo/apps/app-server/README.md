# OneLink App Server (BFF Relay)

Rust Axum service acting as the BFF (Backend-For-Frontend) relay for all client-facing traffic.

## Role

- Sole client entry point at `/api/v1/bff/*`
- Relays requests to downstream microservices
- Handles i18n, region gating, design tokens, screen specs, and contract freeze manifest
- `/api/v1/app/*` route has been removed (Wave 0 boundary freeze)

## Commands

```bash
cargo test -p onelink-app-server     # 167 tests (127 unit + 40 integration)
cargo clippy -p onelink-app-server   # lint check
cargo run -p onelink-app-server      # start dev server
```

## BFF Boundary

All mobile and web clients must connect through `/api/v1/bff/*`. No alternative client-facing route exists.

## Key Modules

- `router.rs` — route definitions and handler implementations
- `dto.rs` — request/response DTOs aligned with BFF contract
- `i18n.rs` — internationalization registry
- `region_gate.rs` — region residency policy enforcement
- `screens.rs` — screen specification manifests
- `design_tokens.rs` — frozen design token set
- `contract_freeze.rs` — BFF contract freeze manifest
- `compliance_audit.rs` — compliance audit trail
- `config.rs` — service configuration
- `state.rs` — shared application state
