# OneLink MVP Event Schemas Draft

> 基于 `11-DATA-EVENT-MODEL.md` 产出
> 目标：可落地的 JSON schema 草案，含 partition_key、idempotency_key、consumers

---

## 1. 通用事件 envelope

所有事件必须包含以下顶层字段：

```json
{
  "event_id": "string (UUID)",
  "event_name": "string (domain.entity.action.v1)",
  "event_version": "string (v1)",
  "occurred_at": "string (ISO8601)",
  "producer": "string (service-name)",
  "trace_id": "string (UUID, optional)",
  "region": "string (optional)",
  "actor_user_id": "string (UUID, optional)",
  "subject_id": "string (UUID, optional)",
  "payload": "object (event-specific)"
}
```

### 元信息约定

| 字段 | 说明 |
|------|------|
| partition_key | 用于 Kafka 分区，通常为 `actor_user_id` 或 `subject_user_id` |
| idempotency_key | 等于 `event_id`，消费者按此去重 |
| ordered_by | 同一分区内按 `occurred_at` 排序 |
| consumers | 消费该事件的服务列表 |
| replay_required | 是否需要支持回放 |

---

## 2. 账户域事件

### 2.1 identity.user.registered.v1

| 属性 | 值 |
|------|-----|
| partition_key | payload.user_id |
| idempotency_key | event_id |
| producer | identity-service |
| consumers | profile-service（创建空 profile）, match-service（索引） |
| 触发时机 | 用户完成注册 |
| 主链路 | 是 |

```json
{
  "event_id": "550e8400-e29b-41d4-a716-446655440000",
  "event_name": "identity.user.registered.v1",
  "event_version": "v1",
  "occurred_at": "2026-03-20T10:00:00Z",
  "producer": "identity-service",
  "trace_id": "...",
  "region": "ap-northeast-1",
  "actor_user_id": "user-uuid",
  "subject_id": "user-uuid",
  "payload": {
    "user_id": "uuid",
    "primary_region": "string",
    "primary_language": "string"
  }
}
```

### 2.2 identity.binding.added.v1

| 属性 | 值 |
|------|-----|
| partition_key | actor_user_id |
| idempotency_key | event_id |
| producer | identity-service |
| consumers | （审计/分析） |
| 触发时机 | 用户绑定新登录方式 |
| 主链路 | 否 |

```json
{
  "payload": {
    "user_id": "uuid",
    "provider": "email|sms|google|wechat|...",
    "binding_id": "uuid"
  }
}
```

### 2.3 identity.user.logged_in.v1

| 属性 | 值 |
|------|-----|
| partition_key | actor_user_id |
| idempotency_key | event_id |
| producer | identity-service |
| consumers | （审计/分析） |
| 触发时机 | 用户登录成功 |
| 主链路 | 否 |

```json
{
  "payload": {
    "user_id": "uuid",
    "session_id": "uuid",
    "login_method": "string"
  }
}
```

---

## 3. 聊天域事件

### 3.1 chat.ai_conversation.created.v1

| 属性 | 值 |
|------|-----|
| partition_key | actor_user_id |
| idempotency_key | event_id |
| producer | ai-chat-service |
| consumers | （审计） |
| 触发时机 | 创建新 AI 会话 |
| 主链路 | 否 |

```json
{
  "payload": {
    "conversation_id": "uuid",
    "user_id": "uuid"
  }
}
```

### 3.2 chat.user_message.created.v1

| 属性 | 值 |
|------|-----|
| partition_key | actor_user_id |
| idempotency_key | event_id |
| producer | ai-chat-service |
| consumers | context-service（记忆抽取） |
| 触发时机 | 用户发送消息后 |
| 主链路 | 是 |

```json
{
  "payload": {
    "conversation_id": "uuid",
    "message_id": "uuid",
    "user_id": "uuid",
    "content_type": "text"
  }
}
```

### 3.3 chat.ai_message.created.v1

| 属性 | 值 |
|------|-----|
| partition_key | actor_user_id |
| idempotency_key | event_id |
| producer | ai-chat-service |
| consumers | （审计/训练） |
| 触发时机 | AI 回复写入后 |
| 主链路 | 否 |

