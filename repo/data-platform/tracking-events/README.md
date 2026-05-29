# OneLink Client Tracking Events — Phase 2 Baseline

## Overview

This directory defines the client-side analytics event taxonomy for OneLink App/Web.
These events are emitted by the frontend application and are distinct from the backend domain events in `data-platform/event-schemas/`.

## Event Schema

- `client-events-v1.json` — JSON Schema for all client tracking events

## Event Catalog

| Event Name | Trigger | Required Fields |
|------------|---------|-----------------|
| `page.view` | Screen/route navigation | `page_name` |
| `registration.started` | Registration form opened | `provider` |
| `registration.completed` | Registration API success | `user_id`, `provider` |
| `login.started` | Login form opened | `provider` |
| `login.completed` | Login API success | `user_id`, `provider` |
| `chat.message.sent` | User sends chat message | `user_id`, `conversation_id`, `content_type` |
| `chat.message.received` | AI reply received | `user_id`, `conversation_id`, `response_latency_ms` |
| `profile.confirmation.viewed` | Profile confirmation card shown | `user_id`, `completion_rate`, `missing_dimensions` |
| `profile.fact.confirmed` | User confirms a profile fact | `user_id`, `fact_type`, `fact_value` |
| `profile.fact.dismissed` | User dismisses a profile fact | `user_id`, `fact_type`, `fact_value` |
| `find.intent.submitted` | Find-person query submitted | `user_id`, `query` |
| `recommendation.exposed` | Recommendation card shown | `user_id`, `result_set_id`, `candidate_count` |
| `dm.message.sent` | DM sent | `user_id`, `thread_id`, `recipient_user_id` |
| `report.submitted` | User reports content | `user_id`, `target_type`, `target_id`, `reason` |
| `error.occurred` | Client-side error | `error_type`, `error_code` |

## Common Fields

All events include:

| Field | Type | Description |
|-------|------|-------------|
| `event_name` | string | Event identifier |
| `occurred_at` | ISO 8601 | Client timestamp |
| `session_id` | string | Client session identifier |
| `user_id` | UUID | Authenticated user (if available) |
| `platform` | enum | `ios` / `android` / `web` |
| `app_version` | string | Semantic version |
| `screen` | string | Current screen/route |
| `trace_id` | string | Correlation ID (if available) |

## Relationship to Backend Events

Client tracking events are **complementary** to backend domain events (`data-platform/event-schemas/`):
- Backend events capture domain state changes (e.g., `identity.user.registered.v1`)
- Client events capture user interactions and UI state (e.g., `registration.started`)
- Both share `user_id` and `trace_id` for correlation

## SDK Integration (Future)

Events should be batched and sent to a tracking endpoint (e.g., `POST /api/v1/tracking/events`).
The SDK should:
1. Auto-capture `page.view` events on route change
2. Auto-capture `error.occurred` on unhandled exceptions
3. Batch events (max 20 per batch, flush every 5s or on visibility change)
4. Retry with exponential backoff on network failure
5. Respect `Do Not Track` browser setting
