#!/usr/bin/env bash
# Chat → memory projection → profile 可见：本地开发态辅助脚本。
# 端口：identity 8081, profile 8082, bff 8083, ai-chat 8085, question-service 8086, context 8089, model-gateway 8090
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
SMOKE="$REPO_ROOT/scripts/smoke-chat-memory-profile.sh"
SMOKE_Q="$REPO_ROOT/scripts/smoke-questionnaire-profile.sh"
SMOKE_PERSIST="$REPO_ROOT/scripts/smoke-chat-memory-persistence.sh"
SMOKE_CONTEXT_LOG="$REPO_ROOT/scripts/smoke-chat-memory-context-log.sh"
SMOKE_ACTIVATION="$REPO_ROOT/scripts/smoke-chat-memory-activation.sh"
BENCHMARK="$REPO_ROOT/scripts/benchmark-asmr-lite-v1.sh"
BENCHMARK_V2="$REPO_ROOT/scripts/benchmark-asmr-lite-v2.sh"
BENCHMARK_V2_1="$REPO_ROOT/scripts/benchmark-asmr-lite-v2.1.sh"
BENCHMARK_V2_2="$REPO_ROOT/scripts/benchmark-asmr-lite-v2.2.sh"
SMOKE_RT_OBS="$REPO_ROOT/scripts/smoke-chat-memory-runtime-observability.sh"
SMOKE_FORGET="$REPO_ROOT/scripts/smoke-chat-memory-forgetting-decisions.sh"

usage() {
  cat <<'EOF'
用法: run-chat-memory-profile-slice.sh <子命令>

  print-ports     打印本轮默认端口矩阵（含 question-service）
  print-start     打印建议在独立终端中执行的 cargo run 命令（前台日志）
  smoke           假定服务已全部启动，执行 HTTP smoke（含 profile + ASMR-Lite observability；内嵌 Phase A facts/traits/completion 断言）
  smoke-questionnaire  Phase C：问卷 → context → profile（需 question 8086 + 已重编 BFF）
  benchmark-v1    假定服务已全部启动，执行最小 ASMR-Lite benchmark v1（成功样本 + 升级样本）
  benchmark-v2    假定服务已全部启动，执行固定小数据集 benchmark v2（Memory QA + Temporal & Update）
  benchmark-v2.1  假定服务已全部启动，执行 v2.1 增补（L1>lexical 歧视样本 + entity_hits 断言）
  smoke-activation  需 DATABASE_URL + 001 + 003 + 003_context_activation；验证 importance_score / access_count / last_accessed_at
  smoke-persistence  需 DATABASE_URL + 已执行 003/003_context_idempotency；本脚本编排跨重启 persistence smoke（单独起停 context-service）
  smoke-context-log  需 DATABASE_URL + users + 003_context（含 context_logs）；真实 POST /internal/context/build 后断言 DB 新增一行
  smoke-runtime-obs  需 DATABASE_URL + 001 + 003 + 011；routing 持久化 + asmr-lite 跨重启 last_observation 一致
  smoke-forgetting   需 DATABASE_URL + users + forgetting_decisions；POST /internal/memory/forgetting/decide 写库
  benchmark-v2.2    需 DATABASE_URL + 010 policy_configs；启动 context 后验证 graph/rerank 进入 retrieval_used（policy 只读）
  start-bg        后台启动 7 个服务（含 question-service；日志在 /tmp/onlink-<crate>.log）；打印 PID

环境变量（start-bg / 手动多终端）：
  可覆盖各服务 PORT，默认见 print-ports
  INTERNAL_SHARED_SECRET — ai-chat / context / profile / question-service 间 dev-only 内部 relay 用；
    未设置时实现默认 **onelink-dev-internal-token**；**须多进程同值**。
  QUESTION_SERVICE_BASE_URL — BFF 聚合 pending 用；start-bg 会设为 http://127.0.0.1:8086。

注意：
  - 主业务逻辑在各自 crate 内；本脚本只做编排与验证入口。
  - `benchmark-v1` 为最小固定跑法；更完整的 benchmark 维度仍见 OneLink/docs/archive/rules-legacy-2026-05-15/Rules-V2/EXECUTION/asmr-lite-benchmark-and-acceptance-checklist.md。
  - `benchmark-v2` / `benchmark-v2.1` 为固定小数据集 runner，不覆盖 v1 / smoke 入口；v2.1 见 `tests/integration/ASMR_LITE_BENCHMARK_V2.1.md`。
  - `smoke-activation`：Phase 1 activation 门禁；要求已执行 `003_context_activation.sql`。
  - `smoke-persistence`：若需持久化验证，对 context 进程设置 `DATABASE_URL` 并先跑 SQL 草案；与无库时的 `smoke` 入口互补。
  - `smoke-context-log`：`context_logs` 写入门禁（与 persistence smoke 独立；仅需 003_context 含 context_logs，不强制 idempotency 表）。
  - `smoke-runtime-obs`：011_runtime_observability + routing 行跨重启 asmr-lite。
  - `smoke-forgetting`：forgetting_decisions 在线写入。
  - `benchmark-v2.2`：policy_configs 只读 + graph/rerank 模式门禁（非全栈 chat）。
  - 上述三脚本退出码：0 成功；1 实现/断言失败；2 环境或 SQL 前置缺失（便于区分「缺库/缺表」与「实现坏了」）。
  - 可选 Cargo 壳（不替代本脚本）：见 `tests/integration/CHAT_MEMORY_PROFILE_SLICE.md` — `RUN_SLICE_HTTP_SMOKE=1`（chat smoke）、`RUN_SLICE_QUESTIONNAIRE_SMOKE=1`（问卷 smoke）。
EOF
}

