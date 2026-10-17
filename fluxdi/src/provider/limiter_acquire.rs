use super::*;

impl Limiter {
    pub(super) fn from_limits(limits: Limits) -> Option<Shared<Self>> {
        let max = limits.max_concurrent_creations?;

        #[cfg(feature = "thread-safe")]
        {
            Some(Shared::new(Self {
                max,
                policy: limits.policy,
                current: std::sync::Mutex::new(0),
                condvar: std::sync::Condvar::new(),
                timeout: limits.timeout,
                #[cfg(feature = "resource-limit-async")]
                async_waiters: std::sync::Mutex::new(Vec::new()),
            }))
        }

        #[cfg(not(feature = "thread-safe"))]
        {
            Some(Shared::new(Self {
                max,
                policy: limits.policy,
                current: std::cell::Cell::new(0),
                timeout: limits.timeout,
            }))
        }
    }

    pub(super) fn try_acquire(
        limiter: &Shared<Self>,
        type_name: &str,
    ) -> Result<CreationPermit, Error> {
        #[cfg(feature = "thread-safe")]
        {
            if limiter.max == 0 {
                return Err(Error::resource_limit_exceeded(
                    type_name,
                    "max_concurrent_creations must be greater than 0",
                ));
            }

            let mut current = limiter.current.lock().unwrap();
            let deadline = limiter
                .timeout
                .map(|timeout| std::time::Instant::now() + timeout);
            loop {
                if *current < limiter.max {
                    *current += 1;
                    return Ok(CreationPermit {
                        limiter: limiter.clone(),
                    });
                }

                match limiter.policy {
                    Policy::Deny => {
                        return Err(Error::resource_limit_exceeded(
                            type_name,
                            format!("max_concurrent_creations={}", limiter.max).as_str(),
                        ));
                    }
                    Policy::Block => {
                        if let Some(deadline) = deadline {
                            let now = std::time::Instant::now();
                            if now >= deadline {
                                return Err(Error::resource_limit_exceeded(
                                    type_name,
                                    format!(
                                        "max_concurrent_creations={} timeout={:?}",
                                        limiter.max,
                                        limiter.timeout.unwrap_or_default()
                                    )
                                    .as_str(),
                                ));
                            }

                            let remaining = deadline.saturating_duration_since(now);
                            let (next_guard, wait_result) =
                                limiter.condvar.wait_timeout(current, remaining).unwrap();
                            current = next_guard;

                            if wait_result.timed_out() && *current >= limiter.max {
                                return Err(Error::resource_limit_exceeded(
                                    type_name,
                                    format!(
                                        "max_concurrent_creations={} timeout={:?}",
                                        limiter.max,
                                        limiter.timeout.unwrap_or_default()
                                    )
                                    .as_str(),
                                ));
                            }
                        } else {
                            current = limiter.condvar.wait(current).unwrap();
                        }
                    }
                }
            }
        }

        #[cfg(not(feature = "thread-safe"))]
        {
            if limiter.max == 0 {
                return Err(Error::resource_limit_exceeded(
                    type_name,
                    "max_concurrent_creations must be greater than 0",
                ));
            }

            let current = limiter.current.get();
            if current < limiter.max {
                limiter.current.set(current + 1);
                return Ok(CreationPermit {
                    limiter: limiter.clone(),
                });
            }

            match limiter.policy {
                Policy::Deny => Err(Error::resource_limit_exceeded(
                    type_name,
                    format!("max_concurrent_creations={}", limiter.max).as_str(),
                )),
                Policy::Block => Err(Error::resource_limit_exceeded(
                    type_name,
                    if limiter.timeout.is_some() {
                        "policy=Block (with timeout) requires `thread-safe` feature"
                    } else {
                        "policy=Block requires `thread-safe` feature"
                    },
                )),
            }
        }
    }

    #[cfg(all(feature = "thread-safe", feature = "resource-limit-async"))]
    pub(super) async fn try_acquire_async(
        limiter: &Shared<Self>,
        type_name: &str,
    ) -> Result<CreationPermit, Error> {
        if limiter.max == 0 {
            return Err(Error::resource_limit_exceeded(
                type_name,
                "max_concurrent_creations must be greater than 0",
            ));
        }
        let deadline = limiter
            .timeout
            .map(|duration| tokio::time::Instant::now() + duration);
        loop {
            let receiver = {
                let mut current = limiter.current.lock().unwrap();
                if *current < limiter.max {
                    *current += 1;
                    return Ok(CreationPermit {
                        limiter: limiter.clone(),
                    });
                }
                if limiter.policy == Policy::Deny {
                    return Err(Error::resource_limit_exceeded(
                        type_name,
                        &format!("max_concurrent_creations={}", limiter.max),
                    ));
                }
                let (sender, receiver) = futures::channel::oneshot::channel();
                let mut waiters = limiter.async_waiters.lock().unwrap();
                waiters.retain(|waiter| !waiter.is_canceled());
                waiters.push(sender);
                receiver
            };
            if let Some(deadline) = deadline {
                tokio::time::timeout_at(deadline, receiver)
                    .await
                    .map_err(|_| {
                        Error::resource_limit_exceeded(
                            type_name,
                            &format!(
                                "max_concurrent_creations={} timeout={:?}",
                                limiter.max, limiter.timeout
                            ),
                        )
                    })?
                    .ok();
            } else {
                let _ = receiver.await;
            }
        }
    }
}
