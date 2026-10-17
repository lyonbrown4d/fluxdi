# 静态模块

FluxDI 2.0.0 的 `static_module!` 宏提供显式的类型化组装，支持按绑定选择模块导入、Rust 原生条件、可选参数、集合参数，以及同步和异步工厂。需要启用 `macros`；异步依赖图还要求 `async-factory`。核心仍然独立于 HTTP 框架。

```toml
fluxdi = { version = "2.0.0", features = ["macros"] }
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

同步工厂是函数路径，接收声明的依赖参数并返回 `Result<Shared<T>, Error>`。工厂在首次解析时按需执行。外部值具有根作用域。导入模块实例和外部值按两者在声明中出现的先后顺序传入构造方法。这些值构成显式边界：宏无法检查它们的构造过程。

## 编译期条件

模块、局部工厂绑定、外部输入、导入块和单个导入项均可使用[原生 `#[cfg(...)]` 与 `#[cfg_attr(...)]`](https://doc.rust-lang.org/reference/conditional-compilation.html)。Rust 先选择启用的声明，宏再解析并检查这些声明的依赖图。未启用的声明不提供类型、构造方法参数、依赖边、异步传播、作用域检查或集合成员。未启用的声明仍须保持 DSL 语法结构完整。条件遵循编译当前 crate 时的 Rust 配置；宏不读取进程环境变量来判断条件，也不求值运行时条件。

`cfg_attr` 可以添加所在位置支持的属性：`cfg`、文档属性、局部工厂的 `scope`，以及工厂、外部绑定和导入项的 `set`。它不能为外部绑定或导入绑定添加作用域。feature 选择可能改变 `new(...)` 的参数列表，以及 getter 推断出的同步或异步签名；签名有差异时，调用方须使用匹配的条件。

导出绑定的条件在定义它的 crate 中求值。来源 crate 禁用了某个导出后，导入该绑定仍会编译失败，使用 `optional(local_key)` 也不会跳过导入。应在消费方使用相应条件禁用导入。不支持自动判断来源导出是否可用的弱导入。

条件预处理会一起处理 token 相同的重复条件，并通过 Rust 原生宏展开再次调用宏。模块包含许多独立条件时仍受 Rust 的 `recursion_limit` 限制；必要时提高 crate 的限制。

## 可选参数和集合参数

工厂参数支持三种形式：

| 参数 | 工厂接收的类型 | 目标不存在时 |
| --- | --- | --- |
| `key` | `Shared<T>` | 编译错误 |
| `optional(key)` | `Option<Shared<T>>` | `None` |
| `all(group)` | `Vec<Shared<T>>` | 空向量 |

```rust
use fluxdi::{Error, Shared, static_module};

struct Config;
trait Plugin: Send + Sync {}
struct FirstPlugin;
struct SecondPlugin;
impl Plugin for FirstPlugin {}
impl Plugin for SecondPlugin {}
struct Service(Option<Shared<Config>>, Vec<Shared<dyn Plugin>>);

fn first_plugin() -> Result<Shared<dyn Plugin>, Error> {
    Ok(Shared::new(FirstPlugin))
}
fn second_plugin() -> Result<Shared<dyn Plugin>, Error> {
    Ok(Shared::new(SecondPlugin))
}
fn service(config: Option<Shared<Config>>, plugins: Vec<Shared<dyn Plugin>>)
    -> Result<Shared<Service>, Error> {
    Ok(Shared::new(Service(config, plugins)))
}

static_module! {
    struct App {
        #[cfg(feature = "debug")]
        external config: Config;
        #[scope(root)]
        #[set(plugins)]
        first: dyn Plugin = first_plugin();
        #[cfg(feature = "debug")]
        #[scope(root)]
        #[set(plugins)]
        second: dyn Plugin = second_plugin();
        service: Service = service(optional(config), all(plugins));
    }
}
```

`optional(key)` 允许目标未声明或被条件禁用，因此拼写错误也可能得到 `None`。已有绑定解析成功后返回 `Some`；工厂错误原样传播，可选参数不会隐藏初始化失败。

`#[set(group)]` 将绑定显式标记为局部集合的成员。工厂、外部绑定和单个导入项均可携带此属性。使用 `#[set(first, second)]` 或多个 `set` 属性可加入多个组；空属性和同一组内的重复成员标记会报错。导入成员必须在导入位置重新标记；来源模块的集合不会自动导入。导入成员仍保留其提供者身份和作用域。

`all(group)` 按声明顺序选择启用的成员，导入项位于其导入块的声明位置。它不会扫描类型或查询动态注册表。每次为消费方的参数构造向量，不会创建独立缓存的集合提供者；各成员仍遵循自身的缓存规则。依赖按工厂参数顺序和集合成员顺序依次解析。遇到首个错误后停止解析并传播错误，不调用消费方工厂。

Rust 检查参数类型并执行正常的类型强制转换，包括将兼容的具体类型 `Shared` 值用于 trait 对象参数。集合的所有成员都须符合工厂参数类型。不支持任意参数表达式或嵌套包装；名称为 `optional` 和 `all` 的裸绑定标识符仍可作为必需参数。

只有存在的可选目标和启用的集合成员会形成依赖边。这些边与必需依赖一样参与循环检查、根绑定捕获检查和异步推断。缺失的可选目标和空集合不增加依赖边。运行时容器与 `Injectable` 沿用原有集合契约，包括空运行时集合报错；静态参数的空向量行为不改变该契约。

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
fluxdi = { version = "2.0.0", features = ["macros", "async-factory"] }
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

条件选择完成后，宏展开会拒绝未知的必需绑定标识符、重复标识符、构造循环、无效作用域及生成方法的名称冲突。循环诊断包含依赖链。工厂参数或返回类型不兼容时，Rust 类型检查会失败。

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

导入的模块组成封闭的有向无环依赖图：来源模块不能反向依赖导入方。尚不支持自动发现、按类型隐式选择可选依赖或集合、循环模块导入，以及生命周期启动与关闭排序。`Injectable` 仍是独立的运行时容器派生 API；静态组装尚不等同于完整 CDI。

`new`、`create_child`、`create_scope`、`in_context`、`injector`、`clone` 和 `default` 是保留绑定名称；与生成的隐藏导出方法重名的绑定也会被拒绝。生成的隐藏提供者和元数据辅助方法供内部组装使用，不属于应用 API。不支持模块泛型。模块和导入块支持文档属性与原生条件属性；局部工厂还支持 `#[scope(...)]` 和 `#[set(...)]`。外部绑定与导入绑定支持 `#[set(...)]`，但不能覆盖来源作用域。核心与宏 crate 使用同一 2.0.0 版本边界；启用 `macros` 时，核心精确依赖 `fluxdi-macros` 2.0.0。

将静态模块接入请求作用域的方法参见 [Web 集成](./web-integrations.md)。运行可执行示例：

```bash
cargo run -p fluxdi --example static_module --features macros
cargo run -p fluxdi --example static_composition_async --features macros,async-factory
cargo run -p fluxdi --example static_conditionals --features macros
cargo run -p fluxdi --example static_conditionals --no-default-features --features macros
```

最后两条命令分别启用和禁用默认的 `debug` feature，展示可选配置和插件集合成员的变化。
