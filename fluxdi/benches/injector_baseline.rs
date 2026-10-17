use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use fluxdi::{Injector, Provider, Shared};
use std::hint::black_box;

#[cfg(feature = "thread-safe")]
#[path = "injector_baseline/concurrency.rs"]
mod concurrency;

#[derive(Debug)]
struct CachedService(u64);

#[derive(Debug)]
struct TransientService(u64);

fn bench_cached_resolve(c: &mut Criterion) {
    let injector = Injector::root();
    injector.provide::<CachedService>(Provider::root(|_| Shared::new(CachedService(42))));
    let _ = injector.resolve::<CachedService>();

    c.bench_function("resolve_cached", |b| {
        b.iter(|| {
            let value = injector.resolve::<CachedService>();
            black_box(value.0)
        });
    });
}

fn bench_transient_resolve(c: &mut Criterion) {
    let injector = Injector::root();
    injector.provide::<TransientService>(Provider::transient(|_| Shared::new(TransientService(7))));

    c.bench_function("resolve_transient", |b| {
        b.iter(|| {
            let value = injector.resolve::<TransientService>();
            black_box(value.0)
        });
    });
}

fn bench_first_resolve(c: &mut Criterion) {
    c.bench_function("resolve_first", |b| {
        b.iter_batched(
            || {
                let injector = Injector::root();
                injector
                    .provide::<CachedService>(Provider::root(|_| Shared::new(CachedService(42))));
                injector
            },
            |injector| {
                let value = injector.resolve::<CachedService>();
                // Keep setup state alive until Criterion drops the output,
                // outside the measured routine.
                black_box((injector, value))
            },
            BatchSize::SmallInput,
        );
    });
}

#[cfg(feature = "thread-safe")]
fn bench_concurrent_resolve(c: &mut Criterion) {
    concurrency::bench(c);
}

#[cfg(not(feature = "thread-safe"))]
fn bench_concurrent_resolve(_c: &mut Criterion) {}

fn bench_provider_registration(c: &mut Criterion) {
    c.bench_function("provider_registration", |b| {
        b.iter(|| {
            let injector = Injector::root();
            injector.provide::<u64>(Provider::transient(|_| Shared::new(1u64)));
            black_box(injector);
        });
    });
}

fn bench_resolve_with_decorator(c: &mut Criterion) {
    let injector = Injector::root();
    injector.provide::<CachedService>(
        Provider::root(|_| Shared::new(CachedService(42))).with_decorator(|inner| inner),
    );
    let _ = injector.resolve::<CachedService>();

    c.bench_function("resolve_with_noop_decorator", |b| {
        b.iter(|| {
            let value = injector.resolve::<CachedService>();
            black_box(value.0)
        });
    });
}

fn bench_resolve_decorator_baseline(c: &mut Criterion) {
    let mut group = c.benchmark_group("resolve_decorator");
    group.bench_function("without_decorator", |b| {
        let injector = Injector::root();
        injector.provide::<CachedService>(Provider::root(|_| Shared::new(CachedService(42))));
        let _ = injector.resolve::<CachedService>();
        b.iter(|| {
            let value = injector.resolve::<CachedService>();
            black_box(value.0)
        });
    });
    group.bench_function("with_noop_decorator", |b| {
        let injector = Injector::root();
        injector.provide::<CachedService>(
            Provider::root(|_| Shared::new(CachedService(42))).with_decorator(|inner| inner),
        );
        let _ = injector.resolve::<CachedService>();
        b.iter(|| {
            let value = injector.resolve::<CachedService>();
            black_box(value.0)
        });
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_cached_resolve,
    bench_first_resolve,
    bench_transient_resolve,
    bench_concurrent_resolve,
    bench_provider_registration,
    bench_resolve_with_decorator,
    bench_resolve_decorator_baseline
);
criterion_main!(benches);
