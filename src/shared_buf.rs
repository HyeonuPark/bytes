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

/// Interop contract that enables buffer types to convert to and from [`Bytes`] instances.
///
/// All implementations must fulfill the following requirements:
/// * They are cheaply cloneable and thereby shareable between an unlimited number
///   of instances. For example by modifying a reference count.
/// * Instances can be sliced to refer to a subset of the the original buffer.
///
///  # Safety
///
///  Usage of the [`SharedBuf`] and the resulting [`Bytes`] object is undefined
///  unless the functions in this trait produce:
///     * A Valid pointer to a the container object in the form of an [`AtomicPtr`]. For refcounting
///     and reconstruction/downcasting.
///     * A Valid pointer to a single, contiguous data slice as (`const *u8`) where:
///         * The data slice is guaranteed to exist until the [`drop`] function is called.
///         * The data slice must not be mutated for the lifetime of the container.
///     * The supplied `len` must be less than or equal to the actual data slice in memory.
///     * The total size len must be no larger than isize::MAX. See the safety documentation of pointer::offset.
pub unsafe trait SharedBuf: 'static + Sized {
    /// Decompose `Self` into parts used by other shared buffer types
    fn into_parts(this: Self) -> (RefCountPtr, *const u8, usize);

    /// Produces the necessary components to construct a [`Bytes`] instance.
    ///
    /// # Safety
    ///
    /// This implementation must conform to the guarantees described in the the `Safety`
    /// section of this [`SharedBuf`] trait.
    ///
    unsafe fn from_parts(data: &mut RefCountPtr, ptr: *const u8, len: usize) -> Self;

    /// Attempts a clone of Self using the current data pointer, buffer pointer, and len
    ///
    /// Should cheaply clone the object without copying the data where possible. If the underlying
    /// data implementation is cheaply cloneable, this should be trivial to implement.
    ///
    /// # Safety
    ///
    /// This implementation must conform to the guarantees described in the the `Safety`
    /// section of this [`SharedBuf`] trait.
    ///
    unsafe fn try_clone<T: SharedBuf + Sized>(
        data: &RefCountPtr,
        ptr: *const u8,
        len: usize,
    ) -> CloneResult<Self, T>;

    /// Called before the `Bytes::truncate` is processed.
    ///
    /// Implementations of `SharedBuf` must be able to produce correct results
    /// using the `len` property. As an optimization, some implementations do
    /// transparently refer to their underlying buffer, until some copy-on-write activity
    /// forces them to restructure.  This method should invoke that necessary restructuring.
    ///
    /// # Safety
    ///
    /// If the underlying impl doesn't constrain itself by the len parameter, then future calls
    /// to slice will return a slice of incorrect size. This can result in runtime panics
    /// or other undefined behavior.
    unsafe fn try_truncate(
        data: &mut RefCountPtr,
        ptr: *const u8,
        len: usize,
    ) -> Result<(), SharedBufError> {
        // do nothing by default
        let _ = (data, ptr, len);
        Ok(())
    }

    /// Consumes this instance and converts it into a `Vec<u8>`
    ///
    /// The `Bytes` instance for which this is called may not be unique, nor
    /// is it guaranteed to be pointing to the original buffer. `Vec<u8>` expects
    /// to own its contents. Therefore, this function will likely require a copy
    /// from the original buffer into one that is owned by the Vec.
    ///
    /// # Safety
    ///
    /// The resulting Vec will be invalid unless the implementation:
    ///
    /// * Ensures that no other objects own the buffer pointed to by the new Vec.
    /// * The len used by the Vec is valid for its buffer, and matches the supplied `len`
    ///
    /// If you are avoiding allocation, and passing control of the buffer to the Vec, then
    /// you should ensure that all [`required invariants`] are met when constructing the Vec.
    ///
    /// [`required invariants`]: https://doc.rust-lang.org/std/vec/struct.Vec.html#safety
    unsafe fn into_vec(data: &mut RefCountPtr, ptr: *const u8, len: usize) -> Vec<u8>;

    /// Release or decrement the refcount of the underlying resources.
    ///
    /// # Safety
    ///
    /// If this implementation deallocates the underlying buffer, it must ensure
    /// that there are no other instances referring to it. Otherwise the behavior will
    /// be undefined.
    unsafe fn drop(data: &mut RefCountPtr, ptr: *const u8, len: usize);
}

#[derive(Debug)]
/// Errors
pub enum SharedBufError {
    /// This operation would fail due to a missing precondition
    PreconditionInvalid,
    /// This operation would fail because there are too many shared copies
    RefcountTooHigh,
    /// The result of a `try_*` operation in which the operation would not succeed without allocating
    WouldAllocate,
    /// There was an attempt to write or resize a buffer, but the buffer was too small
    InvalidLength,
}

/// Helper type for the SharedBuf::clone method to indicate
/// whether the type return a clone of itself, or upgraded to a new type
#[derive(Debug)]
pub enum CloneResult<T, U> {
    /// The `Self` instance
    Cloned(T),
    /// `Self` was converted into T for this operation
    Promoted(U),
}

/// A buffer
#[derive(Debug)]
/// Standard refcounted buffer that calls its own drop
pub struct Owned {}

#[derive(Debug)]
/// Standard
pub struct OwnedMut {}

#[derive(Debug)]
pub struct Static {}

#[derive(Debug)]
pub struct StaticMut {}

#[derive(Debug)]
pub struct StaticPromotable {}
