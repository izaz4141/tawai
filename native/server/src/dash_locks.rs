use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex, PoisonError};

#[derive(Debug, Default)]
pub struct DashGenerationLocks {
    inner: Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>,
}

impl DashGenerationLocks {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn with_lock<T>(&self, track_id: &str, fut: impl Future<Output = T>) -> T {
        let lock = {
            let mut map = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
            map.entry(track_id.to_owned())
                .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
                .clone()
        };

        let guard = lock.clone().lock_owned().await;
        let result = fut.await;
        drop(guard);

        let mut map = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        if Arc::strong_count(&lock) == 2
            && map
                .get(track_id)
                .is_some_and(|entry| Arc::ptr_eq(entry, &lock))
        {
            map.remove(track_id);
        }

        result
    }
}
