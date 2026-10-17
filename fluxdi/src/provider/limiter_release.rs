use super::*;

impl Limiter {
    pub(super) fn release(&self) {
        #[cfg(feature = "thread-safe")]
        {
            let mut current = self.current.lock().unwrap();
            if *current > 0 {
                *current -= 1;
            }

            #[cfg(feature = "resource-limit-async")]
            let waiters = std::mem::take(&mut *self.async_waiters.lock().unwrap());
            drop(current);
            #[cfg(feature = "resource-limit-async")]
            for waiter in waiters {
                let _ = waiter.send(());
            }

            if self.policy == Policy::Block {
                self.condvar.notify_one();
            }
        }

        #[cfg(not(feature = "thread-safe"))]
        {
            let current = self.current.get();
            if current > 0 {
                self.current.set(current - 1);
            }
        }
    }
}
