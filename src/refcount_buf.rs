/// Refcounted Immutable Buffer
#[allow(unused)]
use crate::loom::sync::atomic::AtomicMut;
use crate::loom::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};
use alloc::{
    alloc::{dealloc, Layout},
    borrow::Borrow,
    boxed::Box,
    string::String,
    vec::Vec,
};


#[cfg(target_pointer_width = "64")]
mod _ptr {
    pub type RefCountPtr = portable_atomic::AtomicPtr<super::Box<dyn super::RefCountBuf>>; 
    pub type RefCountUSize = u64;
}
#[cfg(target_pointer_width = "32")]
mod _ptr {
    pub type RefCountPtr = portable_atomic::Atomicu64;
    pub type RefCountUSize = u32;
}
pub use _ptr::{RefCountPtr, RefCountUSize};

/// A trait for underlying implementations for `Bytes` type.
///
/// All implementations must fulfill the following requirements:
/// - They are cheaply cloneable and thereby shareable between an unlimited amount
///   of components, for example by modifying a reference count.
/// - Instances can be sliced to refer to a subset of the the original buffer.
pub unsafe trait RefCountBuf: {
    /// Decompose `Self` into parts used by `Bytes`.
    fn slice(&self) -> (*const u8, usize);

    /// Create a clone at the specified offset and len
    ///  
    /// If necessary Self can transform itself into a new type that can
    /// accomodate clone/split operations
    /// 
    /// returns the parts necessary to construct a new Bytes instance.
    unsafe fn clone(&self, ptr: *const u8, len: usize) -> (Option<Box<dyn RefCountBuf>>, *const u8, usize);

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

    /// Consumes underlying resources and return `Vec<u8>`, usually with allocation
    unsafe fn into_vec(&self, ptr: *const u8, len: usize) -> Vec<u8>;

    /// Release underlying resources.
    /// Decrement a refcount.  If 0, convert the parts back into T
    /// then invoke T::drop(&mut T) on it.
    unsafe fn drop(self: Box<Self>, ptr: *const u8, len: usize) -> Option<Box<dyn RefCountBuf>>;
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
