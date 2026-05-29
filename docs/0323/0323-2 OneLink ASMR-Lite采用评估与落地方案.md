# OneLink ASMR-Lite 采用评估与落地方案

基于以下材料综合判断：

- `OneLink/Rules-V2/ARCHITECTURE/memory-layer.md`
- `OneLink/Rules-V2/ARCHITECTURE/optimization-layer.md`
- `OneLink/Rules-V2/ARCHITECTURE/system-overview.md`
- `OneLink/Rules-V2/ARCHITECTURE/agent-runtime-and-selective-forgetting.md`
- `OneLink/Rules/04-MATCHING-SAFETY-GOVERNANCE.md`
- `OneLink/Rules/17-MVP-SERVICE-CONTRACTS.md`
- `OneLink/docs/0323/0323-1 We broke the frontier in agent memory.md`
- `OneLink/docs/0323/0323-1 Supermemory的ASMR技术.md`
- 公开仓库与文档：`supermemory`、`memorybench`、`code-chunk`、`supermemory-mcp`

---

## 0. 最终结论

### 0.1 是否采用

结论：**采用思想，不采用原版在线多 Agent 形态。**

OneLink 最适合的不是原版：

- `3 Reader Agents`
- `3 Search Agents`
- `8/12 个 Answer Agents`
- 高频并行投票与聚合

而是演进成：

> **OneLink ASMR-Lite = 异步结构化记忆 + 确定性检索 + 少量按需推理 + AutoResearch 离线优化**

### 0.2 为什么不照搬

原版 ASMR 的核心价值是证明：

1. `Agentic Retrieval > 纯向量检索`
2. 时间冲突与更新问题，确实是记忆系统的核心难点
3. 专业化检索/推理，比一个大 prompt 硬做更强

但它不适合 OneLink 直接生产采用，因为：

1. 在线成本太高
2. 延迟不稳定
3. 难以审计和回放
4. 很难直接扩展到推荐匹配主链路
5. 对十亿级规模不友好

### 0.3 OneLink 的正确版本

OneLink 应采用三层在线路由：

- `L1`：确定性结构化检索，默认路径，0 次额外 LLM 检索调用
- `L2`：统一 `Search Agent + Reason Agent`，默认复杂问题路径，2 次 LLM 调用
- `L3`：少量复杂问题触发复核或时间冲突深推理，额外 +1 次调用

建议目标：

- `75%~85%` 查询走 `L1`
- `12%~20%` 查询走 `L2`
- `2%~5%` 查询走 `L3`

在这个分布下，平均每次查询新增推理调用约为：

```text
0 * 0.80 + 2 * 0.17 + 3 * 0.03 = 0.43 次 / 查询
```

如果把原版 ASMR 按 `8~14 次` LLM 调用 / 查询估算，则 OneLink ASMR-Lite 的推理成本可比原版下降：

- **约 93% ~ 97%**

---

## 1. OneLink 应该吸收什么，不该吸收什么

### 1.1 应吸收

#### A. 六维结构化提取思想

把用户历史从原始文本改造成稳定结构：

- `Personal Information`
- `Preferences`
- `Events`
- `Temporal Data`
- `Updates`
- `Assistant Info`

但 OneLink 不必单独再造一套新存储，而应映射到现有四网络：

| ASMR 六维度 | OneLink 四网络映射 |
|---|---|
| Personal Information | `world` + `entity` |
| Preferences | `opinion` |
| Events | `experience` + `entity` |
| Temporal Data | `experience` + `valid_from/valid_until` |
| Updates | `superseded_by` + `conflict marking` |
| Assistant Info | `experience` + `source_type=assistant` |

一句话：**六维提取保留为抽取视角，四网络继续作为底层统一存储。**

#### B. 主动检索替代盲目向量检索

OneLink 应明确：

- 用户私有记忆，不以“纯向量相似度”作为主判定
- 用户更新、时间冲突、事实修正，优先走结构化检索和时间推理
- 图关系、时间链、冲突链，必须成为第一类检索对象

#### C. 专业化分工

同一个 prompt 做完：

