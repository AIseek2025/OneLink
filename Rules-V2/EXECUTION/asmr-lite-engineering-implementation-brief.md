# OneLink ASMR-Lite 工程实施任务书

> 角色：`GPT 5.4 / Composer 2 / Composer 2 fast / Opus 4.6`
> 阶段：V2 执行阶段 / 记忆与推荐能力扩展
> 目标：把 `ASMR-Lite` 从分析结论转成可执行工程路线，但不破坏当前 `chat -> memory projection -> profile visible` 主闭环边界

---

## 1. 文档定位

本文件不是新的世界观宪法，也不是对 `Rules-V2/ARCHITECTURE/*` 的替代。

它的职责只有一个：

> 把 `OneLink/docs/0323/0323-2 OneLink ASMR-Lite采用评估与落地方案.md` 中已经确认的方向，收束成执行阶段可引用的工程实施口径。

因此它是：

- `Rules-V2/EXECUTION/` 下的 **canonical 执行补充材料**
- 后续扩展 `memory / reasoning / recommendation` 时的统一输入源
- `Composer 2`、`Composer 2 fast`、`GPT 5.4`、`Opus 4.6` 在扩展阶段共同遵守的工程口径

---

## 2. 核心结论

### 2.1 不采用原版 ASMR 在线架构

OneLink **不采用**原版：

- `3 Reader + 3 Searcher + 8/12 Answer Agents`
- 高频并行投票
- 在线读大量原始 session 文本

原因：

1. 在线成本太高
2. 延迟波动太大
3. 审计和回放难度高
4. 不利于推荐链路工程化
5. 不适合十亿级规模方向

### 2.2 采用 OneLink ASMR-Lite

OneLink 采用：

> **异步结构化记忆 + 确定性检索 + 少量按需推理 + AutoResearch 离线优化**

在线默认路线：

- `L1`: 确定性检索
- `L2`: `Search Agent + Reason Agent`
- `L3`: 少量复杂问题复核

离线默认路线：

- 结构化提取
- consolidation
- 冲突检测
- 策略实验
- benchmark 评估
- shadow / canary / rollout

---

## 3. 与现有 V2 架构的关系

### 3.1 继续复用的正式边界

本文件不改写以下既有口径：

- `context-service = Memory Compute Layer`
- `profile-service = profile truth owner`
- `match-service = recommendation feedback owner`
- `AutoResearch = control plane`

与之对应的正式来源仍然是：

- `Rules-V2/ARCHITECTURE/system-overview.md`
- `Rules-V2/ARCHITECTURE/memory-layer.md`
- `Rules-V2/ARCHITECTURE/optimization-layer.md`
- `Rules-V2/ARCHITECTURE/agent-runtime-and-selective-forgetting.md`

### 3.2 本文件补充的仅是执行层

它补的是：

- `L1 / L2 / L3` 路由
- `Search / Reason / Verifier` 职责
- 异步结构化提取顺序
- `match-service` 接入长期记忆信号的节奏
- 本地模型启用顺序
- 扩展阶段的工程边界

---

## 4. 数据模型与存储口径

### 4.1 不新建第二套记忆真相

`ASMR-Lite` 不得新造独立的记忆真相库。

统一落入现有对象：

- `memory_artifacts`
- `memory_summaries`
- `memory_entities`
- `memory_entity_links`
- `profile_facts`
- `profile_traits`
- `recommendation_feedbacks`

### 4.2 六维抽取口径与四网络映射

`ASMR` 的六维抽取视角继续保留，但底层统一映射到 V2 四网络：

| 抽取视角 | OneLink 存储映射 |
|---|---|
| `Personal Information` | `world + entity` |
| `Preferences` | `opinion` |
| `Events` | `experience + entity` |
| `Temporal Data` | `experience + valid_from/valid_until` |
| `Updates` | `superseded_by + conflict marking` |
| `Assistant Info` | `experience + source_type=assistant` |

### 4.3 原始文本的保留原则

不把原始 chunk 作为在线主读对象，但也**不允许完全丢弃原文**。

必须区分：

- 热层：结构化结果、摘要、高命中记忆
- 冷层：原始聊天全文、长粘贴、可审计引用

因此正确口径是：

