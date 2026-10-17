use super::*;

#[test]
fn test_instance_with_empty_struct() {
    #[derive(Debug, PartialEq)]
    struct Empty;

    let empty = Shared::new(Empty);
    let instance = Instance::new(empty);

    assert_eq!(*instance.get(), Empty);
}

#[test]
fn test_instance_with_large_struct() {
    #[derive(Debug)]
    struct LargeStruct {
        data: [u8; 1024],
    }

    let large = Shared::new(LargeStruct { data: [42; 1024] });

    let instance = Instance::new(large);

    assert_eq!(instance.get().data[0], 42);
    assert_eq!(instance.get().data[1023], 42);
}

#[test]
fn test_multiple_instances_same_shared() {
    let data = Shared::new(TestData {
        id: 777,
        name: "multi-instance".to_string(),
    });

    let instance1 = Instance::new(data.clone());
    let instance2 = Instance::new(data.clone());
    let instance3 = Instance::new(data.clone());

    // All instances point to the same allocation
    assert!(Shared::ptr_eq(&instance1.value(), &instance2.value()));
    assert!(Shared::ptr_eq(&instance2.value(), &instance3.value()));

    // Data is consistent across all instances
    assert_eq!(instance1.get().id, 777);
    assert_eq!(instance2.get().id, 777);
    assert_eq!(instance3.get().id, 777);
}

#[test]
fn test_instance_outlives_original_shared() {
    let instance = {
        let data = Shared::new(TestData {
            id: 888,
            name: "lifetime-test".to_string(),
        });
        Instance::new(data)
    }; // data is dropped here

    // Instance still holds valid reference
    assert_eq!(instance.get().id, 888);
    assert_eq!(instance.get().name, "lifetime-test");
}

#[test]
fn test_nested_instances() {
    #[derive(Debug)]
    struct Inner {
        value: u32,
    }

    #[cfg_attr(feature = "debug", derive(Debug))]
    struct Outer {
        inner_instance: Instance<Inner>,
    }

    let inner = Instance::new(Shared::new(Inner { value: 42 }));
    let outer = Instance::new(Shared::new(Outer {
        inner_instance: inner,
    }));

    assert_eq!(outer.get().inner_instance.get().value, 42);
}

#[cfg(feature = "debug")]
#[test]
fn test_instance_debug_format() {
    let data = Shared::new(TestData {
        id: 1,
        name: "debug-test".to_string(),
    });

    let instance = Instance::new(data);
    let debug_str = format!("{:?}", instance);

    // Should contain Instance in the debug output
    assert!(debug_str.contains("Instance"));
}

#[cfg(feature = "thread-safe")]
#[test]
fn test_instance_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}

    // Instance should be Send + Sync when T is Send + Sync
    assert_send_sync::<Instance<TestData>>();
}

#[cfg(feature = "thread-safe")]
#[test]
fn test_instance_can_be_shared_across_threads() {
    use std::sync::Arc;
    use std::thread;

    let instance = Arc::new(Instance::new(Shared::new(TestData {
        id: 123,
        name: "thread-test".to_string(),
    })));

    let handles: Vec<_> = (0..4)
        .map(|_| {
            let instance_clone = Arc::clone(&instance);
            thread::spawn(move || instance_clone.get().id)
        })
        .collect();

    for handle in handles {
        let result = handle.join().unwrap();
        assert_eq!(result, 123);
    }
}

#[cfg(feature = "thread-safe")]
#[test]
fn test_instance_value_can_be_sent_to_thread() {
    use std::thread;

    let instance = Instance::new(Shared::new(TestData {
        id: 456,
        name: "send-test".to_string(),
    }));

    let shared = instance.value();

    let handle = thread::spawn(move || shared.id);

    let result = handle.join().unwrap();
    assert_eq!(result, 456);
}

#[cfg(feature = "thread-safe")]
#[test]
fn test_multiple_threads_accessing_same_instance() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::thread;

    #[derive(Debug)]
    struct SharedCounter {
        value: AtomicU32,
    }

    let instance = Arc::new(Instance::new(Shared::new(SharedCounter {
        value: AtomicU32::new(0),
    })));

    let handles: Vec<_> = (0..10)
        .map(|_| {
            let instance_clone = Arc::clone(&instance);
            thread::spawn(move || {
                for _ in 0..100 {
                    instance_clone.get().value.fetch_add(1, Ordering::SeqCst);
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    let final_value = instance.get().value.load(Ordering::SeqCst);
    assert_eq!(final_value, 1000); // 10 threads * 100 increments
}

#[test]
fn test_realistic_dependency_injection_scenario() {
    #[derive(Debug)]
    struct Config {
        database_url: String,
        cache_ttl: u64,
    }

    #[derive(Debug)]
    struct Application {
        config: Shared<Config>,
    }

    impl Application {
        fn new(config: Shared<Config>) -> Self {
            Self { config }
        }

        fn get_database_url(&self) -> &str {
            &self.config.database_url
        }
    }

    // Simulate DI container resolving a config
    let config_instance = Instance::new(Shared::new(Config {
        database_url: "postgresql://prod".to_string(),
        cache_ttl: 3600,
    }));

    // Application uses the resolved config
    let app = Application::new(config_instance.value());

    assert_eq!(app.get_database_url(), "postgresql://prod");
    assert_eq!(app.config.cache_ttl, 3600);
}

#[test]
fn test_instance_in_collection() {
    let instances: Vec<Instance<TestData>> = vec![
        Instance::new(Shared::new(TestData {
            id: 1,
            name: "one".to_string(),
        })),
        Instance::new(Shared::new(TestData {
            id: 2,
            name: "two".to_string(),
        })),
        Instance::new(Shared::new(TestData {
            id: 3,
            name: "three".to_string(),
        })),
    ];

    assert_eq!(instances.len(), 3);
    assert_eq!(instances[0].get().id, 1);
    assert_eq!(instances[1].get().name, "two");
    assert_eq!(instances[2].get().id, 3);
}

#[test]
fn test_instance_with_option() {
    let some_instance = Some(Instance::new(Shared::new(TestData {
        id: 100,
        name: "optional".to_string(),
    })));

    let none_instance: Option<Instance<TestData>> = None;

    assert!(some_instance.is_some());
    assert!(none_instance.is_none());

    if let Some(instance) = some_instance {
        assert_eq!(instance.get().id, 100);
    }
}
