# OneLink Shared Environment Rollback Procedure

## Scope

This document covers rollback procedures for the OneLink shared environment when capacity or SLO issues are detected during or after deployment.

## Rollback Triggers

- p95 latency exceeds SLO baseline by >2x
- Error rate exceeds 1% on any core link
- Circuit breaker opens on chat.respond, match.recommend, or safety.review
- Budget tracker daily_remaining_ratio drops below 10%
- Bulkhead utilization exceeds 90% sustained for >5 minutes

## Rollback Steps

### 1. Immediate Mitigation

```bash
# Check current degradation status
curl -s http://127.0.0.1:3100/ready | jq .

# If degraded, check which capability is affected
curl -s http://127.0.0.1:3100/metrics | jq '.circuit_breakers, .budget, .bulkheads'
```

### 2. Circuit Breaker Recovery

If circuit breaker is open on a capability:
- Wait for `recovery_timeout_secs` (default: 30s) for half-open transition
- Send a test request to verify recovery
- If still failing, scale down traffic to that capability

### 3. Budget Recovery

If daily budget is near exhaustion:
- Reduce `max_tokens_per_request` for the affected capability
- Enable more aggressive caching (increase `ttl_secs`, increase `max_entries`)
- Switch to lower-cost model provider if available

### 4. Bulkhead Recovery

If bulkhead is at capacity:
- Increase capacity for the affected capability
- Reject new requests with 503 until active count drops
- Enable request queuing if supported

### 5. Service-Level Rollback

For individual service failures:
- Restart the affected service: `systemctl restart onelink-<service>`
- Verify health: `curl -s http://127.0.0.1:<port>/health`
- Verify readiness: `curl -s http://127.0.0.1:<port>/ready` (model-gateway only)

### 6. Full Environment Rollback

If multiple services are affected:
1. Stop all services: `systemctl stop onelink-*`
2. Revert to last known good deployment version
3. Start services in dependency order: identity → profile → ai-chat → context → match → dm → safety → bff → model-gateway
4. Verify all health endpoints return 200
5. Run smoke tests to confirm core links operational

## Kill Switch

For emergency cost control:
- Set `DAILY_BUDGET_TOKENS=0` for the affected capability to immediately block all model invocations
- The model-gateway will return fallback responses for all blocked requests
- Re-enable by restoring the original budget value

## Verification After Rollback

```bash
# Full environment verification
for port in 3100 3101 3102 3103 3104 3105 3106 3107 3108; do
  echo "Port $port: $(curl -s -o /dev/null -w '%{http_code}' http://127.0.0.1:$port/health)"
done

# model-gateway full observability
curl -s http://127.0.0.1:3100/ready | jq .status
curl -s http://127.0.0.1:3100/metrics | jq .
```
