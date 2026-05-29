# OneLink Composer 2 Fast 配套任务书

> 角色：`Composer 2 fast`
> 阶段：V2 执行阶段 / 第一条真实业务纵切面的配套落地
> 目标：在不越权承担主业务实现的前提下，为 `chat -> memory projection -> profile visible` 这条纵切面补齐契约尾差、脚本、测试壳、README 和验证说明

---

## 1. 任务定位

你这轮不是继续做架构设计，也不是主导 `identity-service`、`profile-service`、`bff` 的核心业务逻辑。

你的职责只有一个：

**把已经由总设计师冻结、并由 Composer 2 主任务书定义好的这条纵切面，批量、快速、整齐地铺满仓库的配套层。**

你要补的是：

1. 契约尾差同步
2. README 与运行说明
3. 本地脚本
4. 测试壳与验证文档
5. 批量补齐已有实现周边缺失的样板

你不要做的是：

1. 不决定新的架构边界
2. 不发明新的事件名、字段名、producer
3. 不承担 `identity/profile/bff` 的主实现逻辑
4. 不把占位文档冒充成“主链已实现”

---

## 2. 本任务书与 Composer 2 主任务书的关系

本文件是：

- `Rules-V2/EXECUTION/composer-2-chat-memory-profile-brief.md`

的**配套任务书**。

关系如下：

- `Composer 2`：负责主实现
- `Composer 2 fast`：负责配套落地与收尾

执行原则：

- **主逻辑边界** 以 `composer-2-chat-memory-profile-brief.md` 为准
- **验收标准** 仍以 `Rules/20-FIRST-RUNNABLE-VERTICAL-SLICE-BRIEF.md` 为基线
- 你只能在不改写主实现职责的前提下，补齐配套层
- 你的 OpenAPI、README、脚本、测试壳同步动作，默认发生在 `Composer 2` 主实现完成之后

如果发现需要改字段、改事件、改 service ownership，**停止并交回 GPT 5.4**，不要自己决定。

---

## 3. 你必须遵守的输入源优先级

按下面顺序理解并执行：

1. `Rules-V2/EXECUTION/composer-2-chat-memory-profile-brief.md`
2. `Rules/20-FIRST-RUNNABLE-VERTICAL-SLICE-BRIEF.md`
3. `Rules/15-MVP-OPENAPI-DRAFT.md`
4. `Rules/16-MVP-EVENT-SCHEMAS-DRAFT.md`
5. `Rules/17-MVP-SERVICE-CONTRACTS.md`
6. `Rules-V2/CONTRACTS/context-service-contract.md`
7. `Rules/07-ENGINEERING-RULES.md`
8. `Rules/18-COMPOSER-2-FAST-EXECUTION-BRIEF.md`

如果这些文档之间有表达差异，最终口径以：

- `composer-2-chat-memory-profile-brief.md`
- `Rules/16-MVP-EVENT-SCHEMAS-DRAFT.md`
- `Rules/17-MVP-SERVICE-CONTRACTS.md`

为准。

---

## 4. 你的职责边界

### 4.1 你必须做的事

- 同步 `identity-service`、`profile-service`、`bff` 的 OpenAPI 契约尾差
- 补 `scripts/local/` 下的本地运行脚本
- 补 `tests/integration/` 下的 smoke test 壳或验证文档
- 更新相关服务 README
- 更新根 README 或集成 README 中的本轮运行说明

### 4.2 你可以做但必须克制的事

- 在不改变主逻辑的前提下补充示例请求 / 示例响应
- 为本轮纵切面增加最小验证命令
- 为日志和本地联调补少量说明性文本

### 4.3 你不能做的事

- 不实现 `identity/register`、`identity/login`、`identity/me` 的主逻辑
- 不实现 `profile.memory_projection.requested.v1` 的消费者逻辑
- 不实现 `bff/chat/init` 的主聚合逻辑
- 不新增 infra 依赖来替代主实现缺口
- 不擅自补充新的事件 schema 或修改事件 payload 结构

---

## 5. 当前 repo 现状（你需要据此收尾）

当前仓库里：

- `repo/services/bff/src/http/routes.rs` 仍只有 placeholder
- `repo/platform/contracts/openapi/identity-service.yaml` 仍是 skeleton
- `repo/platform/contracts/openapi/profile-service.yaml` 仍是 skeleton
- `repo/platform/contracts/openapi/bff.yaml` 已存在，但需按本轮实际实现同步
- `repo/scripts/README.md` 仍是占位
- `repo/tests/integration/README.md` 仍是占位

这意味着你这轮的重点不是“再开新坑”，而是把这些配套层真正变成可交付的支撑物。

说明：

- 本轮的 OpenAPI 文件由你统一承担，避免和 `Composer 2` 在同一批 YAML 上重复改动
- 如果主实现尚未稳定，优先等待或基于已完成接口结果同步，不要抢跑改契约

---

## 6. 你本轮必须覆盖的文件区域

至少覆盖：

- `repo/platform/contracts/openapi/identity-service.yaml`
- `repo/platform/contracts/openapi/profile-service.yaml`
- `repo/platform/contracts/openapi/bff.yaml`
- `repo/services/identity-service/README.md`
- `repo/services/profile-service/README.md`
- `repo/services/bff/README.md`
- `repo/scripts/`
- `repo/tests/integration/`

允许按需要补充：

- `repo/README.md`
- `repo/platform/contracts/openapi/README.md`
- `repo/tests/integration/CHAT_MEMORY_PROFILE_SLICE.md`
- `repo/scripts/local/run-chat-memory-profile-slice.sh`

