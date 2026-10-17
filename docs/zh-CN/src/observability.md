# 可观测性

FluxDI 在 `fluxdi::observability` 中提供稳定的可观测性标识符。

## 日志

启用：

```toml
[dependencies]
fluxdi = { path = "../fluxdi", features = ["logging"] }
```

在应用启动时初始化一次：

```rust
fluxdi::try_init_logging().expect("failed to initialize logging");
```

`logging` 读取 `RUST_LOG`，未设置时默认为 `info`。

需要节点级诊断（注册、提供者查找、缓存命中或未命中、工厂执行）时，设置：

```bash
RUST_LOG=fluxdi=trace
```

## 链路追踪

启用：

```toml
[dependencies]
fluxdi = { path = "../fluxdi", features = ["tracing"] }
```

## 指标与 Prometheus

启用指标：

```toml
[dependencies]
fluxdi = { path = "../fluxdi", features = ["metrics"] }
```

导出 Prometheus 指标：

```toml
[dependencies]
fluxdi = { path = "../fluxdi", features = ["prometheus"] }
```

然后调用：

- `Injector::metrics_snapshot()`
- `Injector::prometheus_metrics()`

## OpenTelemetry

启用：

```toml
[dependencies]
fluxdi = { path = "../fluxdi", features = ["opentelemetry"] }
```

辅助函数：

- `fluxdi::opentelemetry_layer(...)`
- `fluxdi::try_init_opentelemetry(...)`

当前集成使用 OpenTelemetry 0.33 和 tracing-opentelemetry 0.34。追踪器类型必须实现 `Tracer + Send + Sync + 'static`，且 `T::Span: Send + Sync`。上游 API 不再包含之前的 `PreSampledTracer` 约束。应用负责追踪器提供者与导出器的安装和关闭。