print_ports() {
  cat <<'EOF'
api-gateway      8080  （本轮纵切面可不启）
identity-service 8081
profile-service  8082
bff              8083
ai-chat-service  8085
question-service 8086  （Phase C；BFF 依赖 QUESTION_SERVICE_BASE_URL）
context-service  8089
model-gateway    8090
EOF
}

print_start() {
  cat <<EOF
在 repo 根目录: cd $REPO_ROOT
export RUST_LOG=info
EOF
  cat <<'EOS'
export INTERNAL_SHARED_SECRET="${INTERNAL_SHARED_SECRET:-onelink-dev-internal-token}"
# ↑ ai-chat / context / profile / question-service（relay -> context）须一致。

终端1: PORT=8090 cargo run -p model-gateway
终端2: PORT=8081 cargo run -p identity-service
终端3: PORT=8082 cargo run -p profile-service
终端4: PORT=8089 cargo run -p context-service
# 可选（持久化）：export DATABASE_URL='postgres://...' 且已执行 003_context.sql + 003_context_idempotency.sql
终端5: PORT=8085 cargo run -p ai-chat-service
终端6: PORT=8086 cargo run -p question-service
终端7: export QUESTION_SERVICE_BASE_URL=http://127.0.0.1:8086 && PORT=8083 cargo run -p bff
EOS
  echo ""
  echo "然后执行: $0 smoke"
}

cmd_smoke() {
  if [[ ! -x "$SMOKE" ]]; then
    chmod +x "$SMOKE" 2>/dev/null || true
  fi
  bash "$SMOKE"
}

cmd_smoke_questionnaire() {
  if [[ ! -x "$SMOKE_Q" ]]; then
    chmod +x "$SMOKE_Q" 2>/dev/null || true
  fi
  bash "$SMOKE_Q"
}

start_bg() {
  cd "$REPO_ROOT"
  export RUST_LOG="${RUST_LOG:-info}"
  export INTERNAL_SHARED_SECRET="${INTERNAL_SHARED_SECRET:-onelink-dev-internal-token}"
  echo "INTERNAL_SHARED_SECRET 已导出（ai-chat/context/profile/question-service 默认一致）；自定义时请多进程同值。"
  : "${MG_PORT:=8090}" "${ID_PORT:=8081}" "${PF_PORT:=8082}" "${CTX_PORT:=8089}" "${CHAT_PORT:=8085}" "${Q_PORT:=8086}" "${BFF_PORT:=8083}"
  export QUESTION_SERVICE_BASE_URL="${QUESTION_SERVICE_BASE_URL:-http://127.0.0.1:${Q_PORT}}"
  PORT="$MG_PORT" cargo run -p model-gateway >"/tmp/onlink-model-gateway.log" 2>&1 &
  echo "started model-gateway PORT=$MG_PORT pid=$! log=/tmp/onlink-model-gateway.log"
  PORT="$ID_PORT" cargo run -p identity-service >"/tmp/onlink-identity-service.log" 2>&1 &
  echo "started identity-service PORT=$ID_PORT pid=$! log=/tmp/onlink-identity-service.log"
  PORT="$PF_PORT" cargo run -p profile-service >"/tmp/onlink-profile-service.log" 2>&1 &
  echo "started profile-service PORT=$PF_PORT pid=$! log=/tmp/onlink-profile-service.log"
  PORT="$CTX_PORT" cargo run -p context-service >"/tmp/onlink-context-service.log" 2>&1 &
  echo "started context-service PORT=$CTX_PORT pid=$! log=/tmp/onlink-context-service.log"
  PORT="$CHAT_PORT" cargo run -p ai-chat-service >"/tmp/onlink-ai-chat-service.log" 2>&1 &
  echo "started ai-chat-service PORT=$CHAT_PORT pid=$! log=/tmp/onlink-ai-chat-service.log"
  PORT="$Q_PORT" cargo run -p question-service >"/tmp/onlink-question-service.log" 2>&1 &
  echo "started question-service PORT=$Q_PORT pid=$! log=/tmp/onlink-question-service.log QUESTION_SERVICE_BASE_URL=$QUESTION_SERVICE_BASE_URL"
  PORT="$BFF_PORT" cargo run -p bff >"/tmp/onlink-bff.log" 2>&1 &
  echo "started bff PORT=$BFF_PORT pid=$! log=/tmp/onlink-bff.log"
  echo ""
  echo "等待服务监听（建议 sleep 8~15 后执行: $0 smoke）"
}