> 在线主读结构化资产，原文保留在冷层供审计、回放、纠错使用。

---

## 5. L1 / L2 / L3 路由

### 5.1 L1：确定性检索层

适用：

- 显式事实查询
- 简单偏好查询
- 时间最近 / 是否过期
- 候选人硬过滤与基础召回

允许使用：

- SQL
- 结构化索引
- 图查询
- 时间过滤
- 冲突链查询
- 规则引擎

默认成本：

- 检索阶段 `0` 次额外 LLM 调用

### 5.2 L2：Search + Reason

适用：

- 需要跨多个证据节点拼接
- 需要隐式线索补全
- 需要多段经历合并结论
- 推荐理由需要结合长期记忆解释

职责拆分：

- `Search Agent`
  - 输入：用户问题 / 找人请求 / 结构化上下文
  - 任务：主动遍历 `memory_artifacts / summaries / entity_links / profile_traits`
  - 输出：候选证据集合、证据排序、冲突标记、时间链
- `Reason Agent`
  - 输入：Search 输出
  - 任务：形成最终结论、推荐理由、冲突解释
  - 输出：回答或排序特征

默认成本：

- `2` 次调用 / 查询

### 5.3 L3：复杂复核层

只在少数复杂问题触发：

- 时间线严重冲突
- 高风险或高价值场景
- `L2` 置信度低
- 用户显式质疑结果

职责：

- `Verifier Agent`
- 时间线重建
- 冲突解释
- 高价值交易 / 招聘 / 婚恋复核

默认成本：

- `L2 + 1` 次调用

### 5.4 路由目标分布

建议控制在：

- `L1`: `75% ~ 85%`
- `L2`: `12% ~ 20%`
- `L3`: `2% ~ 5%`

如果 `L3` 长期超过 `5%`，说明：

- `L1` 结构化质量不足
- `L2` prompt 或工具路由不足
- 需要 AutoResearch 调参，而不是继续增加在线 Agent 数

---

## 6. 路由升级门槛

### 6.1 L1 -> L2

满足任一条件才升级：

- 结构化字段缺失率高
- 命中结果互相矛盾
- 需要跨 3 个以上证据节点推理
- 用户问题包含隐式关系判断
- 推荐候选 Top-K 区分度过低

### 6.2 L2 -> L3

满足任一条件才升级：

- 时间链冲突明显
- 关键事实存在新旧版本竞争
- `confidence < threshold`
- 风险等级升高
- 属于高价值场景

### 6.3 明确禁止

不允许：

- 默认所有查询直接走 `L2`
- 默认所有高价值场景都走 `L3`
- 用“提升正确率”为理由把在线多 Agent 触发率放大

---

## 7. Search / Reason / Verifier 的工程边界

### 7.1 Search Agent

只负责：

- 搜证据
- 标注来源
- 拼接时间链
- 给出候选冲突

不负责：

- 最终画像写入
- 最终推荐结果写入
- 最终处罚或风控结论

### 7.2 Reason Agent

只负责：

- 汇总证据
- 生成结论
- 生成推荐理由
- 输出理由中的置信度和冲突解释

不负责：

- 改记忆真相
- 改画像真相
- 改推荐反馈真相

### 7.3 Verifier Agent

只负责：

- 复核复杂争议结果
- 给出复核意见
- 触发更高等级人工或策略审查门槛

不负责：

- 代替 `profile-service`、`match-service` 写主真相

---

## 8. 异步结构化提取与 consolidation

### 8.1 基本原则

结构化提取必须默认放在异步链路中，而不是让在线查询临时读原始文本再推理。

### 8.2 异步最小流水线

```mermaid
flowchart LR
    newSession[NewSessionEvent] --> extract[StructuredExtraction]
    extract --> dedup[DedupAndNormalize]
    dedup --> conflict[ConflictMarking]
    conflict --> summary[SummaryUpdate]
    summary --> projection[ProjectionCandidate]
    projection --> artifacts[MemoryArtifacts]
```

### 8.3 必做动作

- 提取 `Personal / Preference / Event / Temporal / Update / Assistant`
- 建立 `superseded_by`
- 标记时间冲突
- 更新 `memory_summaries`
- 生成画像投影候选

### 8.4 禁止事项

