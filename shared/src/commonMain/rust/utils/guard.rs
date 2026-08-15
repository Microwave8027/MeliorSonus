use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

pub struct DropGuard(Arc<AtomicBool>);

impl DropGuard {
    pub fn new() -> Self {
        DropGuard(Arc::new(AtomicBool::new(false)))
    }

    pub fn from(val: Arc<AtomicBool>) -> Self {
        DropGuard(val)
    }
}

impl Drop for DropGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}
