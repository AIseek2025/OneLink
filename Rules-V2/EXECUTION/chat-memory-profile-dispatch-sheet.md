# OneLink Chat -> Memory -> Profile 一页式调度单

> 目标：把 `chat -> memory projection -> profile visible` 这条纵切面按固定协作链推进，避免多代理并行时出现越权、重复劳动或边界漂移

---

## 1. 本轮总目标

只推进这一条闭环：

```text
register
 -> login
 -> bff chat init
 -> create/get conversation
 -> send message
 -> ai-chat-service calls context-service
 -> ai-chat-service calls model-gateway
 -> 4 frozen events emitted
 -> profile-service consumes projection
 -> profile/me or profile/me/completion shows visible change
```

本轮 dev-only 事件传递推荐模式也一并冻结：

- 使用 `HTTP envelope relay`
- 生产者异步 `POST` 标准事件 envelope 到消费者的 `POST /internal/events/receive`
- `ai-chat-service` -> `context-service`
- `context-service` -> `profile-service`
- `context.memory.extracted.v1` 与 `context.memory.summary.updated.v1` 允许 `context-service` 内部自消费，但必须保留真实 envelope 与真实日志
- 未来迁移到 Kafka 时，只替换投递层，不改事件 schema、producer、consumer 关系

本轮不扩张到：

- 问卷主链路
- 找人 / 匹配 / 风控
- 真实 PostgreSQL / Kafka / Qdrant 接入

---

## 2. 三份任务书

本轮协作以以下 3 份执行文档为准：

- `Rules-V2/EXECUTION/composer-2-chat-memory-profile-brief.md`
- `Rules-V2/EXECUTION/composer-2-fast-chat-memory-profile-brief.md`
- `Rules-V2/EXECUTION/opus-chat-memory-profile-review-brief.md`

验收基线：

- `Rules/20-FIRST-RUNNABLE-VERTICAL-SLICE-BRIEF.md`

---

## 3. 调度顺序

### Step 0. GPT 5.4

职责：

- 冻结边界
- 维护这 3 份任务书
- 在 `Composer 2` 与 `Composer 2 fast` 完成后做跨服务收口

输入文件：

- `Rules-V2/00-CONSTITUTION.md`
- `Rules-V2/DATA/data-model.md`
- `Rules-V2/CONTRACTS/context-service-contract.md`
- `Rules/16-MVP-EVENT-SCHEMAS-DRAFT.md`
- `Rules/17-MVP-SERVICE-CONTRACTS.md`
- `Rules/20-FIRST-RUNNABLE-VERTICAL-SLICE-BRIEF.md`

禁止事项：

- 不把本轮目标扩张成新业务纵切面
- 不在主实现尚未完成前提前重做大范围文档

交付物：

- 3 份执行任务书
- 最终跨服务收口结果

完成后交给：

- `Opus 4.6` 做开工前预审

---

### Step 1. Opus 4.6（开工前预审）

职责：

- 审查 `Composer 2` 与 `Composer 2 fast` 任务书是否存在边界漏洞、职责重叠、事件漂移风险、假闭环风险

输入文件：

- `Rules-V2/EXECUTION/composer-2-chat-memory-profile-brief.md`
- `Rules-V2/EXECUTION/composer-2-fast-chat-memory-profile-brief.md`
- `Rules-V2/EXECUTION/opus-chat-memory-profile-review-brief.md`
- `Rules/20-FIRST-RUNNABLE-VERTICAL-SLICE-BRIEF.md`

禁止事项：

- 不要求顺手实现问卷、匹配、风控
- 不重新讨论大架构

交付物：

- 审查结论，必须按 `P0 / P1 / P2` 输出
- 结论只能是：
  - 可以开工
  - 需要先修
  - 不建议开工

完成后交给：

- `Composer 2`

---

### Step 2. Composer 2（主实现）

职责：

- 完成本轮主业务实现

必须做：

- `identity-service`
  - `POST /api/v1/identity/register`
  - `POST /api/v1/identity/login`
  - `GET /api/v1/identity/me`
- `profile-service`
  - `GET /api/v1/profile/me`
  - `GET /api/v1/profile/me/completion`
  - 消费 `profile.memory_projection.requested.v1`
- `bff`
  - `GET /api/v1/bff/chat/init`
- dev-only event relay
- 让“记忆已进入画像”真实可见

默认实现选择：

- relay 采用 `HTTP envelope relay`
- `bff` 透传 `Authorization` 头给 `identity-service /api/v1/identity/me`
- `profile-service` 收到投影时若画像不存在，自动创建最小空画像