- 不允许在线临时把原始长文本当主输入
- 不允许因为图谱未命中就回退成“无边界原文全读”
- 不允许在 consolidation 阶段直接写 `profile_*`

---

## 9. AutoResearch 的离线职责

### 9.1 定位

`AutoResearch` 继续作为控制平面，不进入在线主链路。

### 9.2 主要职责

- 失败样本聚类
- prompt 候选生成
- 路由阈值搜索
- 模板权重搜索
- `L3` 触发门槛优化
- shadow / canary / rollout

### 9.3 夜间资源利用

优先用于：

1. 失败样本重跑
2. Benchmark 回放
3. Prompt A/B
4. 路由阈值搜索
5. 小模型蒸馏数据生成

### 9.4 明确禁止

- 不允许直接改代码
- 不允许直接改事件 schema
- 不允许绕过 `Policy Config Store`
- 不允许直接插入在线主回复链路

---

## 10. 推荐匹配的接入策略

### 10.1 当前阶段

当前 `chat -> memory projection -> profile visible` 主切片不扩张到完整推荐主链。

### 10.2 接入顺序

#### Phase 1

长期记忆只作为：

- 召回辅助特征
- 推荐理由辅助证据

不直接进入主排序分数。

#### Phase 2

引入：

- `memory_alignment`
- 长期目标匹配度
- 沟通偏好匹配度
- 时间与地域约束一致性

#### Phase 3

与 `AutoResearch` 策略模板库结合：

- 分场景策略
- 多目标精排
- 复杂场景 `L3` 复核

### 10.3 推荐场景模板

优先覆盖：

1. 婚恋交友
2. 求职招聘
3. 商业合作
4. 学习咨询
5. 投融资
6. 创作者合作
7. 同城兴趣搭子
8. 语言交换
9. 跨境采购/外贸
10. 导师顾问
11. 招聘内推
12. 项目合伙人

### 10.4 不同场景是否需要不同 Agent 数

答案是：

- **需要不同模板**
- **不应默认增加更多 Agent 数量**

真正变化的是：

- 硬过滤规则
- 特征权重
- 推荐理由模板
- `L3` 触发率
- 风险门槛

---

## 11. 本地模型启用顺序

### 11.1 启用原则

`Qwen2.5-7B` 一类本地模型，不应一开始就接管核心在线复杂推理。

### 11.2 推荐顺序

#### 阶段 A：离线辅助

先承担：

- 结构化抽取
- 分类
- 摘要压缩
- 失败样本重写

#### 阶段 B：在线轻任务

再承担：

- `L1` 路由判断
- 召回补充
- 风险预判

#### 阶段 C：部分替代 API

最后才考虑进入：

- `L2` 的局部推理任务

### 11.3 启用门槛

同时满足：

1. 任务定义稳定
2. 数据样本充足
3. 相对教师模型效果 `>= 95%`
4. GPU 运维成本优于 API 成本

---

## 12. 分阶段落地顺序

### 12.1 短期

重点：

- 异步结构化提取
- `L1` 结构化 + 时间 + 图检索
- 失败样本沉淀
- benchmark 第一版

### 12.2 中期

重点：

- `Search Agent + Reason Agent`
- 场景模板库
- AutoResearch 夜间回放
- `memory_alignment` shadow 验证

### 12.3 后期

重点：

- `L3` 复杂复核
- 更强图扩展
- 匹配策略自动优化
- 本地模型承担更多离线任务

---

## 13. 执行阶段硬约束

### 13.1 必须遵守

- 不引入第二套记忆真相
- 不让在线查询默认读原始长文本
- 不让 `AutoResearch` 插主链
- 不让 `Search / Reason / Verifier` 改主写路径
- 不把推荐系统改成在线重型多 Agent 投票

### 13.2 可以接受的简化

- 先用规则 + SQL + 图查询完成 `L1`
- `L2` 先做单模型双阶段
- `L3` 先只覆盖极少数高风险或高价值场景
- benchmark 先做内部可重复运行版本

---

## 14. 一句话目标

> `ASMR-Lite` 不是把 OneLink 改造成一个更重的多 Agent 系统，而是让 OneLink 在不破坏 V2 主边界的前提下，逐步具备更强的长期记忆、时间推理和推荐匹配能力。
