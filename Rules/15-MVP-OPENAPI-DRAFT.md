# OneLink MVP OpenAPI Draft

> 基于 `10-SERVICE-BOUNDARIES.md`、`11-DATA-EVENT-MODEL.md`、`17-MVP-SERVICE-CONTRACTS.md` 产出
> 目标：可落地的接口草案，含方法、路径、请求/响应、鉴权、幂等、错误码

---

## 1. 通用约定

### 1.1 基础路径
- 所有 API 经 api-gateway 统一入口：`/api/v1`
- BFF 聚合接口：`/api/v1/bff/*`
- 领域服务内部接口（BFF 代理）：`/api/v1/identity/*`, `/api/v1/profile/*`, ...
- 内部核心接口（不经前端暴露）：`/internal/*`

### 1.2 鉴权
- 除注册、登录外，所有接口需 `Authorization: Bearer <token>`
- token 由 identity-service 签发，api-gateway 校验

### 1.3 通用错误码
| 码 | 说明 |
|----|------|
| 400 | 请求参数错误 |
| 401 | 未认证 |
| 403 | 无权限 |
| 404 | 资源不存在 |
| 409 | 冲突（如重复注册） |
| 422 | 业务校验失败 |
| 429 | 限流 |
| 500 | 服务内部错误 |

### 1.4 通用响应 envelope
```json
{
  "data": {},
  "error": {
    "code": "string",
    "message": "string",
    "details": {}
  }
}
```

---

## 2. identity-service

### 2.1 POST /api/v1/identity/register
- **作用**：用户注册
- **幂等**：按 `email_or_phone` + `provider` 去重，重复返回 409
- **请求体**：
```json
{
  "provider": "email|sms|password",
  "email": "string (optional)",
  "phone": "string (optional)",
  "password_hash": "string (optional)",
  "verification_code": "string (optional)",
  "primary_region": "string",
  "primary_language": "string"
}
```
- **响应体**：
```json
{
  "user_id": "uuid",
  "session": {
    "token": "string",
    "expires_at": "ISO8601"
  }
}
```
- **错误码**：401（验证码错误）, 409（已注册）

### 2.2 POST /api/v1/identity/login
- **作用**：登录
- **请求体**：
```json
{
  "provider": "email|sms|password|oauth",
  "email": "string (optional)",
  "phone": "string (optional)",
  "password_hash": "string (optional)",
  "oauth_token": "string (optional)",
  "verification_code": "string (optional)"
}
```
- **响应体**：
```json
{
  "user_id": "uuid",
  "session": {
    "token": "string",
    "expires_at": "ISO8601"
  }
}
```

### 2.3 POST /api/v1/identity/bind
- **作用**：绑定登录方式
- **幂等**：同一 provider+subject 重复绑定返回 409
- **请求体**：
```json
{
  "provider": "email|sms|google|wechat|...",
  "email": "string (optional)",
  "phone": "string (optional)",
  "verification_code": "string (optional)",
  "oauth_token": "string (optional)"
}
```
- **鉴权**：必需

### 2.4 POST /api/v1/identity/refresh
- **作用**：刷新会话
- **请求体**：空或 `{ "refresh_token": "string" }`
- **响应体**：
```json
{
  "token": "string",
  "expires_at": "ISO8601"
}
```

### 2.5 GET /api/v1/identity/me
- **作用**：当前用户信息
- **鉴权**：必需
- **响应体**：
```json
{
  "user_id": "uuid",
  "status": "string",
  "primary_region": "string",
  "primary_language": "string",
  "timezone": "string",
  "created_at": "ISO8601"
}
```

---

## 3. profile-service

### 3.1 GET /api/v1/profile/me
- **作用**：获取个人主页
- **鉴权**：必需
- **响应体**：
```json
{
  "user_id": "uuid",
  "display_name": "string",
  "avatar_url": "string",
  "headline": "string",
  "bio": "string",
  "city_level_location": "string",
  "languages": ["string"],
  "is_searchable": true,
  "allow_discovery": true,
  "updated_at": "ISO8601"
}
```

### 3.2 PATCH /api/v1/profile/me
- **作用**：更新个人主页
- **鉴权**：必需
- **请求体**：
```json
{
  "display_name": "string (optional)",
  "avatar_url": "string (optional)",
  "headline": "string (optional)",
  "bio": "string (optional)",
  "city_level_location": "string (optional)",
  "languages": ["string"] (optional),
  "is_searchable": "boolean (optional)",
  "allow_discovery": "boolean (optional)"
}
```

