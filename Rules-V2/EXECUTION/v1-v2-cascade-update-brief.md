# OneLink V1 -> V2 Cascade Update Brief

## 1. 文档目标

把 `Rules-V2` 已冻结的架构结果，转成后续要级联更新的 V1 文档、SQL 草案、OpenAPI 草案、事件 schema 与 repo 契约清单。

本文件不是最终实现，而是后续实现代理的唯一级联任务入口。

---

## 2. 总体原则

### 2.1 先改来源，再改派生物

顺序固定为：

1. `Rules-V2`
2. V1 核心规则文档
3. V1 工程草案
4. `repo/` 契约文件

### 2.2 不要并行发明口径

所有字段名、接口名、事件名都以 `Rules-V2` 为准。

---

## 3. 需要重写的 V1 核心文档

### 3.1 `Rules/00-EXECUTIVE-BLUEPRINT.md`

更新目标：

- 明确 V1 已退居历史蓝图
- 增加指向 `Rules-V2` 的入口说明
- 更新总图为五条主轴版本

### 3.2 `Rules/02-TECH-ARCHITECTURE.md`

更新目标：

- 将 `context-service` 明确升级为双域单进程：`memory domain + session domain`
- 写入 `logical agent + runtime wakeup`
- 写入 `optimization-layer` 作为控制平面
- 写入四层记忆模型

### 3.3 `Rules/03-AI-PROFILE-QUESTIONNAIRE.md`

更新目标：

- 画像与问卷的输入来源改为消费 `memory-layer`
- 明确 `Question Policy` 是优化域之一
- 画像不得绕过 `Memory Layer`

### 3.4 `Rules/04-MATCHING-SAFETY-GOVERNANCE.md`

更新目标：

- 匹配与安全都消费 `memory` 信号，但不拥有其主写权
- 写入 `Matching Policy / Safety & Persuasion Policy`
- 写入 Lumi 的劝导边界

### 3.5 `Rules/05-MODEL-PLATFORM-ROADMAP.md`

更新目标：

- 明确 `AutoResearch` 是控制平面
- 明确 Lumi 的表达策略可优化，但人格宪法不可优化
- 明确双模型 / 多模型由 `model-gateway` 配置路由，不写死在业务代码

### 3.6 `Rules/10-SERVICE-BOUNDARIES.md`

更新目标：

- `context-service` 的新职责拆分
- `optimization-layer` 的边界
- `ai-friend persona` 与业务层的关系

### 3.7 `Rules/11-DATA-EVENT-MODEL.md`

更新目标：

- `memory_artifacts.memory_type -> network_type`
- 新增 `memory_entities`
- 新增 `memory_entity_links`
- 新增 `agent_runtime_checkpoints`
- 新增 `forgetting_decisions`
- 新增 `policy_configs / policy_experiments / policy_rollouts`

### 3.8 `Rules/19-CONTEXT-MEMORY-ARCHITECTURE.md`

更新目标：

- 升级为 `memory-layer + session-layer` 的 V2 视角
- 不再只定义 context-service 的 V1 范式
- 写入检索总架构、选择性遗忘与 checkpoint 版本化

---

## 4. 需要级联更新的 V1 工程草案

### 4.1 `Rules/14-MVP-SQL-SCHEMA-DRAFT.md`

必须更新：

- `memory_artifacts.network_type`
- `evidence_type`
- `content_structured`
- `valid_from / valid_until`
- `entity_refs`
- `superseded_by`
- `memory_entities`
- `memory_entity_links`
- `agent_runtime_checkpoints`
- `forgetting_decisions`
- `policy_configs`

### 4.2 `Rules/15-MVP-OPENAPI-DRAFT.md`

必须更新：

- `/internal/context/build` 请求体增加 `agent_id`
- 响应体增加 `retrieval_used`、`degraded`
- 新增 `/internal/session/checkpoint`
- 新增 `/internal/memory/consolidate`

### 4.3 `Rules/16-MVP-EVENT-SCHEMAS-DRAFT.md`

必须更新：

- `context.memory.extracted.v1` 的候选字段改为 `network_type`
- 记忆整合必须强调 `event_id` 幂等与 replay
- 视需要增加 checkpoint / forgetting 相关事件的预留说明

### 4.4 `Rules/17-MVP-SERVICE-CONTRACTS.md`

必须更新：

- 新增 `session/checkpoint`
- 新增 `memory/consolidate`
- 新增 `optimization-layer -> Policy Config Store` 的控制关系说明

---

## 5. repo 契约更新清单

### 5.1 必改

- `repo/platform/contracts/internal/context-service.yaml`
- `repo/data-platform/db-schema/drafts/003_context.sql`
- `repo/data-platform/event-schemas/context.memory.extracted.v1.json`
- `repo/data-platform/event-schemas/context.memory.summary.updated.v1.json`
- `repo/data-platform/event-schemas/profile.memory_projection.requested.v1.json`

### 5.2 评估后再改

- `repo/services/context-service/README.md`
- `repo/README.md`
- `repo/data-platform/db-schema/drafts/README.md`

---

## 6. 拆分给实现代理的推荐顺序

1. 先更新 `Rules/11`
2. 再更新 `Rules/14`
3. 再更新 `Rules/15`
4. 再更新 `Rules/16`
5. 再更新 `repo/` 契约文件
6. 最后回改 `Rules/02/03/04/05/10/19`

说明：

- 这样做能先把字段和契约钉住，再更新叙述层
- 可减少再次出现“文字已改、契约未改”的错位

---

## 7. 最终判断

> V2 级联更新的重点不是“把旧文档全删掉”，而是让所有仍需保留的 V1 派生草案都严格受 V2 核心文档约束。
