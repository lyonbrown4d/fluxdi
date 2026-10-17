# 更新日志

[English](CHANGELOG.md) | [简体中文](CHANGELOG.zh-CN.md)

本文记录 FluxDI 的所有重要变更。

本文格式基于 [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)，
本项目遵循 [语义化版本](https://semver.org/spec/v2.0.0.html)。

## [Unreleased]

### 新增

- 静态模块支持显式选择导出绑定及设置别名，重导出时保留提供者身份、作用域安全性和异步元数据
- 启用 `macros,async-factory` 后支持静态异步工厂和沿依赖传递的异步 getter，按需初始化并沿用可失败工厂的重试和取消行为
- 新增 `static_composition_async` 可运行示例，展示导入和异步组装

- 提供英文和简体中文版本的书籍、README，以及保留章节对应关系的语言导航；CI 会构建两种语言的书籍并检查章节对应关系

- 新增 `static_module!`，支持显式同步依赖图、外部输入、带类型的 Provider 组装和编译期缺失、重复、循环检查，并拒绝捕获上下文绑定的根绑定
- 新增带类型的 `Injector::try_resolve_provider` 及其异步版本，复用现有上下文缓存和初始化流程，且无需查询注册表
- 新增可选启用的 Axum 和 Actix `request_scope` 中间件及 `RequestScope` 句柄；提取器共享请求上下文，Actix 支持异步工厂
- 新增静态组装示例和编译失败文档测试，以及 HTTP 流水线隔离和取消场景回归测试

- `Injectable` 支持类型、生命周期和 const 泛型以及现有 `where` 子句；支持使用 `#[inject(name = "...")]` 指定命名字段、可选依赖和集合依赖
- 生成 `provider(scope)` 和 `with_injected_dependencies(provider)` 方法以附加推断出的依赖图提示；继续支持原有构造函数
- 新增可失败的可选依赖解析辅助方法，覆盖同步/异步的未命名和命名绑定，并保留初始化错误
- 新增可选依赖图提示和 `DependencyCardinality::Optional`；缺失的可选目标有效，已注册的目标仍会参与循环检测
- 新增首次解析、持久工作线程缓存解析以及线程启动/控制基准测试
- 新增可失败的 `Provider::try_root`、`try_singleton`、`try_scoped`、`try_transient` 及对应的异步构造函数，返回 `Result<Shared<T>, Error>`
- 新增 `Injectable::try_from_injector`，支持可恢复的依赖解析；现有 `from_injector` 构造函数在出错时仍会 panic
- 新增 `Error::factory_failed` 和 `ErrorKind::FactoryFailed`，用于表示应用初始化错误；依赖错误保留原有类型和消息
- 新增性能文档（`docs/src/performance.md`），说明功能影响和基准测试命令
- **Epic H（续）**：新增 `BootstrapOptions::with_parallel_start()`，支持并发执行模块的 `on_start`
- 新增启动生命周期错误聚合：并行启动期间多个模块失败时返回 `Error::bootstrap_aggregate()`
- 新增回滚策略：启动失败时，对已经启动的模块调用 `on_stop`（适用于顺序和并行启动）
- 新增优雅关闭超时：使用 `ShutdownOptions::with_timeout` 后，会在时间预算内尝试调用所有模块的 `on_stop`，不会中途终止

### 变更

- 所有直接依赖的注册表均在工作区依赖声明中使用最新稳定快照，包括 Syn 3、Criterion 0.8、Rand 0.10，以及 OpenTelemetry 0.33 / tracing-opentelemetry 0.34 API 更新
- SeaORM 2 的 SQLite 示例现在拥有独立的工作区和锁文件，可与最新版本的 rusqlite 共存；CI 会检查并审计两套原生依赖图
- 修复根目录 README：此前该文件被保存为无效符号链接

- 更新存在 RustSec 问题的兼容锁定依赖，包括 Crossbeam、Rand、Rustls 和 WebPKI；通过更新 SeaORM/sea-bae 移除了无人维护的 `proc-macro-error2` 依赖链，并更新了已撤回的 Spin 版本
- 更新锁定的 Actix 依赖；其剩余的 `h2 0.3` 公告已记录在 `docs/src/dependency-audit.md`，并继续由 cargo-deny 显示

- 下游对 `DependencyCardinality` 的穷尽匹配必须处理 `Optional`；派生类型必须为生成的 `provider` 和 `with_injected_dependencies` 方法名保留名称
- 扩展宏要求使用更新后的核心 API。发布前，请协调两个 crate 的主版本发布，以免依赖 `^1.x` 宏版本的旧版核心选择到不兼容的生成代码
- 并发基准测试组现在明确区分持久工作线程和每轮启动线程；旧的合并测量结果不应与缓存解析耗时进行比较
- 所有工作区软件包的最低支持 Rust 版本和锁定的开发/CI 基线现为 1.99.1
- 新增 `ErrorKind::FactoryFailed` 后，下游对 `ErrorKind` 的穷尽匹配必须处理这个新变体
- 可失败工厂只缓存成功实例；初始化失败会释放资源许可，并允许后续调用方或正在等待的调用方重试
- `Provider::singleton` 现在按调用方 Injector 上下文缓存；克隆实例共享该上下文，而子 Injector 和新建的运行时作用域拥有各自的 singleton 缓存
- Scoped 实例归最近的运行时作用域所有；嵌套作用域隔离各自的 scoped 缓存，普通子 Injector 则共享最近的作用域

### 修复

- 修复命名、集合和普通绑定中的并发初始化、异步循环检测，以及取消/panic 后的清理
- 修复配置/启动失败后的生命周期回滚，以及启动流程取消后的清理
- 修复异步装饰器、共享同步/异步资源限制，以及无默认功能时的编译

## [1.2.2] - 2026-03-21

### 新增

- **Epic H（部分完成）**：新增 `BootstrapOptions` 和 `ShutdownOptions`，用于控制生命周期
- 通过 `lifecycle` 功能支持可选的启动和关闭超时
- 新增 `Application::bootstrap_with_options()` 和 `Application::shutdown_with_options()`
- 新增结构化关闭错误聚合：多个模块的 `on_stop()` 失败时，仍会尝试调用所有模块，并返回一个聚合错误
- 新增 `Error::shutdown_aggregate()`，用于合并多个关闭失败
- 新增长时间运行服务器的生产模式文档（`docs/src/production-patterns.md`）
- **Epic I**：新增 `Provider::with_decorator()`，用于包装服务（日志、缓存、重试）
- 新增 `examples/decorator`，展示日志和缓存装饰器
- 新增装饰器文档 `docs/src/decorators.md`
- 新增 `bench_resolve_with_decorator` 和 `bench_resolve_decorator_baseline` 基准测试，用于衡量装饰器开销

## [1.2.1] - 2026-03-21

### 变更

- 将 README、mdBook 和示例中的文档版本统一更新为 1.2.1
- 修复 `fluxdi/README.md` 中的编码问题（章节标题中的 emoji 已损坏）

### 新增

- **Epic A 完成**：新增宏与手动 Provider 等效性的文档说明
- 新增 `examples/injectable-macro`，展示如何使用 `#[derive(Injectable)]` 替代手动 Provider 闭包
- 在 `docs/derive-macros.md` 中新增「手动与宏的等效性」章节，提供并列对比
- 在主 README 和 `docs/examples.md` 中新增 Injectable 宏示例条目

### 修复

- 修复 `## Examples`、`## Usage Guide`、`### Architectural Patterns`、`### Developer Experience` 章节标题的显示问题

## [1.2.0]

### 新增

- 新增 `Module::configure()` 作为首选注册钩子（返回 `Result<(), Error>`）
- 通过 `fluxdi-macros` crate 新增 `#[derive(Injectable)]`（`macros` 功能）
- 新增命名绑定（`provide_named`、`resolve_named`），支持为同一 trait 提供多个实现
- 新增多重绑定（`provide_into_set`、`resolve_all`），用于插件流水线
- 新增依赖图工具（`dependency_graph()`、`validate_graph()`、DOT/Mermaid 导出）
- 新增作用域上下文（`create_scope()`、`Provider::scoped`）
- 新增测试覆盖 API（`try_override_provider`）
- 为 `ServiceNotProvided`、`AsyncFactoryRequiresAsyncResolve` 和循环依赖新增错误建议
- 新增 Axum 和 Actix-web 集成提取器（`Resolved<T>`）
- 新增支持异步的模块生命周期钩子（`on_start`、`on_stop`）
- 新增 `Application::bootstrap()` 和 `Application::shutdown()`，用于异步生命周期

## [1.1.0]

### 新增

- 首次发布，提供核心依赖注入功能
- 提供 transient、singleton（root）和 module-scoped 生命周期
- 提供 Injector、Provider、Shared 类型
- 提供支持导入的模块系统
- 提供循环依赖检测
- 提供资源限制（Policy、Limits）
- 支持异步工厂
- 提供指标和 Prometheus 导出
- 集成日志和 tracing

[Unreleased]: https://github.com/DaiYuANg/fluxdi/compare/v1.2.2...HEAD
[1.2.2]: https://github.com/DaiYuANg/fluxdi/compare/v1.2.1...v1.2.2
[1.2.1]: https://github.com/DaiYuANg/fluxdi/compare/v1.2.0...v1.2.1
[1.2.0]: https://github.com/DaiYuANg/fluxdi/compare/v1.1.0...v1.2.0
[1.1.0]: https://github.com/DaiYuANg/fluxdi/releases/tag/v1.1.0
