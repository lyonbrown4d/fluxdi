# Performance

Use the repository benchmarks to measure the current resolve path before choosing a storage backend or changing cache behavior. Historical timings from earlier implementations are not a baseline for the current contextual cache and cycle detection.

## Benchmark Coverage

| Benchmark | Timed work | Setup and interpretation |
| --- | --- | --- |
| `resolve_first` | First root resolution, factory allocation, and cache initialization | Injector creation and provider registration are outside the timed routine; state is retained until output teardown |
| `resolve_cached` | A warmed root binding | Includes provider lookup, context/cache lookup, resolution guards, and shared-value cloning |
| `resolve_transient` | Resolution and construction on each call | No instance cache |
| `provider_registration` | Injector construction and one registration | Does not isolate registration allocation from injector setup |
| `concurrent_cached/resolve/N` | Cached resolution on N persistent workers | Workers are created outside the timed batch and reused for its iterations; each Criterion iteration performs one resolution per worker |
| `concurrent_cached/worker_control/N` | The same worker loop and batch barriers without DI | Control for the harness; scheduling and contention still affect the resolve sample |
| `concurrent_thread_startup/resolve_and_startup/N` | Create N threads, synchronize, resolve 64 times per worker, and join | Includes thread lifecycle and scheduling |
| `concurrent_thread_startup/startup_control/N` | The same thread lifecycle with 64 black-box operations per worker | Illustrates how much of the combined sample can come from the harness |
| `resolve_with_noop_decorator`, `resolve_decorator/*` | Warmed cached bindings with or without decorators | Decoration happens during initialization, so these do not measure decorator execution cost |

For `concurrent_cached`, reported time is elapsed batch duration divided by iterations, rather than the latency of one individual call. Throughput counts all N resolutions per iteration. Thread-startup groups count N × 64 operations per iteration. The old `resolve_concurrent` group included thread startup; its results should not be compared directly with persistent-worker cached results.

## Reproduce

Run from the repository root with the pinned Rust toolchain:

```bash
cargo bench -p fluxdi --bench injector_baseline -- --save-baseline local
cargo bench -p fluxdi --bench injector_baseline --features thread-safe -- --save-baseline thread-safe
cargo bench -p fluxdi --bench injector_baseline --features lock-free -- --save-baseline dashmap
```

Use the same compiler, machine, feature set, and benchmark group for before/after comparisons. Keep named baselines separate across feature sets. Criterion writes reports under the selected Cargo target directory's `criterion` folder. `lock-free` is the historical feature name for DashMap storage, which uses sharded locking.

A short run for checking the measurement setup:

```bash
cargo bench -p fluxdi --bench injector_baseline --features thread-safe -- \
  --warm-up-time 0.1 --measurement-time 0.2 --sample-size 10
```

## Third-Batch Baseline

A short local run on 2026-10-17 used Rust 1.99.1, Darwin arm64, the optimized bench profile, and default + `thread-safe` features. These are the ranges printed by Criterion with 10 samples, 100 ms warm-up, and 200 ms measurement. They are a check of the new harness, not a portable performance guarantee.

| Measurement | Reported range |
| --- | --- |
| Cached root resolution | 86.440–86.841 ns |
| First root resolution | 209.10–211.84 ns |
| Transient resolution | 93.472–96.147 ns |
| Persistent cached resolution, 2 workers | 5.5642–5.6233 million resolutions/s |
| Persistent cached resolution, 32 workers | 2.6545–2.9546 million resolutions/s |
| Startup plus resolution, 2 workers | 82.213–114.15 µs per batch |
| Startup control, 2 workers | 66.497–89.921 µs per batch |

The combined startup measurement contains substantial harness cost and noise. The short persistent-worker run also shows that more workers do not necessarily increase aggregate throughput. Profile contention before attributing the result to a particular map or guard; repeat with longer samples on the target workload before choosing an optimization.

## Follow-Up Evaluation

- **Freeze after registration:** graph snapshots currently do not prevent mutation. A freeze API must define when registration and overrides become errors and how child containers inherit that state before a frozen storage representation is chosen.
- **Lifecycle dependency validation:** graph metadata currently describes provider types, bindings, scopes, and dependency hints. It does not record which module owns a provider or startup order, so module lifecycle validation requires an ownership contract first. Arbitrary factories still need manual hints.
- **Memory:** provider maps, graph metadata, per-context initialization cells, shared pointers, and retained instances all contribute. Heap measurements across container depth, scope count, and overrides are needed; `size_of` alone would omit those allocations.
- **Resolve performance:** use the separated first/cached/concurrent measurements and profiling to choose a concrete change. This batch changes measurement coverage and macro capabilities; it does not introduce a frozen container or claim a faster resolve path.
