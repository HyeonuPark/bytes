use crate::refcount_buf::{Parts, RefCountBuf, RefCountBufError, RefCountPtr};
use core::{ptr, slice, usize};
use crate::refcount_buf::{ RefCountBuf, Parts };
use alloc::vec::Vec;
// ===== impl StaticVtable =====

pub struct StaticImpl(pub &'static [u8]);

unsafe impl RefCountBuf for StaticImpl {

    fn as_parts(&self) ->  Parts {
        let ptr : *const dyn RefCountBuf = self as &dyn RefCountBuf;
        let data = unsafe { RefCountPtr::new(std::mem::transmute(ptr)) };
        (data,
        self.0.as_ptr(),
        self.0.len(),)
    }

    unsafe fn clone(&self, data: &RefCountPtr, ptr: *const u8, len: usize) -> (&RefCountPtr, *const u8, usize) {
        let slice = slice::from_raw_parts(ptr, len);
        
        (data, slice.as_ptr(), slice.len())
    }

    unsafe fn into_vec(&self, _: &mut RefCountPtr, ptr: *const u8, len: usize) -> Vec<u8> {
        let slice = slice::from_raw_parts(ptr, len);
        slice.to_vec()
    }

    unsafe fn try_resize(
        &self,

        ptr: *const u8,
        len: usize,
        can_alloc: bool,
    ) -> Result<(), RefCountBufError> {
        let data = k  
    }

    //unsafe fn try_into_mut(&self, can_alloc: bool) -> Result<Parts, RefCountBufError> {
    //}

    unsafe fn drop(_: &mut RefCountPtr, _: *const u8, _: usize) {
        // nothing to drop for &'static [u8]
    }
}