---

## 7. 你必须交付的内容

### 7.1 OpenAPI 契约尾差同步

你需要把本轮真实会用到的接口，在 OpenAPI 中从纯 placeholder 同步到“最小可联调”状态。

至少包括：

#### identity-service

- `POST /api/v1/identity/register`
- `POST /api/v1/identity/login`
- `GET /api/v1/identity/me`

#### profile-service

- `GET /api/v1/profile/me`
- `GET /api/v1/profile/me/completion`

#### bff

- `GET /api/v1/bff/chat/init`

要求：

- 路径必须与 `Rules/15` 一致
- 请求/响应字段名必须与 `Rules/15` 一致
- 可以保留“draft / in-memory / placeholder-backed”说明
- 不要擅自补出新的业务字段

### 7.2 本地运行脚本

至少交付一份本地脚本，例如：

- `repo/scripts/local/run-chat-memory-profile-slice.sh`

脚本目标：

1. 启动本轮参与服务
2. 给出最小验证顺序
3. 让开发者无需自己猜命令

要求：

- 脚本可以是串行启动说明脚本，不要求完美进程管理
- 如果服务启动依赖已有命令，直接包装这些命令即可

### 7.3 集成测试壳或验证文档

至少交付以下之一，最好同时具备：

1. `repo/tests/integration/chat_memory_profile_slice.rs`
2. `repo/tests/integration/CHAT_MEMORY_PROFILE_SLICE.md`

必须写清：

- 如何注册和登录
- 如何调用 `bff/chat/init`
- 如何发消息
- 如何验证 `profile/me` 或 `profile/me/completion` 发生变化

### 7.4 README 同步

至少更新：

- `repo/services/identity-service/README.md`
- `repo/services/profile-service/README.md`
- `repo/services/bff/README.md`

README 至少说明：

- 当前服务在这轮纵切面中负责什么
- 依赖哪些服务
- 哪些能力仍然是 in-memory / mock / placeholder
- 如何在本地跑这一轮切片

---

## 8. 允许的简化

为了保证你这轮快速收尾，允许：

- README 中明确标注“主逻辑由 Composer 2 实现，本文件只做配套说明”
- 测试先做 smoke shell，不要求一开始就做完整断言
- OpenAPI 先做到最小请求/响应示例，不要求覆盖所有错误分支
- 脚本先做开发态串行版本，不要求上来就处理所有异常退出

但不允许：

- 用文档替代缺失实现，然后声称已打通
- 用脚本偷偷绕过主服务调用边界
- 在 README 或 YAML 中写出和 `Rules/15/16/17/20` 相冲突的新口径

---

## 9. 推荐执行顺序

严格按下面顺序推进：

1. 先读 `composer-2-chat-memory-profile-brief.md`
2. 等 `Composer 2` 的主实现达到可读状态后，再同步 `identity-service.yaml`、`profile-service.yaml`、`bff.yaml`
3. 再补 `scripts/local/run-chat-memory-profile-slice.sh`
4. 再补 `tests/integration/` 的 smoke test 壳或验证文档
5. 最后统一更新 3 个服务 README 与根运行说明

说明：

- 你这轮是收尾，不是抢先做主实现
- 一旦发现契约与实现即将冲突，立即停手并回报 GPT 5.4

---

## 10. 验收标准

### 10.1 配套层完整

完成后必须满足：

- `identity-service.yaml`、`profile-service.yaml`、`bff.yaml` 已同步到本轮最小实现范围
- `scripts/local/` 下已有可执行或可直接参考的本地运行脚本
- `tests/integration/` 下已有 smoke test 壳或验证说明
- `identity-service`、`profile-service`、`bff` 的 README 已更新

### 10.2 不越权

必须满足：

- 没有把主业务逻辑写进脚本或 README
- 没有新增新的 service boundary
- 没有改事件名、字段名、producer 关系
- 没有把 OpenAPI 改成与 `Rules/15` 冲突的新口径

### 10.3 可交接

必须满足：

- Composer 2 完成主实现后，开发者可以直接用你的脚本和说明进行联调
- GPT 5.4 可以直接在你补好的契约与 README 上做最后收口
- Opus 4.6 可以据此直接做验收审查

### 10.4 后续 Benchmark 与扩展阶段对齐

本轮你不负责实现完整 `ASMR-Lite` benchmark，但后续扩展阶段的配套层需要统一对齐以下文档：

- `Rules-V2/EXECUTION/asmr-lite-engineering-implementation-brief.md`
- `Rules-V2/EXECUTION/asmr-lite-benchmark-and-acceptance-checklist.md`

这意味着：

- README 中不要写出与 `L1 / L2 / L3` 路由相冲突的叙述
- 脚本和验证文档应尽量给未来 benchmark 埋点和验证入口留出说明空间
- 如果未来新增 benchmark 运行命令、shadow / canary 说明，应继续沿用这两份文档的口径

---

## 11. 完成后必须汇报的内容

请按以下格式汇报：

1. 同步了哪些 OpenAPI 文件
2. 新增了哪些脚本
3. 新增了哪些测试壳或验证文档
4. 更新了哪些 README
5. 哪些地方仍然依赖 Composer 2 的主实现完成
6. 有没有发现契约和实现之间的新冲突

---

## 12. 一句话目标

> 你这轮不是主力写业务，而是把这条纵切面的“最后一公里配套层”补齐：
> **让仓库的契约、脚本、测试壳和说明文档，跟主实现一起变得可联调、可交接、可验收。**
