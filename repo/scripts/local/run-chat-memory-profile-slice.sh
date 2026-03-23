#!/usr/bin/env bash
# Chat → memory projection → profile 可见：本地开发态辅助脚本。
# 端口：identity 8081, profile 8082, bff 8083, ai-chat 8085, context 8089, model-gateway 8090
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
SMOKE="$REPO_ROOT/scripts/smoke-chat-memory-profile.sh"
BENCHMARK="$REPO_ROOT/scripts/benchmark-asmr-lite-v1.sh"
BENCHMARK_V2="$REPO_ROOT/scripts/benchmark-asmr-lite-v2.sh"
BENCHMARK_V2_1="$REPO_ROOT/scripts/benchmark-asmr-lite-v2.1.sh"

usage() {
  cat <<'EOF'
用法: run-chat-memory-profile-slice.sh <子命令>

  print-ports     打印本轮 6 服务默认端口矩阵
  print-start     打印建议在独立终端中执行的 cargo run 命令（前台日志）
  smoke           假定服务已全部启动，执行 HTTP smoke（含 profile + ASMR-Lite observability）
  benchmark-v1    假定服务已全部启动，执行最小 ASMR-Lite benchmark v1（成功样本 + 升级样本）
  benchmark-v2    假定服务已全部启动，执行固定小数据集 benchmark v2（Memory QA + Temporal & Update）
  benchmark-v2.1  假定服务已全部启动，执行 v2.1 增补（L1>lexical 歧视样本 + entity_hits 断言）
  start-bg        后台启动 6 个服务（需已编译；日志在 /tmp/onlink-<crate>.log）；打印 PID

环境变量（start-bg / 手动多终端）：
  可覆盖各服务 PORT，默认见 print-ports
  INTERNAL_SHARED_SECRET — ai-chat / context / profile 间 dev-only 内部 relay 用；
    未设置时实现默认 **onelink-dev-internal-token**；**ai-chat / context / profile 三进程须同值**。

注意：
  - 主业务逻辑在各自 crate 内；本脚本只做编排与验证入口。
  - `benchmark-v1` 为最小固定跑法；更完整的 benchmark 维度仍见 Rules-V2/EXECUTION/asmr-lite-benchmark-and-acceptance-checklist.md。
  - `benchmark-v2` / `benchmark-v2.1` 为固定小数据集 runner，不覆盖 v1 / smoke 入口；v2.1 见 `tests/integration/ASMR_LITE_BENCHMARK_V2.1.md`。
EOF
}

print_ports() {
  cat <<'EOF'
api-gateway      8080  （本轮纵切面可不启）
identity-service 8081
profile-service  8082
bff              8083
ai-chat-service  8085
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
# ↑ ai-chat / context / profile 必须一致；其余进程不消费该口令。

终端1: PORT=8090 cargo run -p model-gateway
终端2: PORT=8081 cargo run -p identity-service
终端3: PORT=8082 cargo run -p profile-service
终端4: PORT=8089 cargo run -p context-service
终端5: PORT=8085 cargo run -p ai-chat-service
终端6: PORT=8083 cargo run -p bff
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

start_bg() {
  cd "$REPO_ROOT"
  export RUST_LOG="${RUST_LOG:-info}"
  export INTERNAL_SHARED_SECRET="${INTERNAL_SHARED_SECRET:-onelink-dev-internal-token}"
  echo "INTERNAL_SHARED_SECRET 已导出（ai-chat/context/profile 默认一致）；自定义时请三进程同值。"
  : "${MG_PORT:=8090}" "${ID_PORT:=8081}" "${PF_PORT:=8082}" "${CTX_PORT:=8089}" "${CHAT_PORT:=8085}" "${BFF_PORT:=8083}"
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

case "${1:-}" in
  print-ports) print_ports ;;
  print-start) print_start ;;
  smoke) cmd_smoke ;;
  benchmark-v1) cmd_benchmark_v1 ;;
  benchmark-v2) cmd_benchmark_v2 ;;
  benchmark-v2.1) cmd_benchmark_v2_1 ;;
  start-bg) start_bg ;;
  ""|-h|--help|help) usage ;;
  *) echo "未知子命令: $1"; usage; exit 1 ;;
esac
