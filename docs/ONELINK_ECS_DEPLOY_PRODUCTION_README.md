# OneLink ECS Production Deploy README

## 0. 当前结果

截至当前，`OneLink` 已在共享阿里云 ECS 上完成独立部署并可访问：

- 线上地址：`https://onelink.cool`
- 登录页：`https://onelink.cool/login`
- ECS 入口：`admin@8.218.209.218`
- 本地 SSH 别名：`ssh onelink-ecs`
- 证书：已完成 Let’s Encrypt 签发并启用 HTTPS

当前已完成的线上验证：

- 注册成功，不再出现 `register failed: 422`
- 首页成功，不再出现 `home failed: 404`
- 聊天页成功，不再出现 `chat init failed: 404`
- 画像页成功，不再出现 `atob` 解码报错
- 问卷页成功，不再出现 `onboarding failed: 404`

说明：

- 本文档前半部分保留了部署前风险、约束和推荐拓扑，作为上线设计依据
- 本节与文末“最终实绩”记录的是本轮实际已完成状态

## 1. 目的

本文档记录 `OneLink` 在共享阿里云 ECS 上的独立部署约束、已核实环境、当前阻塞、推荐拓扑和正式上线步骤。

适用目标：

- 域名：`https://onelink.cool`
- 共享 ECS：`8.218.209.218`
- 原则：**只新增 OneLink 自己的目录、服务、Nginx 站点与证书，不影响同机其他正式项目**

## 2. 当前已核实事实

### 2.1 项目状态

- 编排入口已进入 `planning / closeout` 态，不再继续泛化开发。
- `iter216` 审计为 `approved`，但 closeout owner 最终结论仍是 `Blocked No-Go`。
- 阻塞原因不是“自动开发失败”，而是：
  - 缺真实 pre-prod 输入
  - 缺真实生产变量与真实运行态 evidence
  - 移动端正式签名/发布链未闭环

### 2.2 ECS 状态

已实际核实：

- SSH 可用：`ssh admin@8.218.209.218`
- 当前机上已有多个正式项目运行
- 当前**不存在** OneLink 专属对象：
  - `/opt/onelink` 不存在
  - `/var/www/onelink` 不存在
  - 没有 `onelink` systemd 服务
  - 没有 `onelink` Nginx conf
  - 没有 `onelink` 证书

服务器现有可用运行时：

- `nginx`
- `certbot`
- `docker`
- `node`
- `npm`
- `cargo`
- `rustc`
- `java`

### 2.3 当前同机占用

已观察到：

- `80/443` 由 `nginx` 占用
- `3040` 已被 `bigbrain-www`
- `8008` 已被 `bigbrain-api`
- `9000` 已被 `skyline-api`
- `18000` 已被 `ipright-api`

结论：

- OneLink 必须使用**独立目录、独立 systemd 服务名、独立 Nginx server block、独立本机端口**
- 不允许复用其他项目服务或修改其他站点配置

## 3. 当前硬阻塞

### 3.1 发布阻塞

- 当前 Git 工作区为大面积脏工作树，包含大量未提交源码、文档、生成物与本地编排痕迹
- 其中明确不应直接入库/发布的内容包括：
  - `repo/apps/mobile/android/.gradle/`
  - `repo/apps/mobile/android/app/.cxx/`
  - `repo/apps/mobile/ios/Pods/`
  - `repo/apps/mobile/ios/build/`
  - `repo/apps/web/node_modules/`
  - `repo/apps/web/dist/`
  - `.codemaster_orchestration/`
  - `.dbg/`
  - `.DS_Store`

结论：

- 不能直接在当前工作区执行 `git push` 或打生产发布包
- 必须先整理 OneLink 的干净发布基线

### 3.2 生产运行阻塞

OneLink 当前并非单体站点，至少涉及：

