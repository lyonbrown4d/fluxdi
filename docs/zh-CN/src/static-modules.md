# 静态模块

尚未发布的 `static_module!` 宏提供显式的同步组装方式。需要启用 `macros`；核心仍然独立于 HTTP 框架。

```toml
fluxdi = { path = "../fluxdi", features = ["macros"] }
```

## 声明依赖图

```rust
use fluxdi::{Error, Shared, static_module};

struct Config(String);
struct Repository(Shared<Config>);
struct Service(Shared<Repository>);

fn repository(config: Shared<Config>) -> Result<Shared<Repository>, Error> {
    Ok(Shared::new(Repository(config)))
}

fn service(repository: Shared<Repository>) -> Result<Shared<Service>, Error> {
    Ok(Shared::new(Service(repository)))
}

static_module! {
    struct App {
        #[scope(scoped)]
        service: Service = service(repository);
        #[scope(root)]
        repository: Repository = repository(config);
        external config: Config;
    }
}

let app = App::new(Shared::new(Config("memory://demo".into())));
let request = app.create_scope();
let a = request.service()?;
let b = request.service()?;
assert!(Shared::ptr_eq(&a, &b));
# Ok::<(), Error>(())
```

通过绑定标识符选择依赖。多个绑定可以具有相同类型；`service(primary_repository)` 显式选择其中一个。支持 trait 对象和类型别名，Rust 会检查工厂参数与返回类型。声明顺序不决定构造顺序。

每个工厂都是函数路径，接收声明的 `Shared<T>` 参数并返回 `Result<Shared<T>, Error>`。工厂在首次解析时按需执行。外部值按声明顺序传入 `new`，具有根作用域。这些值构成显式边界：宏无法检查它们的构造过程。

## 编译检查

宏展开会拒绝未知绑定标识符、重复标识符、构造循环、无效作用域及生成方法的名称冲突。循环诊断包含依赖链。工厂参数或返回类型不兼容时，Rust 类型检查会失败。

根绑定不能直接或间接捕获 `singleton` 或 `scoped` 绑定，包括通过瞬态工厂捕获。在没有作用域代理的情况下，这种依赖会把首个调用方的上下文服务保留在全局缓存中。根绑定可以依赖根绑定或外部绑定，也可以保留瞬态工厂生成的值，前提是该工厂的依赖图不包含上下文绑定。

这些检查覆盖声明的依赖图。工厂内部的网络调用、配置、全局状态，或对另行捕获的动态容器的调用，仍然属于运行时问题。编译成功不代表初始化一定成功。

## 上下文与缓存

| 声明 | 缓存位置 |
| --- | --- |
| 无作用域属性，或 `#[scope(singleton)]` | 调用方注入器上下文 |
| `#[scope(root)]` | 注入器树的根 |
| `#[scope(scoped)]` | 最近的 `create_scope()` 边界 |
| `#[scope(transient)]` | 不缓存 |

生成的方法：

- `new(external_values...)`：创建根上下文和类型化提供者。
- 每个绑定名称都是返回 `Result<Shared<T>, Error>` 的 getter。
- `clone()`：共享上下文与提供者身份。
- `create_child()`：创建普通子上下文。
- `create_scope()`：创建隔离的作用域上下文。
- `in_context(&Injector)`：在给定上下文中复用这些提供者。
- `injector()`：返回模块当前上下文。

没有外部输入的模块也实现 `Default`。

需要保持根实例共享时，应使用同一注入器树中的上下文。`in_context` 使用给定树的根缓存，不检查祖先关系。嵌套作用域隔离作用域实例。作用域内的普通子级复用该作用域的实例，同时保留自己的调用方上下文单例缓存。

生成的依赖连接代码持有类型化提供者句柄，并直接调用 `try_resolve_provider`。它遵循运行时的初始化、并发、错误传播和重试规则。这些绑定不会注册到动态类型或名称注册表中，因此动态覆盖和 `dependency_graph()` 不会改变或描述静态依赖图。其余缓存与循环保护机制仍在运行时执行；本 API 不承诺零查找或零分配开销。

## 当前限制

首版静态组装 API 支持带有外部输入的单个封闭同步依赖图。尚不支持模块导入、自动发现、条件绑定、隐式选择可选依赖或集合、生成异步工厂，或生命周期启动与关闭排序。需要时，将可选值、集合或已初始化的异步资源显式作为外部输入传入。`Injectable` 仍是独立的运行时容器派生 API。

`new`、`create_child`、`create_scope`、`in_context`、`injector`、`clone` 和 `default` 是保留绑定名称。不支持模块泛型和非文档属性。发布本 API 时，应同时发布核心 crate 和宏 crate。

将静态模块接入请求作用域的方法参见 [Web 集成](./web-integrations.md)。运行可执行示例：

```bash
cargo run -p fluxdi --example static_module --features macros
```
