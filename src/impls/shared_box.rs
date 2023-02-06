#[allow(unused)]
use crate::loom::sync::atomic::AtomicMut;
use crate::loom::sync::atomic::{AtomicUsize, Ordering};
use crate::refcount_buf::RefCountBuf;
use alloc::{
    alloc::{dealloc, Layout},
    boxed::Box,
    vec::Vec,
};
use core::{mem, ptr, slice, usize};

// ===== impl SharedBoxVtable =====

pub struct SharedBoxInner {
    // Holds arguments to dealloc upon Drop, but otherwise doesn't use them
    pub(crate) buf: *mut u8,
    pub(crate) cap: usize,
    pub(crate) ref_cnt: AtomicUsize,
}

impl Drop for SharedBoxInner {
    fn drop(&mut self) {
        unsafe { dealloc(self.buf, Layout::from_size_align(self.cap, 1).unwrap()) }
    }
}

struct SharedBox {
    inner: *mut SharedBoxInner, 
    ptr: *const u8,
    len: usize,
}

impl SharedBox {
    pub fn new(buf: *mut u8, cap: usize, ref_cnt: AtomicUsize, ptr: *const u8, len: usize) -> SharedBox {
        let sbi = Box::new(SharedBoxInner {
            buf, cap, ref_cnt,
        });
        SharedBox {
            inner: Box::into_raw(sbi),
            ptr,
            len,
        }
    }

    pub fn release_inner(&mut self) {
        // `SharedBox` storage... follow the drop steps from Arc.
        let inner = unsafe { &*self.inner };
        if inner.ref_cnt.fetch_sub(1, Ordering::Release) != 1 {
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
        inner.ref_cnt.load(Ordering::Acquire);

        // Drop the data
        drop(unsafe { Box::from_raw(self.inner) });
    }
}

unsafe impl RefCountBuf for SharedBox {
    fn slice(&self) -> (*const u8, usize) {
        (self.ptr, self.len)
    }

    unsafe fn clone(
        &self,
        ptr: *const u8,
        len: usize,
    ) -> (Option<Box<dyn RefCountBuf>>, *const u8, usize) {
        let old_size = (*self.inner).ref_cnt.fetch_add(1, Ordering::Relaxed);

        if old_size > usize::MAX >> 1 {
            crate::abort();
        }

        (None, ptr, len)
    }

    unsafe fn into_vec(&mut self, ptr: *const u8, len: usize) -> Vec<u8> {
        let inner = unsafe { &*self.inner };
        if inner 
            .ref_cnt
            .compare_exchange(1, 0, Ordering::AcqRel, Ordering::Relaxed)
            .is_ok()
        {
            let buf = inner.buf;
            let cap = inner.cap;

            // Copy back buffer
            ptr::copy(ptr, buf, len);

            Vec::from_raw_parts(buf, len, cap)
        } else {
            let v = slice::from_raw_parts(ptr, len).to_vec();
            self.release_inner(); 
            v
        }
    }

    unsafe fn drop(&mut self, _ptr: *const u8, _len: usize) {
        self.release_inner() 
    }
}

