# 派生宏

尚未发布的工作区扩展了 `#[derive(Injectable)]`，支持泛型、命名绑定、可选依赖、集合依赖及推断出的依赖图提示。要求 Rust 1.99.1 或更新版本。

本地工作区用法：

```toml
[dependencies]
fluxdi = { path = "../fluxdi", features = ["macros"] }
```

## 注册派生服务

```rust
use fluxdi::{Injectable, Injector, Provider, Scope, Shared};

struct Clock(u64);

#[derive(Injectable)]
struct AppService {
    clock: Shared<Clock>,
}

fn main() -> Result<(), fluxdi::Error> {
    let injector = Injector::root();
    injector.provide::<Clock>(Provider::root(|_| Shared::new(Clock(42))));
    injector.provide::<AppService>(AppService::provider(Scope::Root));
    injector.try_validate_graph()?;
    assert_eq!(injector.try_resolve::<AppService>()?.clock.0, 42);
    Ok(())
}
```

`YourType::provider(scope)` 创建可失败提供者，并附加从字段推断出的依赖图提示。使用 `Scope::Module` 获得调用方上下文单例行为；`Root`、`Scoped` 和 `Transient` 分别对应各自的生命周期。生成的提供者仍可使用装饰器、限制、命名注册和集合注册。

## 字段声明

| 字段 | 解析方法 | 依赖图要求 |
| --- | --- | --- |
| `Shared<T>` | `try_resolve::<T>()` | 一个未命名绑定 |
| `#[inject(name = "main")] Shared<T>` | `try_resolve_named::<T>("main")` | 该名称对应的绑定 |
| `Option<Shared<T>>` | `try_resolve_optional::<T>()` | 允许绑定不存在 |
| `#[inject(name = "main")] Option<Shared<T>>` | `try_resolve_optional_named::<T>("main")` | 允许该命名绑定不存在 |
| `Vec<Shared<T>>` | `try_resolve_all::<T>()` | 至少一个集合成员 |

支持 `fluxdi::Shared<T>`、`std::option::Option<Shared<T>>` 和 `std::vec::Vec<Shared<T>>` 等限定路径。宏不会检查任意重命名的类型别名。命名集合依赖、未知选项、重复名称和不支持的字段形式会被拒绝。

可选依赖仅在绑定本身不存在时返回 `None`。已有工厂内部缺少依赖、初始化失败、要求异步解析以及循环依赖仍然属于错误。旧的 `optional_resolve*` 辅助方法会丢弃所有错误；派生构造方法改用新的可失败辅助方法。

集合依赖保留现有的 `resolve_all` 契约：没有成员时返回错误，并按从父级到子级的注册顺序解析成员。成员初始化错误会向上传播。集合不能命名。

## 泛型

类型参数、生命周期参数、const 参数以及已有的 `where` 子句都会保留：

```rust
#[derive(Injectable)]
struct Pipeline<T: ?Sized, const N: usize>
where
    T: Backend,
{
    backend: Shared<T>,
    #[inject(name = "fallback")]
    fallback: Option<Shared<T>>,
    plugins: Vec<Shared<T>>,
    bytes: Shared<[u8; N]>,
}
```

生成的 impl 要求每个依赖类型及服务本身实现 `fluxdi::Service`。该统一标记要求 `'static`；启用 `thread-safe` 时还要求 `Send + Sync`。泛型声明无需重复这些约束。本地运行时构建仍接受非 `Send` 服务。借用依赖必须具有 `'static` 生命周期才能参与 DI。

<a id="generated-api"></a>

## 生成的 API

派生宏生成四个固有方法：

- `from_injector(&Injector) -> Shared<Self>`：同步构造，遇错 panic。
- `try_from_injector(&Injector) -> Result<Shared<Self>, Error>`：同步构造并传播依赖错误。
- `provider(Scope) -> Provider<Self>`：创建带有推断依赖图提示的可失败提供者。
- `with_injected_dependencies(Provider<Self>) -> Provider<Self>`：为自定义工厂附加相同提示，不替换该工厂。

如果类型已经定义了同名方法，派生前需要重命名该方法。原有构造方法仍然可用。通过 `Provider::root(AppService::from_injector)` 或 `Provider::try_root(AppService::try_from_injector)` 调用仍然有效，但不会自动附加提示：任意提供者闭包的内部依赖仍然不可见。

使用自定义构造方法或异步工厂时，显式附加一次提示：

```rust
let provider = AppService::with_injected_dependencies(
    Provider::try_root(|inj| AppService::try_from_injector(inj))
);
injector.provide::<AppService>(provider);
```

构造方法是同步的，不会等待异步依赖工厂。两个构造方法都不捕获用户代码的 panic。初始化错误与重试规则参见[可失败工厂](./fallible-factories.md)，验证方式参见[依赖图工具](./graph-tooling.md)。

## 限制与示例

仅支持具名字段结构体。不支持元组结构体、单元结构体、枚举、联合体、普通配置字段及上述形式以外的嵌套包装。此类构造应使用手动工厂。

运行 `cargo run -p injectable-macro-example`，查看注册与依赖图验证的组合用法。
