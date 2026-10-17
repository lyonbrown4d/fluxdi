# 快速开始

## 安装

本指南介绍尚未发布的检出代码，要求 **Rust 1.99.1 或更新版本**。`rust-toolchain.toml` 固定了该稳定版本。在核心与宏变更协同发布前，请使用本地 crate：

```toml
[dependencies]
fluxdi = { path = "../fluxdi" }
```

直接依赖版本维护在根目录的 `[workspace.dependencies]` 中。SeaORM SQLite 示例使用独立工作区，因为它要求的原生 SQLite 绑定版本范围与 complex 示例使用的最新 rusqlite 不同。使用 `cargo run --manifest-path examples/seaorm-sqlite/Cargo.toml --locked` 运行。

## 最小示例

```rust
use fluxdi::{Application, Injector, Module, Provider, Shared};

#[derive(Debug)]
struct Config(&'static str);

struct AppModule;

impl Module for AppModule {
    fn configure(&self, injector: &Injector) -> Result<(), fluxdi::Error> {
        injector.provide::<Config>(Provider::root(|_| Shared::new(Config("prod"))));
        Ok(())
    }
}

fn main() {
    fluxdi::init_logging();

    let mut app = Application::new(AppModule);
    app.bootstrap_sync().expect("bootstrap failed");

    let config = app.injector().resolve::<Config>();
    println!("mode={}", config.0);
}
```

## 构建与测试

```bash
cargo check --workspace
cargo test --workspace
```

## 本地运行日志

运行示例或应用时，启用详细的 DI 节点日志：

```bash
RUST_LOG=fluxdi=trace cargo run -p basic
```

## 可选的派生宏

```toml
[dependencies]
fluxdi = { path = "../fluxdi", features = ["macros"] }
```

然后使用：

```rust
use fluxdi::{Injectable, Shared};

#[derive(Injectable)]
struct AppService {
    dep: Shared<MyDependency>,
}
```
