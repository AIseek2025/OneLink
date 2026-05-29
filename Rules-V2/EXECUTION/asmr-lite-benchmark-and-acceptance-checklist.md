# OneLink ASMR-Lite Benchmark 数据与验收标准清单

> 角色：`GPT 5.4 / Composer 2 / Composer 2 fast / Opus 4.6`
> 阶段：V2 执行阶段 / 记忆、推理、推荐匹配扩展
> 目标：为 `ASMR-Lite` 提供统一 benchmark 口径、目标阈值、上线门槛与回退策略

---

## 1. 文档定位

本文件不是实现文档，而是：

- `ASMR-Lite` 扩展阶段的 **统一 benchmark 与验收口径**
- `Composer 2` 与 `Composer 2 fast` 的验证入口参考
- `GPT 5.4` 收口与 `Opus 4.6` 验收时的共同量化基线

一句话：

> 没有 benchmark，这条路线只能停留在“看起来合理”；有了 benchmark，才有资格进入“工程上可持续优化”。

---

## 2. Benchmark 总体结构

`ASMR-Lite` 不只评估问答准确率，至少覆盖四类：

1. `Memory QA`
2. `Temporal & Update Reasoning`
3. `Matching Recommendation`
4. `Cost / Latency / Resource`

---

## 3. 数据集分层

### 3.1 Memory QA

覆盖五类任务：

- 事实类
- 偏好类
- 事件类
- 时间类
- 修正类

示例：

- “用户现在在哪个城市？”
- “用户是否不喜欢被推销式沟通？”
- “最近一次提到找投资人是什么时候？”
- “旧偏好是否已被新偏好替代？”

### 3.2 Temporal & Update Reasoning

单独收集：

- 时间线冲突
- 旧事实覆盖新事实
- 版本 supersede
- 过期事实误命中
- 时间链重建错误

### 3.3 Matching Recommendation

至少按场景拆分：

1. 婚恋交友
2. 求职招聘
3. 商业合作
4. 学习咨询
5. 投融资
6. 创作者合作
7. 同城兴趣/搭子
8. 语言交换
9. 跨境采购 / 外贸

### 3.4 Failure Cases

必须单独维护失败样本池，包括：

- 时间错判
- 偏好误判
- 推荐理由貌似合理但结果无效
- 负反馈过高
- 高风险请求未升级
- `L3` 触发过多

---

## 4. 对照组设计

至少保留 4 组：

### 4.0 与仓库脚本的命名映射（避免误读）

`repo/scripts/benchmark-asmr-lite-v2.sh` / `v2.1.sh` 中的 **`Baseline-A` / `Baseline-B`** 表示 **本地 shell 词法 scaffold**（全量 setup / 末条 setup + `if contains`），**不是**本节 4.1 / 4.2 所指的远期「向量」「单次 LLM」对照组。验收 v2/v2.1 时以 `tests/integration/ASMR_LITE_BENCHMARK_V2.md` 与 **`ASMR_LITE_BENCHMARK_V2.1.md`** 为准。

### 4.1 Baseline-A

纯向量 + 简单规则

### 4.2 Baseline-B

单大模型单次推理

### 4.3 Baseline-C

`L1` 确定性检索

### 4.4 Candidate

`L1 + L2 + L3` 完整 `ASMR-Lite`

说明：

- 不允许只拿 Candidate 自己和自己对比
- 不允许只拿离线样例不拿真实失败样本对比

---

## 5. 关键指标

### 5.1 Memory 指标

- `MemoryHit@K`
- `PreferencePrecision`
- `EvidenceCompleteness`
- `ContradictionMissRate`
- `FreshnessAccuracy`

### 5.2 Temporal 指标

- `TemporalOrderingAccuracy`
- `ConflictResolutionAccuracy`
- `SupersedeChainAccuracy`
- `ExpiredFactLeakRate`

### 5.3 Matching 指标

- `Recall@K`
- `Precision@K`
- `CTR`
- `DMStartRate`
- `ReplyRate`
- `EffectiveConnectionRate`
- `NegativeFeedbackRate`

### 5.4 Efficiency 指标

- `P50 latency`
- `P95 latency`
- `LLM calls per query`
- `tokens per query`
- `background_ingestion_lag`
- `cost per 1k queries`

---

## 6. 路由专项指标

### 6.1 L1

必须观察：

- `L1 share`
- `L1 miss escalated to L2`
- `L1 false confidence rate`

### 6.2 L2

必须观察：

- `L2 success rate`
- `L2 conflict handling accuracy`
- `L2 escalation to L3 rate`

### 6.3 L3

必须观察：

- `L3 share`
- `L3 overturn rate`
- `L3 high-value save rate`

### 6.4 目标分布

建议长期控制在：

- `L1`: `75% ~ 85%`
- `L2`: `12% ~ 20%`
- `L3`: `2% ~ 5%`

