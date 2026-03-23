# OneLink V2 Repo Impact Review

## 1. 文档目标

评估 `Rules-V2` 冻结后，对现有 `repo/` 的影响范围，明确哪些内容需要最小契约更新，哪些内容不需要重铺。

---

## 2. 总结结论

### 2.1 不需要重铺的部分

现有 `repo/` 基础骨架可以保留：

- Rust workspace 结构
- `services/context-service` 目录
- `platform/contracts/internal/context-service.yaml`
- `data-platform/db-schema/drafts/003_context.sql`
- `data-platform/event-schemas/*.json`

### 2.2 需要最小更新的部分

V2 真正需要更新的是“字段语义”和“接口插槽”，不是目录结构。

重点变化集中在：

1. `memory_type -> network_type`
2. `memory_artifacts` schema 升级
3. 增加 `memory_entities`
4. 增加 `memory_entity_links`
5. 增加 `agent_runtime_checkpoints`
6. 增加 `forgetting_decisions`
7. 增加 `Policy Config Store` 相关对象
8. `context-service` 增加 `session/checkpoint` 与 `memory/consolidate` 契约

---

## 3. 逐项影响评估

### 3.1 `repo/platform/contracts/internal/context-service.yaml`

当前状态：

- 已有 `/internal/context/build`
- 已有 `/internal/memory/write`
- 已有 `/internal/memory/search`

V2 最小影响：

- `ContextBuildRequest` 新增 `agent_id`
- `ContextBuildResponse` 新增 `retrieval_used` 与 `degraded`
- 新增 `/internal/session/checkpoint`
- 新增 `/internal/memory/consolidate`
- 搜索结果从 `memory_type` 升级为 `network_type`

### 3.2 `repo/data-platform/db-schema/drafts/003_context.sql`

当前状态：

- 仅覆盖 `memory_summaries`
- `memory_artifacts`
- `context_logs`

V2 最小影响：

- `memory_artifacts.memory_type` 升级为 `network_type`
- 新增 `evidence_type`
- 新增 `content_structured`
- 新增 `valid_from / valid_until`
- 新增 `entity_refs`
- 新增 `superseded_by`
- 新增 `memory_entities`
- 新增 `memory_entity_links`
- 新增 `agent_runtime_checkpoints`
- 新增 `forgetting_decisions`

### 3.3 `repo/data-platform/event-schemas/`

当前状态：

- 已有 `context.memory.extracted.v1`
- 已有 `context.memory.summary.updated.v1`
- 已有 `profile.memory_projection.requested.v1`

V2 最小影响：

- `context.memory.extracted.v1` 中 artifact 字段从 `memory_type` 升级为 `network_type`
- `profile.memory_projection.requested.v1` 保留，但可补充 `projection_inputs_version`
- 增加 checkpoint、遗忘、策略实验相关事件只在 Phase 2 之后再考虑，不要求立即新增

### 3.4 `repo/services/context-service/`

当前状态：

- 骨架存在
- 命名仍偏 V1 `context + memory compute`

V2 最小影响：

- 保持目录不变
- 在内部模块说明中显式拆为 `memory domain` 与 `session domain`
- 不需要重命名服务目录
- 不需要改 Rust workspace 结构

---

## 4. 明确不做的事

V2 当前阶段不建议做：

- 重铺 `repo/` 目录树
- 新增第二套并行 `repo-v2/`
- 批量重命名全部服务
- 为图检索单独新建微服务
- 为 `AutoResearch` 新建一整套生产服务骨架

---

## 5. 推荐的最小改动顺序

1. 先更 `Rules-V2` 文档
2. 再更新 `context-service.yaml`
3. 再更新 `003_context.sql`
4. 再更新 3 个 context 事件 schema
5. 最后再决定是否需要补 `optimization-layer` 或 `persona` 的 repo 占位文件

---

## 6. 最终判断

> `repo/` 不需要重建，只需要围绕 `context-service`、上下文字段、checkpoint、遗忘与策略配置做最小契约升级。
