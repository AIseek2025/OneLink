use deadpool_postgres::{Manager, Pool, Runtime};
use tokio_postgres::NoTls;

fn database_url() -> String {
    std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://onelink:onelink-local-dev-password@127.0.0.1:5432/onelink".to_string()
    })
}

async fn make_pool(database_url: &str) -> Pool {
    let pg_cfg = database_url
        .parse::<tokio_postgres::Config>()
        .expect("DATABASE_URL must be a valid tokio-postgres config");
    let mgr = Manager::new(pg_cfg, NoTls);
    Pool::builder(mgr)
        .runtime(Runtime::Tokio1)
        .build()
        .expect("failed to build deadpool pool")
}

macro_rules! per_service_persistence_test {
    ($name:ident, $tables:expr, $svc_label:expr) => {
        #[tokio::test]
        #[ignore = "requires RUN_PERSISTENCE_SMOKE=1 and Docker Postgres with DDL applied"]
        async fn $name() {
            let url = database_url();
            let pool = make_pool(&url).await;
            let client = pool.get().await.expect("failed to get pg client");

            for table in $tables {
                let count: i64 = client
                    .query_one(&format!("SELECT COUNT(*)::bigint FROM {}", table), &[])
                    .await
                    .expect(&format!(
                        "{}: query {} failed — DDL may not be applied or table missing",
                        $svc_label, table
                    ))
                    .get(0);
                assert!(
                    count >= 0,
                    "{}: table {} must exist (got count={})",
                    $svc_label,
                    table,
                    count
                );
                eprintln!("  {} {}: {} rows", $svc_label, table, count);
            }

            eprintln!("{} persistence smoke: PASS", $svc_label);
        }
    };
}

macro_rules! per_service_insert_persistence_test {
    ($name:ident, $tables:expr, $svc_label:expr, $insert_sqls:expr, $count_table:expr) => {
        #[tokio::test]
        #[ignore = "requires RUN_PERSISTENCE_SMOKE=1 and Docker Postgres with DDL applied"]
        async fn $name() {
            let url = database_url();
            let pool = make_pool(&url).await;
            let client = pool.get().await.expect("failed to get pg client");

            for table in $tables {
                let count: i64 = client
                    .query_one(&format!("SELECT COUNT(*)::bigint FROM {}", table), &[])
                    .await
                    .expect(&format!(
                        "{}: query {} failed — DDL may not be applied or table missing",
                        $svc_label, table
                    ))
                    .get(0);
                assert!(
                    count >= 0,
                    "{}: table {} must exist (got count={})",
                    $svc_label,
                    table,
                    count
                );
                eprintln!("  {} {}: {} rows", $svc_label, table, count);
            }

            let client = pool
                .get()
                .await
                .expect("failed to get pg client for insert");
            let count_before: i64 = client
                .query_one(
                    &format!("SELECT COUNT(*)::bigint FROM {}", $count_table),
                    &[],
                )
                .await
                .unwrap()
                .get(0);

            for sql in $insert_sqls {
                client.execute(sql, &[]).await.expect(&format!(
                    "{}: insert must succeed when DDL is applied",
                    $svc_label
                ));
            }

            let count_after: i64 = client
                .query_one(
                    &format!("SELECT COUNT(*)::bigint FROM {}", $count_table),
                    &[],
                )
                .await
                .unwrap()
                .get(0);
            assert!(
                count_after > count_before,
                "{}: row count must increase after insert (before={}, after={})",
                $svc_label,
                count_before,
                count_after
            );
            eprintln!(
                "  {}: insert verified (before={}, after={})",
                $svc_label, count_before, count_after
            );

            eprintln!("{} persistence smoke: PASS", $svc_label);
        }
    };
}

per_service_insert_persistence_test!(
    identity_service_persistence_smoke,
    ["users", "identity_bindings", "sessions"],
    "identity-service",
    [
        "INSERT INTO users (status, primary_region, primary_language, timezone, password_hash, created_at, updated_at) \
         VALUES ('active', 'US', 'en', 'UTC', 'test-hash-persist-smoke', now(), now())",
    ],
    "users"
);

per_service_insert_persistence_test!(
    profile_service_persistence_smoke,
    ["profiles", "profile_facts", "profile_traits"],
    "profile-service",
    [
        "INSERT INTO users (status, primary_region, primary_language, timezone, password_hash, created_at, updated_at) \
         VALUES ('active', 'US', 'en', 'UTC', 'test-hash-persist-smoke-profile', now(), now())",
        "INSERT INTO profiles (user_id, display_name, city_level_location, updated_at) \
         SELECT id, 'persist-smoke-user', 'US', now() FROM users WHERE password_hash = 'test-hash-persist-smoke-profile' ORDER BY created_at DESC LIMIT 1",
    ],
    "profiles"
);

per_service_insert_persistence_test!(
    context_service_persistence_smoke,
    [
        "memory_artifacts",
        "memory_summaries",
        "memory_entities",
        "memory_entity_links",
        "agent_runtime_checkpoints"
    ],
    "context-service",
    [
        "INSERT INTO users (status, primary_region, primary_language, timezone, password_hash, created_at, updated_at) \
         VALUES ('active', 'US', 'en', 'UTC', 'test-hash-persist-smoke-context', now(), now())",
        "INSERT INTO memory_artifacts (user_id, network_type, evidence_type, memory_level, content, source_type, source_service, created_at, updated_at) \
         SELECT id, 'world', 'fact', 'working', 'persist-smoke-artifact-content', 'chat', 'context-service', now(), now() \
         FROM users WHERE password_hash = 'test-hash-persist-smoke-context' ORDER BY created_at DESC LIMIT 1",
    ],
    "memory_artifacts"
);

per_service_persistence_test!(
    ai_chat_service_persistence_smoke,
    ["ai_conversations", "ai_messages", "ai_message_contents"],
    "ai-chat-service"
);

per_service_persistence_test!(
    question_service_persistence_smoke,
    [
        "question_catalog",
        "question_deliveries",
        "question_answers"
    ],
    "question-service"
);

per_service_persistence_test!(
    match_service_persistence_smoke,
    [
        "find_requests",
        "recommendation_cards",
        "recommendation_feedbacks"
    ],
    "match-service"
);

per_service_persistence_test!(
    safety_service_persistence_smoke,
    [
        "report_tickets",
        "user_blocks",
        "risk_assessments",
        "moderation_actions"
    ],
    "safety-service"
);

per_service_persistence_test!(
    dm_service_persistence_smoke,
    ["dm_threads", "dm_participants", "dm_messages"],
    "dm-service"
);
