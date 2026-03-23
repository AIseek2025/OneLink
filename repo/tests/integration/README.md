# integration tests

- **`CHAT_MEMORY_PROFILE_SLICE.md`** — Chat → memory → profile 纵切面验证步骤、日志观察、与 benchmark 清单衔接说明；含 **Bearer**、**`INTERNAL_SHARED_SECRET` / `x-internal-token`**、**identity `expires_at`**、`context-service` / `ai-chat-service` 的 observability、`benchmark v1/v2` 入口与 smoke 注意事项。
- **`ASMR_LITE_BENCHMARK_V2.md`** — `benchmark v2` 的固定小数据集、baseline 骨架、runner 输出字段与边界说明。
- **`asmr_benchmark_v2/`** — `Memory QA`、`Temporal & Update` 两类固定样本集。
- **Rust 壳**：`integration_chat_memory_profile_slice`（workspace 成员，`tests/chat_memory_profile_slice.rs`）。  
  - `cargo test -p integration_chat_memory_profile_slice`  
  - `RUN_SLICE_HTTP_SMOKE=1` + 六服务已启动时可跑 shell smoke。

多服务 Testcontainers 等可在后续迭代追加。
