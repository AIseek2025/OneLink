# Pre-Launch Task Book

## 1. 目的

本文件是 OneLink 在“既有开发 phase 已完成”之后，进入正式上线前所需执行的全套准备工作任务规划书。

目标不是继续无边界开发，而是把项目从：

- 已完成既定开发 phase
- 具备阶段性交付与审计通过
- 已有 Web / App / BFF / Rust 服务基础

推进到：

- 具备正式预发演练条件
- 具备正式上线放行条件
- 具备发布失败时的回滚和应急处置条件

## 2. 当前状态基线

根据当前代码、审计与编排状态，OneLink 已满足以下前提：

- 当前已定义 phases 已全部完成，项目进入规划态，而不是继续编码派发。
- `iteration 216` 已补齐移动端原始测试日志、双平台 bundle 证据，并修复真实缺陷 `repo/apps/mobile/index.js`。
- 当前 Web 构建、移动端静态门禁与 Metro bundle 已具备较好的本地自动化证据。

但项目当前仍不能直接宣称“具备正式上线条件”，因为至少仍存在以下缺口：

- 集成 readiness 仍为 `ready: no`
- Redis 当前不可达，集成环境未进入 ready
- 尚未完成真机 / 模拟器 E2E
- 尚未形成 CocoaPods / Gradle / 签名 / store 构建完整证据
- 尚未形成正式预发环境的 API 健康检查、全链路烟测和回滚演练证据

因此，本任务书默认 OneLink 当前所处阶段为：

- **开发完成态**
- **上线准备未完成态**

## 3. 上线定义

只有同时满足以下四类条件，OneLink 才能被认定为“具备上线条件”：

### 3.1 产品与前台完成

- Web 前台主链路可完整使用
- App 主链路可完整使用
- Admin / 审核后台具备最小可运营能力
- 所有关键用户可见文案、异常提示、空态、风控提示均可交付

### 3.2 工程与构建完成

- Web build、test、typecheck 全通过
- App lint、unit test、关键流程测试、iOS/Android 构建全通过
- Rust workspace 的 test / clippy / fmt / contract / persistence smoke 全通过
- OpenAPI、README、规则文档、上线脚本与仓库事实一致

### 3.3 运行与容量完成

- 核心服务 health / ready / metrics 可用
- 核心链路已做烟测、健康检查、压测或最小容量验证
- 核心故障降级、fallback、kill switch、回滚路径明确
- 预发环境已完成一轮正式演练

### 3.4 发布与运营完成

- 发布步骤、签名、版本管理、回滚说明完整
- 值班、告警、监控、应急联系人和升级路径明确
- 合规、i18n、区域与数据权利入口完成复核
- 对外公告、客服 FAQ、运营/审核 SOP 准备完成

## 4. 总体执行顺序

上线前准备固定分为 8 个 Track：

1. `Track A` 启动与冻结
2. `Track B` Web / App / Backend 功能完工复核
3. `Track C` 构建、测试与契约门禁
4. `Track D` 集成环境与预发环境准备
5. `Track E` 运行、容量、监控与告警
6. `Track F` 合规、i18n、区域与高风险文案复核
7. `Track G` 发布、回滚与应急演练
8. `Track H` 上线决策与发布日执行

建议执行窗口为 2-3 周，按“先验证、再补缺、再演练、最后放行”的顺序推进。

## 5. Track A：启动与冻结

### 目标

把项目从“开发 phase 结束”切换到“上线准备阶段”，冻结放行边界和决策口径。

### 必做任务

- 冻结本次上线目标版本号、发布日期窗口和目标用户范围
- 冻结上线范围内的功能清单、接口清单、模型链路清单
- 明确本次上线不包含的功能与已知 followup
- 建立上线 owner 组：产品、Web、移动端、Rust 后端、平台、QA、运营、合规
- 建立单一真相源文档清单：规则、README、OpenAPI、运行脚本、发布说明
- 建立上线指挥群 / 值班表 / 决策升级路径

