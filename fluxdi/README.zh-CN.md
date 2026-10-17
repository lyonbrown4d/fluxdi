# FluxDI

[English](README.md) · [简体中文](README.zh-CN.md)

FluxDI 是面向 Rust 的类型安全依赖注入容器。核心独立于 HTTP 框架；Axum 和 Actix-web 适配器均为可选功能。

本 README 介绍**尚未发布的工作区**，要求 **Rust 1.99.1 或更新版本**。核心与宏变更需要协调发布后才能从 registry 安装。[中文手册](../docs/zh-CN/src/introduction.md)是正式参考。

## 本地依赖

```toml
[dependencies]
fluxdi = { path = "../fluxdi" }
```

添加 `features = ["macros"]`，即可使用 `Injectable` 和 `static_module!`；也可以按需选择下方的异步工厂、线程安全、适配器和可观测性 feature。

## 手动注册

```rust
use fluxdi::{Error, Injector, Provider, Shared};

struct Config(&'static str);
struct Service(Shared<Config>);

fn main() -> Result<(), Error> {
    let injector = Injector::root();
    injector.provide::<Config>(Provider::root(|_| Shared::new(Config("prod"))));
    injector.provide::<Service>(Provider::try_singleton(|inj| {
        let config = inj.try_resolve::<Config>()?;
        Ok(Shared::new(Service(config)))
    }));

    let service = injector.try_resolve::<Service>()?;
    assert_eq!(service.0.0, "prod");
    Ok(())
}
```

`Shared<T>` 默认使用 `Rc<T>`，启用 `thread-safe` 后使用 `Arc<T>`。线程安全服务必须满足 `Send + Sync + 'static`；本地服务可以不满足 `Send`。运行时注入器支持命名绑定、有序集合、装饰器和覆盖。

## 作用域契约

| 提供者 | 缓存生命周期 |
| --- | --- |
| `Provider::root` / `Scope::Root` | 在注入器树中全局共享 |
| `Provider::singleton` / `Scope::Module` | 每个调用方注入器上下文缓存一次 |
| `Provider::scoped` / `Scope::Scoped` | 最近的 `create_scope()` 边界 |
| `Provider::transient` / `Scope::Transient` | 每次解析都创建新实例 |

克隆的句柄共享上下文。普通子级拥有自己的单例缓存；位于运行时作用域内时，它们共享该作用域的缓存。嵌套作用域隔离作用域实例。本地绑定优先于继承的缓存值。

对有缓存绑定的并发首次解析共享一次初始化。可失败的 `try_*` 工厂仅缓存成功结果；错误会传播并释放创建许可，允许后续尝试。每个调用方最多尝试初始化一次。用户代码的 panic 不会转换为 `Error`。

异步工厂要求启用 `async-factory`，未缓存时要求异步解析。成功缓存的值可以同步读取。如果同步调用方遇到已经异步运行的初始化任务，会收到 `AsyncFactoryRequiresAsyncResolve`。完整并发规则参见[提供者](../docs/zh-CN/src/providers-and-resolution.md)和[可失败工厂](../docs/zh-CN/src/fallible-factories.md)。

## 两种宏用法

`#[derive(Injectable)]` 为运行时容器解析生成构造方法。它支持带有 `Shared<T>`、`Option<Shared<T>>` 和 `Vec<Shared<T>>` 字段的泛型具名字段结构体，以及命名依赖。注册 `YourType::provider(scope)`，即可附加推断出的依赖图提示。任意工厂闭包仍然需要显式提示。参见[派生宏](../docs/zh-CN/src/derive-macros.md)。

`static_module!` 声明显式的类型化依赖图，支持按绑定选择模块导入及同步、异步工厂。同步依赖图只需启用 `macros`：

```rust
use fluxdi::{Error, Shared, static_module};

struct Config(&'static str);
struct Service(Shared<Config>);

fn service(config: Shared<Config>) -> Result<Shared<Service>, Error> {
    Ok(Shared::new(Service(config)))
}

static_module! {
    struct App {
        #[scope(scoped)]
        service: Service = service(config);
        external config: Config;
    }
}

fn main() -> Result<(), Error> {
    let app = App::new(Shared::new(Config("prod")));
    let scope = app.create_scope();
    let service = scope.service()?;
    assert_eq!(service.0.0, "prod");
    Ok(())
}
```

