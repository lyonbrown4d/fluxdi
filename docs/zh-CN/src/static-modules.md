# 静态模块

尚未发布的 `static_module!` 宏提供显式的类型化组装，支持按绑定选择模块导入，以及同步和异步工厂。需要启用 `macros`；异步依赖图还要求 `async-factory`。核心仍然独立于 HTTP 框架。

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

同步工厂是函数路径，接收声明的 `Shared<T>` 参数并返回 `Result<Shared<T>, Error>`。工厂在首次解析时按需执行。外部值具有根作用域。导入模块实例和外部值按两者在声明中出现的先后顺序传入构造方法。这些值构成显式边界：宏无法检查它们的构造过程。

## 按绑定导入模块

导入块显式选择来源模块的绑定：

```rust
static_module! {
    struct Data {
        #[scope(root)]
        repository: Repository = repository(config);
        external config: Config;
    }
}

static_module! {
    struct Frontend {
        import data: crate::Data {
            primary_repository: Repository = repository;
        };
        service: Service = service(primary_repository);
        external label: String;
    }
}

let data = Data::new(Shared::new(Config("memory://demo".into())));
let app = Frontend::new(data.clone(), Shared::new(String::from("web")));
let shared_tree = app.in_context(data.injector());
```

导入别名 `data` 对应一个按值传入的构造方法参数。调用方需要保留来源模块时，传入 `data.clone()`。块内的 `local_key: Type = source_key;` 为选中的绑定重命名；省略 `= source_key` 时，两端使用同一个标识符。导入块以 `;` 结束。模块路径可以使用限定 Rust 路径，包括其他 crate 的路径。相同类型仍可对应不同绑定。局部绑定和导入绑定的标识符不能重复，导入别名也不能重复。

导入会克隆类型化提供者句柄，保留来源提供者的身份与作用域。导入不会调用来源 getter、修改来源模块，也不会保留整个来源模块及其上下文。`Frontend::new` 创建新的根上下文，不继承 `data` 的缓存，即使 `data` 已经解析过。要复用来源注入器树，应显式调用 `in_context(data.injector())`，如示例中的 `shared_tree`。同一来源实例的多次导入在导入方上下文中共享提供者身份。

## 异步工厂

```toml
fluxdi = { path = "../fluxdi", features = ["macros", "async-factory"] }
```

工厂自身返回 future 时，使用 `async` 声明：

```rust
async fn open_repository(config: Shared<Config>) -> Result<Shared<Repository>, Error> {
    Ok(Shared::new(Repository(config)))
}

static_module! {
    struct AsyncData {
        #[scope(root)]
        async repository: Repository = open_repository(config);
        external config: Config;
    }
}

static_module! {
    struct AsyncApp {
        import data: crate::AsyncData {
            async primary_repository: Repository = repository;
        };
        service: Service = service(primary_repository);
    }
}

let data = AsyncData::new(Shared::new(Config("memory://demo".into())));
let app = AsyncApp::new(data);
let service = app.service().await?;
```

异步工厂产出 `Future<Output = Result<Shared<T>, Error>>`。只要任一依赖是异步绑定，当前绑定也会生成异步 getter，即使它自己的工厂是同步函数。示例中的 `service` 先等待依赖，再调用 `service(...)`，不会等待该函数本身。该推断沿局部依赖传递。导入项必须显式标明来源绑定的最终异步状态；遗漏或多写 `async` 都会导致编译失败，包括重导出，以及因来源依赖而变为异步的绑定。

`new` 仍然同步执行，工厂按需初始化。解析按工厂调用中参数的书写顺序依次等待依赖，不会并行启动，也不引入执行器。未启用 `thread-safe` 时，工厂可以返回非 `Send` future；启用后沿用工厂的 `Send + Sync` 约束和 future 的 `Send` 约束。仅成功初始化的值会缓存，错误允许后续解析重试。初始化被取消后会释放初始化状态，后续调用方可以重试。具体规则与[可失败工厂](./fallible-factories.md)一致。

