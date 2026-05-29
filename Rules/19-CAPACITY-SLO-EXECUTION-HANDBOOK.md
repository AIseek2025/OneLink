# Capacity SLO Execution Handbook

## 1. 目的

本文件把 `rules/12-CAPACITY-AND-SLO.md` 转成执行手册，明确：

- 需要补哪些观测面
- 需要写哪些压测脚本
- 需要做哪些故障注入
- 如何形成发布门禁

## 2. 执行范围

优先覆盖以下链路：

- 注册 / 登录
- Lumi 聊天
- context build
- 画像读取
- 找人请求
- 推荐请求

优先覆盖以下服务：

- `identity-service`
- `ai-chat-service`
- `context-service`
- `profile-service`
- `bff`
- `match-service`
- `model-gateway`

## 3. 观测面补齐

每个共享环境服务必须至少具备：

- `health`
- `ready`
- `metrics` 或明确等价观测面

等价观测面最低要求：

- 依赖检查状态
- 请求量和错误率
- 延迟统计
- fallback / degrade 统计
- trace id 可串联

若没有标准 `ready/metrics`，必须在 README 说明等价观测面映射关系。

## 4. 压测脚本清单

必须编写：

- 注册 / 登录压测
- Lumi 聊天首包压测
- context build 压测
- 画像读取压测
- 找人请求压测
- 推荐请求压测

脚本必须能输出：

- 并发配置
- p50 / p95 / p99
- error rate
- timeout 数
- 资源消耗摘要
- 成本指标摘要

## 5. 故障注入清单

必须覆盖：

- provider timeout
- provider rate limit
- provider 5xx
- cache miss 放大
- queue backlog
- DB 慢查询
- 下游服务不可达

每个故障注入必须有：

- 触发方式
- 预期降级方式
- 观测指标
- 恢复步骤

## 6. 服务级任务表

| 服务 | 必补内容 |
|------|----------|
| `identity-service` | 登录/register 延迟、会话存储健康、ready |
| `ai-chat-service` | 会话吞吐、首包延迟、fallback 统计 |
| `context-service` | context build 延迟、retrieval mode、degraded 标记 |
| `profile-service` | 读取延迟、画像确认写入成功率 |
| `bff` | 端到端 trace、聚合接口延迟、用户态错误率 |
| `match-service` | 候选召回延迟、空结果率、反馈写回成功率 |
| `model-gateway` | provider latency、fallback、budget、cache 命中率 |

## 7. 报告模板

每次压测报告必须包含：

- 测试目标
- 环境说明
- 数据规模
- 并发与时长
- 指标结果
- 失败样本
- 瓶颈判断
- 改进建议
- 是否允许进入下一阶段

## 8. 发布门禁

以下条件缺一不可：

- 关键服务观测面可用
- 核心链路压测已执行
- 高成本路径有预算说明
- 有 fallback 和 kill switch
- 有回滚说明

## 9. 执行顺序

推荐顺序：

1. 补观测面
2. 补压测脚本
3. 补故障注入
4. 首轮压测
5. 修瓶颈
6. 二轮压测
7. 回写报告和门禁结论

## 10. 与其他规则的关系

- 原则文件见 `rules/12-CAPACITY-AND-SLO.md`
- 总收口规划见 `rules/14-DELIVERY-CLOSURE-PLAN.md`
