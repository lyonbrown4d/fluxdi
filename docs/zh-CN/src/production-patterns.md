# 长期运行服务的生产环境模式

本指南介绍将 FluxDI 应用作为长期运行服务（HTTP 服务、worker 等）时的推荐方式。

## 生命周期 Feature

启用 `lifecycle` feature，获得超时支持：

```toml
[dependencies]
fluxdi = { path = "../fluxdi", features = ["lifecycle"] }
```

## 并行启动模块

并行运行模块的 `on_start` 钩子，缩短启动时间：

```rust
let opts = BootstrapOptions::default().with_parallel_start(true);
app.bootstrap_with_options(opts).await?;
```

并行启动期间，如果多个模块失败，所有失败会聚合为单个错误，与关闭行为类似。可以通过 `Error::bootstrap_aggregate` 在程序中访问。

无论顺序启动还是并行启动，只要任一模块启动失败，已经启动的模块都会回滚：返回错误前，按逆序调用这些模块的 `on_stop`。

## 启动超时

避免模块阻塞导致启动无限等待：

```rust
use std::time::Duration;
use fluxdi::{Application, BootstrapOptions};

let opts = BootstrapOptions::default().with_timeout(Duration::from_secs(30));
app.bootstrap_with_options(opts).await?;
```

## 使用 Ctrl+C 优雅关闭

等待关闭信号后再停止：

```rust
#[tokio::main]
async fn main() -> Result<(), fluxdi::Error> {
    let mut app = Application::new(MyModule);
    app.bootstrap().await?;

    tokio::signal::ctrl_c().await.expect("failed to listen for signal");
    app.shutdown().await
}
```

完整实现参见 `examples/dual-http-random-port`。

## 关闭超时（优雅关闭）

避免模块清理缓慢导致关闭无限等待。配置超时后，关闭操作仍会**优雅关闭**：在剩余时间预算内尝试调用每个模块的 `on_stop`。所有模块都会被尝试，不会只中止一部分；超时和失败会聚合为单个错误。

```rust
let opts = ShutdownOptions::default().with_timeout(Duration::from_secs(10));
app.shutdown_with_options(opts).await?;
```

## 后台服务任务

启动流程会等待 `on_start` 返回；将服务放到后台运行，使 `bootstrap()` 能够返回：

```rust
fn on_start(&self, injector: Shared<Injector>) -> ModuleLifecycleFuture {
    Box::pin(async move {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
        let app = injector.resolve::<Router>().as_ref().clone();

        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        Ok(())
    })
}
```

## 在 on_stop 中中止后台任务

在 `on_start` 中派生任务时，保存任务句柄，并在 `on_stop` 中中止：

```rust
#[derive(Default)]
struct ServerRuntime {
    tasks: Mutex<Vec<JoinHandle<()>>>,
}

fn on_start(&self, injector: Shared<Injector>) -> ModuleLifecycleFuture {
    Box::pin(async move {
        let runtime = injector.resolve::<ServerRuntime>();
        let task = tokio::spawn(server_loop());
        runtime.tasks.lock().unwrap().push(task);
        Ok(())
    })
}

fn on_stop(&self, injector: Shared<Injector>) -> ModuleLifecycleFuture {
    Box::pin(async move {
        let runtime = injector.resolve::<ServerRuntime>();
        for task in runtime.tasks.lock().unwrap().drain(..) {
            task.abort();
        }
        Ok(())
    })
}
```

完整模式参见 `examples/dual-http-random-port`。

## 错误处理

- **启动失败**：提前返回；应用尚未启动。
- **关闭失败**：FluxDI 聚合所有模块失败。通过 `err.message` 查看完整列表。记录错误，并以非零退出码退出。
- **关闭错误聚合**：尝试关闭所有模块，返回列出每项失败的单个错误。

## 模块结构

按职责组织：

1. **配置或基础设施模块**（数据库、缓存、配置）：不包含 HTTP。
2. **Web 模块**：依赖基础设施，在 `on_start` 中启动 HTTP 监听器。
3. **根模块**：导入基础设施模块与 Web 模块，可选择处理信号。

示例结构：

```
AppModule
├── DatabaseModule   (on_start: connect, on_stop: disconnect)
├── CacheModule      (on_start: connect)
└── WebModule        (on_start: bind + spawn server, on_stop: abort tasks)
```

## 相关示例

- `examples/axum-lifecycle`：带有生命周期的最小 HTTP 服务。
- `examples/dual-http-random-port`：多个服务、ctrl_c 与任务中止。
- `examples/axum`：使用 DI 的完整 REST API。
