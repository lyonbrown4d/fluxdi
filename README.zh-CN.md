# FluxDI

[English](./README.md) · [简体中文](./README.zh-CN.md)

与框架无关的 Rust 依赖注入库，提供显式的编译期装配 API 和动态运行时容器。可选的 Axum、Actix 集成提供 HTTP 请求作用域。

FluxDI 2.0.0 要求 Rust 1.99.1 或更新版本。使用说明见 [crate 指南](./fluxdi/README.zh-CN.md)、[中文文档](./docs/zh-CN/src/introduction.md)和[英文文档](./docs/src/introduction.md)。

使用默认同步功能安装核心 crate：

```toml
[dependencies]
fluxdi = "2.0.0"
```

启用 `features = ["macros"]`，即可使用 `Injectable` 和 `static_module!`。从 1.x 升级前，请阅读 [2.0 迁移说明](./CHANGELOG.zh-CN.md#200---2026-10-18)。本地开发时，在仓库根目录运行：

```bash
cargo test --workspace --all-features --locked
cargo run -p fluxdi --example static_module --features macros --locked
cargo run -p fluxdi --example static_composition_async --features macros,async-factory --locked
cargo run -p fluxdi --example static_conditionals --features macros --locked
cargo run -p fluxdi --example static_conditionals --no-default-features --features macros --locked
cargo run --manifest-path examples/seaorm-sqlite/Cargo.toml --locked
```

直接依赖版本集中在 `[workspace.dependencies]` 管理。SeaORM 示例使用独立 workspace 和锁文件，隔离其原生 SQLite 绑定与 complex 示例所用最新版 rusqlite 的版本冲突。

构建两种语言的文档及统一入口：

```bash
cargo install mdbook --locked
sh docs/build.sh
python3 -m http.server --bind 127.0.0.1 --directory docs/book 8000
```

打开 `http://localhost:8000`。语言切换会保留当前章节。构建时要求每个英文页面都有对应中文页面。依赖审计结果及尚未解决的上游公告见[依赖审计](./docs/zh-CN/src/dependency-audit.md)。
版本变更见[更新日志](./CHANGELOG.zh-CN.md)。维护者可参考[本地发布指南](./docs/zh-CN/src/local-release.md)。
