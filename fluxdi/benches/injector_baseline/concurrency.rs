use criterion::{BenchmarkId, Criterion, Throughput};
use fluxdi::{Injector, Provider, Shared};
use std::hint::black_box;
use std::sync::{
    Arc, Barrier,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

struct Workers {
    start: Arc<Barrier>,
    finish: Arc<Barrier>,
    iterations: Arc<AtomicU64>,
    stop: Arc<AtomicBool>,
    handles: Vec<JoinHandle<()>>,
}

impl Workers {
    fn new(count: usize, resolve: bool) -> Self {
        let injector = Arc::new(Injector::root());
        injector.provide::<u64>(Provider::root(|_| Shared::new(11)));
        injector.resolve::<u64>();
        let start = Arc::new(Barrier::new(count + 1));
        let finish = Arc::new(Barrier::new(count + 1));
        let iterations = Arc::new(AtomicU64::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let handles = (0..count)
            .map(|_| {
                let (start, finish, iterations, stop, injector) = (
                    start.clone(),
                    finish.clone(),
                    iterations.clone(),
                    stop.clone(),
                    injector.clone(),
                );
                thread::spawn(move || {
                    loop {
                        start.wait();
                        if stop.load(Ordering::Relaxed) {
                            break;
                        }
                        for index in 0..iterations.load(Ordering::Relaxed) {
                            if resolve {
                                black_box(*injector.resolve::<u64>());
                            } else {
                                black_box(index);
                            }
                        }
                        finish.wait();
                    }
                })
            })
            .collect();
        Self {
            start,
            finish,
            iterations,
            stop,
            handles,
        }
    }

    fn measure(&self, iterations: u64) -> Duration {
        self.iterations.store(iterations, Ordering::Relaxed);
        let started = Instant::now();
        self.start.wait();
        self.finish.wait();
        started.elapsed()
    }
}

impl Drop for Workers {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.start.wait();
        for handle in self.handles.drain(..) {
            handle.join().unwrap();
        }
    }
}

pub(super) fn bench(c: &mut Criterion) {
    let mut persistent = c.benchmark_group("concurrent_cached");
    for count in [2usize, 4, 8, 16, 32] {
        persistent.throughput(Throughput::Elements(count as u64));
        for (label, resolve) in [("resolve", true), ("worker_control", false)] {
            persistent.bench_with_input(BenchmarkId::new(label, count), &count, |b, &count| {
                // Spawn outside the timed batch and reuse workers throughout it.
                // The control uses the same barriers and loop without DI work.
                let workers = Workers::new(count, resolve);
                b.iter_custom(|iterations| workers.measure(iterations));
            });
        }
    }
    persistent.finish();

    let mut startup = c.benchmark_group("concurrent_thread_startup");
    for count in [2usize, 4, 8, 16, 32] {
        startup.throughput(Throughput::Elements((count * 64) as u64));
        for (label, resolve) in [("resolve_and_startup", true), ("startup_control", false)] {
            startup.bench_with_input(BenchmarkId::new(label, count), &count, |b, &count| {
                let injector = Arc::new(Injector::root());
                injector.provide::<u64>(Provider::root(|_| Shared::new(11)));
                injector.resolve::<u64>();
                b.iter(|| {
                    let start = Arc::new(Barrier::new(count));
                    let handles: Vec<_> = (0..count)
                        .map(|_| {
                            let (start, injector) = (start.clone(), injector.clone());
                            thread::spawn(move || {
                                start.wait();
                                for index in 0..64 {
                                    if resolve {
                                        black_box(*injector.resolve::<u64>());
                                    } else {
                                        black_box(index);
                                    }
                                }
                            })
                        })
                        .collect();
                    for handle in handles {
                        handle.join().unwrap();
                    }
                });
            });
        }
    }
    startup.finish();
}