- 找事实
- 找隐含线索
- 重建时间线
- 做最终结论

理论上可行，但生产上不优。

OneLink 应拆成最小专业化：

- `Search Agent`
- `Reason Agent`
- `Verifier Agent`（仅复杂问题）

#### D. Benchmark 先行

supermemory 最大的工程启发之一，不是“用了多少 Agent”，而是它非常强调：

- 公开 benchmark
- 数据集分层
- 系统间可比性

OneLink 应借鉴 `memorybench` 的思路，建立自己的：

- 记忆 benchmark
- 时间冲突 benchmark
- 推荐匹配 benchmark
- 成本与延迟 benchmark

### 1.2 不应吸收

#### A. 在线大规模并行多 Agent

这只适合研究极限，不适合生产主路径。

#### B. 让 Agent 去读大量原始 session 文本

OneLink 已经有 `memory-layer` 和 `consolidation pipeline` 方向，应该优先让 Agent 读：

- `memory_artifacts`
- `memory_summaries`
- `memory_entities`
- `memory_entity_links`

而不是每次回看原始文本。

#### C. 把匹配推荐也做成重型投票系统

推荐匹配要可控、可审计、可灰度，应该由：

- 硬过滤
- 召回
- 精排
- 多样性
- 解释层

组成，而不是在线并行十几个 Agent 投票。

---

## 2. 效率和效果能提升多少

以下数字不是承诺值，而是**建议作为 OneLink 内部 benchmark 的目标区间**。

### 2.1 相对“纯向量记忆 + 简单规则”基线

#### 记忆与推理效果

- 显式事实命中率：**+10% ~ +18%**
- 偏好理解准确率：**+12% ~ +20%**
- 时间冲突/更新识别准确率：**+20% ~ +35%**
- 多轮关系线索恢复能力：**+15% ~ +25%**

#### 推荐匹配效果

在有足够画像和反馈数据的场景下：

- 推荐点击率：**+8% ~ +15%**
- 私信发起率：**+10% ~ +18%**
- 有效连接率：**+12% ~ +22%**
- 低质量卡片率：**-15% ~ -30%**

#### 系统效率

- 相比原版 ASMR：延迟下降 **80% ~ 90%**
- 相比原版 ASMR：推理成本下降 **93% ~ 97%**
- 相比纯向量方案：复杂问题成本会上升，但只在 `L2/L3` 小比例触发

### 2.2 推荐目标值

建议先把内部目标定成：

| 指标 | 3 个月目标 | 6 个月目标 |
|---|---:|---:|
| 记忆命中率 | `>= 88%` | `>= 92%` |
| 时间冲突正确率 | `>= 85%` | `>= 92%` |
| 复杂问题正确率 | `>= 90%` | `>= 95%` |
| 推荐有效连接率提升 | `>= 10%` | `>= 18%` |
| 平均新增推理调用 | `<= 0.8 次/查询` | `<= 0.6 次/查询` |

---

## 3. OneLink Benchmark 设计

这一部分对应计划里的第一个待办。

### 3.1 Benchmark 总体结构

OneLink 不应该只测“问答准确率”，而要测四类问题：

1. `Memory QA`
2. `Temporal & Update Reasoning`
3. `Matching Recommendation`
4. `Cost / Latency / Resource`

### 3.2 数据集分层

#### A. 记忆问答集

按五类任务构造：

- 事实类：用户在哪里、做什么、会什么
- 偏好类：喜欢什么、不喜欢什么、沟通风格
- 事件类：发生过什么、最近做过什么
- 时间类：先后顺序、最新版本、是否过期
- 修正类：旧事实被新事实替代

#### B. 推荐匹配集

按场景构造：

- 婚恋交友
- 求职招聘
- 商业合作
- 学习咨询
- 投融资
- 同城兴趣/搭子
- 创作者合作
- 跨境采购/外贸

#### C. 失败样本集

专门收集：

- 时间线错判
- 旧事实覆盖新事实
- 偏好误判
- 推荐理由看似合理但实际无效
- 高风险请求未升级

### 3.3 指标设计