该宏在编译期检查声明中缺失的绑定、循环、作用域和工厂类型。根绑定不能捕获已声明的上下文依赖。初始化仍然可能在运行时失败；网络操作、配置和外部资源构造不在这些检查范围内。

静态绑定独立于动态注册表：动态覆盖和依赖图快照不会描述或改变这些绑定。导入保留来源提供者的身份和作用域；构造方法按声明顺序接收导入模块实例及外部值，并创建新上下文。异步依赖图要求启用 `macros` 和 `async-factory`，异步依赖会使生成的 getter 变为异步。原生 `cfg`/`cfg_attr` 在依赖图检查前选择启用的声明。`optional(key)` 在绑定缺失时传入 `None`，保留工厂错误；`all(group)` 按声明顺序收集通过 `#[set(group)]` 显式标记的局部成员，允许空向量。只有启用的依赖边参与循环、根绑定捕获和异步检查。条件可能改变构造参数和 getter 的异步签名。尚不支持自动发现、循环导入或生命周期排序，不是完整的 CDI 实现。导入别名、条件、集合成员、上下文复用、异步标记及重试行为参见[静态模块](../docs/zh-CN/src/static-modules.md)。

## 应用与 HTTP 生命周期

`Module::configure` 注册提供者；`Application::bootstrap` 和 `shutdown` 运行生命周期钩子。启动选项支持并行执行；`lifecycle` feature 增加超时能力。启动失败会回滚已成功启动的模块。失败或被取消的启动钩子必须自行清理部分创建的资源。

HTTP 适配器需要显式安装 `request_scope` 中间件，才能为每个请求创建运行时作用域。没有该中间件时，`Resolved<T>` 从框架状态解析。请求作用域的持有在响应生成时结束，此时流式响应体不一定已经处理完成。流式响应和派生任务必须保留所需的服务或作用域句柄。每个请求没有自动异步关闭机制。参见[生命周期](../docs/zh-CN/src/modules-and-lifecycle.md)、[生产环境模式](../docs/zh-CN/src/production-patterns.md)和 [Web 集成](../docs/zh-CN/src/web-integrations.md)。

## Feature 列表

| Feature | 用途 |
| --- | --- |
| `default` | 启用 `debug` |
| `debug` | 用于容器和提供者诊断的 Debug 实现 |
| `thread-safe` | 线程安全容器与基于 `Arc` 的共享服务 |
| `lock-free` | DashMap 存储；启用 `thread-safe`；使用分片锁 |
| `async-factory` | 异步提供者工厂 |
| `resource-limit-async` | 异步资源限制；启用 `thread-safe`、`async-factory` 和 Tokio |
| `macros` | 重新导出 `Injectable` 和 `static_module!` |
| `lifecycle` | 基于 Tokio 的生命周期超时 |
| `axum` | 可选的 Axum 适配器；启用 `thread-safe` |
| `actix` | 可选的 Actix-web 适配器；启用 `thread-safe` |
| `tracing` | Tracing 埋点 |
| `logging` | Tracing subscriber 与 `RUST_LOG` 支持；启用 `tracing` |
| `metrics` | 注入器指标快照 |
| `prometheus` | Prometheus 文本导出；启用 `metrics` |
| `opentelemetry` | OpenTelemetry 追踪集成；启用 `tracing` |

可观测性与基准测试限制参见[可观测性](../docs/zh-CN/src/observability.md)和[性能](../docs/zh-CN/src/performance.md)。

## 仓库命令

在仓库根目录使用固定的工具链运行：

```bash
cargo check --workspace --locked
cargo test --workspace --locked
cargo run -p basic --locked
cargo run -p fluxdi --example static_module --features macros --locked
cargo run -p fluxdi --example static_composition_async --features macros,async-factory --locked
cargo run -p fluxdi --example static_conditionals --features macros --locked
cargo run -p fluxdi --example static_conditionals --no-default-features --features macros --locked
sh docs/build.sh
```

文档命令会构建两种语言版本。工作区依赖版本集中定义在根清单中。SeaORM 的 SQLite 示例具有独立的工作区和锁文件，以避免原生 SQLite ABI 依赖冲突：

```bash
cargo run --manifest-path examples/seaorm-sqlite/Cargo.toml --locked
```

其他示例使用 `cargo run -p <package>`。详情参见[示例列表](../docs/zh-CN/src/examples.md)和[依赖审计](../docs/zh-CN/src/dependency-audit.md)。