若出现：

- `L3 > 5%`
- 或 `L2` 长期错误率居高不下

优先动作应该是：

- 补结构化提取
- 调整 prompt
- 调整路由阈值

而不是直接增加更多在线 Agent。

---

## 7. 推荐目标区间

以下不是承诺值，而是建议的内部目标区间。

### 7.1 效果提升目标

相对 `Baseline-A`：

- 显式事实命中率：`+10% ~ +18%`
- 偏好理解准确率：`+12% ~ +20%`
- 时间冲突识别：`+20% ~ +35%`
- 推荐点击率：`+8% ~ +15%`
- 私信发起率：`+10% ~ +18%`
- 有效连接率：`+12% ~ +22%`

### 7.2 效率目标

相对原版 `ASMR` 研究路径：

- 推理成本下降：`93% ~ 97%`
- 延迟下降：`80% ~ 90%`

---

## 8. 3 个月与 6 个月目标

| 指标 | 3 个月目标 | 6 个月目标 |
|---|---:|---:|
| 记忆命中率 | `>= 88%` | `>= 92%` |
| 时间冲突正确率 | `>= 85%` | `>= 92%` |
| 复杂问题正确率 | `>= 90%` | `>= 95%` |
| 推荐有效连接率提升 | `>= 10%` | `>= 18%` |
| 平均新增推理调用 | `<= 0.8` | `<= 0.6` |
| `L3` 触发率 | `<= 6%` | `<= 5%` |

---

## 9. 上线门槛

### 9.1 Shadow

只有满足以下条件，才允许从离线 benchmark 进入 shadow：

- 所有核心指标均优于 `Baseline-A`
- `L3` 触发率未失控
- `P95 latency` 在目标预算内
- 失败样本中没有高风险场景系统性回归

### 9.2 Canary

只有满足以下条件，才允许从 shadow 进入 canary：

- shadow 至少连续 7 天稳定
- 关键推荐场景未出现显著负反馈上升
- 成本未突破预算
- 无新增边界污染或写路径污染

### 9.3 Rollout

只有满足以下条件，才允许扩大 rollout：

- canary 至少连续 14 天稳定
- 有效连接率真实提升
- `NegativeFeedbackRate` 未显著恶化
- `L3` 触发率仍在控制区间

---

## 10. 回退策略

若以下任一条件成立，必须回退：

- `NegativeFeedbackRate` 显著上升
- `P95 latency` 超预算
- `L3` 触发率异常飙升
- 时间冲突类正确率明显下降
- 高风险场景误判率不可接受

回退顺序：

1. 先回退模板或 prompt
2. 再回退路由阈值
3. 再关闭 `L3`
4. 最后回退到 `L1` 或 `Baseline-A`

不允许：

- 问题出现后继续硬顶 rollout
- 为了保留“新架构上线”表面成功而隐藏指标退化

---

## 11. 埋点与日志最低要求

### 11.1 查询级埋点

每次查询至少记录：

- 路由层级：`L1 / L2 / L3`
- 是否升级
- 触发原因
- 命中证据数
- 时间冲突数
- 置信度
- token 与调用数
- 延迟

### 11.2 推荐级埋点

每次结果集至少记录：

- 场景类型
- 模板版本
- 候选数
- 召回来源
- 是否使用长期记忆信号
- 是否触发 `L2 / L3`
- 曝光、点击、私信、回复、负反馈

### 11.3 失败样本沉淀

必须能回收到：

- 输入
- 路由选择
- 证据
- 输出
- 指标结果
- 用户负反馈

---

## 12. 角色分工

### 12.1 Composer 2

需要保证：

- 代码结构可支持 `L1 / L2 / L3`
- 异步结构化提取链路可扩展
- 日志与埋点预留齐全

### 12.2 Composer 2 fast

需要保证：

- README、脚本、测试壳与 benchmark 入口一致
- 验证文档能说明如何跑基准或至少如何验证核心埋点

### 12.3 GPT 5.4

需要保证：

- 文档、实现、OpenAPI、埋点口径一致
- 不能让 benchmark 文档和实现脱节

### 12.4 Opus 4.6

需要保证：

- 验收不只看“能跑”，还要看是否满足 benchmark 清单的口径约束

---

## 13. 当前阶段的现实边界

### 13.1 本轮不要求

- 立刻跑完完整推荐 benchmark
- 立刻接真实十亿级流量
- 立刻上线本地小模型替代 API

### 13.2 本轮必须预埋

- 路由层级概念
- 埋点字段
- benchmark 文档入口
- 失败样本收集机制

---

## 14. 一句话目标

> `ASMR-Lite` 的 benchmark 不是拿来证明方案“听起来更先进”，而是拿来证明它在 OneLink 里确实更准、更快、更省，并且可以被持续优化。
