# 可失败工厂

这些 API 属于 FluxDI 2.0.0，要求 Rust 1.99.1 或更新版本。

## 同步初始化

初始化可能失败时，使用 `Provider::try_root`、`try_singleton`、`try_scoped` 或 `try_transient`。这些方法均接收返回 `Result<Shared<T>, fluxdi::Error>` 的工厂，生命周期与对应的不可失败版本相同。

```rust
use fluxdi::{Error, Injector, Provider, Shared};

struct Port(u16);

fn main() -> Result<(), Error> {
    let injector = Injector::root();
    injector.provide::<Port>(Provider::try_root(|_| {
        let raw = std::env::var("PORT")
            .map_err(|error| Error::factory_failed("Port", &error.to_string()))?;
        let port = raw.parse::<u16>()
            .map_err(|error| Error::factory_failed("Port", &error.to_string()))?;
        Ok(Shared::new(Port(port)))
    }));

    let port = injector.try_resolve::<Port>()?;
    println!("port={}", port.0);
    Ok(())
}
```

在同步工厂中，通过 `inj.try_resolve::<Dependency>()?` 传播依赖错误。`#[derive(Injectable)]` 也提供[可失败构造方法](./derive-macros.md#generated-api)。

## 异步初始化

启用 `async-factory` 后，使用 `try_root_async`、`try_singleton_async`、`try_scoped_async` 或 `try_transient_async`。工厂接收拥有所有权的注入器，并返回产出 `Result<Shared<T>, Error>` 的 future。

```rust
injector.provide::<Service>(Provider::try_singleton_async(|inj| async move {
    let dependency = inj.try_resolve_async::<Dependency>().await?;
    let service = Service::connect(dependency).await
        .map_err(|error| Error::factory_failed("Service", &error.to_string()))?;
    Ok(Shared::new(service))
}));
let service = injector.try_resolve_async::<Service>().await?;
```

尚未缓存的异步提供者要求异步解析。成功缓存的实例也可以同步读取。启用 `thread-safe` 时，工厂必须满足 `Send + Sync`，其 future 必须满足 `Send`。

## 错误、缓存与重试

- `try_resolve*` 方法原样传播工厂错误，包括命名绑定和集合绑定的错误。`Error::factory_failed` 用于构造应用错误；依赖错误保留原始分类。
- 根作用域、调用方上下文单例和运行时作用域提供者仅缓存成功创建的实例。瞬态提供者在每次解析时初始化。
- 初始化出错后，调用方收到 `Err`。创建许可会被释放，后续调用方可以再次尝试初始化。并发等待该实例的调用方会被唤醒，并可能执行下一次尝试；它们不一定收到与第一个调用方相同的错误。每个调用方最多执行一次初始化尝试。
- 装饰器仅在初始化成功后按注册顺序执行。资源限制通过 `.with_limits(...)` 配置。
- `resolve*` 和 `from_injector` 保持遇错 panic 的行为。可失败 API 传播 `Result` 错误，不会将用户代码的 panic 转换为 `Error`。

`ErrorKind` 新增了 `FactoryFailed` 变体。升级时，下游代码中穷尽匹配所有变体的位置必须为它添加分支。