cmd_benchmark_v1() {
  if [[ ! -x "$BENCHMARK" ]]; then
    chmod +x "$BENCHMARK" 2>/dev/null || true
  fi
  bash "$BENCHMARK"
}

cmd_benchmark_v2() {
  if [[ ! -x "$BENCHMARK_V2" ]]; then
    chmod +x "$BENCHMARK_V2" 2>/dev/null || true
  fi
  bash "$BENCHMARK_V2"
}

cmd_benchmark_v2_1() {
  if [[ ! -x "$BENCHMARK_V2_1" ]]; then
    chmod +x "$BENCHMARK_V2_1" 2>/dev/null || true
  fi
  bash "$BENCHMARK_V2_1"
}

cmd_smoke_persistence() {
  if [[ ! -x "$SMOKE_PERSIST" ]]; then
    chmod +x "$SMOKE_PERSIST" 2>/dev/null || true
  fi
  bash "$SMOKE_PERSIST"
}

cmd_smoke_activation() {
  if [[ ! -x "$SMOKE_ACTIVATION" ]]; then
    chmod +x "$SMOKE_ACTIVATION" 2>/dev/null || true
  fi
  bash "$SMOKE_ACTIVATION"
}

cmd_smoke_context_log() {
  if [[ ! -x "$SMOKE_CONTEXT_LOG" ]]; then
    chmod +x "$SMOKE_CONTEXT_LOG" 2>/dev/null || true
  fi
  bash "$SMOKE_CONTEXT_LOG"
}

cmd_smoke_runtime_obs() {
  if [[ ! -x "$SMOKE_RT_OBS" ]]; then
    chmod +x "$SMOKE_RT_OBS" 2>/dev/null || true
  fi
  bash "$SMOKE_RT_OBS"
}

cmd_smoke_forgetting() {
  if [[ ! -x "$SMOKE_FORGET" ]]; then
    chmod +x "$SMOKE_FORGET" 2>/dev/null || true
  fi
  bash "$SMOKE_FORGET"
}

cmd_benchmark_v2_2() {
  if [[ ! -x "$BENCHMARK_V2_2" ]]; then
    chmod +x "$BENCHMARK_V2_2" 2>/dev/null || true
  fi
  bash "$BENCHMARK_V2_2"
}

case "${1:-}" in
  print-ports) print_ports ;;
  print-start) print_start ;;
  smoke) cmd_smoke ;;
  smoke-questionnaire) cmd_smoke_questionnaire ;;
  benchmark-v1) cmd_benchmark_v1 ;;
  benchmark-v2) cmd_benchmark_v2 ;;
  benchmark-v2.1) cmd_benchmark_v2_1 ;;
  smoke-activation) cmd_smoke_activation ;;
  smoke-persistence) cmd_smoke_persistence ;;
  smoke-context-log) cmd_smoke_context_log ;;
  smoke-runtime-obs) cmd_smoke_runtime_obs ;;
  smoke-forgetting) cmd_smoke_forgetting ;;
  benchmark-v2.2) cmd_benchmark_v2_2 ;;
  start-bg) start_bg ;;
  ""|-h|--help|help) usage ;;
  *) echo "未知子命令: $1"; usage; exit 1 ;;
esac
