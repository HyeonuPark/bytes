/// Refcounted Immutable Buffer
#[allow(unused)]
use crate::loom::sync::atomic::{AtomicMut, AtomicU128, AtomicUsize, Ordering};
use alloc::{
    boxed::Box,
    vec::Vec,
};
use core::ptr;

#[cfg(target_pointer_width = "64")]
mod _ptr {
    pub type RefCountPtr = portable_atomic::AtomicU128;
    pub type RefCountUSize = u128;
}
#[cfg(target_pointer_width = "32")]
mod _ptr {
    pub type RefCountPtr = portable_atomic::Atomicu64;
    pub type RefCountUSize = u64;
}
pub use _ptr::{RefCountPtr, RefCountUSize};

#[macro_export]
macro_rules! ref_to_dyn_ptr {
    ($p:expr) => {
        std::mem::transmute($p.load(Ordering::Relaxed))
    };
}

#[macro_export]
macro_rules! dyn_ptr_to_ref {
    ($n:expr) => {
        RefCountPtr::new(std::mem::transmute::<_, RefCountUSize>($n))
    };
}

#[macro_export]
macro_rules! dyn_ptr_to_usz {
    ($n:expr) => {
        std::mem::transmute::<_, RefCountUSize>($n)
    };
}

pub(crate) trait AtomicMutPtr {
    fn with_mut_ptr<F, R>(&mut self, f: F) -> R
where
        F: FnOnce(&mut *mut dyn RefCountBuf) -> R;
}

impl AtomicMutPtr for RefCountPtr {
    fn with_mut_ptr<F, R>(&mut self, f: F) -> R
where
        F: FnOnce(&mut *mut dyn RefCountBuf) -> R
    {
        let mut ptr: &mut *mut dyn RefCountBuf = unsafe { core::mem::transmute(self.get_mut()) };
        f(ptr)
    }
}

/// A trait for underlying implementations for `Bytes` type.
///
/// All implementations must fulfill the following requirements:
/// - They are cheaply cloneable and thereby shareable between an unlimited amount
///   of components, for example by modifying a reference count.
/// - Instances can be sliced to refer to a subset of the the original buffer.
pub unsafe trait RefCountBuf {
    /// Decompose `Self` into parts used by `Bytes`.
    fn slice(&self) -> (*const u8, usize);

    /// Create a clone at the specified offset and len
    ///  
    /// If necessary Self can transform itself into a new type that can
    /// accomodate clone/split operations
    ///
    /// returns the parts necessary to construct a new Bytes instance.
    unsafe fn clone(
        &self,
        ptr: *const u8,
        len: usize,
    ) -> (Option<Box<dyn RefCountBuf>>, *const u8, usize);

    /// Called before the `Bytes::truncate` is processed.  
    /// Useful if the implementation needs some preparation step for it.
    /// If the conversion can't be conducted without allocation:
    ///     If `can_alloc` is true, then go ahead and allocate
    ///     Else return Error
    unsafe fn try_resize(
        &self,
        ptr: *const u8,
        len: usize,
        can_alloc: bool,
    ) -> Result<Option<Box<dyn RefCountBuf>>, RefCountBufError> {
        // do nothing by default
        let _ = (ptr, len, can_alloc);
        Ok(None)
    }

    /// Attempt to convert this buffer from mutable to immutable.
    /// The default implementation is a no-op.
    /// Any buffers that implement a mutable buffer should override this method.
    unsafe fn freeze(&self, ptr: *const u8, len: usize) -> Option<Box<dyn RefCountBuf>> {
        None
    }

    /// Attempt to convert this buffer into mutable without allocating
    /// If the conversion can't be conducted without allocation:
    ///     If `can_alloc` is true, then go ahead and allocate
    ///     Else return Error
    unsafe fn try_into_mut(
        &self,
        ptr: *const u8,
        len: usize,
        can_alloc: bool,
    ) -> Result<Option<Box<dyn RefCountBuf>>, RefCountBufError> {
        Err(RefCountBufError::Unsupported)
    }

    /// Consumes underlying resources and returns `Vec<u8>`
    /// typically allocates if references to `self` are > 1 
    unsafe fn into_vec(&mut self, ptr: *const u8, len: usize) -> Vec<u8>;

    /// Release underlying resources.
    /// Decrement a refcount.  If 0, convert the parts back into T
    /// then invoke T::drop(&mut T) on it.
    unsafe fn drop(&mut self, ptr: *const u8, len: usize);
}

#[derive(Debug)]
/// Errors
pub enum RefCountBufError {
    /// This operation would fail due to a missing precondition
    PreconditionInvalid,
    /// This operation would fail because there are too many shared copies
    RefcountTooHigh,
    /// The result of a `try_*` operation in which the operation would not succeed without allocating
    WouldAllocate,
    /// There was an attempt to write or resize a buffer, but the buffer was too small
    InvalidLength,
    /// Supplied data or resize length woud go past the end of the buffer
    OutOfBounds,
    /// This operation is unsupported in this implementation
    Unsupported,
}
