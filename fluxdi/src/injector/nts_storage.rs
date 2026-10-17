use super::*;

impl Injector {
    pub(crate) fn store_provider<T>(&self, provider: Provider<T>) -> Result<(), Error>
    where
        T: ?Sized + 'static,
    {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>();
        let scope = provider.scope;
        let graph_meta = ProviderGraphMeta::of::<T>(scope, provider.dependency_hints.clone());

        #[cfg(feature = "tracing")]
        debug!(
            type_name = type_name,
            scope = %scope,
            op = "provider_store",
            "Storing provider definition"
        );

        let mut providers = self.inner.providers.borrow_mut();
        if providers.contains_key(&type_id) {
            #[cfg(feature = "tracing")]
            debug!(
                type_name = type_name,
                scope = %scope,
                op = "provider_store",
                "Provider registration rejected: duplicate binding"
            );
            return Err(Error::provider_already_registered(
                type_name,
                scope.to_string().as_str(),
            ));
        }
        providers.insert(type_id, Shared::new(provider));
        drop(providers);
        self.inner
            .graph_providers
            .borrow_mut()
            .insert(type_id, graph_meta);

        #[cfg(feature = "tracing")]
        debug!(
            type_name = type_name,
            scope = %scope,
            op = "provider_store",
            "Provider definition stored"
        );

        Ok(())
    }

    pub(crate) fn store_set_provider<T>(&self, provider: Provider<T>) -> Result<(), Error>
    where
        T: ?Sized + 'static,
    {
        let type_id = TypeId::of::<T>();
        #[cfg(feature = "tracing")]
        let type_name = std::any::type_name::<T>();
        #[cfg(feature = "tracing")]
        let scope = provider.scope;
        let graph_meta =
            ProviderGraphMeta::of::<T>(provider.scope, provider.dependency_hints.clone());
        let mut providers = self.inner.set_providers.borrow_mut();
        providers
            .entry(type_id)
            .or_default()
            .push(Shared::new(provider));
        drop(providers);
        self.inner
            .graph_set_providers
            .borrow_mut()
            .entry(type_id)
            .or_default()
            .push(graph_meta);

        #[cfg(feature = "tracing")]
        debug!(
            type_name = type_name,
            scope = %scope,
            op = "provider_store_set",
            "Provider appended to set binding"
        );

        Ok(())
    }

    pub(crate) fn store_named_provider<T>(
        &self,
        name: &str,
        provider: Provider<T>,
    ) -> Result<(), Error>
    where
        T: ?Sized + 'static,
    {
        let key = NamedTypeKey::of::<T>(name);
        let type_name = std::any::type_name::<T>();
        let scope = provider.scope;
        let graph_meta = ProviderGraphMeta::of::<T>(scope, provider.dependency_hints.clone());

        #[cfg(feature = "tracing")]
        debug!(
            type_name = type_name,
            name = %name,
            scope = %scope,
            op = "provider_store_named",
            "Storing named provider definition"
        );

        let mut providers = self.inner.named_providers.borrow_mut();
        if providers.contains_key(&key) {
            #[cfg(feature = "tracing")]
            debug!(
                type_name = type_name,
                name = %name,
                scope = %scope,
                op = "provider_store_named",
                "Named provider registration rejected: duplicate binding"
            );
            return Err(Error::provider_already_registered_named(
                type_name,
                name,
                scope.to_string().as_str(),
            ));
        }
        providers.insert(key.clone(), Shared::new(provider));
        drop(providers);
        self.inner
            .graph_named_providers
            .borrow_mut()
            .insert(key, graph_meta);

        #[cfg(feature = "tracing")]
        debug!(
            type_name = type_name,
            name = %name,
            scope = %scope,
            op = "provider_store_named",
            "Named provider definition stored"
        );

        Ok(())
    }

    pub(crate) fn replace_provider<T>(&self, provider: Provider<T>) -> Result<(), Error>
    where
        T: ?Sized + 'static,
    {
        let previous = self
            .resolve_provider::<T>()
            .map_err(|_| Error::service_not_provided_for_override(std::any::type_name::<T>()))?;
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>();
        #[cfg(feature = "tracing")]
        let scope = provider.scope;
        let graph_meta =
            ProviderGraphMeta::of::<T>(provider.scope, provider.dependency_hints.clone());

        #[cfg(feature = "tracing")]
        debug!(
            type_name = type_name,
            scope = %scope,
            op = "provider_replace",
            "Replacing provider definition"
        );

        let mut providers = self.inner.providers.borrow_mut();
        if !providers.contains_key(&type_id) {
            #[cfg(feature = "tracing")]
            debug!(
                type_name = type_name,
                scope = %scope,
                op = "provider_replace",
                "Provider replace rejected: no previous binding"
            );
            return Err(Error::service_not_provided_for_override(type_name));
        }
        providers.insert(type_id, Shared::new(provider));
        drop(providers);
        self.inner
            .graph_providers
            .borrow_mut()
            .insert(type_id, graph_meta);

        self.clear_initialization(&previous);

        #[cfg(feature = "tracing")]
        debug!(
            type_name = type_name,
            scope = %scope,
            op = "provider_replace",
            "Provider definition replaced and cache invalidated"
        );

        Ok(())
    }
}
