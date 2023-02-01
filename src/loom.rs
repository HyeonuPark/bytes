#[cfg(not(all(test, loom)))]
pub(crate) mod sync {
    pub(crate) mod atomic {
        pub(crate) use portable_atomic::{AtomicU128, AtomicUsize, Ordering};

        pub(crate) trait AtomicMut {
            type Size; 
            fn with_mut<F, R>(&mut self, f: F) -> R
            where
                F: FnOnce(&mut Self::Size) -> R;
        }

        impl AtomicMut for AtomicU128 {
            type Size = u128; 
            fn with_mut<F, R>(&mut self, f: F) -> R
            where
                F: FnOnce(&mut Self::Size) -> R,
            {
                let data = self.load(Ordering::Relaxed);
                f(&mut data)
            }
        }
    }
}

#[cfg(all(test, loom))]
pub(crate) mod sync {
    pub(crate) mod atomic {
        pub(crate) use loom::sync::atomic::{AtomicU128, AtomicMut, AtomicUsize, Ordering};
    }
}
