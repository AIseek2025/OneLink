# ASMR-Lite Benchmark v2.1

## 文档目标

在 **不替换、不破坏** `smoke-chat-memory-profile.sh`、`benchmark-asmr-lite-v1.sh`、`benchmark-asmr-lite-v2.sh` 的前提下，提供一层 **可交给 Opus 验收** 的增强套件，用于证明：

- **OneLink-L1** 在固定样本上可以 **强于** 本仓库内的 **本地词法 scaffold**（而非「大家全对、无法区分」）。
- **entity-aware** 行为可通过 **`entity_hits > 0`** 在 observability 中 **稳定观测**（与当前 `context-service` 实现一致，见 `l1_policy` / `collect_l1_evidence`）。

v2.1 **不是** matching / rollout / canary benchmark，也不承诺向量检索或完整 L2/L3 在线执行。

## 与 v2 的关系

| 入口 | 作用 |
|------|------|
| `scripts/benchmark-asmr-lite-v2.sh` | 基线套件：`Memory QA` + `Temporal & Update`，要求 **OneLink-L1 全通过**；对照组为同一套 lexical scaffold。 |
| `scripts/benchmark-asmr-lite-v2.1.sh` | **增补**套件：歧视性样本 + `entity_hits` 断言；**不修改** v2 数据文件。 |

推荐验收顺序：**smoke → v1 → v2 → v2.1**。

## 数据目录

`repo/tests/integration/asmr_benchmark_v2_1/`

| 文件 | 样本类别 |
|------|-----------|
| `l1_beats_lexical.json` | **L1 vs lexical discrimination**：查询措辞刻意避开脚本内「现在+哪/城市」槽位，且末条 setup 不含答案，使 **Lexical-Full / Lexical-Latest 均失败**，L1 仍应命中 `expected_contains`。 |
| `entity_observable.json` | **Entity-aware observability**：在 `routing.last_observation.entity_hits` 上断言 **`>= 1`**（与实现中 location 实体及查询匹配一致）。 |

每个 JSON 含：`suite`、`setup_messages`、`cases[]`；每 case 含：`id`、`query`、`expected_contains`、`expected_candidate_route`；可选 **`benchmark_v2_1`**：

- `expect_lexical_full_pass` / `expect_lexical_latest_pass`：`true` / `false`（若省略则不对该对照组做额外断言）。
- `min_entity_hits`：非负整数，与 observability 的 `entity_hits` 比较。
- `rationale`：给人读的说明（runner 不解析）。

## 对照组（baseline）含义与命名

脚本输出与字段仍沿用 **Baseline-A / Baseline-B** 历史命名，其 **真实含义** 为：

| 对外打印名 | 脚本内 | 含义 |
|------------|--------|------|
| **Lexical-FullTranscript** | `Baseline-A` / `baseline_a()` | 将 **全部** `setup_messages` 拼接成一段文本，跑 **固定 if/contains 规则**（城市 / 偏好 / 投资人 / 远程等），**非**向量、**非**大模型。 |
| **Lexical-LatestMessage** | `Baseline-B` / `baseline_b()` | 仅对 **最后一条** setup 跑同一套规则。 |
| **OneLink-L1** | `POST /internal/context/build` | 当前 ASMR-Lite **确定性 L1**（`executed_route` 仍为 `L1`），并读 `GET /internal/observability/asmr-lite` 做形状与 `entity_hits` 检查。 |

**重要**：二者与 `Rules-V2/EXECUTION/asmr-lite-benchmark-and-acceptance-checklist.md` 中远期「向量 / 单次 LLM」类 Baseline-A/B **不是同一东西**；v2/v2.1 文档与脚本注释中的 baseline **仅指本仓库 shell scaffold**。

## Runner 输出（可复跑、可验收）

`benchmark-asmr-lite-v2.1.sh` 对每个 case 打印：

- `expected_contains`、两路 lexical 的 **原始输出** 与 **pass/fail**
- OneLink-L1 的 `candidate_route` / `executed_route` / **pass/fail** / **`entity_hits`**
- **`>>> VERDICT:`** 行：`L1=WIN|LOSE`、`Lexical-Full=WIN|LOSE`、`Lexical-Latest=WIN|LOSE`、`L1-only-beat-both=YES|NO`
- 若配置了 `min_entity_hits`，打印 **`entity_hits` 校验 OK**

套件结束打印 **suite summary**（含 `L1-only-beat-both-lexical` 计数）。

## 前置条件

与 v2 相同：

- `model-gateway`、`identity-service`、`profile-service`、`context-service`、`ai-chat-service`、`bff` 已按默认端口启动
- `INTERNAL_SHARED_SECRET` 在 **ai-chat / context / profile** 三进程一致
- 本机：`curl`、`jq`、`python3`

环境变量：

- `ASMR_BENCHMARK_V2_1_DATA_DIR`：覆盖数据目录（默认指向 `repo/tests/integration/asmr_benchmark_v2_1`）

## 运行

```bash
cd OneLink/repo
bash scripts/benchmark-asmr-lite-v2.1.sh
```

或：

```bash
bash scripts/local/run-chat-memory-profile-slice.sh benchmark-v2.1
```

## Observability / 埋点字段（与实现对齐）

v2.1 与 v2 一样依赖 `routing.last_observation` 中以下字段可读（**null 则失败**）：

`summary_hits`、`artifact_hits`、`entity_hits`、`route_confidence`、`estimated_llm_calls`、`estimated_tokens`、`query_preview`、`upgraded`、`query_preference_polarity`、`evidence_preference_polarity`。

其中 **`query_preference_polarity` / `evidence_preference_polarity`** 与 distiller 落库的 **`preference_polarity`** 一致语义，见 `Rules-V2/CONTRACTS/context-service-contract.md` 与 `context-service` README。

## 当前边界（勿过度承诺）

- **in-memory** MVP；无真实 Kafka / PG / Qdrant
- **executed_route** 固定 **L1**；L2/L3 仅为 candidate 与埋点
- Lexical scaffold **可被刻意绕过**；它用于证明「L1 + 记忆检索 > 几条 if」，**不是** SOTA 对话系统对比
- 若某次 **实现变更** 使歧视样本或 `entity_hits` 语义变化，应 **先改实现再改数据集/文档**，禁止文档强于实现

## Opus 验收建议清单

1. 六服务就绪后跑通 v2.1，全套件 exit 0。
2. 在 `l1_beats_lexical` 中至少看到 **`L1-only-beat-both=YES`**。
3. 在 `entity_observable` 中看到 **`entity_hits >= 1`** 且与日志一致。
4. 抽查 `memory_context` 中含 `query_polarity_hint` / `pref_top` 等实现已写入的片段（与 contract 一致即可）。
