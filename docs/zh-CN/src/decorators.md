# 装饰器

使用 `Provider::with_decorator()` 包装解析得到的实例，添加日志、缓存、重试等通用行为，无需修改业务服务。

## 基本用法

```rust
use fluxdi::{Injector, Provider, Shared};

trait Service: Send + Sync {
    fn do_work(&self) -> String;
}

struct RealService;
impl Service for RealService {
    fn do_work(&self) -> String { "done".to_string() }
}

// Logging decorator
struct LoggingService {
    inner: Shared<dyn Service>,
}
impl Service for LoggingService {
    fn do_work(&self) -> String {
        println!("before");
        let r = self.inner.do_work();
        println!("after: {}", r);
        r
    }
}

let injector = Injector::root();
injector.provide::<dyn Service>(
    Provider::singleton(|_| Shared::new(RealService) as Shared<dyn Service>)
        .with_decorator(|inner| Shared::new(LoggingService { inner }) as Shared<dyn Service>),
);

let svc = injector.resolve::<dyn Service>();
svc.do_work();  // Logs "before" and "after"
```

## 多个装饰器

可以链式添加装饰器；执行顺序为基础实例 → 第一个装饰器 → 第二个装饰器：

```rust
provider
    .with_decorator(logging_decorator)
    .with_decorator(caching_decorator)
```

## 顺序与作用域

- 每次**工厂**运行时，装饰器都会执行。
- 对于单例，工厂运行一次，因此每个装饰器也执行一次。
- 对于瞬态提供者，工厂在每次解析时运行，装饰器也随之执行。
- 作用域（单例、瞬态等）保持不变。

## 示例

`examples/decorator` 提供了包含日志和缓存装饰器的可运行示例。
