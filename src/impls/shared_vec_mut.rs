use alloc::{
    alloc::{dealloc, Layout},
    boxed::Box,
    vec::Vec,
};
use core::{mem, ptr, slice, usize};
use crate::refcount_buf::RefCountBuf;
#[allow(unused)]
use crate::loom::sync::atomic::AtomicMut;
use crate::loom::sync::atomic::{AtomicUsize, Ordering};
use core::cmp;

// Thread-safe reference-counted container for the shared storage. This mostly
// the same as `core::sync::Arc` but without the weak counter. The ref counting
// fns are based on the ones found in `std`.
//
// The main reason to use `SharedVecMut` instead of `core::sync::Arc` is that it ends
// up making the overall code simpler and easier to reason about. This is due to
// some of the logic around setting `Inner::arc` and other ways the `arc` field
// is used. Using `Arc` ended up requiring a number of funky transmutes and
// other shenanigans to make it work.
pub struct SharedVecMut {
    vec: Vec<u8>,
    original_capacity_repr: usize,
    ref_count: AtomicUsize,
}

// Buffer storage strategy flags.
const KIND_ARC: usize = 0b0;
const KIND_VEC: usize = 0b1;
const KIND_MASK: usize = 0b1;



#[cfg(target_pointer_width = "64")]
const PTR_WIDTH: usize = 64;
#[cfg(target_pointer_width = "32")]
const PTR_WIDTH: usize = 32;


impl SharedVecMut {
    fn is_unique(&self) -> bool {
        // The goal is to check if the current handle is the only handle
        // that currently has access to the buffer. This is done by
        // checking if the `ref_count` is currently 1.
        //
        // The `Acquire` ordering synchronizes with the `Release` as
        // part of the `fetch_sub` in `release_shared`. The `fetch_sub`
        // operation guarantees that any mutations done in other threads
        // are ordered before the `ref_count` is decremented. As such,
        // this `Acquire` will guarantee that those mutations are
        // visible to the current thread.
        self.ref_count.load(Ordering::Acquire) == 1
    }
    unsafe fn increment(&self) {
        let old_size = self.ref_count.fetch_add(1, Ordering::Relaxed);

        if old_size > isize::MAX as usize {
            crate::abort();
        }
    }

    unsafe fn release(&mut self) {
        // `SharedVecMut` storage... follow the drop steps from Arc.
        if self.ref_count.fetch_sub(1, Ordering::Release) != 1 {
            return;
        }

        // This fence is needed to prevent reordering of use of the data and
        // deletion of the data.  Because it is marked `Release`, the decreasing
        // of the reference count synchronizes with this `Acquire` fence. This
        // means that use of the data happens before decreasing the reference
        // count, which happens before this fence, which happens before the
        // deletion of the data.
        //
        // As explained in the [Boost documentation][1],
        //
        // > It is important to enforce any possible access to the object in one
        // > thread (through an existing reference) to *happen before* deleting
        // > the object in a different thread. This is achieved by a "release"
        // > operation after dropping a reference (any access to the object
        // > through this reference must obviously happened before), and an
        // > "acquire" operation before deleting the object.
        //
        // [1]: (www.boost.org/doc/libs/1_55_0/doc/html/atomic/usage_examples.html)
        //
        // Thread sanitizer does not support atomic fences. Use an atomic load
        // instead.
        self.ref_count.load(Ordering::Acquire);

        // Drop the data
        drop(Box::from_raw(self));
    }

}


// ===== impl SharedVecMutVtable =====

struct SharedVecMutImpl {
    shared: *mut SharedVecMut,
    ptr: *const u8,
    len: usize,
}

unsafe impl RefCountBuf for SharedVecMutImpl {

    fn slice(&self) -> (*const u8, usize) {
        (self.ptr, self.len)
    }

    unsafe fn clone(&self, ptr: *const u8, len: usize) -> (Option<Box<dyn RefCountBuf>>, *const u8, usize) {
        (*self.shared).increment();
        (None, ptr, len)
    }

    unsafe fn into_vec(&mut self, ptr: *const u8, len: usize) -> Vec<u8> {
        if (*self.shared).is_unique() {
            let shared = &mut *self.shared;

            // Drop shared
            let mut vec = mem::replace(&mut shared.vec, Vec::new());
            (*shared).release();

            // Copy back buffer
            ptr::copy(ptr, vec.as_mut_ptr(), len);
            vec.set_len(len);

            vec
        } else {
            let v = slice::from_raw_parts(ptr, len).to_vec();
            (*self.shared).release();
            v
        }
    }

    unsafe fn drop(&mut self, _ptr: *const u8, _len: usize) {
        (*self.shared).release()
    }
}