## 编译检查

宏展开会拒绝未知绑定标识符、重复标识符、构造循环、无效作用域及生成方法的名称冲突。循环诊断包含依赖链。工厂参数或返回类型不兼容时，Rust 类型检查会失败。

根绑定不能直接或间接捕获 `singleton` 或 `scoped` 绑定，包括通过瞬态工厂捕获。在没有作用域代理的情况下，这种依赖会把首个调用方的上下文服务保留在全局缓存中。根绑定可以依赖根绑定或外部绑定，也可以保留瞬态工厂生成的值，前提是该工厂的依赖图不包含上下文绑定。导入和重导出的元数据会在模块边界间保留此项检查，包括多层重导出和瞬态绑定的间接依赖；导入不会使原本依赖上下文的绑定变得可供根绑定捕获。

这些检查覆盖声明的依赖图。工厂内部的网络调用、配置、全局状态，或对另行捕获的动态容器的调用，仍然属于运行时问题。编译成功不代表初始化一定成功。

## 上下文与缓存

| 声明 | 缓存位置 |
| --- | --- |
| 无作用域属性，或 `#[scope(singleton)]` | 调用方注入器上下文 |
| `#[scope(root)]` | 注入器树的根 |
| `#[scope(scoped)]` | 最近的 `create_scope()` 边界 |
| `#[scope(transient)]` | 不缓存 |

生成的方法：

- `new(import_instances_and_external_values...)`：创建新的根上下文和类型化提供者。
- 每个绑定名称都是 getter，同步绑定返回 `Result<Shared<T>, Error>`；最终为异步的绑定返回产出该结果的 future。
- `clone()`：共享上下文与提供者身份。
- `create_child()`：创建普通子上下文。
- `create_scope()`：创建隔离的作用域上下文。
- `in_context(&Injector)`：在给定上下文中复用这些提供者。
- `injector()`：返回模块当前上下文。

没有导入和外部输入的模块也实现 `Default`。

需要保持根实例共享时，应使用同一注入器树中的上下文。`in_context` 使用给定树的根缓存，不检查祖先关系。嵌套作用域隔离作用域实例。作用域内的普通子级复用该作用域的实例，同时保留自己的调用方上下文单例缓存。

生成的依赖连接代码持有类型化提供者句柄，并直接调用 `try_resolve_provider`；异步代码使用 `try_resolve_provider_async`。两者遵循运行时的初始化、并发、错误传播和重试规则。这些绑定不会注册到动态类型或名称注册表中，因此动态覆盖和 `dependency_graph()` 不会改变或描述静态依赖图。其余缓存与循环保护机制仍在运行时执行；本 API 不承诺零查找或零分配开销。

## 当前限制

导入的模块组成封闭的有向无环依赖图：来源模块不能反向依赖导入方。尚不支持自动发现、条件绑定、隐式选择可选依赖或集合、循环模块导入，以及生命周期启动与关闭排序。需要时，将可选值和集合显式作为外部输入传入。`Injectable` 仍是独立的运行时容器派生 API；静态组装尚不等同于完整 CDI。

`new`、`create_child`、`create_scope`、`in_context`、`injector`、`clone` 和 `default` 是保留绑定名称；与生成的隐藏导出方法重名的绑定也会被拒绝。生成的隐藏提供者和元数据辅助方法供内部组装使用，不属于应用 API。不支持模块泛型。模块和导入属性只支持文档属性；局部绑定还支持 `#[scope(...)]`。发布本 API 时，应同时发布核心 crate 和宏 crate。

将静态模块接入请求作用域的方法参见 [Web 集成](./web-integrations.md)。运行可执行示例：

```bash
cargo run -p fluxdi --example static_module --features macros
cargo run -p fluxdi --example static_composition_async --features macros,async-factory
```
