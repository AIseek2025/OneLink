# Global I18n Compliance Execution Handbook

## 1. 目的

本文件把 `rules/13-GLOBAL-I18N-AND-COMPLIANCE.md` 转成执行手册，明确：

- 如何清理硬编码文案
- 如何建立多语言资源体系
- 如何落地语言 / 地区 / 时区 / 通知语言字段
- 如何补齐用户数据权利接口
- 如何把区域策略从原则推进到工程文档

## 2. 优先级

执行顺序固定为：

1. 文案资源化
2. locale / region / timezone 数据模型
3. App/Web/BFF 链路打通
4. 用户数据权利接口
5. 区域与驻留策略文档化

## 3. 文案资源化清单

必须清理：

- App 用户可见文案硬编码
- Web 用户可见文案硬编码
- BFF 响应中直写中文文案
- 安全提示、举报、隐私、申诉临时文案

必须建立：

- `zh-CN` 资源
- `en` 资源
- message key 规范
- 术语表
- 审核流

禁止：

- 直接把中文文案写死在页面组件里
- 让 App 和 Web 维护不同 key 体系

## 4. 数据字段模型

必须独立建模并打通：

- `locale`
- `region`
- `timezone`
- `content_language`
- `notification_language`

要求：

- 用户 profile 可读写
- BFF 能正确透传
- App/Web 设置页可操作
- 默认值和 fallback 明确

## 5. 用户数据权利接口

必须提供真实接口或明确执行流程：

- 查看关键记忆与画像事实
- 导出关键数据
- 纠正关键事实
- 删除关键事实

至少覆盖：

- profile facts
- memory summaries
- key memory artifacts
- settings / locale / consent

## 6. 高风险文案审核

以下文案必须进入审核流：

- 拒绝
- 劝导
- 举报
- 拉黑
- 申诉
- 隐私
- 数据删除
- 跨境说明

审核输出必须包含：

- message key
- 中文文案
- 英文文案
- owner
- 最近审核时间

## 7. 区域与驻留策略落地

必须写清楚：

- 用户所属区域
- 数据驻留区域
- 模型调用区域
- 日志存储区域
- 跨境传输依据
- 区域故障降级方式

至少先形成工程文档与部署说明，不允许只在规则层停留抽象表述。

## 8. 执行任务表

| 主题 | 必做任务 |
|------|----------|
| App/Web 文案 | 抽离文案、替换组件硬编码、引入资源加载 |
| BFF | 统一 message key、locale 字段与 fallback |
| Profile/Settings | 读写 locale/region/timezone/content_language/notification_language |
| Compliance | 查看/导出/纠正/删除接口与说明 |
| 文案审核 | 建立 key 表、审核表、版本记录 |
| 区域策略 | 写部署说明、路由规则、降级说明 |

## 9. 验收标准

达到以下条件才可宣称已进入 Phase 5 完成态：

- App/Web 至少中英文可切换
- 硬编码用户文案已基本清除
- 语言、地区、时区、通知语言可独立配置
- 用户数据权利入口可演示
- 区域与驻留策略有工程说明和回滚说明

## 10. 与其他规则的关系

- 原则文件见 `rules/13-GLOBAL-I18N-AND-COMPLIANCE.md`
- 总收口规划见 `rules/14-DELIVERY-CLOSURE-PLAN.md`
- App 规划见 `rules/15-APP-DELIVERY-PLAN.md`
