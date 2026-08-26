use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// RAII Drop Guard ensuring atomic flags are safely reset when dropped or when threads exit.
pub struct DropGuard(Arc<AtomicBool>);

impl DropGuard {
    pub fn new() -> Self {
        DropGuard(Arc::new(AtomicBool::new(false)))
    }

    pub fn from(val: Arc<AtomicBool>) -> Self {
        DropGuard(val)
    }
}

impl Default for DropGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for DropGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}