主要改动区域：

- `repo/services/identity-service/`
- `repo/services/profile-service/`
- `repo/services/bff/`
- `repo/platform/shared-libs/`（如确实需要 dev-only relay）

禁止事项：

- 不让 `ai-chat-service` 直写画像
- 不让 `context-service` 直写画像
- 不改冻结事件名、字段名、producer
- 不把 `bff` 变成业务真相 owner

交付物：

- 主实现代码
- 最小可见结果
- 对应接口可本地验证

完成后交给：

- `Composer 2 fast`

---

### Step 3. Composer 2 fast（配套落地）

职责：

- 补齐契约尾差、README、脚本、测试壳、验证文档

必须做：

- 同步：
  - `identity-service.yaml`
  - `profile-service.yaml`
  - `bff.yaml`
- 新增或补齐：
  - `scripts/local/run-chat-memory-profile-slice.sh`
  - `tests/integration/` 下的 smoke test 壳或验证文档
  - `identity-service` / `profile-service` / `bff` README

禁止事项：

- 不承担主业务逻辑
- 不新增新的 service boundary
- 不发明新的事件 schema 或 payload

交付物：

- 配套契约
- 本地脚本
- 测试壳
- 验证文档
- README

完成后交给：

- `GPT 5.4`

---

### Step 4. GPT 5.4（跨服务收口）

职责：

- 修掉接口、字段、调用链、文档和实现之间的最后差异

重点检查：

- `identity/profile/bff` 实现是否与 `Rules/15` 一致
- dev-only relay 是否仍保留真实 producer / consumer 关系
- OpenAPI / README / 测试壳是否与实现一致

禁止事项：

- 不重写 Composer 2 的主实现
- 不无故扩大范围

交付物：

- 最终收口后的仓库状态
- 可交给 `Opus 4.6` 验收的版本

完成后交给：

- `Opus 4.6`

---

### Step 5. Opus 4.6（完工后验收）

职责：

- 按 `Rules/20` 做最终验收

重点检查：

- 4 条事件是否真实打通
- `profile-service` 是否真实消费并形成画像结果
- `bff` 是否只做聚合
- 有没有假闭环、写路径污染、命名漂移

交付物：

- 验收结论，必须按：
  - `P0 / P1 / P2 findings`
  - `Residual Risks`
  - `Acceptance Verdict`

最终结论只能是：

- 通过，可进入下一轮
- 条件通过，需要先补若干问题
- 不通过，必须返工

---

## 3.1 扩展阶段参考执行材料

如果本轮主切片完成后，要继续推进 `ASMR-Lite` 的记忆 / 推理 / 推荐匹配增强，统一参考以下新增执行材料：

- `Rules-V2/EXECUTION/asmr-lite-engineering-implementation-brief.md`
- `Rules-V2/EXECUTION/asmr-lite-benchmark-and-acceptance-checklist.md`

作用：

- 前者负责工程实施路径
- 后者负责 benchmark、验收口径、上线门槛与回退策略

说明：

- 它们不改变本轮 `chat -> memory projection -> profile visible` 的范围
- 它们只作为下一阶段扩展工作的 canonical 输入源

---

## 4. 每个代理的禁止事项速查

### GPT 5.4

- 不提前扩张到第二条业务纵切面
- 不在主实现前大范围重写规则

### Opus 4.6

- 不编码
- 不重新讨论大架构
- 不用“更优雅”为理由扩大范围

### Composer 2

- 不改边界
- 不越权写画像
- 不绕过 `context-service`

### Composer 2 fast

- 不承担主实现
- 不新增 schema / ownership
- 不用脚本伪装实现

---

## 5. 并行发单顺序

你实际发单时，建议严格按下面顺序：

1. 先把 `Composer 2` 任务书发给 `Opus 4.6` 做预审
2. `Opus` 回答“可以开工”后，先发给 `Composer 2`
3. `Composer 2` 主实现达到可读状态后，再发给 `Composer 2 fast` 同步 OpenAPI、README、脚本、测试壳
4. 两者完成后，把结果交回 `GPT 5.4` 收口
5. 最后再发给 `Opus 4.6` 做验收

不要一开始就四个一起盲跑。

---

## 6. 一句话调度原则

> `GPT` 负责定边界，`Opus` 负责拦风险，`Composer 2` 负责主实现，`Composer 2 fast` 负责把主实现变得可联调、可交接、可验收。