### 3.3 GET /api/v1/profile/me/discovery
- **作用**：获取被找设置
- **鉴权**：必需

### 3.4 PATCH /api/v1/profile/me/discovery
- **作用**：更新被找设置
- **鉴权**：必需
- **请求体**：
```json
{
  "can_be_found": "boolean",
  "accepted_request_types": ["string"],
  "accepted_languages": ["string"],
  "accepted_regions": ["string"],
  "max_intro_frequency_per_week": 10,
  "allow_cross_language": true,
  "available_time_windows": []
}
```

### 3.5 GET /api/v1/profile/me/completion
- **作用**：获取画像完成度
- **鉴权**：必需
- **响应体**：
```json
{
  "completion_rate": 0.65,
  "required_dimensions": ["occupation", "skills", "..."],
  "filled_dimensions": ["occupation", "skills"],
  "missing_dimensions": ["goals"]
}
```

### 3.6 GET /api/v1/profile/me/follows
- **作用**：查看关注列表
- **鉴权**：必需
- **响应体**：
```json
{
  "items": [
    {
      "followee_user_id": "uuid",
      "followee_display_name": "string",
      "followee_avatar_url": "string",
      "followed_at": "ISO8601"
    }
  ],
  "next_cursor": "string"
}
```

### 3.7 POST /api/v1/profile/follow
- **作用**：关注
- **鉴权**：必需
- **幂等**：已关注返回 200（已存在）
- **请求体**：
```json
{
  "followee_user_id": "uuid",
  "source": "recommendation|search|profile_visit",
  "source_card_id": "uuid (optional)"
}
```

### 3.8 DELETE /api/v1/profile/follow
- **作用**：取消关注
- **鉴权**：必需
- **幂等**：未关注返回 200（已不存在）
- **路径**：`/api/v1/profile/follow/{followee_user_id}`

### 3.9 GET /api/v1/profile/{userId}
- **作用**：获取他人主页（按可见性过滤）
- **鉴权**：必需

---

## 4. ai-chat-service

### 4.1 POST /api/v1/chat/conversations
- **作用**：创建或获取 AI 会话
- **鉴权**：必需
- **幂等**：按 user_id 返回最近活跃会话或新建
- **请求体**：
```json
{
  "idempotency_key": "uuid (optional)"
}
```
- **响应体**：
```json
{
  "conversation_id": "uuid",
  "status": "active",
  "created_at": "ISO8601"
}
```

### 4.2 POST /api/v1/chat/conversations/{conversationId}/messages
- **作用**：发送用户消息
- **鉴权**：必需
- **幂等**：按 idempotency_key 去重
- **请求体**：
```json
{
  "content_type": "text",
  "content_text": "string",
  "idempotency_key": "uuid (required)"
}
```
- **响应体**：
```json
{
  "user_message_id": "uuid",
  "ai_message_id": "uuid",
  "ai_content_text": "string",
  "created_at": "ISO8601"
}
```

### 4.3 GET /api/v1/chat/conversations/{conversationId}/messages
- **作用**：拉取会话消息列表
- **鉴权**：必需
- **查询**：`?limit=50&before=message_id`
- **响应体**：
```json
{
  "items": [
    {
      "message_id": "uuid",
      "sender_type": "user|assistant|system",
      "content_text": "string",
      "created_at": "ISO8601"
    }
  ],
  "next_cursor": "string"
}
```

### 4.4 GET /api/v1/chat/conversations/{conversationId}/context
- **作用**：获取当前会话上下文摘要
- **鉴权**：必需
- **响应体**：
```json
{
  "context_version": 1,
  "summary": "string",
  "last_message_at": "ISO8601"
}
```

---

## 5. dm-service

### 5.1 POST /api/v1/dm/threads
- **作用**：创建私信线程
- **鉴权**：必需
- **幂等**：同一 (initiator, recipient) 返回已存在线程
- **请求体**：
```json
{
  "recipient_user_id": "uuid",
  "source_card_id": "uuid (optional)",
  "idempotency_key": "uuid (optional)"
}
```
- **响应体**：
```json
{
  "thread_id": "uuid",
  "thread_type": "stranger|connected",
  "status": "active",
  "created_at": "ISO8601"
}
```

