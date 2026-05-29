#!/usr/bin/env bash
# 在 **6 个独立终端** 中启动纵向切片所需进程（前台运行，便于看日志）。
# 用法：在 repo/ 下执行 `bash scripts/run-local-vertical-slice-terminals.sh`
# 然后另开终端执行 `bash scripts/smoke-chat-memory-profile.sh`
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
echo "在 6 个终端分别运行："
echo ""
echo "1) model-gateway"
echo "   cd $ROOT && RUST_LOG=info cargo run -p model-gateway"
echo ""
echo "2) identity-service"
echo "   cd $ROOT && RUST_LOG=info cargo run -p identity-service"
echo ""
echo "3) profile-service"
echo "   cd $ROOT && RUST_LOG=info cargo run -p profile-service"
echo ""
echo "4) context-service"
echo "   cd $ROOT && RUST_LOG=info cargo run -p context-service"
echo ""
echo "5) ai-chat-service"
echo "   cd $ROOT && RUST_LOG=info cargo run -p ai-chat-service"
echo ""
echo "6) bff"
echo "   cd $ROOT && RUST_LOG=info cargo run -p bff"
echo ""
echo "端口矩阵：8081 identity, 8082 profile, 8083 bff, 8085 ai-chat, 8089 context, 8090 model-gateway"
