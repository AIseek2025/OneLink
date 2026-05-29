# Capacity Test Plan

## 1. 目的

本文件提供 OneLink 容量测试和 SLO 验证的标准计划模板。

## 2. 测试对象

必须覆盖：

- 注册 / 登录
- 聊天首包
- context build
- 画像读取
- 找人请求
- 推荐请求

## 3. 环境要求

每次压测前记录：

- 环境名称
- 服务版本
- 数据规模
- provider 配置
- 缓存配置
- trace 开关

## 4. 用例模板

每条压测用例必须包含：

- 用例名称
- 目标链路
- 请求模型
- 并发数
- 时长
- warmup
- 成功率门槛
- p95 目标
- 成本指标

## 5. 推荐用例

### Case 1：注册 / 登录

- 目标：验证身份链路稳定性
- 关注：延迟、错误率、会话存储

### Case 2：聊天首包

- 目标：验证 App/Web 主入口体验
- 关注：首包时间、fallback、token 成本

### Case 3：context build

- 目标：验证上下文构建性能
- 关注：retrieval mode、degraded、budget

### Case 4：画像读取

- 目标：验证 profile 基线能力
- 关注：读取延迟、错误率

### Case 5：找人请求

- 目标：验证连接前置链路
- 关注：排队、澄清、审查

### Case 6：推荐请求

- 目标：验证候选返回与解释链路
- 关注：空结果率、解释延迟、反馈写回

## 6. 指标模板

必须记录：

- p50
- p95
- p99
- error rate
- timeout count
- CPU / memory
- cache hit ratio
- fallback rate
- cost per request

## 7. 故障注入模板

每次至少选一项：

- provider timeout
- provider 429
- provider 5xx
- DB 慢查询
- cache miss
- queue backlog

记录：

- 故障触发点
- 预期降级
- 实际降级
- 是否满足门禁

## 8. 输出模板

报告必须包含：

- 结论
- 指标表
- 瓶颈判断
- 风险等级
- 是否允许发布
- 后续优化建议

## 9. 发布决策规则

若以下任一不满足，则阻塞共享环境发布：

- 成功率不达标
- p95 明显超标
- 无法回滚
- 无 fallback
- 高成本路径失控

## 10. 与其他规则的关系

- 原则见 `rules/12-CAPACITY-AND-SLO.md`
- 执行手册见 `rules/19-CAPACITY-SLO-EXECUTION-HANDBOOK.md`
