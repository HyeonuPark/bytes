use super::{SharedBox, SharedBoxImpl};
#[allow(unused)]
use crate::loom::sync::atomic::{AtomicMut, AtomicUsize, Ordering};
use crate::refcount_buf::{RefCountBuf, RefCountBufError};
use alloc::{
    alloc::{dealloc, Layout},
    boxed::Box,
    vec::Vec,
};
use core::{mem, ptr, usize};

pub struct BoxedSlice {
    cap: usize,
    ptr: *const u8,
}

impl BoxedSlice {
    pub fn new(buf: Box<[u8]>) -> Self {
        let cap = buf.len();
        let ptr = buf.as_ptr();
        mem::forget(buf);
        BoxedSlice { cap, ptr }
    }
}

unsafe impl RefCountBuf for BoxedSlice {
    fn slice(&self) -> (*const u8, usize) {
        let ptr = self.ptr;
        let len = self.cap;
        (ptr, len)
    }

    unsafe fn clone(
        &self,
        ptr: *const u8,
        len: usize,
    ) -> (Option<Box<dyn RefCountBuf>>, *const u8, usize) {
        let cap = self.cap;
        let buf: *mut u8 = self.ptr.cast_mut();
        let shared = Box::new(SharedBox {
            buf,
            cap,
            // Initialize refcount to 2. One for this reference, and one
            // for the new clone
            ref_cnt: AtomicUsize::new(2),
        });
        (Some(shared), ptr, len)
    }

    unsafe fn try_resize(
        &self,
        ptr: *const u8,
        len: usize,
        _can_alloc: bool,
    ) -> Result<Option<Box<dyn RefCountBuf>>, RefCountBufError> {
        // The BoxedSlice "promotable" vtables do not store the capacity,
        // so we cannot truncate while using this repr. We *have* to
        // promote using `clone` so the capacity can be stored.
        drop(self.clone(ptr, len));
        Ok(None)
    }

    unsafe fn into_vec(&mut self, ptr: *const u8, len: usize) -> Vec<u8> {
        let cap = self.cap;
        let buf: *mut u8 = self.ptr.cast_mut();
        self.drop(ptr, len);
        // Copy back buffer
        ptr::copy(ptr, buf, len);
        Vec::from_raw_parts(buf, len, cap)
    }

    unsafe fn drop(&mut self, _ptr: *const u8, _len: usize) {
        let buf = self.ptr;
        let len = self.cap;
        let _b = unsafe { Box::from_raw(buf as *mut [u8; 1]) };
    }
}
