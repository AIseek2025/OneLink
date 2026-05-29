# Memory Agent And Context

## 1. 总目标

OneLink 的核心不是“聊天更长”，而是“长期理解更准”。记忆系统必须在低成本、高可解释、可纠正、可遗忘的前提下支持 Lumi、匹配、安全和画像演化。

## 2. 四层记忆

1. Hot Session：当前请求必要上下文。
2. Working Memory：近期摘要和活跃任务状态。
3. Persistent Memory：长期高价值认知单元。
4. Cold Archive：原始聊天、长粘贴、历史状态和可回放材料。

UI 可以展示无限时间线，推理只允许有限预算。

## 3. 四类认知网络

`memory_artifacts.network_type` 使用四类：

- `world`：客观、相对稳定、可验证事实。
- `experience`：用户与系统之间发生过的经历和时间线。
- `opinion`：偏好、态度、立场和可能变化的判断。
- `entity`：人、组织、地点、项目、话题等实体认知。

## 4. Context Layer

上下文编译必须分层：

- 不变层：身份、长期偏好、人格宪法、稳定画像。
- 可变层：当前任务、近期消息摘要、活跃关系、最新反馈。
- 再编译层：按任务和预算动态生成 prompt slots。

`context-service` 的 `POST /internal/context/build` 必须显式返回 selected ids、retrieval modes、degraded 和 token budget。

## 5. 选择性遗忘

候选记忆按价值分层：

- 高价值画像内容：长期保留。
- 中价值会话内容：摘要保留。
- 低价值长粘贴：摘要、指纹、冷存储引用。
- 通用知识问答：默认不长期记回答全文，只保留兴趣或困惑信号。

遗忘必须记录对象、理由、policy 版本、时间和冷存储引用。

## 6. 个人记忆容器

状态：候选产品/架构能力，当前冻结的是“逻辑上的用户记忆容器”概念，不冻结具体物理落盘形态。

吸收 `.claude/` 模式，OneLink 可定义逻辑上的 `.onelink/` 用户记忆容器。物理上可以在服务端多租户存储，产品上必须让用户感知为“我的 AI 个人空间”。

建议逻辑结构：

```text
.onelink/
  preferences.json
  context_cache/
  threads/
  graph/
  logs/
```

它承载偏好、长期任务、关系子图、关键决策日志，并支持导出、删除、纠正和审计。这里的目录结构是产品隐喻和逻辑接口示意，不代表当前必须按文件系统结构实现。

## 7. Skill 体系

状态：候选能力，MVP 只冻结元数据与回放日志预留，不冻结自动生成、自动发布或自动上线。

Skill 是系统从成功轨迹中抽象出的可复用路径，不是静态工具列表。

标准闭环：

1. 用户使用。
2. 产生事件、工具调用、结果和反馈。
3. 系统识别可复用模式。
4. 脱敏、泛化、抽象 slots。
5. 沙箱验证。
6. 版本化发布。
7. 灰度复用并继续观测。

MVP 只需要预留 Skill 元数据和回放日志，不要求完整自动生成上线。

## 8. AutoResearch

AutoResearch 默认优化前三类 policy：Memory、Session、Retrieval。Matching、Question、Safety & Persuasion 先预埋，待真实样本足够后再激活。

## 9. LLMLingua / 压缩策略

状态：候选能力，当前冻结为“可插拔压缩评测方向”，不绑定特定供应商或外部模型。

LLMLingua 类能力可作为 context-service 或 model-gateway 的可插拔压缩器，目标是降低 token 30-50%、提升长上下文命中质量并辅助安全检测。MVP 阶段先做评测和可关闭开关，不把外部压缩模型作为硬依赖。
