# OneLink V2 Implementation Brief

## 1. 文档目标

为后续实现代理提供一份基于 `Rules-V2` 的执行任务书，明确先做什么、后做什么、哪些能力先预埋、哪些能力先默认关闭。

---

## 2. 实施总原则

### 2.1 先冻结，后实现

所有实现必须以以下文档为输入优先级：

1. `Rules-V2/00-CONSTITUTION.md`
2. `Rules-V2/ARCHITECTURE/system-overview.md`
3. `Rules-V2/ARCHITECTURE/memory-layer.md`
4. `Rules-V2/ARCHITECTURE/session-layer.md`
5. `Rules-V2/ARCHITECTURE/optimization-layer.md`
6. `Rules-V2/ARCHITECTURE/agent-runtime-and-selective-forgetting.md`
7. `Rules-V2/ARCHITECTURE/ai-friend-persona-and-growth.md`
8. `Rules-V2/DATA/data-model.md`
9. `Rules-V2/CONTRACTS/context-service-contract.md`

### 2.2 不要重铺目录

优先在现有 `repo/` 中最小更新，不新建第二套并行仓库。

---

## 3. 第一步实现目标

第一轮实现不追求把 V2 全部做完，而追求：

1. 把 V2 的关键字段与契约更新进现有骨架
2. 保持 `context-service` 仍是主链路核心
3. 为 V2 新能力预埋最小插槽

---

## 4. 第一轮必须完成的项

### 4.1 context-service 契约升级

更新：

- `/internal/context/build`
- `/internal/session/checkpoint`
- `/internal/memory/consolidate`

### 4.2 记忆模型升级

更新：

- `memory_artifacts.network_type`
- `memory_entities`
- `memory_entity_links`
- `forgetting_decisions`

### 4.3 运行时升级

引入：

- `agent_runtime_checkpoints`
- `schema_version`
- runtime 休眠/唤醒的最小接口占位

### 4.4 Policy Config Store 占位

至少定义：

- 配置对象
- 读取方式
- 默认值

不要求第一轮就实现完整 `AutoResearch`。

---

## 5. 第一轮允许默认关闭的能力

### 5.1 自动优化域

默认只开启：

- `Memory Policy`
- `Session Policy`
- `Retrieval Policy`

### 5.2 检索能力

默认只开启：

- 结构化
- 语义
- 时间

预埋但默认关闭：

- 图扩展
- 完整 rerank

### 5.3 Lumi 高风险能力

默认保留模板式劝导，不做自动优化。

---

## 6. 推荐实施顺序

1. 更新 `Rules/11` 与 `Rules/14`
2. 更新 `Rules/15` 与 `context-service.yaml`
3. 更新 `003_context.sql`
4. 更新 3 个 context 事件 schema
5. 更新 `context-service` README 与内部模块说明
6. 最后再扩展到 optimization / persona 占位

---

## 7. 禁止事项

- 不把 `AutoResearch` 插进在线主链路
- 不把 Lumi 宪法写成可热更新配置
- 不把图检索做成 MVP 必须依赖
- 不因 V2 升级而重铺全部 repo 目录
- 不把 `logical agent` 错做成“每用户常驻进程”

---

## 8. 完成标志

当以下事项全部成立时，第一轮 V2 实施准备完成：

- `Rules-V2` 作为新权威已可引用
- V1 工程草案已有级联更新清单
- repo 最小影响面已明确
- 后续实现代理有清晰的契约更新顺序

---

## 9. 一句话目标

> 第一轮 V2 不是做完全部新系统，而是把未来两年都不会后悔的边界、字段和接口先做对。
