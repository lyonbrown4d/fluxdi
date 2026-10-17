# 依赖图工具

FluxDI 可以从提供者元数据导出依赖图并验证：

- `injector.dependency_graph()`：返回依赖图模型。
- `injector.validate_graph()`：返回缺失依赖或循环问题。
- `injector.try_validate_graph()`：依赖图无效时返回 `Err`。

## 声明依赖

依赖图验证依赖提供者的依赖提示：

```rust
use fluxdi::{Injector, Provider, Shared};

struct A;
struct B;

let injector = Injector::root();
injector.provide::<A>(
    Provider::singleton(|_| Shared::new(A)).with_dependency::<B>(),
);
injector.provide::<B>(Provider::singleton(|_| Shared::new(B)));

let report = injector.validate_graph();
assert!(report.is_valid());
```

其他提示：

- `with_named_dependency::<T>("name")`
- `with_set_dependency::<T>()`（要求至少一个集合成员）
- `with_optional_dependency::<T>()`
- `with_optional_named_dependency::<T>("name")`

使用 `#[derive(Injectable)]` 时，注册 `YourType::provider(scope)`，即可从字段声明推断这些提示。自定义工厂则使用 `YourType::with_injected_dependencies(provider)` 包装。参见[派生宏](./derive-macros.md)。

可选提示使用 `DependencyCardinality::Optional`。可选目标不存在时视为有效。已注册的目标会创建 `optional` 或 `optional:name` 边，并参与循环验证。已有对 `DependencyCardinality` 的穷尽匹配必须处理新增变体。

依赖图验证只检查元数据，不初始化服务，也不验证工厂能否成功。任意闭包内部的依赖仍需要显式提示。依赖图快照不会冻结容器，也不阻止后续注册变更。

编译期验证与生成的类型化依赖连接参见[静态模块](./static-modules.md)。静态绑定独立于动态注册表，不包含在这些快照中。

## DOT 与 Mermaid 输出

```rust
let graph = injector.dependency_graph();
let dot = graph.to_dot();
let mermaid = graph.to_mermaid();
```

## 适用于 CI 的检查

可以在测试或启动时快速报告错误：

```rust
injector.try_validate_graph()?;
```

可运行示例：

```bash
cargo run -p graph-tooling-example
```