```json
{
  "payload": {
    "conversation_id": "uuid",
    "message_id": "uuid",
    "user_id": "uuid"
  }
}
```

---

## 4. 私信域事件

### 4.1 dm.thread.created.v1

| 属性 | 值 |
|------|-----|
| partition_key | actor_user_id 或 initiator_user_id |
| idempotency_key | event_id |
| producer | dm-service |
| consumers | match-service（反馈归并） |
| 触发时机 | 创建私信线程 |
| 主链路 | 是（当来自推荐卡片发起时） |

```json
{
  "payload": {
    "thread_id": "uuid",
    "initiator_user_id": "uuid",
    "recipient_user_id": "uuid",
    "thread_type": "stranger|connected",
    "source_card_id": "uuid (optional)"
  }
}
```

### 4.2 dm.message.created.v1

| 属性 | 值 |
|------|-----|
| partition_key | actor_user_id |
| idempotency_key | event_id |
| producer | dm-service |
| consumers | match-service（反馈归并）, safety-service（异步回扫） |
| 触发时机 | 私信发送成功 |
| 主链路 | 是 |

```json
{
  "payload": {
    "thread_id": "uuid",
    "message_id": "uuid",
    "sender_user_id": "uuid",
    "recipient_user_id": "uuid",
    "content_type": "text",
    "source_card_id": "uuid (optional)"
  }
}
```

### 4.3 dm.message.reviewed.v1

| 属性 | 值 |
|------|-----|
| partition_key | subject_id (message_id 或 target_user_id) |
| idempotency_key | event_id |
| producer | safety-service |
| consumers | match-service（反馈归并） |
| 触发时机 | 私信审核完成 |
| 主链路 | 是 |

```json
{
  "payload": {
    "message_id": "uuid",
    "thread_id": "uuid",
    "review_result": "approved|rejected",
    "risk_level": "string"
  }
}
```

---

## 5. 画像域事件

### 5.1 profile.profile.updated.v1

| 属性 | 值 |
|------|-----|
| partition_key | actor_user_id |
| idempotency_key | event_id |
| producer | profile-service |
| consumers | match-service（召回索引更新） |
| 触发时机 | 用户编辑主页字段 |
| 主链路 | 是 |

```json
{
  "payload": {
    "user_id": "uuid",
    "updated_fields": ["display_name", "bio", "headline", "..."],
    "updated_at": "ISO8601"
  }
}
```

### 5.2 profile.discovery_preference.updated.v1

| 属性 | 值 |
|------|-----|
| partition_key | actor_user_id |
| idempotency_key | event_id |
| producer | profile-service |
| consumers | match-service（召回索引更新） |
| 触发时机 | 被找设置变更 |
| 主链路 | 是 |

```json
{
  "payload": {
    "user_id": "uuid",
    "can_be_found": "boolean",
    "updated_at": "ISO8601"
  }
}
```

### 5.3 profile.visibility_rule.updated.v1

| 属性 | 值 |
|------|-----|
| partition_key | actor_user_id |
| idempotency_key | event_id |
| producer | profile-service |
| consumers | match-service |
| 触发时机 | 可见性规则变更 |
| 主链路 | 是 |

```json
{
  "payload": {
    "user_id": "uuid",
    "updated_at": "ISO8601"
  }
}
```

### 5.4 profile.fact.extracted.v1

| 属性 | 值 |
|------|-----|
| partition_key | actor_user_id |
| idempotency_key | event_id |
| producer | profile-service |
| consumers | （内部，触发 upsert） |
| 触发时机 | 从聊天/问卷抽取事实后 |
| 主链路 | 是 |

```json
{
  "payload": {
    "user_id": "uuid",
    "fact_id": "uuid",
    "fact_type": "string",
    "fact_key": "string",
    "source_ref_id": "uuid",
    "confidence": "number"
  }
}
```

### 5.5 profile.fact.upserted.v1

| 属性 | 值 |
|------|-----|
| partition_key | actor_user_id |
| idempotency_key | event_id |
| producer | profile-service |
| consumers | profile-service（trait/embedding 更新）, match-service（索引） |
| 触发时机 | 事实写入/更新完成 |
| 主链路 | 是 |

