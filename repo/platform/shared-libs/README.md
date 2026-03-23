# shared-libs

- **`onelink-event-envelope`**：`Rules/16` 通用事件 envelope 的共享 serde 类型，供 dev-only HTTP relay 与各服务对齐字段。

其他跨服务 crate（tracing 扩展、错误码等）可按需增量添加。
