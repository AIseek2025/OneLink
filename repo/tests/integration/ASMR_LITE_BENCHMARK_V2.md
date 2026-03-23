# ASMR-Lite Benchmark v2

**姊妹篇（推荐一并跑验收）：** [ASMR_LITE_BENCHMARK_V2.1.md](./ASMR_LITE_BENCHMARK_V2.1.md) — 歧视性样本（**OneLink-L1 > 本地 lexical**）与 **`entity_hits` 断言**，脚本：`scripts/benchmark-asmr-lite-v2.1.sh`。

## 目标

`benchmark v2` 不替代现有 `smoke-chat-memory-profile.sh` 与 `benchmark-asmr-lite-v1.sh`。

它补的是一层**可复跑的小数据集基线**：

- 固定两类样本：`Memory QA`、`Temporal & Update`
- 固定三路输出：`Baseline-A`、`Baseline-B`、`OneLink-L1`
- 固定检查点：查询级 route / evidence / confidence / token MVP 埋点是否真实可读

当前仍是 **L1 executed / L2-L3 candidate only**，不是完整在线多阶段 agent benchmark。

## 数据目录

- `repo/tests/integration/asmr_benchmark_v2/memory_qa.json`
- `repo/tests/integration/asmr_benchmark_v2/temporal_update.json`

每个数据文件至少包含：

- `suite`
- `setup_messages`
- `cases[]`
- `cases[].query`
- `cases[].expected_contains`
- `cases[].expected_candidate_route`

## 三个 baseline（命名诚实映射）

- **`Baseline-A`** = **Lexical-FullTranscript**：脚本内将 **全部** `setup_messages` 拼接后跑 **固定 if/contains 规则**（非向量、非 LLM）。
- **`Baseline-B`** = **Lexical-LatestMessage**：仅对 **最后一条** setup 跑同一套规则。
- **`OneLink-L1`**：真实调用 `context-service` `POST /internal/context/build` 的当前 **确定性 L1**（`executed_route` 仍为 L1）。

说明：

- 与 `Rules-V2/EXECUTION/asmr-lite-benchmark-and-acceptance-checklist.md` 中远期「向量 / 单次大模型」对照组 **不是同一含义**；v2 仅指 **本仓库 shell scaffold**。
- 若需要证明 **L1 强于该 scaffold**（避免「全员答对」），请跑 **v2.1**。
- 本轮重点是把样本、runner、输出字段与真实实现收口

## 运行

```bash
cd OneLink/repo
chmod +x scripts/benchmark-asmr-lite-v2.sh
bash scripts/benchmark-asmr-lite-v2.sh
```

或通过编排脚本：

```bash
bash scripts/local/run-chat-memory-profile-slice.sh benchmark-v2
```

## 前置条件

- `model-gateway`、`identity-service`、`profile-service`、`context-service`、`ai-chat-service`、`bff` 已启动
- `INTERNAL_SHARED_SECRET` 在 `ai-chat-service` / `context-service` / `profile-service` 三进程一致
- 本机已安装 `curl`、`jq`、`python3`
- 本 runner 只走公开链路 + 已暴露的 internal 接口；**不伪造**事件 schema，不绕过鉴权

## 当前输出字段

runner 至少会打印：

- 样本类别与 case id
- `Baseline-A` 命中结果
- `Baseline-B` 命中结果
- `OneLink-L1` 的 `candidate_route` / `executed_route`
- `OneLink-L1` 的 `memory_context`
- `OneLink-L1` 的 `task_context`

同时会断言 `GET /internal/observability/asmr-lite` 的 `routing.last_observation` 已带出以下查询级字段：

- `upgraded`
- `summary_hits`
- `artifact_hits`
- `entity_hits`
- `route_confidence`
- `estimated_llm_calls`
- `estimated_tokens`
- `query_preview`
- `query_preference_polarity`
- `evidence_preference_polarity`

## 本轮边界

- in-memory 存储不变
- `executed_route` 仍固定为 `L1`
- `L2/L3` 当前仍只作为 `candidate_route` 与失败样本/埋点口径出现
- `Baseline-A/B` 是 benchmark scaffold，不是正式模型能力承诺