```json
{
  "payload": {
    "user_id": "uuid",
    "fact_id": "uuid",
    "fact_type": "string",
    "fact_key": "string",
    "status": "active|superseded|...",
    "source_type": "chat|questionnaire|behavior|explicit_edit"
  }
}
```

### 5.6 profile.fact.conflict_detected.v1

| 属性 | 值 |
|------|-----|
| partition_key | actor_user_id |
| idempotency_key | event_id |
| producer | profile-service |
| consumers | （内部/通知） |
| 触发时机 | 检测到事实冲突 |
| 主链路 | 否 |

```json
{
  "payload": {
    "user_id": "uuid",
    "fact_key": "string",
    "existing_fact_id": "uuid",
    "conflicting_source_ref_id": "uuid"
  }
}
```

### 5.7 profile.trait.updated.v1

| 属性 | 值 |
|------|-----|
| partition_key | actor_user_id |
| idempotency_key | event_id |
| producer | profile-service |
| consumers | profile-service（embedding 重算） |
| 触发时机 | trait 更新完成 |
| 主链路 | 是 |

```json
{
  "payload": {
    "user_id": "uuid",
    "trait_id": "uuid",
    "trait_type": "string",
    "trait_key": "string"
  }
}
```

### 5.8 profile.summary.updated.v1

| 属性 | 值 |
|------|-----|
| partition_key | actor_user_id |
| idempotency_key | event_id |
| producer | profile-service |
| consumers | match-service（名片展示） |
| 触发时机 | 画像摘要更新完成 |
| 主链路 | 是 |

```json
{
  "payload": {
    "user_id": "uuid",
    "source_fact_version": "uuid"
  }
}
```

### 5.9 profile.embedding.updated.v1

| 属性 | 值 |
|------|-----|
| partition_key | actor_user_id |
| idempotency_key | event_id |
| producer | profile-service |
| consumers | match-service（向量索引） |
| 触发时机 | 向量更新完成 |
| 主链路 | 是 |

```json
{
  "payload": {
    "user_id": "uuid",
    "embedding_type": "match_profile|question_targeting|safety_context",
    "source_fact_version": "uuid"
  }
}
```

---

## 5A. 上下文与记忆域事件

### 5A.1 context.memory.extracted.v1

| 属性 | 值 |
|------|-----|
| partition_key | actor_user_id |
| idempotency_key | event_id |
| producer | context-service |
| consumers | context-service（自消费，进入 distill / merge / write 流程） |
| 触发时机 | context-service 从聊天或问卷事件中抽取出原始记忆候选 |
| 主链路 | 否 |
| replay_required | 是，MVP 起 consolidation pipeline 必须支持基于 `event_id` 重放 |

```json
{
  "payload": {
    "user_id": "uuid",
    "conversation_id": "uuid (optional)",
    "source_event_id": "uuid",
    "source_event_name": "chat.user_message.created.v1|question.answered.v1",
    "artifact_candidates": [
      {
        "network_type": "world|experience|opinion|entity",
        "evidence_type": "fact|inference",
        "memory_level": "working|persistent",
        "content": "string",
        "content_structured": {},
        "source_type": "chat|questionnaire|behavior",
        "confidence": 0.87,
        "importance_score": 0.76,
        "consistency_score": 0.92,
        "entity_refs": []
      }
    ]
  }
}
```

### 5A.2 context.memory.summary.updated.v1

| 属性 | 值 |
|------|-----|
| partition_key | actor_user_id |
| idempotency_key | event_id |
| producer | context-service |
| consumers | ai-chat-service（刷新 working memory 摘要视图）, context-service（审计/重放） |
| 触发时机 | working memory 摘要压缩并落库完成 |
| 主链路 | 否 |
| replay_required | 是 |

```json
{
  "payload": {
    "user_id": "uuid",
    "conversation_id": "uuid",
    "summary_id": "uuid",
    "summary_type": "working_memory",
    "token_count": 320,
    "source_message_range": {
      "from_message_id": "uuid",
      "to_message_id": "uuid"
    }
  }
}
```

### 5A.3 profile.memory_projection.requested.v1

