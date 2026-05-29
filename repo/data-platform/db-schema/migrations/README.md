# migrations

Versioned database migrations managed by `refinery` + `tokio-postgres`.

## File Naming Convention

Files follow refinery's format: `V{version}__{name}.sql`

- `V001__identity.sql` — identity-service core tables (users, sessions, bindings)
- `V002__profile.sql` — profile-service tables (profiles, facts, traits, follows)
- `V003__context.sql` — context-service core tables (memory_artifacts, summaries, entities)
- `V003_1__context_activation.sql` — activation scoring ALTER on memory_artifacts
- `V003_2__context_idempotency.sql` — checkpoint & consolidate dedupe tables
- `V004__ai_chat.sql` — ai-chat-service tables (conversations, messages)
- `V005__dm.sql` — dm-service tables (threads, participants, messages)
- `V006__question.sql` — question-service tables (catalog, deliveries, answers)
- `V007__match.sql` — match-service tables (find_requests, recommendations)
- `V008__safety.sql` — safety-service tables (risk_assessments, reports, blocks)
- `V009__model_gateway.sql` — model-gateway invocation logs
- `V010__optimization.sql` — optimization-layer policy config tables
- `V011__runtime_observability.sql` — context routing & failure event tables

## Running Migrations

### Via refinery crate (recommended for CI and production)

```bash
# Build the migration runner
cargo build -p onelink-migration-runner

# Run against a database
DATABASE_URL="postgres://onelink:pw@localhost:5432/onelink" \
  cargo run -p onelink-migration-runner

# Or with explicit URL argument
cargo run -p onelink-migration-runner "postgres://onelink:pw@host:5432/db"
```

### Via shell script (legacy, still supported)

```bash
./scripts/apply-ddl.sh "postgres://onelink:pw@localhost:5432/onelink"
```

## Relationship to drafts/

`drafts/` contains the original DDL design drafts. `migrations/` contains the
same SQL but with refinery-compatible version prefixes. When modifying schema,
update the draft first, then copy to migrations with the next version number.

**Do not edit migration files directly** — create a new versioned migration
instead (refinery requires monotonically increasing version numbers).