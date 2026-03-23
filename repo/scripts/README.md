# scripts

本地与 CI 辅助脚本（生成代码、检查契约漂移、纵切面联调等）。

## Chat → Memory → Profile 纵切面（推荐入口）

- **`local/run-chat-memory-profile-slice.sh`**：`print-ports` / `print-start` / `start-bg` / `smoke`（**chmod +x** 已要求）。
- `smoke-chat-memory-profile.sh`：HTTP 串行 smoke（需六服务已启动，`curl` + `jq`）。
- `run-local-vertical-slice-terminals.sh`：仅打印逐终端 `cargo run` 说明（与上类似，保留兼容）。
