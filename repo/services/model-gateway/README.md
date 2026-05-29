# model-gateway

## 服务职责
统一模型与能力路由、配额、熔断、审计日志；**所有 LLM 调用必经此服务**。

## 拥有的数据
`model_invocation_logs`。

## 对外接口
内部契约见 `OneLink/repo/platform/contracts/internal/model-gateway.yaml`（不对外暴露给前端 OpenAPI）。

## 依赖
外部 API 或自研推理（后续）；被 ai-chat、match、safety 等调用。

## 不负责
业务表写入（除自身日志）。

## 当前规范入口
当前裁决顺序为：运行代码与测试 > `repo/platform/contracts/` > `repo/data-platform/` > `OneLink/rules/` > `OneLink/docs/archive/`。

## 文档来源
当前规范：`OneLink/rules/02-SYSTEM-ARCHITECTURE.md`、`OneLink/rules/03-SERVICE-BOUNDARIES.md`、`OneLink/rules/09-NEXT-DEVELOPMENT-PLAN.md`。  
历史依据：`OneLink/docs/archive/rules-legacy-2026-05-15/Rules/05-MODEL-PLATFORM-ROADMAP.md`, `OneLink/docs/archive/rules-legacy-2026-05-15/Rules/10-SERVICE-BOUNDARIES.md`

## V2 最小实现状态

当前骨架已补入：

- `POST /internal/v1/invoke`

当前仅做本地 vertical slice 占位：

- `chat.respond` 返回可重复的 Lumi mock reply
- 其他 capability 返回通用占位响应

## 运行（本地）

```bash
cd OneLink/repo
RUST_LOG=info PORT=8090 cargo run -p model-gateway
```