| 属性 | 值 |
|------|-----|
| partition_key | actor_user_id |
| idempotency_key | event_id |
| producer | context-service |
| consumers | profile-service |
| 触发时机 | 记忆蒸馏完成后，请求将部分记忆投影进画像事实层 |
| 主链路 | 否 |
| replay_required | 是 |

```json
{
  "payload": {
    "user_id": "uuid",
    "projection_id": "uuid",
    "memory_ids": ["uuid"],
    "conversation_id": "uuid (optional)",
    "source_event_id": "uuid",
    "projection_reason": "chat_distillation|question_answered|behavior_update"
  }
}
```

---

## 6. 社交域事件

### 6.1 social.follow.created.v1

| 属性 | 值 |
|------|-----|
| partition_key | follower_user_id |
| idempotency_key | event_id |
| producer | profile-service |
| consumers | match-service（反馈归并） |
| 触发时机 | 关注成功 |
| 主链路 | 是 |

```json
{
  "payload": {
    "follow_id": "uuid",
    "follower_user_id": "uuid",
    "followee_user_id": "uuid",
    "source": "recommendation|search|profile_visit",
    "source_card_id": "uuid (optional)"
  }
}
```

### 6.2 social.follow.removed.v1

| 属性 | 值 |
|------|-----|
| partition_key | follower_user_id |
| idempotency_key | event_id |
| producer | profile-service |
| consumers | match-service |
| 触发时机 | 取消关注 |
| 主链路 | 是 |

```json
{
  "payload": {
    "follow_id": "uuid",
    "follower_user_id": "uuid",
    "followee_user_id": "uuid"
  }
}
```

---

## 7. 问题域事件

### 7.1 question.delivery.created.v1

| 属性 | 值 |
|------|-----|
| partition_key | user_id |
| idempotency_key | event_id |
| producer | question-service |
| consumers | （审计） |
| 触发时机 | 投放记录写入数据库 |
| 主链路 | 否 |

```json
{
  "payload": {
    "delivery_id": "uuid",
    "user_id": "uuid",
    "variant_id": "uuid",
    "delivery_channel": "onboarding_form|ai_chat|profile_completion",
    "requirement_tier": "starter_required|profile_required|optional"
  }
}
```

### 7.2 question.delivered.v1

| 属性 | 值 |
|------|-----|
| partition_key | user_id |
| idempotency_key | event_id |
| producer | question-service |
| consumers | （审计） |
| 触发时机 | 问题实际展示给用户 |
| 主链路 | 否 |

```json
{
  "payload": {
    "delivery_id": "uuid",
    "user_id": "uuid",
    "variant_id": "uuid"
  }
}
```

### 7.3 question.answered.v1

| 属性 | 值 |
|------|-----|
| partition_key | user_id |
| idempotency_key | event_id |
| producer | question-service |
| consumers | context-service（记忆抽取） |
| 触发时机 | 用户提交答案 |
| 主链路 | 是 |

```json
{
  "payload": {
    "answer_id": "uuid",
    "delivery_id": "uuid",
    "user_id": "uuid",
    "variant_id": "uuid",
    "answer_payload": {},
    "answer_state": "answered|skipped|decline"
  }
}
```

### 7.4 question.skipped.v1

| 属性 | 值 |
|------|-----|
| partition_key | user_id |
| idempotency_key | event_id |
| producer | question-service |
| consumers | （分析） |
| 触发时机 | 用户跳过问题 |
| 主链路 | 否 |

```json
{
  "payload": {
    "delivery_id": "uuid",
    "user_id": "uuid",
    "variant_id": "uuid"
  }
}
```

---

## 8. 匹配域事件

### 8.1 match.request.submitted.v1

| 属性 | 值 |
|------|-----|
| partition_key | user_id |
| idempotency_key | event_id |
| producer | match-service |
| consumers | safety-service（风险评估） |
| 触发时机 | 找人请求提交 |
| 主链路 | 是 |

```json
{
  "payload": {
    "find_request_id": "uuid",
    "user_id": "uuid",
    "raw_query": "string",
    "parsed_goal": "string",
    "status": "pending"
  }
}
```

### 8.2 match.request.blocked.v1

