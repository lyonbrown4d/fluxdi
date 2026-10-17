# 快速开始

## 安装

FluxDI **2.0.0** 要求 **Rust 1.99.1 或更新版本**。仓库的 `rust-toolchain.toml` 固定了该稳定版本。使用默认同步功能安装核心 crate：

```toml
[dependencies]
fluxdi = "2.0.0"
```

本地开发时，可将 registry 版本替换为 `path = "../fluxdi"`。核心与宏 crate 使用同一 2.0.0 版本边界；启用 `macros` 时精确依赖 `fluxdi-macros` 2.0.0。

直接依赖版本维护在根目录的 `[workspace.dependencies]` 中。SeaORM SQLite 示例使用独立工作区，因为它要求的原生 SQLite 绑定版本范围与 complex 示例使用的最新 rusqlite 不同。使用 `cargo run --manifest-path examples/seaorm-sqlite/Cargo.toml --locked` 运行。

## 从 1.x 迁移

- 将工具链升级到 Rust 1.99.1 或更新版本。使用 `fluxdi` 2.0.0 及对应的 `fluxdi-macros` 2.0.0；核心对该宏版本使用精确依赖。
- 对 `ErrorKind` 和 `DependencyCardinality` 做穷尽匹配时，分别补充 `FactoryFailed` 和 `Optional` 分支。
- 检查对 `Provider::singleton` / `Scope::Module` 的使用：实例按调用方注入器上下文缓存。克隆共享缓存；子级和新运行时作用域拥有各自的单例缓存。需要在整棵注入器树中共享时，使用 `Provider::root` / `Scope::Root`。
- 为 derive 生成的固有方法 `from_injector`、`try_from_injector`、`provider` 和 `with_injected_dependencies` 预留名称，重命名有冲突的应用方法。静态模块还预留 `new`、`create_child`、`create_scope`、`in_context`、`injector`、`clone`、`default` 及生成的隐藏导出方法名。

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
fluxdi = { version = "2.0.0", features = ["macros"] }
```

然后使用：

```rust
use fluxdi::{Injectable, Shared};

#[derive(Injectable)]
struct AppService {
    dep: Shared<MyDependency>,
}
```
