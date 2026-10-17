# Observability

FluxDI exposes stable observability identifiers in `fluxdi::observability`.

## Logging

Enable:

```toml
[dependencies]
fluxdi = { path = "../fluxdi", features = ["logging"] }
```

Initialize once at app startup:

```rust
fluxdi::try_init_logging().expect("failed to initialize logging");
```

`logging` reads `RUST_LOG` and defaults to `info` when unset.

For node-level diagnostics (registration, provider lookup, cache hit/miss, factory execution), set:

```bash
RUST_LOG=fluxdi=trace
```

## Tracing

Enable:

```toml
[dependencies]
fluxdi = { path = "../fluxdi", features = ["tracing"] }
```

## Metrics / Prometheus

Enable metrics:

```toml
[dependencies]
fluxdi = { path = "../fluxdi", features = ["metrics"] }
```

Prometheus export:

```toml
[dependencies]
fluxdi = { path = "../fluxdi", features = ["prometheus"] }
```

Then call:

- `Injector::metrics_snapshot()`
- `Injector::prometheus_metrics()`

## OpenTelemetry

Enable:

```toml
[dependencies]
fluxdi = { path = "../fluxdi", features = ["opentelemetry"] }
```

Helpers:

- `fluxdi::opentelemetry_layer(...)`
- `fluxdi::try_init_opentelemetry(...)`

The current integration uses OpenTelemetry 0.33 and tracing-opentelemetry 0.34.
Tracer types must implement `Tracer + Send + Sync + 'static`, with `T::Span: Send + Sync`.
The previous `PreSampledTracer` bound is no longer part of the upstream API.
The application owns tracer-provider/exporter installation and shutdown.