| 属性 | 值 |
|------|-----|
| partition_key | user_id |
| idempotency_key | event_id |
| producer | match-service |
| consumers | （审计） |
| 触发时机 | 请求被风控拦截 |
| 主链路 | 是 |

```json
{
  "payload": {
    "find_request_id": "uuid",
    "user_id": "uuid",
    "risk_level": "string",
    "reason": "string"
  }
}
```

### 8.3 match.clarification.required.v1

| 属性 | 值 |
|------|-----|
| partition_key | user_id |
| idempotency_key | event_id |
| producer | match-service |
| consumers | （前端展示） |
| 触发时机 | 需要用户澄清 |
| 主链路 | 是 |

```json
{
  "payload": {
    "find_request_id": "uuid",
    "user_id": "uuid",
    "clarification_questions": []
  }
}
```

### 8.4 match.result_set.served.v1

| 属性 | 值 |
|------|-----|
| partition_key | user_id |
| idempotency_key | event_id |
| producer | match-service |
| consumers | match-service（反馈归并） |
| 触发时机 | 推荐结果返回给用户 |
| 主链路 | 是 |

```json
{
  "payload": {
    "result_set_id": "uuid",
    "find_request_id": "uuid",
    "user_id": "uuid",
    "card_ids": ["uuid", "..."],
    "candidate_count": 10
  }
}
```

### 8.5 match.card.impression_logged.v1

| 属性 | 值 |
|------|-----|
| partition_key | actor_user_id |
| idempotency_key | event_id |
| producer | match-service |
| consumers | match-service（反馈归并） |
| 触发时机 | 名片曝光上报（前端 → BFF → match-service 落表后发事件） |
| 主链路 | 是 |

```json
{
  "payload": {
    "card_id": "uuid",
    "result_set_id": "uuid",
    "actor_user_id": "uuid",
    "position": 1
  }
}
```

### 8.6 match.card.clicked.v1

| 属性 | 值 |
|------|-----|
| partition_key | actor_user_id |
| idempotency_key | event_id |
| producer | match-service |
| consumers | match-service（反馈归并） |
| 触发时机 | 用户点击名片（前端 → BFF → match-service 落表后发事件） |
| 主链路 | 是 |

```json
{
  "payload": {
    "card_id": "uuid",
    "actor_user_id": "uuid",
    "candidate_user_id": "uuid"
  }
}
```

### 8.7 match.card.dm_started.v1

| 属性 | 值 |
|------|-----|
| partition_key | actor_user_id |
| idempotency_key | event_id |
| producer | dm-service |
| consumers | match-service（反馈归并） |
| 触发时机 | 从名片发起私信（dm-service 创建线程后发出，match-service 消费） |
| 主链路 | 是 |

```json
{
  "payload": {
    "card_id": "uuid",
    "actor_user_id": "uuid",
    "candidate_user_id": "uuid",
    "thread_id": "uuid"
  }
}
```

### 8.8 match.card.dismissed.v1

| 属性 | 值 |
|------|-----|
| partition_key | actor_user_id |
| idempotency_key | event_id |
| producer | match-service |
| consumers | match-service（反馈归并） |
| 触发时机 | 用户关闭/忽略名片（前端 → BFF → match-service 落表后发事件） |
| 主链路 | 是 |

```json
{
  "payload": {
    "card_id": "uuid",
    "actor_user_id": "uuid",
    "dismiss_reason": "string (optional)"
  }
}
```

### 8.9 match.card.reported.v1

| 属性 | 值 |
|------|-----|
| partition_key | actor_user_id |
| idempotency_key | event_id |
| producer | safety-service |
| consumers | match-service（反馈归并） |
| 触发时机 | 用户举报名片（safety-service 创建工单后发出，match-service 消费） |
| 主链路 | 是 |

```json
{
  "payload": {
    "card_id": "uuid",
    "actor_user_id": "uuid",
    "candidate_user_id": "uuid",
    "report_ticket_id": "uuid",
    "reason_code": "string"
  }
}
```

---

## 9. 安全治理域事件

### 9.1 safety.assessment.completed.v1

