use onelink_internal_auth::{validate_secret_for_env, DEV_INTERNAL_SECRET};
use sha2::Digest;

#[test]
fn default_secret_blocked_in_staging() {
    assert!(validate_secret_for_env(DEV_INTERNAL_SECRET, "staging").is_err());
}

#[test]
fn default_secret_blocked_in_production() {
    assert!(validate_secret_for_env(DEV_INTERNAL_SECRET, "production").is_err());
}

#[test]
fn default_secret_allowed_in_dev() {
    assert!(validate_secret_for_env(DEV_INTERNAL_SECRET, "dev").is_ok());
}

#[test]
fn short_secret_blocked_in_production() {
    assert!(validate_secret_for_env("short-secret-only-20chars", "production").is_err());
}

#[test]
fn long_secret_allowed_in_production() {
    assert!(validate_secret_for_env(
        "a-very-long-secret-that-is-at-least-32-characters-long",
        "production"
    )
    .is_ok());
}

#[test]
fn session_tokens_must_be_hashed_not_stored_raw() {
    let token = "olk_00000000-0000-0000-0000-000000000001";
    let mut hasher = sha2::Sha256::new();
    sha2::Digest::update(&mut hasher, token.as_bytes());
    let hash = format!("sha256:{}", hex::encode(sha2::Digest::finalize(hasher)));
    assert!(hash.starts_with("sha256:"), "token hash must be prefixed");
    assert!(
        !hash.contains(token),
        "token hash must not contain raw token"
    );
    assert_ne!(hash, token, "stored value must differ from raw token");
}
