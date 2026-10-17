use super::*;

impl Injector {
    pub(crate) fn store_provider<T>(&self, provider: Provider<T>) -> Result<(), Error>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>();
        let scope = provider.scope;
        let scope_text = scope.to_string();
        let graph_meta = ProviderGraphMeta::of::<T>(scope, provider.dependency_hints.clone());

        #[cfg(feature = "tracing")]
        debug!(
            type_name = type_name,
            scope = %scope,
            op = "provider_store",
            "Storing provider definition"
        );

        #[cfg(feature = "lock-free")]
        {
            match self.inner.providers.entry(type_id) {
                DashEntry::Occupied(_) => {
                    #[cfg(feature = "tracing")]
                    debug!(
                        type_name = type_name,
                        scope = %scope,
                        op = "provider_store",
                        "Provider registration rejected: duplicate binding"
                    );
                    return Err(Error::provider_already_registered(
                        type_name,
                        scope_text.as_str(),
                    ));
                }
                DashEntry::Vacant(entry) => {
                    entry.insert(Shared::new(provider));
                }
            }
            self.inner.graph_providers.insert(type_id, graph_meta);
        }

        #[cfg(not(feature = "lock-free"))]
        {
            let mut providers = self.inner.providers.write().unwrap();
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
                    scope_text.as_str(),
                ));
            }
            providers.insert(type_id, Shared::new(provider));
            drop(providers);
            self.inner
                .graph_providers
                .write()
                .unwrap()
                .insert(type_id, graph_meta);
        }

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
        T: ?Sized + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        #[cfg(feature = "tracing")]
        let type_name = std::any::type_name::<T>();
        #[cfg(feature = "tracing")]
        let scope = provider.scope;
        let graph_meta =
            ProviderGraphMeta::of::<T>(provider.scope, provider.dependency_hints.clone());

        #[cfg(feature = "lock-free")]
        {
            match self.inner.set_providers.entry(type_id) {
                DashEntry::Occupied(mut entry) => {
                    entry.get_mut().push(Shared::new(provider));
                }
                DashEntry::Vacant(entry) => {
                    entry.insert(vec![Shared::new(provider)]);
                }
            }
            match self.inner.graph_set_providers.entry(type_id) {
                DashEntry::Occupied(mut entry) => {
                    entry.get_mut().push(graph_meta);
                }
                DashEntry::Vacant(entry) => {
                    entry.insert(vec![graph_meta]);
                }
            }
        }

        #[cfg(not(feature = "lock-free"))]
        {
            let mut providers = self.inner.set_providers.write().unwrap();
            providers
                .entry(type_id)
                .or_default()
                .push(Shared::new(provider));
            drop(providers);
            self.inner
                .graph_set_providers
                .write()
                .unwrap()
                .entry(type_id)
                .or_default()
                .push(graph_meta);
        }

        #[cfg(feature = "tracing")]
        debug!(
            type_name = type_name,
            scope = %scope,
            op = "provider_store_set",
            "Provider appended to set binding"
        );

        Ok(())
    }
}
