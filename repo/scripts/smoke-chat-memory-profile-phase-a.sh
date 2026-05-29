#!/usr/bin/env bash
# 可选入口：与 smoke-chat-memory-profile.sh 完全等价。
# Phase A 结构化断言已合并进主 smoke；保留本路径供文档/CI 锚点引用，避免「另起一套」验证逻辑。
set -euo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
exec bash "$ROOT/smoke-chat-memory-profile.sh"