- `identity-service`
- `profile-service`
- `ai-chat-service`
- `question-service`
- `context-service`
- `match-service`
- `safety-service`
- `dm-service`
- `model-gateway`
- `bff`
- `app-server`
- `web`

且多个服务在 `非 dev` 环境下要求：

- `DATABASE_URL` 必填
- `INTERNAL_SHARED_SECRET` 不能使用 dev 默认值

因此：

- 不能用 `ONELINK_ENV=dev` + in-memory 模式伪装生产上线
- 必须准备真实生产环境变量和数据库

### 3.3 域名与证书阻塞

- 巡检时 `onelink.cool` 尚未稳定解析到 ECS
- 服务器当前没有 `onelink` 现成证书

因此：

- 在 DNS 生效前无法完成 `certbot` 证书申请
- 在证书签发前无法完成正式 HTTPS 放量验收

## 4. 推荐生产拓扑

### 4.1 目录

- 代码根：`/opt/onelink/current`
- release 根：`/opt/onelink/releases`
- shared：`/opt/onelink/shared`
- 生产环境文件：`/opt/onelink/shared/onelink.env`
- Web 静态根：`/var/www/onelink/current`
- certbot webroot：`/var/www/certbot`
- 日志目录：`/var/log/onelink`

### 4.2 服务

建议服务名：

- `onelink-identity.service`
- `onelink-profile.service`
- `onelink-ai-chat.service`
- `onelink-question.service`
- `onelink-context.service`
- `onelink-match.service`
- `onelink-safety.service`
- `onelink-dm.service`
- `onelink-model-gateway.service`
- `onelink-bff.service`
- `onelink-app-server.service`

### 4.3 本机端口

建议独立端口矩阵：

- `18101` identity
- `18102` profile
- `18105` ai-chat
- `18106` question
- `18107` match
- `18108` safety
- `18109` dm
- `18110` model-gateway
- `18111` context
- `18113` bff
- `18121` app-server

说明：

- `web` 走静态文件，不需要独立 node 常驻端口
- `nginx` 统一对外暴露 `onelink.cool`
- `/api/v1/bff/*` 反代到 `127.0.0.1:18121`
- `app-server` 再转发到 `bff`

## 5. 推荐 Nginx 规则

站点文件建议：

- `/etc/nginx/conf.d/onelink.cool.conf`

建议能力：

- `http://onelink.cool` 先支持 `/.well-known/acme-challenge/`
- 证书签发完成后强制跳转 `https://onelink.cool`
- `/` -> `root /var/www/onelink/current`
- `/api/v1/bff/` -> `http://127.0.0.1:18121`
- `/health` -> 可选转发到 `app-server` 或返回静态存活探针

约束：

- 只新增 `onelink.cool` 自己的 server block
- 不修改其他项目 `server_name`
- 不复用其他项目证书路径

## 6. 生产环境变量最小集合

建议至少提供：

```env
ONELINK_ENV=production
INTERNAL_SHARED_SECRET=<32+ chars>
DATABASE_URL=postgres://onelink:<password>@127.0.0.1:5544/onelink

IDENTITY_SERVICE_BASE_URL=http://127.0.0.1:18101
PROFILE_SERVICE_BASE_URL=http://127.0.0.1:18102
AI_CHAT_SERVICE_BASE_URL=http://127.0.0.1:18105
QUESTION_SERVICE_BASE_URL=http://127.0.0.1:18106
MATCH_SERVICE_BASE_URL=http://127.0.0.1:18107
SAFETY_SERVICE_BASE_URL=http://127.0.0.1:18108
DM_SERVICE_BASE_URL=http://127.0.0.1:18109
ADMIN_SERVICE_BASE_URL=http://127.0.0.1:18121

BFF_BASE_URL=http://127.0.0.1:18113
APP_PORT=18121
PORT=<per-service port>

CORS_ALLOWED_ORIGINS=https://onelink.cool
```

说明：