#### Memory

- `Hit@K`
- `ConflictResolutionAccuracy`
- `UpdateFreshnessAccuracy`
- `PreferencePrecision`
- `EvidenceCompleteness`

#### Matching

- `Recall@K`
- `Precision@K`
- `CTR`
- `DMStartRate`
- `ReplyRate`
- `EffectiveConnectionRate`
- `NegativeFeedbackRate`

#### Efficiency

- `P50/P95 latency`
- `LLM calls per query`
- `token/query`
- `CPU/GPU minute per 1k queries`
- `background ingestion lag`

### 3.4 对照组

至少保留四组：

1. `Baseline-A`：纯向量 + 简单规则
2. `Baseline-B`：一个大模型单次推理
3. `OneLink-L1`：确定性检索
4. `OneLink-L1/L2/L3`：完整 ASMR-Lite

### 3.5 与 MemoryBench 的借鉴关系

OneLink 不必照搬对外 benchmark，但应借鉴 `memorybench` 的三个原则：

1. 数据集可分层
2. provider / pipeline 可替换
3. 评测过程可复跑

建议内部做成：

```text
dataset -> runner -> retrieval pipeline -> answerer -> evaluator -> report
```

---

## 4. L1 / L2 / L3 路由设计

这一部分对应计划里的第二个待办。

### 4.1 三层定义

#### L1：确定性检索层

适用：

- 事实查询
- 简单偏好查询
- 明确过滤型找人
- 已结构化字段足够的推荐请求

能力：

- SQL / 索引 / 图查询
- 时间窗口过滤
- 冲突标记读取
- 规则引擎与 hard filter

成本：

- 检索阶段 `0` 次额外 LLM 调用

#### L2：统一 Search + Reason

适用：

- 信息跨多节点
- 需要隐含线索拼接
- 需要跨事件总结用户意图
- 推荐需要更复杂理由生成

能力：

1. `Search Agent` 从结构化资产主动遍历
2. `Reason Agent` 基于证据做结论

成本：

- `2` 次调用 / 查询

#### L3：复杂复核层

适用：

- 时间冲突严重
- 高风险推荐/风控边界
- 证据冲突大
- 结果置信度低
- 高价值请求（如高净值合作、核心招聘）

能力：

- `Verifier Agent`
- 时间线重建
- 冲突解释
- 高风险复核

成本：

- `L2 + 1` 次调用

### 4.2 升级门槛

#### L1 -> L2

满足任一条件升级：

- 结构化字段缺失率 `> 30%`
- 命中结果互相矛盾
- 需要跨 3 个以上证据节点推理
- 查询中含有“为什么 / 最近变化 / 更适合谁”之类组合推理
- 推荐结果 Top-K 分数差过小，说明区分度不足

#### L2 -> L3

满足任一条件升级：

- 时间链冲突
- `confidence < threshold`
- 高风险场景
- 用户明确质疑系统结果
- 高价值交易/招聘/婚恋场景

### 4.3 不建议的做法

- 不要默认所有查询直接走 L2
- 不要让 match-service 自己拼长期记忆
- 不要为了提高正确率把 L3 触发率放大到两位数

---

## 5. 多用户共享 Agent 资源与按需调度

这对应你列出的第 1 个问题。

### 5.1 核心原则

OneLink 应坚持：

> 每用户一个逻辑 Agent，不是每用户一个常驻进程。

实现上采用：

- `Logical Agent`：用户级持久索引、checkpoint、记忆引用
- `Runtime Worker Pool`：共享推理 worker
- `Queue + Priority + Budget`：统一调度

### 5.2 三类队列

#### A. 实时队列

- 聊天主链路
- 找人请求
- 高优先级风控

#### B. 异步记忆队列

- 新 session 消化
- 结构化抽取
- 冲突检测
- summary 更新

#### C. 夜间优化队列

- AutoResearch 回放
- 策略实验
- Prompt 蒸馏
- 模板更新

### 5.3 资源分配策略

建议配额：

- 实时队列：`60%`
- 异步记忆：`25%`
- 夜间优化：`15%`

