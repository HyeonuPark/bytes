use crate::refcount_buf::{RefCountBuf, RefCountBufError, RefCountPtr};
use core::{ptr, slice, usize};
use alloc::{ boxed::Box, vec::Vec };

// ===== impl StaticVtable =====

pub struct StaticImpl(pub &'static [u8]);

unsafe impl RefCountBuf for StaticImpl {

    fn slice(&self) -> (*const u8, usize) {
        let ptr : *const dyn RefCountBuf = self as &dyn RefCountBuf;
        (self.0.as_ptr(),
        self.0.len(),)
    }

    unsafe fn clone(&self, ptr: *const u8, len: usize) -> (Option<Box<dyn RefCountBuf>>, *const u8, usize) {
        let slice = slice::from_raw_parts(ptr, len);
        
        (None, slice.as_ptr(), slice.len())
    }

    unsafe fn into_vec(&mut self, ptr: *const u8, len: usize) -> Vec<u8> {
        let slice = slice::from_raw_parts(ptr, len);
        slice.to_vec()
    }

    unsafe fn try_resize(
        &self,
        ptr: *const u8,
        len: usize,
        can_alloc: bool,
    ) -> Result<Option<Box<dyn RefCountBuf>>, RefCountBufError> {
        Ok(None) 
    }

    unsafe fn try_into_mut(
        &self,
        ptr: *const u8,
        len: usize,
        can_alloc: bool,
    ) -> Result<Option<Box<dyn RefCountBuf>>, RefCountBufError> {
        Ok(None) 
    }

    unsafe fn drop(&mut self, _: *const u8, _: usize) {
        // nothing to drop for &'static [u8]
    }
}

