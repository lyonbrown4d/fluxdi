use super::*;
use futures::channel::oneshot;

use crate::runtime::Service;

#[derive(Clone, Copy)]
enum Owner {
    #[cfg(feature = "thread-safe")]
    Sync(std::thread::ThreadId),
    #[cfg(not(feature = "thread-safe"))]
    Sync,
    #[cfg(feature = "async-factory")]
    Async,
}

struct State<T: ?Sized + 'static> {
    value: Option<Shared<Instance<T>>>,
    owner: Option<Owner>,
    waiters: Vec<oneshot::Sender<()>>,
}

pub(super) struct Initialization<T: ?Sized + 'static> {
    // Retain provider identity so its address cannot be reused while cached.
    _provider: Shared<Provider<T>>,
    #[cfg(feature = "thread-safe")]
    state: std::sync::Mutex<State<T>>,
    #[cfg(not(feature = "thread-safe"))]
    state: std::cell::RefCell<State<T>>,
    #[cfg(feature = "thread-safe")]
    changed: std::sync::Condvar,
}

#[cfg(feature = "thread-safe")]
type StateGuard<'a, T> = std::sync::MutexGuard<'a, State<T>>;
#[cfg(not(feature = "thread-safe"))]
type StateGuard<'a, T> = std::cell::RefMut<'a, State<T>>;

struct Creation<T: ?Sized + 'static> {
    cell: Shared<Initialization<T>>,
}

#[cfg(feature = "async-factory")]
enum AsyncLookup<T: ?Sized + 'static> {
    Ready(Shared<Instance<T>>),
    Create(Creation<T>),
    Wait(oneshot::Receiver<()>),
}

impl<T: ?Sized + Service> Initialization<T> {
    pub(super) fn new(provider: Shared<Provider<T>>) -> Self {
        let state = State {
            value: None,
            owner: None,
            waiters: Vec::new(),
        };
        Self {
            _provider: provider,
            #[cfg(feature = "thread-safe")]
            state: std::sync::Mutex::new(state),
            #[cfg(not(feature = "thread-safe"))]
            state: std::cell::RefCell::new(state),
            #[cfg(feature = "thread-safe")]
            changed: std::sync::Condvar::new(),
        }
    }

    fn lock(&self) -> StateGuard<'_, T> {
        #[cfg(feature = "thread-safe")]
        {
            self.state.lock().unwrap_or_else(|err| err.into_inner())
        }
        #[cfg(not(feature = "thread-safe"))]
        {
            self.state.borrow_mut()
        }
    }

    pub(super) fn get(&self) -> Option<Shared<Instance<T>>> {
        self.lock().value.clone()
    }

    pub(super) fn get_or_try_init(
        self: &Shared<Self>,
        create: impl FnOnce() -> Result<Shared<Instance<T>>, Error>,
    ) -> Result<Shared<Instance<T>>, Error> {
        if let Some(value) = self.reserve_sync()? {
            return Ok(value);
        }
        let creation = Creation { cell: self.clone() };
        let value = create()?;
        self.lock().value = Some(value.clone());
        drop(creation);
        Ok(value)
    }

    #[cfg(feature = "thread-safe")]
    fn reserve_sync(&self) -> Result<Option<Shared<Instance<T>>>, Error> {
        let mut state = self.lock();
        loop {
            if let Some(value) = &state.value {
                return Ok(Some(value.clone()));
            }
            match state.owner {
                None => {
                    state.owner = Some(Owner::Sync(std::thread::current().id()));
                    return Ok(None);
                }
                Some(Owner::Sync(owner)) if owner == std::thread::current().id() => {
                    return Err(initialization_in_progress::<T>());
                }
                #[cfg(feature = "async-factory")]
                Some(Owner::Async) => return Err(initialization_in_progress::<T>()),
                _ => {
                    state = self
                        .changed
                        .wait(state)
                        .unwrap_or_else(|err| err.into_inner());
                }
            }
        }
    }

    #[cfg(not(feature = "thread-safe"))]
    fn reserve_sync(&self) -> Result<Option<Shared<Instance<T>>>, Error> {
        let mut state = self.lock();
        if let Some(value) = &state.value {
            return Ok(Some(value.clone()));
        }
        if state.owner.is_some() {
            return Err(initialization_in_progress::<T>());
        }
        state.owner = Some(Owner::Sync);
        Ok(None)
    }

    #[cfg(feature = "async-factory")]
    fn lookup_async(self: &Shared<Self>) -> AsyncLookup<T> {
        let mut state = self.lock();
        if let Some(value) = &state.value {
            return AsyncLookup::Ready(value.clone());
        }
        if state.owner.is_none() {
            state.owner = Some(Owner::Async);
            return AsyncLookup::Create(Creation { cell: self.clone() });
        }
        let (sender, receiver) = oneshot::channel();
        state.waiters.retain(|waiter| !waiter.is_canceled());
        state.waiters.push(sender);
        AsyncLookup::Wait(receiver)
    }

    #[cfg(feature = "async-factory")]
    pub(super) async fn get_or_try_init_async<F>(
        self: &Shared<Self>,
        create: impl FnOnce() -> F,
    ) -> Result<Shared<Instance<T>>, Error>
    where
        F: std::future::Future<Output = Result<Shared<Instance<T>>, Error>>,
    {
        loop {
            match self.lookup_async() {
                AsyncLookup::Ready(value) => return Ok(value),
                AsyncLookup::Wait(receiver) => {
                    let _ = receiver.await;
                }
                AsyncLookup::Create(creation) => {
                    let value = create().await?;
                    self.lock().value = Some(value.clone());
                    drop(creation);
                    return Ok(value);
                }
            }
        }
    }
}

impl<T: ?Sized + 'static> Drop for Creation<T> {
    fn drop(&mut self) {
        #[cfg(feature = "thread-safe")]
        let mut state = self
            .cell
            .state
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        #[cfg(not(feature = "thread-safe"))]
        let mut state = self.cell.state.borrow_mut();
        state.owner = None;
        let waiters = std::mem::take(&mut state.waiters);
        drop(state);
        #[cfg(feature = "thread-safe")]
        self.cell.changed.notify_all();
        for waiter in waiters {
            let _ = waiter.send(());
        }
    }
}

fn initialization_in_progress<T: ?Sized>() -> Error {
    Error::new(
        crate::ErrorKind::AsyncFactoryRequiresAsyncResolve,
        format!(
            "Initialization already in progress for {}; use async resolve to wait without blocking its executor",
            std::any::type_name::<T>()
        ),
    )
}