在夜间低峰可临时切到：

- 实时队列：`25%`
- 异步记忆：`35%`
- 夜间优化：`40%`

这样回答了你第 4、18、19、22 个问题的核心：**Agent 资源必须共享，异步和夜间批处理是必须的，不然规模不成立。**

---

## 6. 统一存储与结构化提取

这对应第 9、10、15、20、22 个问题。

### 6.1 统一存储方案

不新建独立的 “ASMR 数据库”。

统一进入现有：

- `memory_artifacts`
- `memory_summaries`
- `memory_entities`
- `memory_entity_links`
- `profile_facts`
- `profile_traits`
- `recommendation_feedbacks`

### 6.2 为什么统一存储是对的

这样可以避免：

- 同一事实存两份
- 画像和记忆冲突
- 推荐服务自己维护一套私有事实
- 线上和离线特征口径不一致

### 6.3 预计算优先

OneLink 应在异步链路里提前做：

- 冲突检测
- supersede 链建立
- 时间窗口计算
- 偏好更新聚合
- 用户画像投影请求

这样在线就不必重复做。

### 6.4 对向量的定位

OneLink 不该取消向量，但要降级其角色：

- **人和关系记忆**：结构化 + 图 + 时间优先
- **公共知识**：RAG / 向量搜索继续保留
- **代码或文档索引**：可借鉴 `code-chunk`

一句话：**向量保留，但从“主脑”降级为“辅助召回器”。**

---

## 7. 单大模型 vs 小规模多 Agent

这对应第 6、7、21 个问题。

### 7.1 一个大模型 + 一个 prompt 可以吗

可以，**但不应该作为主架构**。

它适合作为：

- benchmark 对照组
- 冷启动阶段 fallback
- 少量复杂长尾问题的临时方案

不适合作为主架构，因为：

- 检索与推理耦合
- 审计差
- 可回放性差
- 成本波动大
- 很难做策略化降级

### 7.2 正确方案

建议采用：

- 默认 `L1`
- 复杂问题进 `L2`
- 极复杂问题才进 `L3`

这比“全量多 Agent”更合理，也比“全量单大模型”更可控。

### 7.3 你提的“单智能体 + 规则引擎”版本

这是非常适合 OneLink MVP 的。

可以定义为：

- `Search Agent = 图查询 + SQL + temporal filter + conflict lookup`
- `Answer Agent = 单次解释与理由生成`

这是一个很好的一阶段版本，预计能吃到：

- **约 70% ~ 80% 的效果提升**
- **只增加 10% ~ 25% 的复杂问题成本**

---

## 8. 匹配推荐模板库

这一部分对应计划里的第三个待办。

### 8.1 主流场景分类

除了你列出的四类，我建议 OneLink 主流模板至少覆盖：

1. 婚恋交友
2. 求职招聘
3. 商业合作
4. 学习咨询
5. 投融资
6. 创作者合作
7. 同城兴趣搭子
8. 语言交换
9. 跨境采购/渠道分销
10. 导师顾问/专家咨询
11. 招聘内推
12. 项目合伙人寻找

### 8.2 各场景的默认模板

| 场景 | 核心目标 | 默认层级 | 额外 Agent / 模块 |
|---|---|---|---|
| 婚恋交友 | 关系预期匹配 + 安全 | `L2` | 风险复核更常见 |
| 求职招聘 | 技能/岗位/可用性精确匹配 | `L1 -> L2` | 硬过滤最重 |
| 商业合作 | 供需、区域、语言、信任度 | `L2` | 关系图与信誉更重要 |
| 学习咨询 | 专业能力 + 沟通风格 | `L1 -> L2` | 偏好理解更重要 |
| 投融资 | 阶段、赛道、地域、可信度 | `L2 -> L3` | 高价值请求建议复核 |
| 创作者合作 | 内容风格 + 受众互补 | `L2` | 隐式线索多 |
| 同城兴趣搭子 | 兴趣、地域、时间 | `L1` | 可快速扩展 |
| 语言交换 | 语言对、时间带、风格 | `L1` | 结构化字段为主 |
| 跨境采购/外贸 | 品类、区域、语言、履约风险 | `L2 -> L3` | 风控要求高 |
| 导师顾问 | 专长、风格、收费/可用性 | `L1 -> L2` | 推荐解释要强 |
| 招聘内推 | 公司/岗位/背景 + 意愿 | `L1 -> L2` | 图关系有价值 |
| 项目合伙人 | 能力互补 + 信任 + 目标一致 | `L2 -> L3` | 长期价值高 |

