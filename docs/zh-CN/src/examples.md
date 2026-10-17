# 示例

主工作区包含以下示例：

- `fluxdi/examples/static_module.rs`（编译期依赖图与类型化组装；要求 `macros`）
- `fluxdi/examples/static_composition_async.rs`（按绑定选择模块导入与异步工厂；要求 `macros,async-factory`）

- `examples/basic`
- `examples/injectable-macro`（宏与手动实现的等价性）
- `examples/decorator`（通过 Provider::with_decorator 添加日志和缓存）
- `examples/complex`
- `examples/async-factory`
- `examples/module-sync`
- `examples/module-async`
- `examples/module-mixed-sync-async`
- `examples/scoped-context`
- `examples/named-bindings`
- `examples/multi-binding-pipeline`
- `examples/graph-tooling`
- `examples/dual-http-random-port`
- `examples/axum`
- `examples/axum-lifecycle`
- `examples/actix`

`examples/seaorm-sqlite` 使用独立工作区。最新 SeaORM/SQLx 与 rusqlite 要求不兼容的 `libsqlite3-sys` 版本；使用独立锁文件后，两种集成都可以采用最新稳定版本。

Web 请求作用域示例：

- `examples/axum` -> `GET /scoped/debug`
- `examples/actix` -> `GET /hello/scoped`

运行示例：

```bash
cargo run -p basic
cargo run -p graph-tooling-example
cargo run -p injectable-macro-example
cargo run -p decorator-example
cargo run -p fluxdi --example static_module --features macros
cargo run -p fluxdi --example static_composition_async --features macros,async-factory
cargo run --manifest-path examples/seaorm-sqlite/Cargo.toml --locked
```

示例默认初始化 FluxDI 日志。需要详细的 DI 节点诊断时，运行：

```bash
RUST_LOG=fluxdi=trace cargo run -p basic
```
