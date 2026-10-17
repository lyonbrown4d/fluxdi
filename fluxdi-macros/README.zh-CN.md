# fluxdi-macros

[English](README.md) · [简体中文](README.zh-CN.md)

FluxDI **尚未发布工作区**的过程宏，要求 **Rust 1.99.1 或更新版本**。核心与宏需要协调发布后才能从 registry 安装。本地开发时通过 `fluxdi` 启用宏：

```toml
[dependencies]
fluxdi = { path = "../fluxdi", features = ["macros"] }
```

## 运行时容器派生宏

```rust
use fluxdi::{Injectable, Scope, Shared};

#[derive(Injectable)]
struct AppService<T: ?Sized> {
    backend: Shared<T>,
    #[inject(name = "fallback")]
    fallback: Option<Shared<T>>,
    plugins: Vec<Shared<T>>,
}

// After registering all required dependencies:
// injector.provide::<AppService<MyBackend>>(AppService::provider(Scope::Root));
```

`Injectable` 保留类型、生命周期、const 泛型及 `where` 子句。它支持字段为 `Shared<T>`、`Option<Shared<T>>` 或 `Vec<Shared<T>>` 的具名字段结构体。生成的依赖类型和服务必须满足 `fluxdi::Service`：要求 `'static`，启用 `thread-safe` 时还要求 `Send + Sync`。

它生成 `from_injector`、`try_from_injector`、`provider(scope)` 和 `with_injected_dependencies(provider)`。后两者附加依赖图提示；单独使用构造方法不会附加。构造方法是同步的，不会等待异步工厂，也不会捕获用户代码的 panic。

可选字段仅在绑定不存在时返回 `None`。工厂错误会向上传播。集合要求至少一个注册成员，且不能命名。类型约束和错误契约参见[派生宏参考](../docs/zh-CN/src/derive-macros.md)。

## 显式静态组装

`static_module!` 声明类型化依赖图，支持外部输入、按绑定选择模块导入，以及同步或异步工厂调用。宏展开检查缺失绑定标识符、循环、作用域捕获和方法冲突；Rust 检查工厂参数与返回类型。根绑定不能捕获已声明的上下文依赖，即使通过瞬态绑定间接捕获也不允许。

提供者按需初始化，使用运行时缓存并允许初始化失败。编译成功不代表网络操作、配置或外部资源一定初始化成功。静态绑定位于动态注册表之外；动态覆盖和依赖图快照不会改变或描述这些绑定。

导入显式选择绑定标识符，可指定别名，并在重导出时保留来源提供者的作用域和安全元数据。异步依赖图还要求启用 `async-factory`；异步依赖会使 getter 变为异步，导入标记必须匹配来源元数据。尚不支持自动发现、条件绑定、循环导入或生命周期排序。语法、上下文复用与可运行示例参见[静态模块](../docs/zh-CN/src/static-modules.md)；两种注册方式与全部 feature 参见[核心 README](../fluxdi/README.zh-CN.md)。

## 仓库命令

在仓库根目录运行：

```bash
cargo run -p injectable-macro-example --locked
cargo run -p fluxdi --example static_module --features macros --locked
cargo run -p fluxdi --example static_composition_async --features macros,async-factory --locked
cargo test --workspace --locked
sh docs/build.sh
```

文档命令会构建英文和简体中文手册。
