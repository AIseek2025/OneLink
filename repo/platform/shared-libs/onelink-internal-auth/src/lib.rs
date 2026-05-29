pub const DEV_INTERNAL_SECRET: &str = "onelink-dev-internal-token";
pub const INTERNAL_TOKEN_HEADER: &str = "x-internal-token";

use axum::http::{HeaderMap, Request, StatusCode};
use subtle::ConstantTimeEq;

pub fn verify_internal_token(headers: &HeaderMap, expected: &str) -> Result<(), StatusCode> {
    if expected.is_empty() {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    let provided = headers
        .get(INTERNAL_TOKEN_HEADER)
        .and_then(|v| v.to_str().ok());
    let ok = match provided {
        Some(p) => p.as_bytes().ct_eq(expected.as_bytes()).into(),
        None => false,
    };
    if ok {
        Ok(())
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

pub fn verify_internal_token_from_request<B>(
    req: &Request<B>,
    expected: &str,
) -> Result<(), StatusCode> {
    verify_internal_token(req.headers(), expected)
}

pub fn validate_secret_for_env(secret: &str, env_mode: &str) -> Result<(), String> {
    if env_mode != "dev" && secret == DEV_INTERNAL_SECRET {
        return Err(format!(
            "FATAL: default INTERNAL_SHARED_SECRET '{}' is forbidden in non-dev environment (ONELINK_ENV={}). Set INTERNAL_SHARED_SECRET to a strong value.",
            DEV_INTERNAL_SECRET, env_mode
        ));
    }
    if env_mode != "dev" && secret.len() < 32 {
        return Err(format!(
            "FATAL: INTERNAL_SHARED_SECRET too short ({}) for non-dev environment (ONELINK_ENV={}). Minimum 32 characters.",
            secret.len(), env_mode
        ));
    }
    Ok(())
}

pub fn validate_env_mode_explicit(env_mode: &str) -> Result<(), String> {
    let raw = std::env::var("ONELINK_ENV").unwrap_or_default();
    if raw.is_empty() && env_mode == "dev" {
        tracing::warn!(
            "ONELINK_ENV is not explicitly set — defaulting to 'dev'. \
             In staging/production deployments, ONELINK_ENV MUST be explicitly set \
             to avoid security misclassification."
        );
    }
    Ok(())
}

pub mod observability_ip_allowlist {
    use axum::{
        extract::{ConnectInfo, Request},
        http::StatusCode,
        middleware::Next,
        response::{IntoResponse, Response},
    };
    use std::net::{IpAddr, SocketAddr};

    fn is_loopback(ip: IpAddr) -> bool {
        match ip {
            IpAddr::V4(v4) => v4.is_loopback(),
            IpAddr::V6(v6) => v6.is_loopback(),
        }
    }

    fn extract_client_ip(request: &Request) -> Option<IpAddr> {
        if let Some(xff) = request.headers().get("x-forwarded-for") {
            if let Ok(val) = xff.to_str() {
                if let Some(first) = val.split(',').next() {
                    if let Ok(ip) = first.trim().parse::<IpAddr>() {
                        return Some(ip);
                    }
                }
            }
        }
        if let Some(ConnectInfo(addr)) = request.extensions().get::<ConnectInfo<SocketAddr>>() {
            return Some(addr.ip());
        }
        None
    }

    fn env_mode_from_request(_request: &Request) -> String {
        let raw = std::env::var("ONELINK_ENV")
            .unwrap_or_default()
            .to_lowercase();
        if raw.is_empty() {
            tracing::warn!("ONELINK_ENV is not set — defaulting to 'dev'. In production/staging deployments, ONELINK_ENV MUST be explicitly set to avoid security misclassification.");
            "dev".to_string()
        } else {
            raw
        }
    }

    pub fn require_explicit_env_mode() -> Result<(), String> {
        let raw = std::env::var("ONELINK_ENV").unwrap_or_default();
        if raw.is_empty() {
            return Err("ONELINK_ENV is not set. In non-dev deployments, ONELINK_ENV MUST be explicitly set (e.g. 'staging' or 'production') to avoid security misclassification of the IP allowlist middleware.".to_string());
        }
        Ok(())
    }

    pub async fn ip_allowlist_layer(request: Request, next: Next) -> Response {
        let ip = extract_client_ip(&request);
        let env_mode = env_mode_from_request(&request);
        match ip {
            Some(ip) if is_loopback(ip) => next.run(request).await,
            Some(ip) => {
                tracing::warn!(client_ip = %ip, "observability endpoint accessed from non-loopback IP — denied");
                StatusCode::FORBIDDEN.into_response()
            }
            None if env_mode == "dev" => {
                tracing::debug!("observability endpoint: client IP undetermined in dev mode — allowed (in-process/test scenario)");
                next.run(request).await
            }
            None => {
                tracing::warn!("observability endpoint: client IP undetermined in non-dev mode (ONELINK_ENV={}) — denied", env_mode);
                StatusCode::FORBIDDEN.into_response()
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use axum::{body::Body, routing::get, Router};
        use tokio::sync::Mutex;
        use tower::ServiceExt;

        static ENV_LOCK: Mutex<()> = Mutex::const_new(());

        #[test]
        fn loopback_ipv4_is_allowed() {
            assert!(is_loopback("127.0.0.1".parse::<IpAddr>().unwrap()));
        }

        #[test]
        fn loopback_ipv6_is_allowed() {
            assert!(is_loopback("::1".parse::<IpAddr>().unwrap()));
        }

        #[test]
        fn non_loopback_is_denied() {
            assert!(!is_loopback("192.168.1.1".parse::<IpAddr>().unwrap()));
            assert!(!is_loopback("10.0.0.1".parse::<IpAddr>().unwrap()));
            assert!(!is_loopback("172.16.0.1".parse::<IpAddr>().unwrap()));
        }

        async fn ok_handler() -> &'static str {
            "ok"
        }

        fn test_router() -> Router {
            Router::new()
                .route("/test", get(ok_handler))
                .layer(axum::middleware::from_fn(ip_allowlist_layer))
        }

        #[tokio::test]
        async fn runtime_loopback_ip_is_allowed() {
            let app = test_router();
            let addr: SocketAddr = "127.0.0.1:12345".parse().unwrap();
            let request = axum::http::Request::builder()
                .uri("/test")
                .extension(ConnectInfo(addr))
                .body(Body::empty())
                .unwrap();
            let resp = app.oneshot(request).await.unwrap();
            assert_eq!(resp.status(), axum::http::StatusCode::OK);
        }

        #[tokio::test]
        async fn runtime_non_loopback_ip_is_denied() {
            let app = test_router();
            let addr: SocketAddr = "192.168.1.1:12345".parse().unwrap();
            let request = axum::http::Request::builder()
                .uri("/test")
                .extension(ConnectInfo(addr))
                .body(Body::empty())
                .unwrap();
            let resp = app.oneshot(request).await.unwrap();
            assert_eq!(resp.status(), axum::http::StatusCode::FORBIDDEN);
        }

        #[tokio::test]
        async fn runtime_no_ip_in_dev_is_allowed() {
            let _guard = ENV_LOCK.lock().await;
            let prev = std::env::var("ONELINK_ENV").ok();
            std::env::set_var("ONELINK_ENV", "dev");
            let app = test_router();
            let request = axum::http::Request::builder()
                .uri("/test")
                .body(Body::empty())
                .unwrap();
            let resp = app.oneshot(request).await.unwrap();
            assert_eq!(resp.status(), axum::http::StatusCode::OK);
            match prev {
                Some(v) => std::env::set_var("ONELINK_ENV", v),
                None => std::env::remove_var("ONELINK_ENV"),
            }
        }

        #[tokio::test]
        async fn runtime_no_ip_in_staging_is_denied() {
            let _guard = ENV_LOCK.lock().await;
            let prev = std::env::var("ONELINK_ENV").ok();
            std::env::set_var("ONELINK_ENV", "staging");
            let app = test_router();
            let request = axum::http::Request::builder()
                .uri("/test")
                .body(Body::empty())
                .unwrap();
            let resp = app.oneshot(request).await.unwrap();
            assert_eq!(resp.status(), axum::http::StatusCode::FORBIDDEN);
            match prev {
                Some(v) => std::env::set_var("ONELINK_ENV", v),
                None => std::env::remove_var("ONELINK_ENV"),
            }
        }

        #[tokio::test]
        async fn runtime_no_ip_in_production_is_denied() {
            let _guard = ENV_LOCK.lock().await;
            let prev = std::env::var("ONELINK_ENV").ok();
            std::env::set_var("ONELINK_ENV", "production");
            let app = test_router();
            let request = axum::http::Request::builder()
                .uri("/test")
                .body(Body::empty())
                .unwrap();
            let resp = app.oneshot(request).await.unwrap();
            assert_eq!(resp.status(), axum::http::StatusCode::FORBIDDEN);
            match prev {
                Some(v) => std::env::set_var("ONELINK_ENV", v),
                None => std::env::remove_var("ONELINK_ENV"),
            }
        }

        #[tokio::test]
        async fn runtime_xff_loopback_is_allowed() {
            let app = test_router();
            let request = axum::http::Request::builder()
                .uri("/test")
                .header("x-forwarded-for", "127.0.0.1")
                .body(Body::empty())
                .unwrap();
            let resp = app.oneshot(request).await.unwrap();
            assert_eq!(resp.status(), axum::http::StatusCode::OK);
        }

        #[tokio::test]
        async fn runtime_xff_non_loopback_is_denied() {
            let app = test_router();
            let request = axum::http::Request::builder()
                .uri("/test")
                .header("x-forwarded-for", "10.0.0.1")
                .body(Body::empty())
                .unwrap();
            let resp = app.oneshot(request).await.unwrap();
            assert_eq!(resp.status(), axum::http::StatusCode::FORBIDDEN);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_accepts_valid_token() {
        let mut headers = HeaderMap::new();
        headers.insert(
            INTERNAL_TOKEN_HEADER,
            "test-secret-at-least-32-chars!!".parse().unwrap(),
        );
        assert!(verify_internal_token(&headers, "test-secret-at-least-32-chars!!").is_ok());
    }

    #[test]
    fn verify_rejects_invalid_token() {
        let mut headers = HeaderMap::new();
        headers.insert(INTERNAL_TOKEN_HEADER, "wrong-token".parse().unwrap());
        assert_eq!(
            verify_internal_token(&headers, "test-secret-at-least-32-chars!!"),
            Err(StatusCode::UNAUTHORIZED)
        );
    }

    #[test]
    fn verify_rejects_missing_header() {
        let headers = HeaderMap::new();
        assert_eq!(
            verify_internal_token(&headers, "test-secret-at-least-32-chars!!"),
            Err(StatusCode::UNAUTHORIZED)
        );
    }

    #[test]
    fn verify_rejects_empty_expected() {
        let mut headers = HeaderMap::new();
        headers.insert(INTERNAL_TOKEN_HEADER, "any-token".parse().unwrap());
        assert_eq!(
            verify_internal_token(&headers, ""),
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        );
    }

    #[test]
    fn secret_validation_blocks_default_in_staging() {
        assert!(validate_secret_for_env(DEV_INTERNAL_SECRET, "staging").is_err());
        assert!(validate_secret_for_env(DEV_INTERNAL_SECRET, "production").is_err());
    }

    #[test]
    fn secret_validation_accepts_default_in_dev() {
        assert!(validate_secret_for_env(DEV_INTERNAL_SECRET, "dev").is_ok());
    }

    #[test]
    fn secret_validation_blocks_short_in_production() {
        assert!(validate_secret_for_env("short-secret-only-20chars", "production").is_err());
    }

    #[test]
    fn secret_validation_accepts_long_in_production() {
        assert!(validate_secret_for_env(
            "a-very-long-secret-that-is-at-least-32-characters-long",
            "production"
        )
        .is_ok());
    }
}