### 8.3 Agent 数量是否因场景不同而不同

**是，但不应是“固定更多 Agent”，而应是“模板不同、触发门槛不同”。**

更准确地说，不同场景差异在于：

- 用哪些硬过滤
- 用哪些特征权重
- L3 触发率多高
- 风险门禁多严
- 推荐理由模板是否不同

### 8.4 是否要提前准备 Agent 组合模板

**要。**

但不要先做“模型微调版 Agent”，而要先做：

- `Prompt Template`
- `Tool Access Template`
- `Feature Weight Template`
- `Escalation Template`
- `Reason Style Template`

也就是先做**策略模板库**，不是先做模型训练。

---

## 9. AutoResearch 如何持续调优多 Agent / 多模板

这对应第 17、18 个问题。

### 9.1 目标

AutoResearch 的职责不是在线代替用户服务，而是不断降低：

- `L2` 错误率
- `L3` 触发率
- 单次查询成本
- 时间冲突误判率
- 推荐负反馈率

### 9.2 调优闭环

```mermaid
flowchart LR
    logs[OnlineLogsAndFeedback] --> cluster[FailureClustering]
    cluster --> replay[OfflineReplay]
    replay --> proposal[GeneratePromptAndPolicyCandidates]
    proposal --> score[BenchmarkScoring]
    score --> shadow[ShadowValidation]
    shadow --> canary[Canary]
    canary --> config[PolicyConfigStore]
```

### 9.3 AutoResearch 重点优化对象

#### Memory Policy

- 哪些内容写长期记忆
- 哪些只留摘要
- 哪些直接遗忘

#### Retrieval Policy

- 何时从 `L1` 升到 `L2`
- 何时从 `L2` 升到 `L3`
- 各场景召回 Top-K

#### Matching Policy

- 场景模板权重
- `memory_alignment` 权重
- 多样性与头部抑制

### 9.4 夜间低成本算力如何用

建议优先放到：

1. 失败样本重跑
2. Prompt A/B 实验
3. 路由阈值搜索
4. 风险案例复盘
5. 小模型蒸馏数据生成

---

## 10. 本地小模型什么时候启用

这对应第 23 个问题。

### 10.1 结论

**不要一开始就让本地 7B 接管核心在线推理。**

先让它承担：

- 结构化抽取
- 分类
- 风险初筛
- 模板选择
- 离线摘要压缩

### 10.2 启用时机

满足以下四个条件再启用：

1. 任务定义稳定，标签体系不再频繁改
2. 已积累足够高质量样本
3. 本地模型效果达到教师模型 `>= 95%`
4. GPU 成本明显优于 API 成本，且运维可控

### 10.3 推荐顺序

#### 第一阶段

本地小模型只做离线：

- 抽取
- 分类
- 预打分

#### 第二阶段

本地小模型做在线轻任务：

- L1 路由判定
- 召回补充
- 风险预判

#### 第三阶段

API 大模型只保留：

- L2 深推理
- L3 复杂复核

---

## 11. 好友功能与社交图判断

这对应第 24 个问题。

### 11.1 现在的规划里有没有“添加好友”

**有雏形，但不是完整的双向好友系统。**

现有材料里已经有：

- `follows` 表
- `follow / unfollow`
- `social.follow.created.v1`
- `social.follow.removed.v1`

这说明当前更接近：

- 关注系统
- 推荐反馈系统
- 社交关系事件雏形

还不是完整的：

- 双向好友
- 好友请求
- 好友状态管理
- 熟人通信权限模型

### 11.2 如果要支持“已认识的人互加好友”

建议 Phase 2 补：

