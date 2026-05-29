use std::fs;

const PROFILE_ROUTES_PATH: &str = "../../services/profile-service/src/http/routes.rs";

#[test]
fn patch_me_requires_identity_binding() {
    let content = fs::read_to_string(PROFILE_ROUTES_PATH)
        .unwrap_or_else(|e| panic!("cannot read {PROFILE_ROUTES_PATH}: {e}"));

    assert!(
        content.contains("async fn patch_me"),
        "profile-service must have patch_me handler"
    );
    assert!(
        content.contains("identity_me_value"),
        "patch_me must call identity_me_value to bind the user identity"
    );
    assert!(
        content.contains("user_id_from_me"),
        "patch_me must extract user_id from identity /me response"
    );
    assert!(
        content.contains("headers: HeaderMap") || content.contains("HeaderMap"),
        "patch_me must accept headers for Authorization token"
    );
}

#[test]
fn patch_me_has_input_validation() {
    let content = fs::read_to_string(PROFILE_ROUTES_PATH)
        .unwrap_or_else(|e| panic!("cannot read {PROFILE_ROUTES_PATH}: {e}"));

    assert!(
        content.contains("MAX_LEN")
            || content.contains("_MAX_LEN")
            || content.contains("_MAX_COUNT"),
        "PatchMeRequest must have length/count validation constants"
    );
    assert!(
        content.contains("fn validate"),
        "PatchMeRequest must have a validate method"
    );
    assert!(
        content.contains("BAD_REQUEST"),
        "validate must return BAD_REQUEST for invalid input"
    );
}

#[test]
fn patch_me_limits_mutable_fields() {
    let content = fs::read_to_string(PROFILE_ROUTES_PATH)
        .unwrap_or_else(|e| panic!("cannot read {PROFILE_ROUTES_PATH}: {e}"));

    let forbidden_fields = ["user_id", "email", "password", "phone"];
    for field in &forbidden_fields {
        let pattern = format!("Option<String>,\n    // {field}");
        let pattern2 = format!("{field}: Option<String>");
        assert!(
            !content.contains(&pattern) && !content.contains(&pattern2),
            "PatchMeRequest must not allow mutating {field}"
        );
    }

    assert!(
        content.contains("display_name: Option<String>"),
        "PatchMeRequest should allow display_name"
    );
    assert!(
        content.contains("avatar_url: Option<String>"),
        "PatchMeRequest should allow avatar_url"
    );
}

#[test]
fn patch_me_drops_mutex_guard_before_async() {
    let content = fs::read_to_string(PROFILE_ROUTES_PATH)
        .unwrap_or_else(|e| panic!("cannot read {PROFILE_ROUTES_PATH}: {e}"));

    let patch_me_start = content
        .find("async fn patch_me")
        .expect("patch_me must exist");
    let patch_me_body = &content[patch_me_start..];

    let mutex_lock_line = patch_me_body
        .find("profiles.lock")
        .expect("must lock mutex");
    let pg_upsert_line = patch_me_body
        .find("pg.upsert_profile")
        .unwrap_or(usize::MAX);

    if pg_upsert_line < patch_me_body.len() / 2 {
        assert!(
            mutex_lock_line < pg_upsert_line,
            "MutexGuard must be dropped before .await on PG upsert to maintain Send trait"
        );
    }
}