- `DATABASE_URL` 建议使用 OneLink 独立 Postgres，不与其他项目共享 schema
- `INTERNAL_SHARED_SECRET` 生产禁止使用默认 dev 值
- `BFF_BASE_URL` 供 `app-server` 指向 `bff`

## 7. 数据库建议

建议 OneLink 使用独立 Postgres：

- 容器名：`onelink-postgres`
- 监听：`127.0.0.1:5544`
- 数据目录：`/opt/onelink/shared/postgres`

迁移执行：

```bash
cd /opt/onelink/current/repo
export DATABASE_URL=postgres://onelink:<password>@127.0.0.1:5544/onelink
cargo run -p onelink-migration-runner -- "$DATABASE_URL"
```

说明：

- `platform/migration-runner` 已存在，可执行 `V001` 到 `V013` 迁移
- 生产态不要依赖 in-memory fallback

## 8. 推荐发布流程

### 8.1 本地

1. 从脏工作区筛出干净发布基线
2. 排除生成物、本地编排痕迹和临时目录
3. 在干净 worktree 中完成提交
4. 推送到 `origin/main`

### 8.2 ECS

1. 创建 `/opt/onelink/releases/<release_id>`
2. 上传干净 release 包
3. 切换 `/opt/onelink/current`
4. 安装/校验前端依赖并构建 `apps/web`
5. 构建 Rust release 二进制
6. 准备 OneLink 独立 Postgres
7. 执行数据库迁移
8. 写入 `/opt/onelink/shared/onelink.env`
9. 安装并启动 OneLink 的各个 systemd 服务
10. 配置并加载 `onelink.cool` Nginx 站点
11. DNS 生效后签发证书
12. 执行公网健康检查和主链路 smoke

## 9. 上线验收最小清单

至少确认：

- `curl -I https://onelink.cool/` 返回 `200`
- `curl -sS https://onelink.cool/api/v1/bff/health` 正常
- `curl -sS https://onelink.cool/api/v1/bff/ready` 正常
- 登录、主页、聊天、找人、推荐、私信、安全、设置、合规至少完成一轮 smoke
- `systemctl status` 中所有 `onelink-*` 服务为 `active`
- `nginx -t` 通过
- 数据库迁移已完成
- 日志无持续性 5xx / panic / restart storm

## 10. 当前执行结论

截至本轮巡检：

- **可以**继续做：
  - 代码发布基线筛选
  - OneLink 专属部署资产编写
  - ECS 目录、服务名、端口与 Nginx 规划
  - DNS 生效后的证书准备
- **不应**直接做：
  - 基于当前脏工作区直接 `git push`
  - 以 `dev` / in-memory 形态冒充生产上线
  - 在没有真实生产变量时宣布 `Go`

## 11. 下一步建议

按优先级执行：

1. 清理并确认 OneLink 干净发布基线
2. 补 OneLink ECS 部署脚本、systemd unit、Nginx conf 模板
3. 准备 OneLink 独立生产环境变量与 Postgres
4. 等 `onelink.cool` DNS 生效后申请证书
5. 完成 ECS 部署、健康检查与主链路 smoke

只有以上步骤完成后，OneLink 才能从当前的 `Blocked No-Go` 进入真正的生产放行判断。

## 12. 本轮实际落地结果

### 12.1 已完成部署

- 已建立 OneLink 独立目录、独立环境文件、独立日志目录
- 已建立 OneLink 独立 PostgreSQL 容器，监听 `127.0.0.1:5544`
- 已执行迁移并完成 Rust 服务构建、Web 静态资源发布
- 已建立 OneLink 独立 Nginx 站点和 HTTPS 证书
- 已确认 `https://onelink.cool/login` 可访问且页面标题为 `OneLink`

### 12.2 已完成生产修复

- `app-server` readiness 探针修复：
  - 从错误的 `"{}/api/v1/bff/boot"` 改为正确的 `"{}/ready"`