### 5.2 POST /api/v1/dm/threads/{threadId}/messages
- **作用**：发送消息（含首条私信）
- **鉴权**：必需
- **幂等**：按 idempotency_key 去重
- **请求体**：
```json
{
  "content_type": "text",
  "content_text": "string",
  "idempotency_key": "uuid (required for first message)"
}
```
- **响应体**：
```json
{
  "message_id": "uuid",
  "review_status": "pending|approved|rejected",
  "created_at": "ISO8601"
}
```
- **错误码**：403（陌生人首条限制已用尽）, 403（被拉黑）

### 5.3 GET /api/v1/dm/threads
- **作用**：获取线程列表
- **鉴权**：必需
- **查询**：`?limit=20&cursor=`
- **响应体**：
```json
{
  "items": [
    {
      "thread_id": "uuid",
      "other_user": { "user_id": "uuid", "display_name": "string", "avatar_url": "string" },
      "last_message": { "content_text": "string", "created_at": "ISO8601" },
      "unread_count": 0,
      "updated_at": "ISO8601"
    }
  ],
  "next_cursor": "string"
}
```

### 5.4 GET /api/v1/dm/threads/{threadId}/messages
- **作用**：获取线程消息列表
- **鉴权**：必需
- **查询**：`?limit=50&before=message_id`

### 5.5 POST /api/v1/dm/threads/{threadId}/read
- **作用**：标记已读
- **鉴权**：必需
- **请求体**：
```json
{
  "last_read_message_id": "uuid"
}
```

---

## 6. question-service

### 6.1 GET /api/v1/questions/status
- **作用**：获取基础题包状态
- **鉴权**：必需
- **响应体**：
```json
{
  "starter_required_count": 0,
  "starter_required_total": 10,
  "profile_required_count": 0,
  "profile_required_total": 20,
  "optional_count": 0,
  "can_proceed_to_find": false
}
```

### 6.2 GET /api/v1/questions/pending
- **作用**：拉取待回答问题
- **鉴权**：必需
- **查询**：`?channel=onboarding_form|ai_chat|profile_completion&limit=5`
- **响应体**：
```json
{
  "items": [
    {
      "delivery_id": "uuid",
      "variant_id": "uuid",
      "question_text": "string",
      "question_style": "single_choice|multi_choice|open_text|...",
      "options": [],
      "requirement_tier": "starter_required|profile_required|optional"
    }
  ]
}
```

### 6.3 POST /api/v1/questions/answers
- **作用**：提交答案
- **鉴权**：必需
- **幂等**：按 delivery_id 去重，重复提交返回 200（已存在）
- **请求体**：
```json
{
  "delivery_id": "uuid",
  "variant_id": "uuid",
  "answer_payload": {},
  "answer_state": "answered|skipped|decline",
  "idempotency_key": "uuid (optional)"
}
```
- **响应体**：
```json
{
  "answer_id": "uuid",
  "delivery_id": "uuid",
  "answered_at": "ISO8601"
}
```

### 6.4 GET /api/v1/questions/completion
- **作用**：获取问卷完成度
- **鉴权**：必需
- **响应体**：同 6.1

---

## 7. match-service

### 7.1 POST /api/v1/match/requests
- **作用**：发起找人请求
- **鉴权**：必需
- **幂等**：按 idempotency_key 去重
- **请求体**：
```json
{
  "raw_query": "string",
  "idempotency_key": "uuid (optional)"
}
```
- **响应体**：
```json
{
  "find_request_id": "uuid",
  "status": "pending|need_clarification|blocked|matched",
  "clarification": { "questions": [] } (optional),
  "result_set_id": "uuid (optional)"
}
```
- **错误码**：403（风控拦截）

### 7.2 GET /api/v1/match/requests/{findRequestId}/clarification
- **作用**：获取澄清问题
- **鉴权**：必需

### 7.3 POST /api/v1/match/requests/{findRequestId}/clarification
- **作用**：提交澄清回答
- **鉴权**：必需
- **请求体**：
```json
{
  "answers": { "question_id": "answer_value" }
}
```

### 7.4 GET /api/v1/match/requests/{findRequestId}/results
- **作用**：获取推荐结果集
- **鉴权**：必需
- **响应体**：
```json
{
  "result_set_id": "uuid",
  "cards": [
    {
      "card_id": "uuid",
      "candidate_user_id": "uuid",
      "display_name": "string",
      "avatar_url": "string",
      "headline": "string",
      "reason_summary": "string",
      "rank_position": 1
    }
  ],
  "served_at": "ISO8601"
}
```

