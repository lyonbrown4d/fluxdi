# 异步与混合使用

启用异步提供者工厂：

```toml
[dependencies]
fluxdi = { path = "../fluxdi", features = ["async-factory"] }
```

使用 `Provider::*_async` 注册异步提供者，再通过以下方法解析：

- `try_resolve_async::<T>().await`
- `resolve_async::<T>().await`

同步与异步提供者可以共存于同一个模块。

参见：

- `examples/module-async`
- `examples/module-mixed-sync-async`

使用生成的静态组装代码时，还需启用 `macros`。以 `async key: Type = factory(dependencies);` 声明异步工厂，依赖该绑定的 getter 会自动变为异步。导入和异步标记检查参见[静态模块](./static-modules.md)。
