# OneLink Contract Tests

Static and process-level contract tests enforcing architecture invariants, security baselines, and API schema correctness.

## Test Inventory

| Test File | Category | Description |
|---|---|---|
| `bff_openapi_frozen.rs` | BFF | Validates BFF OpenAPI freeze, version >= 1.1.0, required endpoints, schema structure |
| `bff_app_auth_contract.rs` | BFF/Auth | BFF auth passthrough, no internal secret leakage, profile PATCH auth |
| `bff_internal_auth_contract.rs` | BFF/Auth | BFF internal auth contract |
| `bff_no_internal_bypass.rs` | BFF/Auth | BFF must not bypass auth |
| `profile_patch_me_contract.rs` | Profile | PATCH /me identity binding, validation, field restrictions, mutex guard drop |
| `openapi_schemas_valid.rs` | Schema | All YAML files parse as valid YAML (>= 4) |
| `event_schemas_valid.rs` | Schema | All event JSON schemas are valid JSON (>= 8) |
| `ddl_schema_valid.rs` | Schema | DDL schema validation |
| `phase_promotion_contract.rs` | Phase | Phase promotion gates |
| `persistence_graceful_degradation.rs` | Persistence | Graceful degradation when persistence unavailable |
| `observability_auth_boundary.rs` | Security | Observability endpoints require auth |
| `runtime_auth_coverage.rs` | Security | Runtime auth coverage checks |
| `internal_auth_enforcement.rs` | Security | Internal auth enforcement |
| `internal_network_protection.rs` | Security | Internal network protection |
| `auth_dedup_contract.rs` | Security | Auth deduplication contract |
| `internal_secret_baseline.rs` | Security | Internal secret baseline checks |
| `per_service_persistence_smoke.rs` | Persistence | Per-service persistence smoke tests |
| `persistence_smoke.rs` | Persistence | Basic persistence smoke test |

## Running

```sh
cargo test -p onelink-contract-tests --workspace
```
