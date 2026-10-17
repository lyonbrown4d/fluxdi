use super::initialization::Service;
use super::*;

impl Injector {
    pub(super) fn initialization<T: ?Sized + Service>(
        &self,
        provider: &Shared<Provider<T>>,
    ) -> Shared<Initialization<T>> {
        let key = SetProviderKey::of(provider);
        #[cfg(all(feature = "thread-safe", feature = "lock-free"))]
        let cached = self
            .inner
            .initializations
            .get(&key)
            .map(|entry| entry.value().clone());
        #[cfg(all(feature = "thread-safe", not(feature = "lock-free")))]
        let cached = self
            .inner
            .initializations
            .read()
            .unwrap()
            .get(&key)
            .cloned();
        #[cfg(not(feature = "thread-safe"))]
        let cached = self.inner.initializations.borrow().get(&key).cloned();

        let cell = cached.unwrap_or_else(|| {
            #[cfg(all(feature = "thread-safe", feature = "lock-free"))]
            {
                self.inner
                    .initializations
                    .entry(key)
                    .or_insert_with(|| Shared::new(Initialization::new(provider.clone())))
                    .value()
                    .clone()
            }
            #[cfg(all(feature = "thread-safe", not(feature = "lock-free")))]
            {
                self.inner
                    .initializations
                    .write()
                    .unwrap()
                    .entry(key)
                    .or_insert_with(|| Shared::new(Initialization::new(provider.clone())))
                    .clone()
            }
            #[cfg(not(feature = "thread-safe"))]
            {
                self.inner
                    .initializations
                    .borrow_mut()
                    .entry(key)
                    .or_insert_with(|| Shared::new(Initialization::new(provider.clone())))
                    .clone()
            }
        });
        cell.downcast::<Initialization<T>>()
            .unwrap_or_else(|_| unreachable!("cache type matches its provider"))
    }

    pub(super) fn clear_initialization<T: ?Sized + Service>(&self, provider: &Shared<Provider<T>>) {
        let key = SetProviderKey::of(provider);
        #[cfg(all(feature = "thread-safe", feature = "lock-free"))]
        self.inner.initializations.remove(&key);
        #[cfg(all(feature = "thread-safe", not(feature = "lock-free")))]
        self.inner.initializations.write().unwrap().remove(&key);
        #[cfg(not(feature = "thread-safe"))]
        self.inner.initializations.borrow_mut().remove(&key);
    }
}
