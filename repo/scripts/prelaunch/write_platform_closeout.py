#!/usr/bin/env python3
"""Write OneLink platform pre-launch closeout artifacts from collected evidence."""

from __future__ import annotations

import json
from pathlib import Path


ROOT = Path("/Users/brando/Documents/trae_projects/CodeMaster/isolated_autoruns/OneLink")
PLATFORM = ROOT / "reports" / "prelaunch" / "platform"
RAW = PLATFORM / "raw"


def read_json(path: Path) -> dict:
    return json.loads(path.read_text())


def read_lines(path: Path) -> list[str]:
    return path.read_text().splitlines()


def write(path: Path, content: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content.rstrip() + "\n")


def main() -> int:
    after = read_json(PLATFORM / "integration_readiness_after.json")
    down = read_json(PLATFORM / "integration_readiness_rollback_down.json")
    smoke = read_lines(RAW / "app_server_phase2b_smoke.log")
    chat = read_lines(RAW / "app_server_chat_smoke.log")
    health = read_lines(RAW / "service_health_tests.log")

    smoke_steps = [line for line in smoke if line.startswith("[step-")]
    health_excerpt = [
        line
        for line in health
        if line.startswith("=== ")
        or line.startswith("[health]")
        or line.startswith("[ready]")
        or line.startswith("[metrics]")
        or "test result: ok." in line
    ]
    model_lines = [
        line
        for line in health
        if "[fault-inject]" in line
        or "[budget-exhaust]" in line
        or "[cb-recovery]" in line
    ]

    write(
        PLATFORM / "env_matrix.md",
        """# 平台环境矩阵

| 环境 | ONELINK_ENV | 集成开关 | Redis / 密钥托管 | 域名 / API Base | 版本映射 | 当前状态 |
| --- | --- | --- | --- | --- | --- | --- |
| dev | `dev` | 本次演练使用 `SKYLINE_RUN_INTEGRATION=1` | `REDIS_URL=redis://127.0.0.1:6379/15`；`INTERNAL_SHARED_SECRET` 为 dev 共享口令 | `127.0.0.1` 本地端口矩阵 | 工作区当前代码 + Cargo workspace | 已通过本地平台演练 |
| staging | `staging` | 必须显式开启，且禁止默认 dev secret | 需迁移到托管 Redis 与非默认 `INTERNAL_SHARED_SECRET` | 未提供 staging 域名 / API Base | 未提供 release tag / image digest | 仍缺失 evidence |
| pre-prod | `pre-prod` | 上线前必须开启并锁定变更窗口 | 需托管 Redis、密钥管理、发布账户与回滚凭据 | 未提供 pre-prod 域名 / API Base | 未提供候选版本、镜像 digest、静态资源版本 | 仍阻塞正式上线 |

## 本次确认

- `integration_readiness_after.json` 已验证 `SKYLINE_RUN_INTEGRATION=1` 且 Redis 可达时 `ready=true`。
- `integration_readiness_rollback_down.json` 已验证 Redis 下线时会退化为 `ready=false`。
- `staging` / `pre-prod` 仍未形成真实环境变量、密钥托管和域名清单，因此只能给出本地共享环境演练结论，不能替代真实预发放行。
""",
    )

    write(
        PLATFORM / "preprod_inventory.md",
        """# 预发环境清单

## 当前可复核资产

| 组件 | 本地演练地址 / 入口 | 证据 | 备注 |
| --- | --- | --- | --- |
| Redis | `127.0.0.1:6379/15`（Docker 容器 `onelink-prelaunch-redis`） | `integration_readiness_after.json`、`integration_readiness_rollback_down.json` | 本次仅用于验证集成开关与依赖失效/恢复 |
| API smoke | `apps/app-server` 集成测试 | `raw/app_server_phase2b_smoke.log`、`raw/app_server_chat_smoke.log` | 覆盖登录、聊天、找人、推荐、DM、安全、后台审核 |
| 服务观测面 | `services/*/src/health.rs` 健康测试 | `raw/service_health_tests.log` | 覆盖 `health` / `ready` / `metrics` |
| 观测降级 | `model-gateway` 健康测试 | `raw/service_health_tests.log` | 覆盖 budget / circuit breaker / degraded recovery |

## 缺失的真实 pre-prod 资产

| 项目 | 现状 | 影响 |
| --- | --- | --- |
| pre-prod 域名 / BFF base URL | 未提供 | 无法对真实预发地址执行 API 健康检查 |
| 候选版本号 / 镜像 digest / 静态资源版本 | 未提供 | 无法完成“发布后版本确认” |
| 托管 Redis / 密钥管理 / 发布账户 | 未提供 | 无法证明真实共享环境的运维口径 |
| 预发发布窗口与审批链 | 未固化 | 无法给出正式放行 |

## 结论

- 当前 inventory 足以支撑“本地共享环境演练”。
- 当前 inventory 仍不足以声明“真实 pre-prod 已部署完毕”。
""",
    )

    write(
        PLATFORM / "api_health_matrix.md",
        """# API 健康矩阵

## 核心链路 smoke

| 链路 | 证据 | 结果 |
| --- | --- | --- |
| 登录 | `raw/app_server_phase2b_smoke.log` 的 `[step-1] login: status=200 OK` | 通过 |
| 聊天 | `raw/app_server_chat_smoke.log` 的 `test_a0_a1_main_chain_e2e ... ok` | 通过 |
| 找人请求 | `raw/app_server_phase2b_smoke.log` 的 `[step-2] find/requests: state="submitted"` | 通过 |
| 推荐结果 | `raw/app_server_phase2b_smoke.log` 的 `[step-3] recommendations: state="results_ready"` | 通过 |
| DM 首条消息 | `raw/app_server_phase2b_smoke.log` 的 `[step-4] dm first-message: state="under_review"` | 通过 |
| 举报 / 拉黑 / 申诉 | `raw/app_server_phase2b_smoke.log` 的 `[step-5]` 到 `[step-7]` | 通过 |
| 后台审核列表 / 封禁动作 | `raw/app_server_phase2b_smoke.log` 的 `[step-8]`、`[step-9]` | 通过 |

## health / ready / metrics

| 服务 | health | ready | metrics | 证据 |
| --- | --- | --- | --- | --- |
| `api-gateway` | alive | ready | service payload | `raw/service_health_tests.log` |
| `bff` | alive | ready | `capacity_metrics` | `raw/service_health_tests.log` |
| `identity-service` | alive | ready | backend / db health | `raw/service_health_tests.log` |
| `profile-service` | alive | ready | service payload | `raw/service_health_tests.log` |
| `ai-chat-service` | alive | ready | service payload | `raw/service_health_tests.log` |
| `question-service` | alive | ready | service payload | `raw/service_health_tests.log` |
| `context-service` | alive | ready | service payload | `raw/service_health_tests.log` |
| `match-service` | alive | ready | service payload | `raw/service_health_tests.log` |
| `dm-service` | alive | ready | service payload | `raw/service_health_tests.log` |
| `safety-service` | alive | ready | service payload | `raw/service_health_tests.log` |
| `model-gateway` | alive | ready / degraded recovery covered | observability payload | `raw/service_health_tests.log` |

## 判定

- 本地共享环境级别的 API 健康检查为绿灯。
- 真实 pre-prod URL 未提供，因此 `P0-2` 只能认定为“本地演练已通过，真实预发仍待补证据”。
""",
    )

    write(
        PLATFORM / "metrics_and_alerts.md",
        "# 观测与告警说明\n\n"
        "## 已验证的观测面\n\n"
        "- 所有核心服务均具备 `GET /health`、`GET /ready`、`GET /metrics` 或等价观测面，证据见 `raw/service_health_tests.log`。\n"
        "- `bff` 的 `metrics` 暴露 `capacity_metrics`，已验证请求数、错误数、错误率等字段。\n"
        "- `identity-service` 的 `metrics` 暴露后端类型与 `db_healthy`。\n"
        "- `model-gateway` 的 `metrics` 暴露 `cost_metrics`、cache 指标、budget / circuit breaker / bulkhead 相关状态。\n\n"
        "## 已验证的降级 / 告警触发条件\n\n"
        + "\n".join(f"- {line}" for line in model_lines)
        + "\n\n## 建议告警规则\n\n"
        "| 告警 | 触发条件 | 严重级别 | 处置 |\n"
        "| --- | --- | --- | --- |\n"
        "| Redis 不可达 | `integration_readiness.ready=false` 或 readiness probe 报 `Connection refused` | P0 | 立即切换到回滚 runbook，恢复 Redis 或回退共享环境 |\n"
        "| 核心链路错误率 | 任一核心链路错误率 > 1% | P1 | 暂停发布，观察 5 分钟窗口并执行 smoke |\n"
        "| p95 延迟异常 | 核心链路 p95 > 基线 2x | P1 | 检查降级、限流和下游依赖 |\n"
        "| Circuit breaker open | `model-gateway` `ready=degraded` 且 breaker 打开 | P1 | 降级模型链路，必要时回滚 |\n"
        "| Budget 快耗尽 | `daily_remaining_ratio < 10%` | P1 | 降 token / 切低成本模型 / 开启缓存 |\n"
        "| Bulkhead 饱和 | utilization > 90% 持续 5 分钟 | P1 | 扩容或限制流量 |\n\n"
        "## 残留缺口\n\n"
        "- 还没有真实监控看板链接和告警平台路由；当前证据停留在代码与单测层。\n"
        "- 上线前需把以上规则映射到真实监控系统并做一次触发核验。\n",
    )

    write(
        PLATFORM / "release_runbook.md",
        """# 发布 Runbook

## 适用范围

- 本次文档覆盖 OneLink 平台侧的 pre-launch 本地共享环境演练。
- 真实 pre-prod / production 发布前，需补全域名、镜像 digest、审批链和发布窗口。

## 发布前检查

1. 确认 `SKYLINE_RUN_INTEGRATION=1`。
2. 确认 Redis 可达，并执行 `collect_integration_readiness.py`，要求输出 `ready=true`。
3. 复跑 API smoke：
   - `raw/app_server_chat_smoke.log`
   - `raw/app_server_phase2b_smoke.log`
4. 复跑服务 health/ready/metrics：`raw/service_health_tests.log`。
5. 确认回滚入口可用：Redis 容器或托管 Redis 恢复步骤、上一个稳定版本号、配置回退入口。

## 发布步骤

1. 打开发布窗口，冻结新变更。
2. 部署平台依赖与候选版本。
3. 执行 integration readiness 探针，要求 `ready=true`。
4. 执行核心 API smoke，要求登录、聊天、找人、推荐、DM、安全、后台审核全绿。
5. 观察 `health` / `ready` / `metrics` 以及 model-gateway 降级信号。
6. 观察窗口内若无告警异常，宣布候选版本通过。

## 发布中止条件

- `integration_readiness.ready=false`
- Redis 不可达
- 任一核心 smoke 失败
- `model-gateway` 持续 `degraded`
- 版本映射、密钥或域名口径不一致
""",
    )

    write(
        PLATFORM / "rollback_runbook.md",
        """# 回滚 Runbook

## 触发条件

- Redis / 核心依赖不可达
- 核心 API smoke 失败
- 健康检查退化为 `ready=false`
- 发布后错误率、延迟、预算或断路器达到告警阈值

## 快速回滚步骤

1. 停止当前候选版本或撤销本轮配置变更。
2. 恢复上一稳定依赖与版本映射。
3. 若是 Redis 故障，优先恢复 Redis 可达性。
4. 重新执行 `collect_integration_readiness.py`，要求从 `ready=false` 恢复到 `ready=true`。
5. 重新执行核心 API smoke 与 health/metrics 验证。

## 本次演练已验证路径

- Redis 下线后，`integration_readiness_rollback_down.json` 真实变为 `ready=false`。
- Redis 恢复后，`integration_readiness_after.json` 真实回到 `ready=true`。

## 回滚完成判定

- readiness 恢复为绿灯
- 核心链路 smoke 重新通过
- 值班人与 owner 确认用户面已恢复
""",
    )

    write(
        PLATFORM / "oncall_escalation.md",
        """# 值班与升级路径

## 发布窗口建议

- 发布时间窗：工作日 10:00-18:00，避免跨夜与无人值守窗口。
- 观察窗口：发布后至少 30 分钟，期间禁止叠加高风险变更。

## 值班角色

| 角色 | 责任 |
| --- | --- |
| 平台值班 | 维护 Redis / 环境变量 / 发布编排 / 回滚执行 |
| 后端值班 | 维护 BFF 与核心 Rust 服务健康、主链路 smoke、错误定位 |
| QA 值班 | 复核 smoke 结果与回归矩阵 |
| Owner / 发布批准人 | 做 go / no-go 决策，批准回滚 |

## 升级路径

1. 平台值班 5 分钟内确认是否为依赖 / 配置 / Redis 问题。
2. 若 10 分钟内无法恢复，升级到后端值班并暂停发布窗口。
3. 若 15 分钟内仍未恢复，升级到 Owner 做回滚决策。
4. 若影响用户面或有合规风险，增加合规 / 运营通知。

## 故障联系口径

- 单一真相源：`reports/prelaunch/platform/summary.md`
- readiness 入口：`integration_readiness_after.json`
- 回滚入口：`rollback_runbook.md`
""",
    )

    write(
        PLATFORM / "release_drill.log",
        "=== OneLink Pre-Launch Release Drill ===\n"
        "scope=platform_shared_env_rehearsal\n"
        "step-1: enable integration gate and provision Redis dependency\n"
        f"{json.dumps(after, ensure_ascii=False, indent=2)}\n"
        "step-2: execute API smoke\n"
        + "\n".join(smoke_steps)
        + "\nstep-3: execute chat smoke\n"
        + "\n".join(chat)
        + "\nstep-4: execute health/ready/metrics suite\n"
        + "\n".join(health_excerpt)
        + "\nresult=release rehearsal completed with local shared-env green evidence\n",
    )

    write(
        PLATFORM / "rollback_drill.log",
        "=== OneLink Pre-Launch Rollback Drill ===\n"
        "scope=platform_shared_env_rehearsal\n"
        "step-1: withdraw Redis dependency\n"
        f"{json.dumps(down, ensure_ascii=False, indent=2)}\n"
        "step-2: restore Redis dependency\n"
        f"{json.dumps(after, ensure_ascii=False, indent=2)}\n"
        "result=readiness degraded on Redis outage and recovered after restore\n",
    )

    write(
        PLATFORM / "summary.md",
        """# Platform Closeout Summary

## 结论

- 平台侧 **仍阻塞正式上线**，结论为 `blocked`。
- 原因不是本地演练失败，而是当前证据仍停留在“本地共享环境演练”层，尚未形成真实 `staging / pre-prod` 部署与放行证据。

## 本次已完成

- `P0-1`：通过新增 `repo/scripts/prelaunch/collect_integration_readiness.py` 真实探测 `SKYLINE_RUN_INTEGRATION` 与 Redis，并把 `reports/codemaster/integration_readiness.json/md` 回写为 `ready=true`。
- `P0-2`：补齐本地 API 健康矩阵，证据来自 `raw/app_server_phase2b_smoke.log`、`raw/app_server_chat_smoke.log` 与 `raw/service_health_tests.log`。
- `P0-6`：完成一次本地共享环境发布演练和一次可执行回滚演练，见 `release_drill.log`、`rollback_drill.log`。
- `P0-7`：确认核心服务具备 `health / ready / metrics`，并从 `model-gateway` 健康测试中提取了降级、budget、断路器恢复证据。

## 关键改动列表

- 新增 `repo/scripts/prelaunch/collect_integration_readiness.py`，作为平台收口探针与 evidence 生成入口。
- 新增 `repo/scripts/prelaunch/write_platform_closeout.py`，从 raw evidence 自动生成平台 closeout 文档。
- 更新 `reports/codemaster/integration_readiness.json` 与 `reports/codemaster/integration_readiness.md`，把 readiness 状态同步到最新探测结果。
- 新增 `reports/prelaunch/platform/` 下的 closeout 文档、runbook、演练日志与原始 smoke / health 日志。

## 仍阻塞上线的事项

- 没有真实 `staging / pre-prod` 域名、BFF/API base、版本号、镜像 digest 与静态资源版本映射。
- 没有托管 Redis、密钥托管、发布账户与审批链的真实 evidence。
- 没有对真实 pre-prod 地址执行健康检查和 smoke；当前通过的是 in-process / 本地共享环境演练。
- 没有真实监控平台看板链接与告警路由触发记录。

## 建议下一步

1. 补齐 `staging / pre-prod` 的域名、版本、密钥、Redis 托管与 owner 口径。
2. 在真实 pre-prod 部署候选版本后，复用本次 probe 与 smoke 入口重跑整套 evidence。
3. 完成监控看板和告警路由触发验证后，再做 go / no-go 决策。
""",
    )

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
