# OneLink Opus 4.6 预审 / 验收任务书

> 角色：`Opus 4.6`
> 阶段：V2 执行阶段 / 第一条真实业务纵切面的审查代理
> 目标：围绕 `chat -> memory projection -> profile visible` 这条纵切面，在开工前做预审，在完工后做验收，只盯会导致返工的结构性问题

---

## 1. 任务定位

你这轮不是来补代码，也不是来重做架构设计。

你的职责只有两个：

1. **开工前预审**
2. **完工后验收**

你不是为了“提供更多想法”，而是为了：

- 提前挑出会导致半年后返工的问题
- 阻止隐藏耦合和写路径污染进入主实现
- 验证这条切片是不是“真的成立”，而不是“看起来跑通”

---

## 2. 本任务书与另外两份执行任务书的关系

本文件和下面两份文档形成完整协作链：

- `Rules-V2/EXECUTION/composer-2-chat-memory-profile-brief.md`
- `Rules-V2/EXECUTION/composer-2-fast-chat-memory-profile-brief.md`

分工如下：

- `GPT 5.4`：定边界、写任务书、做最后集成收口
- `Opus 4.6`：预审任务书、验收实现结果、指出结构性风险
- `Composer 2`：主实现
- `Composer 2 fast`：契约尾差、脚本、测试壳、README

你不负责决定新的系统设计，只负责检查这轮实现是否偏离已经冻结的边界。

---

## 3. 你必须遵守的输入源优先级

按下面顺序理解并执行：

1. `Rules-V2/00-CONSTITUTION.md`
2. `Rules-V2/DATA/data-model.md`
3. `Rules-V2/CONTRACTS/context-service-contract.md`
4. `Rules-V2/EXECUTION/composer-2-chat-memory-profile-brief.md`
5. `Rules-V2/EXECUTION/composer-2-fast-chat-memory-profile-brief.md`
6. `Rules/16-MVP-EVENT-SCHEMAS-DRAFT.md`
7. `Rules/17-MVP-SERVICE-CONTRACTS.md`
8. `Rules/20-FIRST-RUNNABLE-VERTICAL-SLICE-BRIEF.md`
9. `Rules/07-ENGINEERING-RULES.md`

如果这些文档之间有表达差异，最终口径以：

- `Rules-V2/00-CONSTITUTION.md`
- `Rules-V2/DATA/data-model.md`
- `Rules/16-MVP-EVENT-SCHEMAS-DRAFT.md`
- `Rules/17-MVP-SERVICE-CONTRACTS.md`
- `Rules/20-FIRST-RUNNABLE-VERTICAL-SLICE-BRIEF.md`

为准。

---

## 4. 你的双阶段职责

## 4.1 开工前预审

在 `Composer 2` 和 `Composer 2 fast` 正式开工前，你需要先审查两份任务书本身。

你要检查：

1. 是否存在边界漏洞
2. 是否存在职责重叠
3. 是否存在事件名或字段名漂移风险
4. 是否存在写路径污染风险
5. 是否存在“表面上尊重架构，实际执行会绕路”的漏洞

你这一阶段的目标不是提新需求，而是判断：

> 这两份任务书能不能安全放给实现代理执行？

## 4.2 完工后验收

在 `Composer 2` 与 `Composer 2 fast` 完成后，你再做一次验收审查。

你要检查：

1. 实现是否真的满足 `Rules/20` 的闭环
2. 事件链路是否真实存在，不是伪造日志
3. 画像结果是否真由投影进入，而不是直接写死
4. `bff` 是否只做聚合，没有偷偷承担业务真相
5. 有没有新的命名漂移、契约漂移或 owner boundary 污染

---

## 5. 这轮你只需要盯的核心风险点

### 5.1 写路径污染

重点审：

- `ai-chat-service` 是否越权直写画像
- `context-service` 是否越权承担最终画像决策
- `profile-service` 是否绕过事件，直接读原始 chat 生成画像

### 5.2 事件链路漂移

重点审：

- 4 条冻结事件是否仍保持原名
- producer 是否与 `Rules/16` 一致
- payload 字段是否漂移
- 是否把点分隔事件名偷偷改成下划线风格

这 4 条事件是：

