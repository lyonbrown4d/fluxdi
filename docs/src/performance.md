# Performance

This document covers FluxDI performance characteristics, benchmarks, and optimization options.

## Feature Impact

| Feature | Resolve (cached) | Registration | Notes |
|---------|------------------|--------------|-------|
| Default (Rc) | ~12 ns | ~150 ns | Fastest single-thread, not Send+Sync |
| `thread-safe` | ~35 ns | ~325 ns | Arc + RwLock; required for multi-thread |
| `lock-free` | ~35 ns | ~3.8 µs | DashMap; better under high concurrency |

## When to Use lock-free

Enable `lock-free` when:

- Many threads resolve from the same injector concurrently (e.g. 16+ workers)
- Read-heavy workload; registration is infrequent (at startup)

Avoid `lock-free` when:

- Single-threaded or low concurrency
- Registration happens frequently

```toml
[dependencies]
fluxdi = { version = "1.2", features = ["thread-safe", "lock-free"] }
```

## Benchmark Commands

Run the baseline benchmarks:

```bash
# Default features (Rc, single-thread)
cargo bench -p fluxdi

# Thread-safe (Arc + RwLock)
cargo bench -p fluxdi --features thread-safe

# Sharded locking (DashMap, legacy feature name: lock-free)
cargo bench -p fluxdi --features "thread-safe,lock-free"
```

## Resolve Path Overview

- **Cached resolve**: Single map lookup + Arc clone; typically 20–36 ns.
- **Transient resolve**: Provider lookup + factory call + allocation; ~100–150 ns.
- **Decorator overhead**: No-op decorator adds &lt;2 ns to cached path.

## Zero-Cost Abstractions

FluxDI uses feature gates so that:

- `tracing`, `metrics`, `logging` add no cost when disabled
- `thread-safe` is optional; default uses `Rc` for lower overhead
- `lock-free` is opt-in for high-concurrency scenarios

## Further Reading

- [LOCK_FREE_OPERATIONS.md](../../roadmaps/LOCK_FREE_OPERATIONS.md) — Lock-contention reduction plan and baseline results