- `friend_requests`
- `friend_edges`
- `friend_state`
- `source_channel`

但这不影响当前 OneLink 的找人与推荐路径先落地。

---

## 12. 十亿级到二十亿级是否能扛住

这对应第 24、25 个问题。

### 12.1 对当前 V2 架构的判断

**方向是对的，但当前 repo 和 MVP 设计远未到“直接上十亿级”状态。**

能保留的是：

- `context-service` 作为记忆计算层
- `profile-service` 作为画像主写
- `match-service` 作为推荐反馈主写
- `AutoResearch` 作为控制平面

必须进一步拆分的，是运行规模层。

### 12.2 十亿级时必须拆出的平台能力

至少要独立出来：

1. `Social Graph Platform`
2. `Realtime Messaging Platform`
3. `Recommendation / Ranking Platform`
4. `Feature Store`
5. `Distributed Vector / Graph Index`
6. `Media Platform`
7. `Risk / Trust Platform`

### 12.3 当前架构能否支撑

#### 作为产品方向

能支撑。

#### 作为今天这套服务直接放大

不能。

原因不是理念错，而是：

- 分片策略未定
- 多区域架构未定
- 图存储规模方案未定
- 消息与媒体链路尚未独立
- 推荐平台还未独立成完整召回/特征/排序架构

---

## 13. 如果以后加直播 / 电商 / 短视频 / 长视频

这对应第 25 个问题。

### 13.1 结论

**OneLink 现有底层可以继续做“人和意图理解层”，但不能直接充当媒体分发主引擎。**

### 13.2 可以复用的部分

- 用户长期理解
- 兴趣变化
- 交易/合作意图理解
- 关系信号
- 风险与信誉

### 13.3 必须新增的部分

#### 直播

- 实时互动流
- 观众画像流式特征
- 低延迟分发
- 主播治理与审核

#### 电商

- 商品图谱
- 库存、订单、支付
- 商家信誉
- 交易风控

#### 短视频 / 长视频

- 内容 embedding 平台
- 观看序列建模
- 多路召回
- 重排序与探索利用
- 媒体转码、CDN、冷热分层

### 13.4 正确关系

未来如果做内容平台，应理解为：

> OneLink 的记忆层是“用户理解与关系层”，不是“内容分发引擎”本身。

---

## 14. 分阶段落地建议

### 14.1 短期（接下来 4~6 周）

目标：先把低成本收益吃到。

1. 实现异步结构化提取
2. 建立 `Personal/Preference/Event/Temporal/Update/Assistant` 抽取视图
3. 让 `L1` 走结构化 + 时间 + 规则检索
4. 做内部 benchmark 第一版
5. 让 `memory_alignment` 先 shadow，不直接进主分

### 14.2 中期（6~12 周）

目标：让复杂问题明显变强。

1. 上 `Search Agent + Reason Agent`
2. 场景模板库上线
3. AutoResearch 夜间回放和阈值优化上线
4. 招聘、合作、学习咨询优先接入

### 14.3 后期（3~6 个月）

目标：让系统进入规模化可复制阶段。

1. 打开图扩展检索
2. 上 `L3` 复杂复核
3. 本地小模型承担更多离线任务
4. 匹配策略开始自动优化
5. 补完整好友/社交图能力

---

## 15. 对 25 个问题的逐条回答

### 1. 多用户共享 Agent 资源、Agent 按需调度

答：**必须共享，绝不能每用户常驻进程。** 用逻辑 Agent + 共享 worker pool + 队列优先级调度。

### 2. 六大向量：个人信息 / 偏好 / 事件 / 时间 / 更新 / 助手信息

答：**建议保留为抽取口径，但统一落到 OneLink 四网络存储。**

### 3. 一个找显式事实，一个找隐含线索，一个做时间线对比/重建时间线

答：**这个专业化拆分是对的。** 但在线默认不必都做成 LLM Agent，L1 可先用图查询和时间过滤替代。

### 4. 用 AutoResearch 做离线监控、实验、调优

答：**完全正确，而且必须夜间化、批处理化。**

