#!/usr/bin/env bash
# 假设以下服务已在默认端口运行：model-gateway 8090, identity 8081, profile 8082,
# bff 8083, ai-chat 8085, context 8089。
#
# 内部 relay：ai-chat → context → profile 依赖 **同一** INTERNAL_SHARED_SECRET（header: x-internal-token）。
# 启动各服务时若未设置 env，实现默认 onelink-dev-internal-token；若你改 secret，须 ai-chat / context / profile 三进程一致。
# 本 smoke 只调用 **公开** API + Bearer，不在此脚本伪造内部调用链。
# 若要跑固定成功样本 + 升级样本，请改用 scripts/benchmark-asmr-lite-v1.sh。
#
# 可选：在运行 smoke 前 export，便于与文档/其他终端对齐（不改变已启动进程的 env）：
#   export INTERNAL_SHARED_SECRET="${INTERNAL_SHARED_SECRET:-onelink-dev-internal-token}"
#
# 依赖：curl、jq
set -euo pipefail

EMAIL="smoke-$(date +%s)@example.com"
PASS="dev-password"

echo "== register =="
REG=$(curl -sS -X POST "http://127.0.0.1:8081/api/v1/identity/register" \
  -H "Content-Type: application/json" \
  -d "{\"provider\":\"email\",\"email\":\"$EMAIL\",\"password_hash\":\"$PASS\",\"primary_region\":\"CN\",\"primary_language\":\"zh\"}")
echo "$REG" | jq .
TOKEN=$(echo "$REG" | jq -r .session.token)
USER_ID=$(echo "$REG" | jq -r .user_id)

echo "== bff chat/init =="
INIT=$(curl -sS "http://127.0.0.1:8083/api/v1/bff/chat/init" \
  -H "Authorization: Bearer $TOKEN")
echo "$INIT" | jq .
CONV=$(echo "$INIT" | jq -r .conversation.conversation_id)

echo "== send message (triggers relay) =="
curl -sS -X POST "http://127.0.0.1:8085/api/v1/chat/conversations/$CONV/messages" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"content_type":"text","content_text":"我对 AI 创业很感兴趣，希望认识投资人","idempotency_key":"smoke-1"}' | jq .

echo "== list messages (GET 需 Bearer + 属主) =="
curl -sS "http://127.0.0.1:8085/api/v1/chat/conversations/$CONV/messages" \
  -H "Authorization: Bearer $TOKEN" | jq .

echo "== wait for async pipeline =="
sleep 1

echo "== profile /me =="
curl -sS "http://127.0.0.1:8082/api/v1/profile/me" \
  -H "Authorization: Bearer $TOKEN" | jq .

echo "== profile /me/completion =="
curl -sS "http://127.0.0.1:8082/api/v1/profile/me/completion" \
  -H "Authorization: Bearer $TOKEN" | jq .

echo "== context observability (ASMR-Lite) =="
curl -sS "http://127.0.0.1:8089/internal/observability/asmr-lite" \
  -H "x-internal-token: ${INTERNAL_SHARED_SECRET:-onelink-dev-internal-token}" | jq .

echo "OK user_id=$USER_ID"
