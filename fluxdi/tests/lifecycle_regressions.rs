use fluxdi::{
    Application, BootstrapOptions, Error, Injector, Module, ModuleLifecycleFuture, Provider, Shared,
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

struct StartedImport(Arc<AtomicUsize>);
impl Module for StartedImport {
    fn on_stop(&self, _: Shared<Injector>) -> ModuleLifecycleFuture {
        self.0.fetch_add(1, Ordering::SeqCst);
        Box::pin(async { Ok(()) })
    }
}
struct ConfigureFailure(Arc<AtomicUsize>);
impl Module for ConfigureFailure {
    fn imports(&self) -> Vec<Box<dyn Module>> {
        vec![Box::new(StartedImport(self.0.clone()))]
    }
    fn configure(&self, _: &Injector) -> Result<(), Error> {
        Err(Error::module_lifecycle_failed(
            "root",
            "configure",
            "failure",
        ))
    }
}
#[tokio::test]
async fn configure_failure_rolls_back_imports_and_does_not_mark_bootstrapped() {
    let stops = Arc::new(AtomicUsize::new(0));
    let mut app = Application::new(ConfigureFailure(stops.clone()));
    assert!(app.bootstrap().await.is_err());
    assert!(!app.is_bootstrapped());
    app.shutdown().await.unwrap();
    assert_eq!(stops.load(Ordering::SeqCst), 1);
}
struct PendingRoot(Arc<AtomicUsize>);
impl Module for PendingRoot {
    fn imports(&self) -> Vec<Box<dyn Module>> {
        vec![Box::new(StartedImport(self.0.clone()))]
    }
    fn on_start(&self, _: Shared<Injector>) -> ModuleLifecycleFuture {
        Box::pin(std::future::pending())
    }
}
#[cfg(feature = "lifecycle")]
#[tokio::test]
async fn startup_timeout_rolls_back_successful_modules_in_both_start_modes() {
    for parallel in [false, true] {
        let stops = Arc::new(AtomicUsize::new(0));
        let mut app = Application::new(PendingRoot(stops.clone()));
        let opts = BootstrapOptions::default()
            .with_parallel_start(parallel)
            .with_timeout(std::time::Duration::from_millis(10));
        assert!(app.bootstrap_with_options(opts).await.is_err());
        assert!(!app.is_bootstrapped());
        app.shutdown().await.unwrap();
        assert_eq!(stops.load(Ordering::SeqCst), 1);
    }
}
#[test]
fn canceled_bootstrap_keeps_started_modules_available_for_shutdown() {
    use std::{future::Future, task::Context};
    for parallel in [false, true] {
        let stops = Arc::new(AtomicUsize::new(0));
        let mut app = Application::new(PendingRoot(stops.clone()));
        let mut startup = Box::pin(
            app.bootstrap_with_options(BootstrapOptions::default().with_parallel_start(parallel)),
        );
        let waker = futures::task::noop_waker();
        assert!(
            startup
                .as_mut()
                .poll(&mut Context::from_waker(&waker))
                .is_pending()
        );
        drop(startup);
        futures::executor::block_on(app.shutdown()).unwrap();
        assert_eq!(stops.load(Ordering::SeqCst), 1);
    }
}
struct LegacyModule;
impl Module for LegacyModule {
    fn providers_async(&self, injector: Shared<Injector>) -> ModuleLifecycleFuture {
        Box::pin(async move {
            injector.provide::<u32>(Provider::root(|_| Shared::new(42)));
            Ok(())
        })
    }
}
#[tokio::test]
async fn async_bootstrap_invokes_legacy_registration_override_in_both_modes() {
    for parallel in [false, true] {
        let mut app = Application::new(LegacyModule);
        app.bootstrap_with_options(BootstrapOptions::default().with_parallel_start(parallel))
            .await
            .unwrap();
        assert_eq!(*app.injector().resolve::<u32>(), 42);
        app.shutdown().await.unwrap();
        assert!(!app.is_bootstrapped());
    }
}