### 5. 维度：事实 / 偏好 / 事件 / 时间 / 纠正

答：**对，建议作为核心 benchmark 任务分类。**

### 6. 一个大模型 + 合理 prompt 完成

答：**可作为对照组和 fallback，不建议作为主架构。**

### 7. 一个小规模多 agent，但只有少数复杂问题才触发

答：**这就是 OneLink 最适合的生产方案。**

### 8. Search Agent + Reason Agent，两次调用，复杂时 +1 复核

答：**强烈建议采用。** 这是 OneLink ASMR-Lite 的核心。

### 9. Hindsight + 原版 ASMR 统一存储

答：**建议统一，但落到现有 memory-layer，不再另起一套存储。**

### 10. 用 Hindsight 结构化资产替代 ASMR 原始文本阅读

答：**完全正确。**

### 11. 用图索引替代向量相似度的盲目搜索

答：**对，但不是完全替代，而是在人和关系记忆上优先。**

### 12. 用确定性推理替代概率性多路径投票

答：**这是 OneLink 降本增效的关键。**

### 13. 用预计算矛盾检测替代运行时重复检测

答：**完全正确，应该放入 consolidation pipeline。**

### 14. 延迟优化：ASMR-Lite < 500ms vs 原版 2-5s

答：**目标合理。** L1 应做到 `< 200ms` 检索侧，L2 控制在 `< 500ms`，L3 允许更高。

### 15. 结构化提取，不存原始 chunk，而存信念节点 / 偏好 / 事件

答：**方向对，但原始文本仍应进冷存储以便审计与回放。**

### 16. 专业化分工，重点考虑婚恋、招聘、合作、学习咨询等

答：**完全正确。** 并且不同场景要不同模板和门槛，但不是每个场景都增加更多常驻 Agent。

### 17. 用 AutoResearch 对多 agent 模式 / 模板持续调优

答：**应该做，而且要以失败样本聚类和 replay 为核心。**

### 18. 后台异步消化 + AutoResearch 监控 L1/L2 失败并优化 L3 触发

答：**这是正确的控制平面形态。**

### 19. 短期：异步结构化提取，前端先用向量 + 简单规则

答：**可行，但建议不是“只用向量”，而是“结构化 + 时间 + 简单规则”，向量仅辅助。**

### 20. 用逻辑推理解决时间冲突；用推理检索替代向量 RAG

答：**在人和关系记忆上正确；在公共知识上不要放弃 RAG。**

### 21. “单智能体 + 规则引擎”的简化版

答：**很适合作为第一阶段线上版本。**

### 22. 从“多 Agent 并行”到“结构化检索 + 元优化”

答：**这是我建议 OneLink 采用的主方向。**

### 23. 什么时候启用本地自有小模型

答：**先离线、后在线；先抽取分类、后复杂推理。**

### 24. 现在是否有添加好友；未来 Telegram / WhatsApp / 微信级规模能否支撑

答：**当前已有 follow 雏形，但不是完整好友系统；架构方向能延展，但今天这套实现距离 10~20 亿级还很远。**

### 25. 如果以后要加直播 / 电商 / 短视频 / 长视频，是否能支撑

答：**只能复用“用户理解层”，不能直接拿当前 chat/context/match 主链路硬撑媒体分发。必须独立建设内容与媒体平台。**

---

## 16. 最终建议

如果只用一句话总结：

> **OneLink 应该采用 supermemory/ASMR 的“结构化记忆 + 主动检索 + 专业化分工 + benchmark 驱动优化”思想，但必须把在线多 Agent 重写成 OneLink 自己的 L1/L2/L3 轻量架构，并把 AutoResearch 固定为离线控制平面。**

推荐最终决策：

1. **现在就采用 ASMR-Lite 路线**
2. **不等待 supermemory 主代码开源后再决定大方向**
3. **等待其主代码开源后，只补充验证实现细节，不改变 OneLink 主架构路线**
4. **优先落地招聘 / 合作 / 学习咨询三个场景**
5. **婚恋和高风险跨境合作场景默认更高门禁，更早接入 L3 复核**

