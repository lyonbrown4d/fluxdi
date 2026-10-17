use crate::error::{Error, ErrorKind};
use std::{
    any::TypeId,
    cell::RefCell,
    collections::HashMap,
    marker::PhantomData,
    rc::Rc,
    sync::{Mutex, OnceLock},
};

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub(crate) struct ResolutionKey {
    pub type_id: TypeId,
    pub context: usize,
    pub provider: usize,
    pub cached: bool,
}

thread_local! {
    static RESOLVE_STACK: RefCell<Vec<ResolutionKey>> = const { RefCell::new(Vec::new()) };
}

type DependencyEdges = HashMap<ResolutionKey, HashMap<ResolutionKey, usize>>;
fn dependencies() -> &'static Mutex<DependencyEdges> {
    static EDGES: OnceLock<Mutex<DependencyEdges>> = OnceLock::new();
    EDGES.get_or_init(|| Mutex::new(HashMap::new()))
}

// Edges live until the nested resolve completes, including suspension while
// waiting for a binding being initialized by another task or thread.
struct DependencyEdge {
    from: ResolutionKey,
    to: ResolutionKey,
}
impl DependencyEdge {
    fn new(to: ResolutionKey) -> Result<Option<Self>, Error> {
        if !to.cached {
            return Ok(None);
        }
        let Some(from) =
            RESOLVE_STACK.with(|stack| stack.borrow().iter().rev().find(|key| key.cached).copied())
        else {
            return Ok(None);
        };
        let mut edges = dependencies().lock().unwrap_or_else(|err| err.into_inner());
        let mut pending = vec![to];
        let mut visited = std::collections::HashSet::new();
        while let Some(key) = pending.pop() {
            if key == from {
                drop(edges);
                return Err(circular_error(to));
            }
            if !visited.insert(key) {
                continue;
            }
            if let Some(next) = edges.get(&key) {
                pending.extend(next.keys().copied());
            }
        }
        *edges.entry(from).or_default().entry(to).or_default() += 1;
        Ok(Some(Self { from, to }))
    }
}
impl Drop for DependencyEdge {
    fn drop(&mut self) {
        let mut edges = dependencies().lock().unwrap_or_else(|err| err.into_inner());
        if let Some(outgoing) = edges.get_mut(&self.from) {
            if let Some(count) = outgoing.get_mut(&self.to) {
                *count -= 1;
                if *count == 0 {
                    outgoing.remove(&self.to);
                }
            }
            if outgoing.is_empty() {
                edges.remove(&self.from);
            }
        }
    }
}

fn circular_error(key: ResolutionKey) -> Error {
    #[cfg(feature = "tracing")]
    tracing::debug!(
        event = crate::observability::EVENT_CIRCULAR_DEPENDENCY,
        type_id = ?key.type_id,
        "Circular dependency detected during resolve"
    );
    Error::new(
        ErrorKind::CircularDependency,
        format!(
            "Circular dependency detected while resolving type_id: {:?} in its injector context",
            key.type_id
        ),
    )
}

/// Detects recursion during synchronous resolution. A guard stays on its thread.
pub struct ResolveGuard {
    key: ResolutionKey,
    _edge: Option<DependencyEdge>,
    _thread: PhantomData<Rc<()>>,
}
impl ResolveGuard {
    pub fn push(type_id: TypeId) -> Result<Self, Error> {
        Self::enter(
            ResolutionKey {
                type_id,
                context: 0,
                provider: 0,
                cached: false,
            },
            false,
        )
    }
    pub(crate) fn push_binding(key: ResolutionKey) -> Result<Self, Error> {
        Self::enter(key, true)
    }
    fn enter(key: ResolutionKey, track_edge: bool) -> Result<Self, Error> {
        if RESOLVE_STACK.with(|stack| stack.borrow().contains(&key)) {
            return Err(circular_error(key));
        }
        let edge = if track_edge {
            DependencyEdge::new(key)?
        } else {
            None
        };
        RESOLVE_STACK.with(|stack| stack.borrow_mut().push(key));
        Ok(Self {
            key,
            _edge: edge,
            _thread: PhantomData,
        })
    }
}
impl Drop for ResolveGuard {
    fn drop(&mut self) {
        RESOLVE_STACK.with(|stack| {
            let last = stack.borrow_mut().pop();
            debug_assert_eq!(last, Some(self.key));
        });
    }
}

/// Install the binding only while polling; preserve dependency edges across waits.
#[cfg(feature = "async-factory")]
pub(crate) async fn guard_async<F, T>(key: ResolutionKey, future: F) -> Result<T, Error>
where
    F: std::future::Future<Output = Result<T, Error>>,
{
    let mut future = std::pin::pin!(future);
    let mut edge = None;
    let mut started = false;
    futures::future::poll_fn(|cx| {
        if !started {
            edge = match DependencyEdge::new(key) {
                Ok(edge) => edge,
                Err(err) => return std::task::Poll::Ready(Err(err)),
            };
            started = true;
        }
        let _guard = match ResolveGuard::enter(key, false) {
            Ok(guard) => guard,
            Err(err) => return std::task::Poll::Ready(Err(err)),
        };
        future.as_mut().poll(cx)
    })
    .await
}