### 产物

- 上线版本冻结说明
- owner / oncall 表
- 上线范围与不在范围内事项清单

### 出口标准

- 版本范围冻结
- owner 明确
- 发版日程初版确认

## 6. Track B：功能完工复核

### 目标

确认当前 Web、App、后台、BFF、关键 Rust 服务已达到“上线前只允许修缺陷，不再扩展目标”的状态。

### Web 任务

- 复核注册、登录、首页、聊天、画像、找人、推荐、私信、安全、设置、合规入口
- 复核 Admin 的指标、举报队列、申诉队列、审核动作
- 复核空态、错误态、加载态、弱网态、鉴权过期处理
- 清除剩余假数据、占位按钮、占位文案
- 复核页面文案与交互是否与真实后端状态一致

### App 任务

- 复核 React Native 客户端入口、导航、认证态、BFF client、状态持久化
- 复核注册、登录、聊天、画像确认、找人、推荐、消息、设置、合规入口
- 复核 iOS / Android 平台差异处理
- 复核离线恢复、弱网、权限拒绝、启动失败与 crash 边界
- 复核 App icon、启动图、版本信息、环境配置

### Backend / BFF 任务

- 复核 BFF 客户端接口与 OpenAPI 一致
- 复核 identity / profile / match / dm / safety / model-gateway 的关键接口是否真实可用
- 清理 placeholder 路由、历史兼容脏口径、未消费字段
- 复核错误码、状态枚举、埋点字段、locale 字段统一

### 出口标准

- 不再存在主链路占位
- 不再存在“代码完成但接口未接通”
- Web / App / BFF / 后端的能力口径一致

## 7. Track C：构建、测试与契约门禁

### 目标

把所有“上线前必须是绿灯”的构建与测试统一收口。

### Web 门禁

- `npm run build`
- `npm run typecheck`
- `npm run test`
- Web smoke / E2E

### App 门禁

- `npm test`
- `npm run lint`
- `npm run typecheck`
- Android Metro bundle
- iOS Metro bundle
- `react-native run-ios` / `run-android` 或等价构建验证
- 模拟器 / 真机关键流程 E2E

### 平台与服务门禁

- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo fmt --check`
- contract tests
- persistence smoke
- process-level smoke
- 关键服务 health / ready 检查

### 文档与契约门禁

- OpenAPI 与实际路由一致
- README 与当前命令、目录结构、环境变量一致
- rules 与实际上线口径一致
- event schema 与埋点字段一致

### 出口标准

- 所有门禁项有原始日志或 CI 证据
- 失败项全部清零或降级为明确不阻塞项

## 8. Track D：集成环境与预发环境准备

### 目标

让项目从“本地可构建”推进到“共享环境可联调、预发环境可演练”。

### 必做任务

- 解决 `integration_readiness` 当前阻塞：启用集成开关、恢复 Redis 可达
- 建立完整 dev / staging / pre-prod 环境变量清单
- 部署 Web、App 所需 BFF 与核心后端服务到预发环境
- 确认数据库、Redis、内部 shared secret、对象存储、第三方 provider 配置齐备
- 建立环境 readiness 页面或报告
- 为 Web / App 配置统一的预发 BFF 域名和 API base

### 重点校验项

- 预发环境登录可用
- 预发环境聊天可用
- 预发环境找人 / 推荐 / DM / 安全链路可用
- 合规、区域、i18n 相关接口可用
- 后台审核与处理链路可用

### 出口标准

- `integration_readiness = ready`
- 预发环境全部关键依赖可达
- Web / App 能接入并完成主链路

## 9. Track E：运行、容量、监控与告警

### 目标

确保上线后项目可观测、可告警、可定位、可降级。

### 必做任务

- 为关键服务确认 `health / ready / metrics` 或等价观测面
- 建立 BFF、identity、profile、match、dm、safety、model-gateway 的核心指标看板
- 确认 trace id 串联方式
- 设置关键告警：错误率、延迟、下游不可达、fallback 激增、队列积压、DB 异常
- 执行最小容量验证：登录、聊天、找人、推荐主链路
- 执行关键故障注入：provider timeout、rate limit、Redis 不可达、DB 慢查询、下游 5xx
- 明确 kill switch、fallback、功能降级策略

### 运行面产物

- 指标看板链接
- 告警规则列表
- 故障注入记录
- 容量验证报告
- 降级与 kill switch 说明

### 出口标准

- 上线关键链路全部可观测
- 告警链路已打通
- 已确认故障时的降级和恢复方式

## 10. Track F：合规、I18n、区域与高风险文案复核

### 目标

把合规和全球化从“规则完成”推进到“上线可审计、可解释、可回滚”。

### 必做任务

- 复核 Web / App 用户可见文案资源化完成情况
- 复核 `zh-CN` / `en` 资源完整性
- 复核 `locale / region / timezone / content_language / notification_language` 的读写链路
- 复核高风险文案审核记录：拒绝、举报、申诉、隐私、删除、跨境说明
- 复核数据权利入口：查看、导出、纠正、删除
- 复核区域与数据驻留说明、模型调用区域、日志存储区域、跨境依据与故障降级

### 产物

- i18n / compliance 复核报告
- 高风险文案审核记录
- 区域与驻留工程说明
- 数据权利演示记录

### 出口标准

- 上线阻塞项全部清零
- 法务 / 合规 / 产品共同确认口径

## 11. Track G：发布、回滚与应急演练

### 目标

确保项目不仅能发出去，而且发坏了能拉回来、出事了有人接。

### 必做任务

- 建立 Web 发版脚本与回滚脚本
- 建立 App iOS / Android 构建、签名、版本号、上传与回滚说明
- 确认 CocoaPods、Gradle wrapper、证书、签名、store metadata、发布账号权限
- 建立数据库迁移执行与回退说明
- 建立 BFF / 后端配置变更回滚说明
- 准备发布日 runbook：谁执行、谁观察、谁批准、谁回滚
- 完成一次正式彩排：构建、部署、烟测、告警观察、回滚演练

### 发布彩排最低覆盖

- Web 预发部署
- Android 构建包
- iOS 构建包
- API 健康检查
- 登录 / 聊天 / 找人 / 推荐 / 私信 / 安全 / 设置 / 合规烟测
- 一次模拟回滚

### 出口标准

- 发布流程可重复执行
- 回滚流程已演练
- 发布当天不需要临时拼接步骤

## 12. Track H：上线决策与发布日执行

### 目标

把所有准备项转成明确的放行结论和发布日动作。

### 发布前 48 小时

- 冻结代码分支与发布版本
- 汇总所有绿灯证据
- 复核所有 blocker 是否清零
- 确认值班表、升级路径、观察窗口
- 确认公告、客服 FAQ、运营说明、审核值班

### 发布前 24 小时

- 复跑关键 build / smoke
- 复核环境密钥与配置
- 复核监控、告警、trace、日志采样
- 确认回滚包与回滚步骤可用

### 发布当天

- 按 runbook 执行 Web / Backend / App 发布
- 执行 API 健康检查与主链路烟测
- 观察关键指标：错误率、延迟、崩溃、登录成功率、消息发送成功率、推荐与 DM 成功率
- 达到观察窗口后宣布上线成功
- 若未达标，按回滚策略执行

### 出口标准

- 形成最终上线结论：`approved` / `approved_with_followups` / `blocked`
- 形成发布日记录与复盘入口

## 13. 分角色任务清单

### 产品 / 项目负责人

- 冻结版本范围
- 确认上线目标与不在范围内事项
- 决策 blocker 是否可接受
- 审批最终放行与回滚决策

### Web 负责人

- 完成 Web 主链路复核
- 完成 Web build / test / smoke
- 完成 Web 发布与回滚说明

### 移动端负责人

- 完成 React Native 双平台构建与 E2E
- 完成 iOS / Android 签名与发布文档
- 完成 App 版本包和 store 准备

### Rust / BFF 后端负责人

- 完成接口稳定性与错误码收口
- 完成 health / ready / metrics 与 smoke
- 完成迁移与回滚说明

### 平台 / 运维负责人

- 完成预发部署、告警、监控、日志、追踪
- 完成容量验证与故障注入
- 完成发布日运行保障与回滚支持

### QA 负责人

- 汇总全链路测试计划
- 执行预发烟测与关键路径回归
- 出具上线验收结论

### 合规 / 运营负责人

- 复核高风险文案
- 复核数据权利入口和用户说明
- 准备客服 FAQ、审核 SOP、事故应答口径

## 14. 任务包与优先级

### P0：阻塞上线，必须完成

- 集成环境 ready 化
- Web / App / Backend 所有构建门禁全绿
- 关键路径烟测全绿
- API 健康检查全绿
- 回滚流程可执行
- App 双平台原生发布链路跑通
- 高风险文案、数据权利、区域驻留说明完成复核

### P1：强烈建议在上线前完成

- 容量基线验证
- 故障注入验证
- 监控看板完善
- 客服 FAQ、运营与审核 SOP
- 发布彩排与一次模拟回滚

### P2：可以进入 followup backlog

- 非核心页面体验优化
- 指标看板扩展
- 非阻塞的说明文档美化
- 次要自动化增强

## 15. 核心证据清单

上线放行前，必须至少收齐以下证据：

- Web build / test / typecheck 日志
- App test / lint / typecheck / bundle / 原生构建日志
- Rust workspace test / clippy / fmt / contract / persistence smoke 日志
- 预发环境健康检查日志
- 全链路烟测结果
- 关键监控截图或导出
- 容量 / 故障注入报告
- i18n / compliance 审核材料
- 发布 runbook
- 回滚 runbook
- 发布彩排记录

## 16. 计划节奏建议

### Week 1

- 完成 Track A
- 完成 Track B
- 启动 Track C
- 修掉所有 P0 缺陷

### Week 2

- 完成 Track C
- 完成 Track D
- 完成 Track E
- 完成 Track F

### Week 3

- 完成 Track G
- 完成 Track H
- 执行正式上线或给出 `blocked` 结论

## 17. 阻塞条件

存在以下任一项时，不得放行：

- 集成环境仍 `ready: no`
- Web / App / Backend 任一关键构建失败
- 登录、聊天、找人、推荐、私信、安全、设置、合规任一主链路烟测失败
- API 健康检查失败
- iOS / Android 任一原生发布链路未跑通
- 无可执行回滚方案
- 高风险文案或数据权利入口未通过复核
- 预发演练未完成

## 18. 最终放行模板

可使用以下结论：

- `approved`
- `approved_with_followups`
- `blocked`

并必须附带：

- 放行日期
- 版本号
- owner
- 已完成证据列表
- 未完成 followups
- 回滚入口

## 19. 与现有文档的关系

- 双端完工规划见 `rules/26-WEB-APP-COMPLETION-PLAN.md`
- 容量与运行门禁见 `rules/19-CAPACITY-SLO-EXECUTION-HANDBOOK.md`
- 多语言与合规执行见 `rules/20-GLOBAL-I18N-COMPLIANCE-EXECUTION-HANDBOOK.md`
- 合规上线检查表见 `rules/25-I18N-COMPLIANCE-CHECKLIST.md`
- 当前集成状态见 `reports/codemaster/integration_readiness.md`
- 当前移动端修复与 bundle 证据见 `reports/work_report_iteration_216.md`

## 20. 结论

OneLink 当前最需要的不是继续扩展开发范围，而是严格执行一轮完整的上线前准备。

本任务书的目标是把“开发完成”与“可以上线”之间的所有工作收口成明确任务包、明确 owner、明确门禁、明确证据、明确回滚。

只有当本文件中的 P0 任务全部完成且所有阻塞条件清零后，OneLink 才能进入正式上线放行决策。
