#!/usr/bin/env bash
# 假设以下服务已在默认端口运行：model-gateway 8090, identity 8081, profile 8082,
# bff 8083, ai-chat 8085, context 8089。
#
# 内部 relay：ai-chat → context → profile 依赖 **同一** INTERNAL_SHARED_SECRET（header: x-internal-token）。
# 启动各服务时若未设置 env，实现默认 onelink-dev-internal-token；若你改 secret，须 ai-chat / context / profile 三进程一致。
# 本 smoke 只调用 **公开** API + Bearer，不在此脚本伪造内部调用链。
# 若要跑固定成功样本 + 升级样本，请改用 scripts/benchmark-asmr-lite-v1.sh。
# Phase A：本脚本在 profile /me 与 completion 请求后会做 **结构化投影** 最小断言（facts / traits / completion 维度），
# 失败时 exit 1；入口路径与依赖不变。
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
  -d '{"content_type":"text","content_text":"我对 AI 创业很感兴趣，希望认识投资人；沟通上不喜欢拐弯抹角，希望直接一点。","idempotency_key":"smoke-1"}' | jq .

echo "== list messages (GET 需 Bearer + 属主) =="
curl -sS "http://127.0.0.1:8085/api/v1/chat/conversations/$CONV/messages" \
  -H "Authorization: Bearer $TOKEN" | jq .

echo "== wait for async pipeline =="
sleep 1

echo "== profile /me =="
PROFILE=$(curl -sS "http://127.0.0.1:8082/api/v1/profile/me" \
  -H "Authorization: Bearer $TOKEN")
echo "$PROFILE" | jq .

echo "== profile /me/completion =="
COMPLETION=$(curl -sS "http://127.0.0.1:8082/api/v1/profile/me/completion" \
  -H "Authorization: Bearer $TOKEN")
echo "$COMPLETION" | jq .

echo "== Phase A: structured projection assertions =="
# 断言“结构化投影已出现”与 completion 维度口径自洽，避免把当前启发式细节（如精确分数）冻结进 smoke。
echo "$PROFILE" | jq -e '
  (."facts" | type == "array") and (."facts" | length > 0) and
  (."traits" | type == "object") and
  (."traits".interest_tags | type == "array") and (."traits".interest_tags | length > 0) and
  (."traits".connection_goal_tags | type == "array") and (."traits".connection_goal_tags | length > 0) and
  (."traits".communication_preferences | type == "array") and (."traits".communication_preferences | length > 0) and
  (."facts" | map(.fact_type) | unique | length) >= 1
' >/dev/null || { echo "smoke failed: profile/me missing Phase A structured fields or empty traits axis" >&2; exit 1; }

echo "$COMPLETION" | jq -e '
  (.required_dimensions | sort) ==
    ["communication_preferences","connection_goals","current_location","display_name","interest_tags"] and
  (.filled_dimensions | index("interest_tags") != null) and
  (.filled_dimensions | index("connection_goals") != null) and
  (.filled_dimensions | index("communication_preferences") != null) and
  (.completion_rate >= 0 and .completion_rate <= 1) and
  (((.filled_dimensions | length) + (.missing_dimensions | length)) == (.required_dimensions | length))
' >/dev/null || { echo "smoke failed: completion dimensions do not match Phase A structured coverage contract (see CHAT_MEMORY_PROFILE_SLICE.md)" >&2; exit 1; }

echo "== context observability (ASMR-Lite) =="
curl -sS "http://127.0.0.1:8089/internal/observability/asmr-lite" \
  -H "x-internal-token: ${INTERNAL_SHARED_SECRET:-onelink-dev-internal-token}" | jq .

echo "OK user_id=$USER_ID"
