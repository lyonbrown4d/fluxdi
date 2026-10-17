# Web 集成

FluxDI 提供可选的 Axum（`axum`）和 Actix-web（`actix`）适配器。两者都会启用 `thread-safe`；核心注入器和静态模块不依赖任何一个框架。

## 请求中间件

在需要请求隔离的路由外围安装一次 `request_scope`。它为每个请求创建一个 `Injector::create_scope()` 上下文。`Resolved<T>` 使用该上下文，因此多个提取器与处理函数内的解析共享作用域实例。并发请求具有独立上下文。根绑定仍在整个注入器树中共享；调用方上下文单例在每个请求上下文中拥有独立缓存。嵌套安装会复用已有作用域。

没有该中间件时，`Resolved<T>` 继续从框架状态解析，不会为每个提取器隐式创建作用域。

### Axum

```rust
use axum::{Router, middleware, routing::get};
use fluxdi::{Injector, Provider, Shared};
use fluxdi::axum::{InjectorState, Resolved, request_scope};

struct Context;

async fn handler(Resolved(a): Resolved<Context>, Resolved(b): Resolved<Context>) -> String {
    Shared::ptr_eq(&a, &b).to_string()
}

let injector = Shared::new(Injector::root());
injector.provide(Provider::scoped(|_| Shared::new(Context)));
let state = InjectorState::new(injector);
let app: Router = Router::new()
    .route("/", get(handler))
    .layer(middleware::from_fn_with_state(state.clone(), request_scope))
    .with_state(state);
```

使用 `Extension<fluxdi::axum::RequestScope>` 获取上下文。其 `injector()` 返回克隆的注入器句柄，可用于命名绑定、集合或请求本地值。需要该句柄的中间件应放在 `request_scope` 下游，并考虑 Axum 的层执行顺序。

### Actix-web

```rust
use actix_web::{App, HttpResponse, middleware, web};
use fluxdi::{Injector, Provider, Shared};
use fluxdi::actix::{Resolved, injector_data, request_scope};

struct Context;

async fn handler(Resolved(a): Resolved<Context>, Resolved(b): Resolved<Context>) -> HttpResponse {
    HttpResponse::Ok().body(Shared::ptr_eq(&a, &b).to_string())
}

let injector = Shared::new(Injector::root());
injector.provide(Provider::scoped(|_| Shared::new(Context)));
let app = App::new()
    .app_data(injector_data(injector))
    .wrap(middleware::from_fn(request_scope))
    .route("/", web::get().to(handler));
```

中间件要求 `app_data` 中配置 `InjectorState`；缺少配置会产生服务器内部错误。导入 `actix_web::HttpMessage` 后，通过 `request.extensions().get::<fluxdi::actix::RequestScope>()` 访问当前上下文。在 await 前克隆作用域。需要该作用域的中间件必须在作用域安装后执行。

启用 `async-factory` 后，两个适配器均支持异步工厂。提供者错误仍会作为错误返回；仅当错误为 `AsyncFactoryRequiresAsyncResolve` 时，提取器才会通过异步解析重试。

## 资源生命周期

下游请求 future 运行期间，中间件持有作用域。请求完成、出错或被取消时释放该句柄。Actix 会从成功响应中移除其安装的扩展，并在请求归还池时清空请求扩展。显式保留请求、作用域、服务或注入器句柄可能延长资源存活时间。

请求处理结束指响应已经生成，不代表整个流式响应体已被消费。流式响应或派生任务必须捕获所需的服务或作用域句柄。丢弃作用域时，缓存按普通 `Arc` 所有权规则释放；每个请求没有自动异步关闭钩子。外部资源必须自行实现取消和清理行为。

避免根工厂动态捕获请求作用域服务：根缓存可能保留首个请求的值。静态依赖图会拒绝根绑定声明的上下文依赖；任意动态工厂仍由调用方负责。

## 请求作用域中的静态模块

静态模块在动态注册表之外保存类型化提供者。应显式使用请求上下文，而不是让 `Resolved<T>` 查找静态绑定：

```rust,ignore
// The middleware state must use this module's injector tree.
let di = InjectorState::new(Shared::new(app.injector().clone()));

// In an Axum handler with State(app) and Extension(scope):
let request_app = app.in_context(scope.injector().as_ref());
let service = request_app.service()?;
```

相同的 `in_context` 调用也适用于 Actix 请求作用域。组装 API 参见[静态模块](./static-modules.md)。集成测试 `fluxdi/tests/static_http.rs` 通过 Axum 流程验证这种连接方式。

## 相关示例

- `examples/axum`：`GET /scoped/debug` 中的中间件与两个提取器。
- `examples/actix`：`GET /hello/scoped` 中的中间件与两个提取器。
- `examples/axum-lifecycle`：应用生命周期集成。
- `examples/scoped-context`：独立于框架的上下文。
- `examples/dual-http-random-port`：服务生命周期。