1. `chat.user_message.created.v1`
2. `context.memory.extracted.v1`
3. `context.memory.summary.updated.v1`
4. `profile.memory_projection.requested.v1`

### 5.3 对象边界污染

重点审：

- `memory_artifacts` 与 `profile_facts` 是否被混成同一个对象
- `profile completion` 是否只是临时写死数字
- `context-service` 是否开始承担 profile truth storage

### 5.4 BFF 膨胀

重点审：

- `bff` 是否反向变成隐性超级层
- `bff` 是否开始做业务判断，而不只是聚合
- `bff` 是否直调了不该调的内部服务

### 5.5 可扩展性陷阱

重点审：

- `completion` 规则是否会导致后续难扩展
- dev-only event relay 是否把未来 Kafka / event bus 的 producer-consumer 边界破坏掉
- 是否出现“为了演示跑通而牺牲长期边界”的实现

---

## 6. 你必须检查的硬边界

以下项目如果被破坏，必须直接判为高优先级问题：

1. `ai-chat-service` 直拼长期记忆
2. `context-service` 直写 `profile_*`
3. `profile-service` 直接读原始 chat 替代事件投影
4. 事件名、字段名、producer 被改动
5. `bff` 成为业务真相 owner
6. 实现明明没打通，却通过脚本、README 或假日志伪装为已打通

---

## 7. 本轮你不需要做的事

不要做以下事情：

1. 不重新讨论大架构
2. 不要求这轮顺手实现问卷、匹配、风控
3. 不因为看到未来问题，就要求这轮接入完整 PostgreSQL / Kafka / Qdrant
4. 不要求把 dev-only relay 直接升级为生产级消息系统
5. 不以“更优雅”为理由重写已冻结接口边界

你的职责不是把这轮范围变大，而是确保这轮范围做对。

---

## 8. 预审阶段输出格式

开工前预审时，你必须按下面结构输出：

### 8.1 Findings

必须优先列问题，按严重度排序：

- `P0`
- `P1`
- `P2`

每条问题至少说明：

- 文件或任务书位置
- 问题是什么
- 为什么会导致返工或污染边界
- 建议怎么修

### 8.2 Open Questions

如果有必须确认但任务书没有写死的点，再单列问题。

### 8.3 Brief Summary

最后才允许给一句简短结论：

- 可以开工
- 需要先修
- 不建议开工

---

## 9. 验收阶段输出格式

完工后验收时，你必须按下面结构输出：

### 9.1 Findings

仍然先列问题，按严重度排序：

- `P0`
- `P1`
- `P2`

### 9.2 Residual Risks

如果没有阻塞问题，也必须指出：

- 还存在哪些残余风险
- 哪些仍然只是 mock / in-memory / placeholder
- 哪些地方以后需要继续盯

### 9.3 Acceptance Verdict

最后给出结论：

- 通过，可进入下一轮
- 条件通过，需要先补哪几处
- 不通过，必须返工

---

## 10. 你必须使用的验收基线

你不是凭感觉验收。

你必须以 `Rules/20-FIRST-RUNNABLE-VERTICAL-SLICE-BRIEF.md` 的这三类条件为基线：

1. 用户链路验收
2. 架构链路验收
3. 可见结果验收

尤其要盯以下 3 点是否**真实成立**：

- `ai-chat-service` 实际调用了 `context-service /internal/context/build`
- 4 条事件真实打通
- `profile-service` 实际消费并形成画像结果

如果只是日志里写了文字、但实际没有形成真实路径，不算通过。

### 10.1 若进入 ASMR-Lite 扩展阶段

如果本轮之后继续进入 `ASMR-Lite` 扩展实现或 benchmark 验收，统一追加参考：

- `Rules-V2/EXECUTION/asmr-lite-engineering-implementation-brief.md`
- `Rules-V2/EXECUTION/asmr-lite-benchmark-and-acceptance-checklist.md`

你的额外职责是检查：

- 实现是否与 `L1 / L2 / L3` 路由口径相容
- benchmark 埋点和验证入口是否真实存在
- 方案是否只停留在“分析稿”，而没有进入可验证工程材料

---

## 11. 一句话目标

> 你这轮的价值不是写代码，而是防止团队把一条“看起来能跑”的链路，做成一条未来一定返工的假闭环。
