# 提供者与解析

FluxDI 支持四种作用域：

- `Provider::transient(...)`：每次解析都创建新实例。
- `Provider::root(...)`：在根注入器作用域中全局缓存。
- `Provider::singleton(...)`：在每个调用方注入器上下文（`Scope::Module`）中缓存一次。克隆的注入器句柄共享该上下文；子注入器和新的运行时作用域各有自己的缓存。
- `Provider::scoped(...)`：在通过 `create_scope()` 创建的每个运行时作用域中缓存一次。

从父级继承的提供者使用调用方上下文解析。本地绑定优先于父级已缓存的实例。嵌套调用 `create_scope()` 会建立新的作用域缓存；该作用域内的普通子注入器共享最近的作用域缓存。根绑定在整个层级中保持共享。

对于有缓存的绑定，并发的首次解析共享一次初始化。初始化任务被取消或发生 panic 后，异步等待方可以重试。如果同步解析遇到正在异步执行的初始化任务，会返回 `AsyncFactoryRequiresAsyncResolve`，避免阻塞执行器；需要等待时应使用异步解析。同步解析尚未缓存的异步工厂仍然返回 `AsyncFactoryRequiresAsyncResolve`。

示例：

```rust
use fluxdi::{Injector, Provider, Shared};

#[derive(Debug)]
struct Logger;

let injector = Injector::root();
injector.provide::<Logger>(Provider::root(|_| Shared::new(Logger)));

let logger = injector.resolve::<Logger>();
```

作用域上下文示例：

```rust
use fluxdi::{Injector, Provider, Shared};

#[derive(Debug)]
struct RequestContext(usize);

let injector = Injector::root();
injector.provide::<RequestContext>(Provider::scoped(|_| Shared::new(RequestContext(1))));

let scope = injector.create_scope();
let first = scope.resolve::<RequestContext>();
let second = scope.resolve::<RequestContext>();

assert!(Shared::ptr_eq(&first, &second));
```

需要可恢复的错误时，优先使用 `try_resolve::<T>()`：

```rust
let logger = injector.try_resolve::<Logger>()?;
```

## 命名绑定

可以按名称为同一类型或 trait 注册多个实现：

```rust
use fluxdi::{Injector, Provider, Shared};

trait Cache: Send + Sync {
    fn id(&self) -> &'static str;
}

struct Redis;
impl Cache for Redis {
    fn id(&self) -> &'static str { "redis" }
}

struct Memory;
impl Cache for Memory {
    fn id(&self) -> &'static str { "memory" }
}

let injector = Injector::root();

injector.provide_named::<dyn Cache>(
    "primary",
    Provider::root(|_| Shared::new(Redis) as Shared<dyn Cache>),
);
injector.provide_named::<dyn Cache>(
    "fallback",
    Provider::root(|_| Shared::new(Memory) as Shared<dyn Cache>),
);

let primary = injector.resolve_named::<dyn Cache>("primary");
let fallback = injector.resolve_named::<dyn Cache>("fallback");

assert_eq!(primary.id(), "redis");
assert_eq!(fallback.id(), "memory");
```

## 多重绑定

可以将多个提供者注册到集合中，再解析全部成员：

```rust
use fluxdi::{Injector, Provider, Shared};

trait Middleware: Send + Sync {
    fn name(&self) -> &'static str;
}

struct Trim;
impl Middleware for Trim {
    fn name(&self) -> &'static str { "trim" }
}

struct Uppercase;
impl Middleware for Uppercase {
    fn name(&self) -> &'static str { "uppercase" }
}

let injector = Injector::root();
injector.provide_into_set::<dyn Middleware>(
    Provider::singleton(|_| Shared::new(Trim) as Shared<dyn Middleware>),
);
injector.provide_into_set::<dyn Middleware>(
    Provider::singleton(|_| Shared::new(Uppercase) as Shared<dyn Middleware>),
);

let pipeline = injector.resolve_all::<dyn Middleware>();
let names: Vec<&str> = pipeline.iter().map(|m| m.name()).collect();
assert_eq!(names, vec!["trim", "uppercase"]);
```

解析顺序确定：

- 先解析父注入器的集合绑定。
- 再解析子注入器的集合绑定。
- 同一注入器内保持注册顺序。

## 覆盖提供者

在测试和集成环境中，可以替换已注册的提供者：

```rust
use fluxdi::{Injector, Provider, Shared};

let injector = Injector::root();
injector.provide::<String>(Provider::root(|_| Shared::new("real".to_string())));

injector
    .try_override_provider::<String>(Provider::root(|_| Shared::new("mock".to_string())))
    .unwrap();
```

注意：

- 覆盖操作要求提供者已经存在。
- 根作用域的覆盖会影响子注入器对根绑定的解析。
- 被覆盖类型的缓存实例会失效。

## 错误建议

FluxDI 错误包含可执行的提示。常见情况包括：

- 服务缺失：建议在 `Module::configure(...)` 中注册。
- 通过同步路径解析异步提供者：建议使用 `try_resolve_async/resolve_async`。
- 提供者重复：建议使用 `override_provider/try_override_provider`。