### 7.5 POST /api/v1/match/feedback
- **作用**：上报名片反馈
- **鉴权**：必需
- **幂等**：按 (card_id, actor_user_id, feedback_type, idempotency_key) 去重
- **请求体**：
```json
{
  "card_id": "uuid",
  "feedback_type": "impression|click|follow|dm_start|dismiss|block|report",
  "dismiss_reason": "string (optional, for dismiss)",
  "idempotency_key": "uuid (optional)"
}
```

---

## 8. safety-service

### 8.1 POST /api/v1/safety/reports
- **作用**：提交投诉
- **鉴权**：必需
- **幂等**：按 (reporter, target_type, target_id, idempotency_key) 去重
- **请求体**：
```json
{
  "target_type": "find_request|recommendation_card|dm_message|profile",
  "target_id": "uuid",
  "reason_code": "string",
  "evidence": {},
  "idempotency_key": "uuid (optional)"
}
```
- **响应体**：
```json
{
  "report_ticket_id": "uuid",
  "status": "open",
  "created_at": "ISO8601"
}
```

### 8.2 GET /api/v1/safety/reports/{reportTicketId}
- **作用**：获取投诉状态
- **鉴权**：必需
- **响应体**：
```json
{
  "report_ticket_id": "uuid",
  "status": "open|in_review|resolved",
  "resolution_summary": "string (optional)",
  "created_at": "ISO8601"
}
```

### 8.3 GET /api/v1/safety/me/moderation
- **作用**：获取处罚摘要
- **鉴权**：必需
- **响应体**：
```json
{
  "active_actions": [
    {
      "action_type": "mute|suspend|...",
      "expires_at": "ISO8601",
      "reason_summary": "string"
    }
  ]
}
```

### 8.4 POST /api/v1/safety/appeals
- **作用**：提交申诉
- **鉴权**：必需
- **幂等**：同一 moderation_action 重复申诉返回 409
- **请求体**：
```json
{
  "moderation_action_id": "uuid",
  "appeal_text": "string",
  "idempotency_key": "uuid (optional)"
}
```

---

## 9. context-service（内部接口，不暴露给前端）

### 9.1 POST /internal/context/build
- **作用**：为 `ai-chat-service`、后续 `match-service` 同步组装上下文包
- **调用方**：`ai-chat-service`（MVP）
- **可见性**：仅内部
- **请求体**：
```json
{
  "user_id": "uuid",
  "agent_id": "uuid",
  "conversation_id": "uuid",
  "input": "string",
  "task_type": "chat",
  "max_tokens": 8000,
  "memory_limit": 20,
  "summary_limit": 3,
  "reply_style": "brief",
  "retrieval_modes": ["structured", "semantic", "temporal"]
}
```
- **响应体**：
```json
{
  "system_prompt": "string",
  "user_context": "string",
  "memory_context": "string",
  "task_context": "string",
  "selected_summary_ids": ["uuid"],
  "selected_memory_ids": ["uuid"],
  "retrieval_used": ["structured", "semantic"],
  "degraded": false,
  "token_budget": {
    "max_tokens": 8000,
    "memory_limit": 20,
    "summary_limit": 3
  }
}
```
- **约束**：
  - `max_tokens`、`memory_limit`、`summary_limit` 为必填
  - 不允许调用方请求“拼接全部原始聊天”
  - 超时或向量检索不可用时，允许降级为 `working memory + recent summaries`

### 9.2 POST /internal/session/checkpoint
- **作用**：保存 runtime checkpoint，支撑逻辑 agent 唤醒/恢复
- **可见性**：仅内部
- **请求体**：
```json
{
  "agent_id": "uuid",
  "user_id": "uuid",
  "conversation_id": "uuid",
  "schema_version": 1,
  "working_summary_ref": "uuid",
  "runtime_state_blob": {
    "active_task": "chat",
    "last_tool_result": null
  },
  "policy_versions": {
    "memory_policy": "v1",
    "session_policy": "v1",
    "retrieval_policy": "v1"
  }
}
```
- **响应体**：
```json
{
  "accepted": true,
  "checkpoint_id": "uuid"
}
```

### 9.3 POST /internal/memory/write
- **作用**：保留内部写入口，语义上更接近 ingest
- **可见性**：仅内部
- **说明**：MVP 绝大多数记忆写入仍然以事件消费为主，不鼓励业务服务同步调用
- **字段说明**：`memory_value_score` 是请求层的统一候选分值，落库时由 `context-service` 映射到 `importance_score` 等内部评分字段，不直接作为 `memory_artifacts` 表字段
- **请求体**：
```json
{
  "event_id": "uuid",
  "user_id": "uuid",
  "source_type": "chat",
  "source_service": "ai-chat-service",
  "source_ref_id": "uuid",
  "raw_text": "string",
  "memory_value_score": 0.81
}
```

