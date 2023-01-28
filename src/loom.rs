#[cfg(not(all(test, loom)))]
pub(crate) mod sync {
    pub(crate) mod atomic {
        pub(crate) use portable_atomic::{AtomicU128, AtomicUsize, Ordering};

        pub(crate) trait AtomicMut<T> {
            fn with_mut<F, R>(&mut self, f: F) -> R
            where
                F: FnOnce(&mut *mut T) -> R;
        }

        impl<T> AtomicMut<T> for AtomicU128 {
            fn with_mut<F, R>(&mut self, f: F) -> R
            where
                F: FnOnce(&mut *mut T) -> R,
            {
                let ptr: *mut dyn T = unsafe { std::mem::transmute(self.load(Ordering::Relaxed)) };
                f(&mut ptr)
            }
        }
    }
}

#[cfg(all(test, loom))]
pub(crate) mod sync {
    pub(crate) mod atomic {
        pub(crate) use loom::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};

        pub(crate) trait AtomicMut<T> {}
    }
}
