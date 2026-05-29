# OneLink Production Acceptance 2026-05-29

## Scope

- Site: `https://onelink.cool`
- ECS: `admin@8.218.209.218`
- This round goal:
  - close the remaining production API gap for `GET /api/v1/bff/profile/{user_id}`
  - switch AI chat from mock model output to real `DeepSeek deepseek-v4-flash`
  - verify chat memory/context works in production

## Root Cause

- The remaining `profile/{user_id}` `404` was caused by `bff` dynamic route matching, not by the web page fallback or `app-server` session handling.
- `app-server` local regression verified that `/api/v1/bff/profile/:id` bridge was functional.
- `bff` route regression verified the dynamic `profile` route was the missing production match point.

## Fix Applied

- Kept `app-server` dynamic routes on the working `:id` syntax used by the current runtime.
- Fixed `bff` dynamic routes for:
  - `/api/v1/bff/profile/:userId`
  - `/api/v1/bff/dm/threads/:threadId`
- Added targeted local regressions for:
  - `app-server` profile detail bridge
  - `bff` dynamic profile route
  - `bff` dynamic dm thread route
- Rebuilt `bff` on ECS with `CC=clang CXX=clang++` to bypass the host `gcc 10.2.1` and `aws-lc-sys` incompatibility check.

## Verification

- Local diagnostics:
  - `repo/apps/app-server/src/router.rs`: clean
  - `repo/services/bff/src/http/routes.rs`: clean
- Local targeted tests:
  - `cargo test -p onelink-app-server --test integration_test test_profile_detail_with_mock_bff`
  - `cargo test -p bff http::routes::tests::test_profile_route_matches_dynamic_user_id -- --exact`
  - `cargo test -p bff http::routes::tests::test_dm_thread_route_matches_dynamic_thread_id -- --exact`
- ECS targeted tests:
  - `onelink-app-server` profile detail regression: pass
  - `bff` profile dynamic route regression: pass
- Production API smoke:
  - `home=200`
  - `chat_init=200`
  - `onboarding=200`
  - `profile/{user_id}=200`

## Result

- The previously remaining production `profile` API `404` is closed.
- Main production Web flows and API-level smoke are now green for the repaired paths.
- The hotfix touched only OneLink-owned code and OneLink-owned services on the shared ECS.

## AI Chat Activation

- `model-gateway` now calls real DeepSeek chat completions instead of returning mock Lumi text.
- Production provider settings are loaded from `/opt/onelink/shared/onelink.env`:
  - `DEEPSEEK_BASE_URL=https://api.deepseek.com`
  - `DEEPSEEK_MODEL=deepseek-v4-flash`
  - `DEEPSEEK_THINKING_TYPE=disabled`
  - `DEEPSEEK_TIMEOUT_MS=60000`
  - `DEEPSEEK_API_KEY` present on ECS only
- `ai-chat-service` internal invoke path now includes the shared internal auth header required by `model-gateway`.
- Only `onelink@model-gateway.service` and `onelink@ai-chat-service.service` were rebuilt and restarted for this activation.

## AI Verification

- Local tests:
  - `cargo test -p model-gateway --test deepseek_provider -- --nocapture`
  - `cargo test -p model-gateway --test runtime_internal_auth -- --nocapture`
  - `cargo test -p ai-chat-service --lib --tests`
- ECS readiness:
  - `model-gateway=/ready`
  - `context-service=/ready`
  - `ai-chat-service=/ready`
  - `bff=/ready`
  - `app-server=/api/v1/bff/ready`
- Production chat smoke:
  - register test user: `200`
  - `chat_init`: `200`
  - first `chat/messages`: `200`
  - first reply included real model marker: `[chat.respond:deepseek-v4-flash]`
- Production memory/context smoke:
  - first message stated user likes hiking and coffee
  - second message asked what was remembered
  - reply correctly recalled `徒步和咖啡`