- auth 桥接修复：
  - 登录/注册改为透传当前 Web 使用的 `email/password` 契约
- Web/BFF/app-server 路由对齐修复：
  - 补齐 `/home`
  - 补齐 `/chat/init`
  - 补齐 `/chat/messages`
  - 补齐 `/onboarding`
  - 补齐 `/questions/answers`
  - 补齐 `/profile/:id`
  - 补齐 `PATCH /profile/me`
- 最后一轮剩余 `profile` 接口修复：
  - 根因不是 `app-server` 页面桥接本身，而是 `bff` 动态路由写法导致 `GET /api/v1/bff/profile/{user_id}` 未命中
  - 将 `bff` 的 `profile/:userId` 与 `dm/threads/:threadId` 动态路由统一修正为当前运行时可匹配写法
  - 复验后 `GET /api/v1/bff/profile/{user_id}` 已从 `404` 恢复为 `200`
- Web 会话处理修复：
  - 不再假设 token 为 JWT
  - 改为持久化 `onelink_user_id`
  - 移除 `profile/auth/analytics` 中对 `atob` 的依赖
- ECS 构建补充说明：
  - 当前共享 ECS 上 `bff` 的 release 重编需使用 `CC=clang CXX=clang++`
  - 原因是系统默认 `gcc 10.2.1` 会触发 `aws-lc-sys` 的编译器 bug 检测
- AI 聊天真实模型接入：
  - `model-gateway` 已从本地 skeleton/mock 回复切换为真实 `DeepSeek` 调用
  - 当前生产模型为 `deepseek-v4-flash`
  - `ai-chat-service -> model-gateway` 已补齐内部鉴权头，避免内部调用被 `401` 拦截
- 记忆链路启用：
  - `context-service` 继续作为聊天上下文与记忆构建入口
  - 当前生产链路已验证同一会话内前一条偏好可被后一条消息引用
- AI 服务部署补充说明：
  - 共享环境文件 `/opt/onelink/shared/onelink.env` 已写入 `DEEPSEEK_BASE_URL`
  - 已写入 `DEEPSEEK_MODEL=deepseek-v4-flash`
  - 已写入 `DEEPSEEK_THINKING_TYPE=disabled`
  - 已写入 `DEEPSEEK_TIMEOUT_MS=60000`
  - 已写入 `DEEPSEEK_API_KEY`
  - 本轮仅重编并重启 `onelink@model-gateway.service` 与 `onelink@ai-chat-service.service`

### 12.3 线上实测结果

- `https://onelink.cool/login#`
  - 注册成功
- `https://onelink.cool/`
  - 首页加载成功
- `https://onelink.cool/chat`
  - 聊天页加载成功
- `https://onelink.cool/profile`
  - 画像页加载成功
- `https://onelink.cool/questionnaire`
  - 问卷页加载成功
- API 级 smoke：
  - 注册后 `home=200`
  - `chat_init=200`
  - `onboarding=200`
  - `profile/{user_id}=200`
- AI 聊天真实模型 smoke：
  - 注册测试用户后 `chat/messages=200`
  - 第一轮回复已返回真实模型标识：`[chat.respond:deepseek-v4-flash]`
  - 示例回复：
    - `你好小明！我是 Lumi，很高兴认识你。徒步和咖啡，真是很棒的组合呢！`
- 记忆链路 smoke：
  - 第一轮输入：`我喜欢徒步和咖啡。请记住这个偏好。`
  - 第二轮输入：`你还记得我刚才说我喜欢什么吗？`
  - 第二轮回复可正确回忆：`你刚才说喜欢徒步和咖啡`

### 12.4 共享 ECS 隔离结论

- 本轮仅操作 OneLink 自己的目录、服务、Nginx 配置和证书
- 未复用其他正式项目服务名、目录或证书路径
- 未修改其他项目 `server_name`
- OneLink 的新增运行对象与其他同机项目保持隔离
