# 模块与生命周期

`Module::configure(...)` 是推荐的提供者注册入口。异步启动调用 `providers_async(...)`，其默认实现调用 `configure(...)`；已有的异步钩子重写仍会执行。

使用以下方法编排异步启动与关闭：

- `Module::on_start(...)`
- `Module::on_stop(...)`
- `Application::bootstrap().await`
- `Application::shutdown().await`

仍可通过 `bootstrap_sync()` 进行纯同步启动。

## 生命周期选项（超时）

启用 `lifecycle` feature，使用启动与关闭超时能力。并行启动选项不依赖该 feature：

```toml
[dependencies]
fluxdi = { path = "../fluxdi", features = ["lifecycle"] }
```

```rust
use std::time::Duration;
use fluxdi::{Application, BootstrapOptions, ShutdownOptions, Injector, Module, Provider, Shared};
use fluxdi::module::ModuleLifecycleFuture;

struct AppModule;

impl Module for AppModule {
    fn configure(&self, injector: &Injector) -> Result<(), fluxdi::Error> {
        injector.provide::<String>(Provider::root(|_| Shared::new("ok".to_string())));
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), fluxdi::Error> {
    let mut app = Application::new(AppModule);

    // Bootstrap with a 30s timeout
    let opts = BootstrapOptions::default().with_timeout(Duration::from_secs(30));
    app.bootstrap_with_options(opts).await?;

    // Shutdown with a 10s timeout
    let opts = ShutdownOptions::default().with_timeout(Duration::from_secs(10));
    app.shutdown_with_options(opts).await?;

    Ok(())
}
```

超时后，启动操作返回错误，并按模块顺序的逆序停止已成功启动的模块。配置错误和启动错误使用相同的回滚路径。清理操作获得单独的超时预算，其时长等于配置的启动超时，因此完整操作可能超过该超时时长。如果调用方直接取消启动操作，应调用 `shutdown()` 清理已启动的模块。失败或被取消的 `on_start` 必须自行管理部分创建的资源；只有成功返回的钩子才进入关闭列表。`is_bootstrapped()` 仅在启动成功后变为 true，关闭后变为 false。启动失败或被取消的应用无法再次启动；重试时需要创建新的 `Application`。

## 聚合关闭错误

如果多个模块在 `on_stop()` 中失败，FluxDI 仍会尝试停止所有模块，并返回列出每项失败的单个聚合错误：

```
Shutdown failed: 2 module(s) reported errors:
  1) Module lifecycle failed: module=WebModule, phase=on_stop, details=...
  2) Module lifecycle failed: module=DatabaseModule, phase=on_stop, details=...
```

这有助于诊断和修复生产环境中的关闭问题。

优雅关闭、超时和后台任务的推荐方式参见[生产环境模式](./production-patterns.md)。

## 基本示例

```rust
use fluxdi::{Application, Error, Injector, Module, Provider, Shared, module::ModuleLifecycleFuture};

struct AppModule;

impl Module for AppModule {
    fn configure(&self, injector: &Injector) -> Result<(), Error> {
        injector.provide::<String>(Provider::root(|_| Shared::new("ok".to_string())));
        Ok(())
    }

    fn on_start(&self, injector: Shared<Injector>) -> ModuleLifecycleFuture {
        Box::pin(async move {
            let value = injector.resolve::<String>();
            println!("started with {}", value);
            Ok(())
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let mut app = Application::new(AppModule);
    app.bootstrap().await?;
    app.shutdown().await
}
```
