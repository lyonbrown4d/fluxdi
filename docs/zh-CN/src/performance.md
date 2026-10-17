# 性能

选择存储后端或更改缓存行为前，应使用仓库基准测试测量当前解析路径。较早实现的历史耗时不能作为当前上下文缓存和循环检测机制的基线。

## 基准测试覆盖范围

| 基准测试 | 计时内容 | 设置与解释 |
| --- | --- | --- |
| `resolve_first` | 首次根解析、工厂分配与缓存初始化 | 注入器创建和提供者注册不在计时流程内；状态保留至输出销毁 |
| `resolve_cached` | 已预热的根绑定 | 包含提供者查找、上下文与缓存查找、解析保护及共享值克隆 |
| `resolve_transient` | 每次调用中的解析与构造 | 无实例缓存 |
| `provider_registration` | 注入器构造与一次注册 | 未将注册分配与注入器设置分开测量 |
| `concurrent_cached/resolve/N` | N 个持久 worker 上的缓存解析 | worker 在计时批次外创建，并在各轮迭代中复用；每次 Criterion 迭代中，每个 worker 解析一次 |
| `concurrent_cached/worker_control/N` | 相同的 worker 循环与批次屏障，不使用 DI | 用于对照测试框架开销；调度与竞争仍会影响解析样本 |
| `concurrent_thread_startup/resolve_and_startup/N` | 创建 N 个线程、同步、每个 worker 解析 64 次并等待线程结束 | 包含线程生命周期与调度 |
| `concurrent_thread_startup/startup_control/N` | 相同的线程生命周期，每个 worker 执行 64 次 black-box 操作 | 展示组合样本中可能来自测试框架的开销 |
| `resolve_with_noop_decorator`、`resolve_decorator/*` | 带有或不带装饰器的已预热缓存绑定 | 装饰发生在初始化期间，因此这些测试不测量装饰器执行成本 |

对于 `concurrent_cached`，报告时间等于批次总耗时除以迭代次数，不是单次调用延迟。吞吐量计入每轮迭代的全部 N 次解析。线程启动测试组每轮计入 N × 64 次操作。旧的 `resolve_concurrent` 组包含线程启动，不能将其结果与持久 worker 的缓存解析结果直接比较。

## 复现

在仓库根目录使用固定的 Rust 工具链运行：

```bash
cargo bench -p fluxdi --bench injector_baseline -- --save-baseline local
cargo bench -p fluxdi --bench injector_baseline --features thread-safe -- --save-baseline thread-safe
cargo bench -p fluxdi --bench injector_baseline --features lock-free -- --save-baseline dashmap
```

前后对比应使用相同的编译器、机器、feature 集合和基准测试组。不同 feature 集合的命名基线应分开保存。Criterion 将报告写入所选 Cargo target 目录下的 `criterion` 文件夹。`lock-free` 是 DashMap 存储的历史 feature 名称；该存储使用分片锁。

用于检查测量设置的短时间运行：

```bash
cargo bench -p fluxdi --bench injector_baseline --features thread-safe -- \
  --warm-up-time 0.1 --measurement-time 0.2 --sample-size 10
```

## 第三批基线

2026-10-17 的一次本地短时间运行使用 Rust 1.99.1、Darwin arm64、优化的 bench profile，以及默认 feature 加 `thread-safe`。下表是 Criterion 在 10 个样本、100 ms 预热和 200 ms 测量条件下输出的区间。这些结果用于检查新的测试框架，不构成可跨环境使用的性能保证。

这些测量在依赖更新前使用 Criterion 0.5.1 完成。当前工作区使用 Criterion 0.8.2；与升级后的依赖图进行性能比较前，应重新运行基准测试。

| 测量项 | 报告区间 |
| --- | --- |
| 缓存根解析 | 86.440–86.841 ns |
| 首次根解析 | 209.10–211.84 ns |
| 瞬态解析 | 93.472–96.147 ns |
| 持久 worker 缓存解析，2 个 worker | 5.5642–5.6233 百万次解析/秒 |
| 持久 worker 缓存解析，32 个 worker | 2.6545–2.9546 百万次解析/秒 |
| 启动加解析，2 个 worker | 每批 82.213–114.15 µs |
| 启动对照，2 个 worker | 每批 66.497–89.921 µs |

组合启动测量包含大量测试框架开销和噪声。持久 worker 的短时间运行也表明，增加 worker 不一定提高总吞吐量。将结果归因于某个映射结构或保护机制前，应先分析竞争；选择优化前，应在目标负载下使用更长的采样时间重复测量。

## 后续评估

- **注册后冻结**：依赖图快照目前不阻止修改。选择冻结后的存储形式前，冻结 API 必须定义何时将注册和覆盖视为错误，以及子容器如何继承该状态。
- **生命周期依赖验证**：依赖图元数据目前描述提供者类型、绑定、作用域和依赖提示，不记录提供者所属模块或启动顺序。因此，模块生命周期验证首先需要明确归属契约。任意工厂仍需手动提示。
- **内存**：提供者映射、依赖图元数据、每个上下文的初始化单元、共享指针与保留实例都会占用内存。需要测量不同容器深度、作用域数量和覆盖操作下的堆内存；仅使用 `size_of` 会漏掉这些分配。
- **解析性能**：使用分开的首次、缓存和并发测量及性能分析来选择具体改动。本批调整测量覆盖范围和宏能力，没有引入冻结容器，也不声称解析路径变快。
