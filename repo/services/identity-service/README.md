# identity-service

## 服务职责
注册、登录、会话、身份绑定与校验相关能力。

## 拥有的数据
`users`, `identity_bindings`, `sessions`, `verification_attempts`（见 `OneLink/Rules/14-MVP-SQL-SCHEMA-DRAFT.md` §3.1）。

## 对外接口
`/api/v1/identity/*` — 见 `repo/platform/contracts/openapi/identity-service.yaml`。

**本轮纵切面（主实现：Composer 2）已接线：** `POST /register`、`POST /login`、`GET /me`。其余路径仍为占位。

**会话过期（与实现一致）**  
`register` / `login` 返回的 `session.expires_at` 为 **真实截止时间**（当前 MVP：**自签发起 30 天**）。  
`GET /me` 在过期后返回 **401**（`token expired`），对应 session 会被删除。联调 smoke 请用**新注册 token**，避免把「过期」误判为链路故障。

## 本地运行（默认端口 **8081**）

```bash
cd repo
RUST_LOG=info PORT=8081 cargo run -p identity-service
```

- **in-memory**：用户、token 进程内存储，重启丢失。  
- **非生产鉴权**：`password_hash` 为开发用明文比对；token 为不透明字符串。

## 依赖
无业务服务硬依赖；BFF / ai-chat / profile 通过 HTTP 调 `GET /me` 校验 Bearer。

## 不负责
用户主页、画像事实、推荐。

## 联调纵切面
见 `repo/tests/integration/CHAT_MEMORY_PROFILE_SLICE.md` 与 `scripts/local/run-chat-memory-profile-slice.sh`。

## 文档来源
`OneLink/Rules/10-SERVICE-BOUNDARIES.md`, `OneLink/Rules/11-DATA-EVENT-MODEL.md`, `OneLink/Rules/15-MVP-OPENAPI-DRAFT.md`