### 9.4 POST /internal/memory/consolidate
- **作用**：触发去重、冲突标记、summary 更新与记忆整合
- **可见性**：仅内部
- **请求体**：
```json
{
  "event_id": "uuid",
  "user_id": "uuid",
  "artifact_ids": ["uuid"],
  "mode": "incremental"
}
```
- **约束**：
  - 必须基于 `event_id` 幂等
  - MVP 起必须可重放

### 9.5 GET /internal/memory/search
- **作用**：内部检索记忆
- **可见性**：仅内部
- **查询**：`?user_id=uuid&query=...&limit=10`
- **说明**：不直接暴露给前端或 BFF

---

## 10. BFF 聚合接口

### 10.1 GET /api/v1/bff/home
- **作用**：首页聚合数据
- **鉴权**：必需
- **聚合**：identity.me, profile.summary, question.progress
- **响应体**：
```json
{
  "user": { "user_id": "uuid", "display_name": "string", "..." },
  "profile_summary": { "completion_rate": 0.65, "..." },
  "question_progress": { "starter_required_count": 5, "..." }
}
```

### 10.2 GET /api/v1/bff/chat/init
- **作用**：AI 聊天页初始化
- **鉴权**：必需
- **聚合**：identity.me, ai-chat.getOrCreateConversation, question.pending
- **响应体**：
```json
{
  "user": {},
  "conversation": { "conversation_id": "uuid", "..." },
  "pending_questions": []
}
```

### 10.3 GET /api/v1/bff/onboarding
- **作用**：冷启动画像建档页
- **鉴权**：必需
- **聚合**：identity.me, question.pending, question.progress
- **响应体**：
```json
{
  "user": {},
  "pending_questions": [],
  "progress": {}
}
```

### 10.4 GET /api/v1/bff/find/results
- **作用**：找人结果页
- **鉴权**：必需
- **查询**：`?find_request_id=uuid`
- **聚合**：identity.me, match.getResultSet, match.getClarification
- **响应体**：
```json
{
  "user": {},
  "find_request": { "status": "matched", "..." },
  "clarification": { "questions": [] } (optional),
  "result_set": { "cards": [] } (optional)
}
```

### 10.5 GET /api/v1/bff/dm/list
- **作用**：私信列表页
- **鉴权**：必需
- **聚合**：identity.me, dm.getThreadList
- **响应体**：
```json
{
  "user": {},
  "threads": []
}
```

### 10.6 GET /api/v1/bff/profile/{userId}
- **作用**：用户主页页
- **鉴权**：必需
- **聚合**：identity.me, profile.getProfile, profile.getFollowStatus
- **响应体**：
```json
{
  "viewer": {},
  "profile": {},
  "follow_status": "following|not_following|self"
}
```

---

## 11. model-gateway（内部接口，不暴露）

### 11.1 能力调用契约（gRPC 或内部 HTTP）

| 能力 | 请求 | 响应 | 调用方 |
|------|------|------|--------|
| chat.respond | messages[], context | text, model_id | ai-chat-service |
| profile.extract_facts | messages[], user_id | facts[] | profile-service |
| profile.summarize | facts[], context | summary | profile-service |
| question.generate | dimension, gaps | variants[] | question-service (Phase 2) |
| question.review | variant_text | risk_level | question-service (Phase 2) |
| safety.classify_request | raw_query | risk_level, codes | safety-service |
| safety.review_message | content_text | risk_level, decision | safety-service |
| embedding.encode | text | vector | context-service, profile-service, match-service |

---

## 12. 幂等接口汇总

| 接口 | 幂等策略 |
|------|----------|
| identity/register | email+provider 去重 |
| identity/bind | provider+subject 去重 |
| chat/conversations (POST) | 返回最近会话或新建 |
| chat/.../messages | idempotency_key |
| dm/threads | (initiator, recipient) 去重 |
| dm/.../messages | idempotency_key（首条必填） |
| questions/answers | delivery_id 去重 |
| match/requests | idempotency_key |
| match/feedback | (card_id, actor, type) + idempotency_key |
| safety/reports | (reporter, target, idempotency_key) 去重 |
| safety/appeals | moderation_action_id 去重 |
