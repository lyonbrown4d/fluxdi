use super::*;

/// Behavior when a resource limit is reached.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Policy {
    /// Return an error immediately when no creation slot is available.
    Deny,
    /// Block until a creation slot becomes available.
    Block,
}

/// Limits applied to provider factory execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// Maximum number of concurrent in-flight factory executions for this provider.
    ///
    /// `None` disables the limit.
    pub max_concurrent_creations: Option<usize>,
    /// Action to take when the limit is reached.
    pub policy: Policy,
    /// Optional timeout used by `Policy::Block`.
    ///
    /// - In `thread-safe` sync resolve, this bounds `Condvar` wait time.
    /// - With `resource-limit-async`, async resolve uses `tokio::time::timeout`.
    pub timeout: Option<Duration>,
}

impl Limits {
    pub const fn unlimited() -> Self {
        Self {
            max_concurrent_creations: None,
            policy: Policy::Deny,
            timeout: None,
        }
    }

    pub const fn deny(max_concurrent_creations: usize) -> Self {
        Self {
            max_concurrent_creations: Some(max_concurrent_creations),
            policy: Policy::Deny,
            timeout: None,
        }
    }

    pub const fn block(max_concurrent_creations: usize) -> Self {
        Self {
            max_concurrent_creations: Some(max_concurrent_creations),
            policy: Policy::Block,
            timeout: None,
        }
    }

    /// Applies a timeout to `Policy::Block`.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Builds block policy with timeout in one call.
    pub fn block_with_timeout(max_concurrent_creations: usize, timeout: Duration) -> Self {
        Self::block(max_concurrent_creations).with_timeout(timeout)
    }
}

impl Default for Limits {
    fn default() -> Self {
        Self::unlimited()
    }
}

#[cfg(not(feature = "thread-safe"))]
#[derive(Debug)]
pub(crate) struct Limiter {
    pub(super) max: usize,
    pub(super) policy: Policy,
    pub(super) current: std::cell::Cell<usize>,
    pub(super) timeout: Option<Duration>,
}

#[cfg(feature = "thread-safe")]
#[derive(Debug)]
pub(crate) struct Limiter {
    pub(super) max: usize,
    pub(super) policy: Policy,
    pub(super) current: std::sync::Mutex<usize>,
    pub(super) condvar: std::sync::Condvar,
    pub(super) timeout: Option<Duration>,
    #[cfg(feature = "resource-limit-async")]
    pub(super) async_waiters: std::sync::Mutex<Vec<futures::channel::oneshot::Sender<()>>>,
}

#[derive(Debug)]
pub(crate) struct CreationPermit {
    pub(super) limiter: Shared<Limiter>,
}

impl Drop for CreationPermit {
    fn drop(&mut self) {
        self.limiter.release();
    }
}
