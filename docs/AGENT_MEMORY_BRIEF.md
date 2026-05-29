# OneLink — Cursor Agent 轻量记忆清单

## 用途

供 **新开 Cursor 对话 / 子 Agent** 快速对齐：产品主线、当前工程入口、技术速查、已定口径与排障指针。  
**不必依赖用户每次口头提醒**；本仓库已启用 Cursor 规则：`.cursor/rules/read-agent-memory-brief.mdc`（`alwaysApply: true`），实质性工作前应先读本文。

- **完整逐字对话导出**（若日后纳入仓库）：优先路径 `docs/CURSOR_AGENT_CHAT_RESTORE.md`。清单不够用时应在该文件中 **搜索 / 翻阅** 再下结论，并说明依据。  
  **当前**：若无该文件，则以 `Rules/`、`Rules-V2/`、`repo/README.md`、集成测试文档与代码为准。

**冲突处理**：本文与 **当前代码** 或 **`Rules/` / `Rules-V2/`** 不一致时，以仓库内 **现行实现与权威规则文档** 为准；可用导出文件核对历史说法。

---

## 身份与协作方式（面向 Agent）

- **仓库形态**：Git 根在 **OneLink 工作区根目录**（与 `repo/` 代码子树、`Rules/` 规范并列；**勿**把规划文档与 `repo/` 混为同一「代码根」口径时混淆职责）。
- **用户偏好（会话规则摘要）**：用户侧说明使用 **简体中文**；期望 **直接在本机执行命令** 排查与验证，而非只罗列待运行指令；代码引用使用仓库约定的 **行号路径引用格式**。
- **记忆边界（跨会话预期，与 AI-news / AIAds 对齐）**：
  - **同一会话**：前文已讨论过的约定仍在上下文里，不必每句重复整份清单。
  - **新聊天 / 新 Agent 会话**：不会在模型里「自动记起」旧线程；也没有跨会话的永久个人记忆。续聊依赖 **本清单 + `.cursor/rules/read-agent-memory-brief.mdc` + `Rules/` / `Rules-V2/` + 代码**。
  - **本机生效**：克隆或 **pull 最新 `main`** 后，Cursor 会加载 always-apply 规则；Agent 仍应通过 **Read/Grep 打开本文与权威文档** 对齐口径，而不是假定全文已在上下文或已「背在权重里」。
  - 若希望新话题零成本对齐，可在开头 **@** `docs/AGENT_MEMORY_BRIEF.md` 或说一句「按记忆清单来」。

---

## 产品主线（一句话）

**OneLink**：依托「AI 好朋友」与一度人脉连接的社交平台。当前工程 focus：**MVP 骨架 + 第一条纵切面（chat → memory → profile）可联调**，其余域多为占位。

---

## 当前推荐调度入口（canonical）

**以 `Rules-V2/EXECUTION/README.md` 为发单与波次索引的权威入口**（文内会标明当前 canonical 调度单、角色任务书与历史归档）。  

**注意**：`repo/README.md` 的「当前阶段 / 多代理发单入口」段落 **可能滞后**；若与 `Rules-V2/EXECUTION/README.md` 冲突，**以后者为准**，并应考虑回写 `repo/README.md` 以消歧。

---

## 工程与目录速查

| 路径 | 说明 |
|------|------|
| `repo/` | Rust 服务、契约、脚本、集成测试（**日常改代码主战场**） |
| `Rules/` | MVP 总设计、服务边界、OpenAPI/SQL/事件草案等 **冻结口径** |
| `Rules-V2/` | V2 架构补充、契约细化、**EXECUTION** 调度单与任务书 |
| `docs/` | 研究笔记与整理稿；**本文**为 Agent 轻量入口 |

纵切面联调说明：`repo/tests/integration/CHAT_MEMORY_PROFILE_SLICE.md`。

---

## 技术栈与规范优先级（摘录）

- **在线服务**：MVP 核心服务为 **Rust**（见 `Rules/18-COMPOSER-2-FAST-EXECUTION-BRIEF.md` 与 `repo/README.md` 服务清单）。
- **理解冲突时的优先口径**：`Rules/10-SERVICE-BOUNDARIES.md`、`Rules/11-DATA-EVENT-MODEL.md`（与任务书 18 一致）。

---

## 本地端口与密钥（纵切面）

默认值（与 `Rules/20` / 纵切面脚本一致，详见 `repo/README.md`）：

| 服务 | 默认端口 |
|------|-----------|
| identity | 8081 |
| profile | 8082 |
| bff | 8083 |
| ai-chat | 8085 |
| context | 8089 |
| model-gateway | 8090 |

**`INTERNAL_SHARED_SECRET`**：`ai-chat` / `context` / `profile` 间 dev-only 内部 relay 须 **同值**（如 `x-internal-token`）。使用 `repo/scripts/local/run-chat-memory-profile-slice.sh start-bg` 时会导出默认 dev token；多终端手动启动时需自行对齐。

**观测**（摘录）：`context-service` 提供 `GET /internal/observability/asmr-lite`（需 internal token）；`ai-chat-service` 提供 `GET /internal/observability/chat-relay`。

---

## 已定决策 / 事实源（替代「WORK_SUMMARY」）

本仓库 **无** 统一 `WORK_SUMMARY.md` 时，以下来源作为「最近决策与验收」索引：

- **当前波次与门禁**：`Rules-V2/EXECUTION/README.md` + 文内指向的调度单 / closeout。
- **纵切面与基准**：`repo/tests/integration/CHAT_MEMORY_PROFILE_SLICE.md`、`Rules-V2/EXECUTION/asmr-lite-benchmark-and-acceptance-checklist.md`（及同目录相关 benchmark 说明）。
- **契约与 schema**：`repo/platform/contracts/`、`repo/data-platform/`。

若日后增加「对话导出」或「工作摘要」单文件，在 **本文文首「用途」** 下增加 **一行固定路径** 即可。

---

## 排障与验证指针

- **一键脚本**：`repo/scripts/local/run-chat-memory-profile-slice.sh`（`print-start` / `start-bg` / `smoke` / `benchmark-v1` 等）。
- **说明文档**：`repo/tests/integration/CHAT_MEMORY_PROFILE_SLICE.md`。
- **契约**：`repo/platform/contracts/openapi/` 下 identity / profile / bff / ai-chat-service 等（以当前分支为准）。

---

## 空白模板（大迭代后填几行即可）

```markdown
## 本轮迭代摘要（日期：YYYY-MM-DD）

- 当前 canonical 调度单：
- 已合并 / 待合并 PR：
- 行为变化（用户可见 / API）：
- 配置与环境变量变化：
- 已知问题 / 下一动作：
```

维护方式：填入后 **或** 写入团队 WORK_SUMMARY **或** 写入个人 `memory/YYYY-MM-DD.md`（若你本地 OpenClaw 使用）；并视情况 **更新本文对应章节**，避免双源长期漂移。

---

## 维护说明

- **改准**：发现本文过时，直接改 `docs/AGENT_MEMORY_BRIEF.md` 或开 issue / 告知维护者同步。
- **长文**：详细论证、对话过程留在 `docs/CURSOR_AGENT_CHAT_RESTORE.md`（若存在）或 `docs/` 子目录笔记。**日常续聊**以 **本文 + `Rules-V2/EXECUTION/README.md`（及文内调度单）+ 权威 Rules + 代码** 为主即可；导出文件用于核对历史说法，交付仍须对齐现状。
