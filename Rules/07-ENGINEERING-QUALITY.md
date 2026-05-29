# Engineering Rules

## 1. 当前优先级

根据 2026-05-15 审计，下一阶段工程优先级为：安全、持久化、自动化测试/CI、文档真相源收敛、App/Web 产品化。

## 2. Rust 工程规则

- 核心在线服务使用 Rust。
- 新增运行路径避免 `unwrap` / `expect`，错误必须显式返回或转换为可观测失败。
- 所有网络调用必须设置超时。
- 内部接口必须校验鉴权、trace id 和请求 schema。
- 共享状态优先使用数据库、Redis、事件日志或明确生命周期的缓存，不把生产关键状态放进进程内 HashMap。

## 3. Go 工程规则

Go 用于辅助工具和平台协作时，也必须遵守契约、观测、测试和配置治理。Go 服务不得绕过 Rust owner service 直接写主表。

## 4. 安全门禁

P0 必须完成：

- 非开发环境禁止使用默认 `INTERNAL_SHARED_SECRET`。
- 启动时按环境强制校验密钥强度和显式配置。
- 账号密码改为服务端安全哈希。
- 会话令牌可撤销、可过期、可持久化、多实例可共享。
- 内部管理接口和观测接口不得裸露给公网。

## 5. 持久化门禁

进入共享测试环境前，以下状态不得只在内存中：

- 用户与会话。
- AI 会话与消息。
- 问卷投放与答案。
- 关键记忆 artifact、summary、context logs、checkpoint。
- 找人请求、推荐结果、反馈。

## 6. 测试门禁

最小 CI 必须包含：

- `cargo fmt --all --check`。
- `cargo clippy --workspace --all-targets -- -D warnings`。
- `cargo test --workspace`。
- 核心 OpenAPI / event schema 校验。
- `chat -> memory -> profile` smoke。

后续补齐 contract tests 和 E2E。App/Web 双端必须共享至少一套 API mock 和关键用户旅程测试。

容量、SLO、压测和成本指标必须按 `rules/12-CAPACITY-AND-SLO.md` 执行；新增高成本模型路径或主链路能力时，必须同步提供压测或成本评估入口。

## 7. 部署门禁

- 提供 Docker 或等价可复制本地环境。
- 提供开发、测试、预发、生产配置模板。
- 配置不得把开发默认密钥带入非开发环境。
- 服务必须提供 health、ready、metrics 或等价观测面。
- 发布必须有回滚步骤。

## 8. 文档门禁

任何行为变化必须同步：

- `rules/` 对应规则。
- `repo/README.md` 或服务 README。
- OpenAPI / internal contract。
- SQL / event schema。
- 测试说明。

历史资料只允许放入 `docs/archive/`，不能重新成为主线入口。

规则目录迁移、命名重组或 archive 收口，必须在 Git 层形成清晰、可审阅的 move/rename 变更，避免长期同时存在“旧目录删除 + 新目录未跟踪”的混乱状态。

`docs/archive/` 不得新增当前开发规范、当前发单入口或新的冻结契约。

`.DS_Store` 等本地系统元数据不得进入新提交；若历史中已有追踪文件，应在规则目录收口提交中单独清理，避免和规则内容修订混在一起。