| 属性 | 值 |
|------|-----|
| partition_key | subject_id (target_id) |
| idempotency_key | event_id |
| producer | safety-service |
| consumers | match-service（找人放行）, dm-service（私信放行） |
| 触发时机 | 风险评估完成 |
| 主链路 | 是 |

```json
{
  "payload": {
    "assessment_id": "uuid",
    "target_type": "find_request|dm_message|profile",
    "target_id": "uuid",
    "risk_level": "L0|L1|L2|L3|L4",
    "decision": "allow|block|review"
  }
}
```

### 9.2 safety.user_block.created.v1

| 属性 | 值 |
|------|-----|
| partition_key | blocker_user_id |
| idempotency_key | event_id |
| producer | safety-service |
| consumers | match-service（召回过滤）, dm-service |
| 触发时机 | 用户拉黑 |
| 主链路 | 是 |

```json
{
  "payload": {
    "block_id": "uuid",
    "blocker_user_id": "uuid",
    "blocked_user_id": "uuid",
    "source": "manual|moderation"
  }
}
```

### 9.3 moderation.report.created.v1

| 属性 | 值 |
|------|-----|
| partition_key | reporter_user_id |
| idempotency_key | event_id |
| producer | safety-service |
| consumers | （工单处理） |
| 触发时机 | 投诉工单创建 |
| 主链路 | 是 |

```json
{
  "payload": {
    "report_ticket_id": "uuid",
    "reporter_user_id": "uuid",
    "target_type": "string",
    "target_id": "uuid",
    "reason_code": "string"
  }
}
```

### 9.4 moderation.action.executed.v1

| 属性 | 值 |
|------|-----|
| partition_key | target_user_id |
| idempotency_key | event_id |
| producer | safety-service |
| consumers | （通知/索引更新） |
| 触发时机 | 处罚执行 |
| 主链路 | 是 |

```json
{
  "payload": {
    "moderation_action_id": "uuid",
    "target_user_id": "uuid",
    "action_type": "warn|throttle|mute|suspend|ban",
    "duration_seconds": "number (optional)",
    "source_ticket_id": "uuid"
  }
}
```

### 9.5 moderation.appeal.submitted.v1

| 属性 | 值 |
|------|-----|
| partition_key | user_id |
| idempotency_key | event_id |
| producer | safety-service |
| consumers | （工单处理） |
| 触发时机 | 用户提交申诉 |
| 主链路 | 是 |

```json
{
  "payload": {
    "appeal_case_id": "uuid",
    "moderation_action_id": "uuid",
    "user_id": "uuid",
    "appeal_text": "string"
  }
}
```

### 9.6 moderation.appeal.resolved.v1

| 属性 | 值 |
|------|-----|
| partition_key | user_id |
| idempotency_key | event_id |
| producer | safety-service |
| consumers | （通知） |
| 触发时机 | 申诉处理完成 |
| 主链路 | 是 |

```json
{
  "payload": {
    "appeal_case_id": "uuid",
    "user_id": "uuid",
    "resolution": "upheld|overturned",
    "resolution_summary": "string"
  }
}
```

---

## 10. 模型平台域事件

### 10.1 model.invocation.completed.v1

| 属性 | 值 |
|------|-----|
| partition_key | trace_id 或 capability_name |
| idempotency_key | event_id |
| producer | model-gateway |
| consumers | （计费/审计/分析） |
| 触发时机 | 模型调用完成 |
| 主链路 | 否 |

```json
{
  "payload": {
    "invocation_id": "uuid",
    "capability_name": "string",
    "provider": "string",
    "model_id": "string",
    "prompt_version": "string",
    "latency_ms": 1234,
    "cost_estimate": 0.001,
    "status": "success|failure",
    "trace_id": "uuid"
  }
}
```

---

## 11. 投递语义与消费者要求

| 规则 | 说明 |
|------|------|
| 投递语义 | at-least-once |
| 幂等 | 消费者必须按 event_id 去重 |
| 乱序 | 以 occurred_at 为业务时间，遇乱序以更晚者为准 |
| 分区 | 用户相关事件以 actor_user_id 或 subject_user_id 分区 |
| replay_required | 画像管线、推荐反馈归并、训练数据采集需支持回放 |
