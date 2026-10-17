# 迭代规划

本规划基于仓库审查结果。每批先完成实现和相应回归测试，再进入下一批；各批的验收状态以实际执行结果为准。

## 已确定的作用域语义

`Provider::singleton()` 按发起解析的 injector 上下文缓存。克隆句柄共享上下文，子容器和新建 runtime scope 使用各自的 singleton 缓存。`Provider::root()` 在整棵容器树内共享；`Provider::scoped()` 归属于最近的 runtime scope。嵌套 scope 不继承外层 scoped 实例，scope 内的普通子容器共享最近 scope 的 scoped 实例。

## 第一批：正确性修复

状态：已完成代码验收，已提交并推送（`2e61040`）。

- 修复异步循环检测的误报、线程迁移和并发初始化等待环。
- 合并同一上下文、同一绑定的并发首次初始化，支持取消或 panic 后重试。
- 按有效绑定和作用域定位缓存，保留命名绑定和集合绑定的独立身份。
- 修复启动配置失败、超时和取消后的清理，恢复旧异步注册钩子的兼容调用。
- 修复关闭默认 feature 的编译错误、异步装饰器遗漏和同步/异步限流额度分离。
- 增加行为回归测试和 CI feature 组合测试，同步对外说明。

本批本地验收：workspace 全功能测试共 224 项通过；workspace 全目标、全功能 Clippy 和关闭默认 feature 的 Clippy 通过；默认、关闭默认、单线程异步、线程安全、DashMap、异步限流等组合测试通过。格式和修改过的 Markdown 本地链接检查通过。环境未安装 mdBook，未执行文档站点构建。

旧 `complex` 示例的两个仓储 trait 对 `async_trait` 生成代码的 `double_must_use` lint 做了局部兼容处理。后续更新该依赖时应复核是否可以删除这些局部标注；workspace 的 Clippy 命令和告警门槛保持不变。

## 第二批：可返回错误的工厂和构造

状态：已完成实现与本地验收。

- 新增四种作用域的 `Provider::try_*` 同步工厂及对应异步工厂，返回 `Result<Shared<T>, Error>`。
- derive 新增 `try_from_injector`，保留原有 `from_injector` 的错误时 panic 行为。
- 工厂错误保留原始类别和消息；仅缓存成功结果。失败后释放创建额度，允许后续调用或等待中的调用重试；装饰器只处理成功结果。
- 新增 `ErrorKind::FactoryFailed` 和错误构造辅助方法。下游对 `ErrorKind` 做穷尽匹配时，需要处理新增变体。
- 全部 19 个 workspace 包的最低 Rust 要求更新为 1.99.1；开发工具链和 CI 基线固定到该版本，CI 保留 beta/nightly 测试。

本批本地验收：workspace 全功能测试共 234 项通过；CI 对应的 8 个 feature 组合全部通过；workspace 全目标、全功能 Clippy 和关闭默认 feature 的 Clippy 通过；Rust API 文档构建、格式与文档本地链接检查通过。环境未安装 mdBook，未执行文档站点构建。

用法与失败重试规则见[可返回错误的工厂](./fallible-factories.md)。

## 第三批：宏能力与后续优化

状态：待实施。

扩展 derive 对泛型、命名绑定、可选依赖和集合依赖的支持，并从构造声明生成依赖图元数据。具体宏语法在本批开始时确定。

后续评估构建后冻结容器、生命周期依赖校验、内存占用和性能优化。优化前先建立能区分初始化、缓存命中和线程调度开销的基准。

## 范围调整

HTTP 请求作用域不纳入本轮迭代；现有通用 injector scope API 保留。
