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
use crate::buf::UninitSlice;
use crate::impls::ArcBoxMut;

// The max original capacity value. Any `Bytes` allocated with a greater initial
// capacity will default to this.
const MAX_ORIGINAL_CAPACITY_WIDTH: usize = 17;
// The original capacity algorithm will not take effect unless the originally
// allocated capacity was at least 1kb in size.
const MIN_ORIGINAL_CAPACITY_WIDTH: usize = 10;
// The original capacity is stored in powers of 2 starting at 1kb to a max of
// 64kb. Representing it as such requires only 3 bits of storage.
const ORIGINAL_CAPACITY_MASK: usize = 0b11100;
const ORIGINAL_CAPACITY_OFFSET: usize = 2;

// When the storage is in the `Vec` representation, the pointer can be advanced
// at most this value. This is due to the amount of storage available to track
// the offset is usize - number of KIND bits and number of ORIGINAL_CAPACITY
// bits.
const VEC_POS_OFFSET: usize = 5;
const MAX_VEC_POS: usize = usize::MAX >> VEC_POS_OFFSET;
const NOT_VEC_POS_MASK: usize = 0b11111;

struct VecMut {
    ptr: ptr::NonNull<u8>,
    cap: usize,
    len: usize,
    orig_cap_pos: usize,
}

impl VecMut {
    unsafe fn rebuild_vec(ptr: *mut u8, mut len: usize, mut cap: usize, off: usize) -> Vec<u8> {
        let ptr = ptr.offset(-(off as isize));
        len += off;
        cap += off;

        Vec::from_raw_parts(ptr, len, cap)
    }

    #[inline]
    unsafe fn get_vec_pos(&mut self) -> (usize, usize) {
        (self.orig_cap_pos >> VEC_POS_OFFSET, self.orig_cap_pos)
    }

    #[inline]
    unsafe fn set_vec_pos(&mut self, pos: usize, prev: usize) {
        debug_assert!(pos <= MAX_VEC_POS);

        self.orig_cap_pos = (pos << VEC_POS_OFFSET) | (prev & NOT_VEC_POS_MASK);
    }

    #[inline]
    fn uninit_slice(&mut self) -> &mut UninitSlice {
        unsafe {
            let ptr = self.ptr.as_ptr().add(self.len);
            let len = self.cap - self.len;

            UninitSlice::from_raw_parts_mut(ptr, len)
        }
    }
    
    unsafe fn promote_to_shared(&mut self, ref_cnt: usize) -> Box<dyn RefCountBuf> {
        debug_assert!(ref_cnt == 1 || ref_cnt == 2);

        let original_capacity_repr =
            (self.orig_cap_pos as usize & ORIGINAL_CAPACITY_MASK) >> ORIGINAL_CAPACITY_OFFSET;

        // The vec offset cannot be concurrently mutated, so there
        // should be no danger reading it.
        let off = (self.orig_cap_pos as usize) >> VEC_POS_OFFSET;

        // First, allocate a new `Shared` instance containing the
        // `Vec` fields. It's important to note that `ptr`, `len`,
        // and `cap` cannot be mutated without having `&mut self`.
        // This means that these fields will not be concurrently
        // updated and since the buffer hasn't been promoted to an
        // `Arc`, those three fields still are the components of the
        // vector.
        Box::new(Shared {
            vec: rebuild_vec(self.ptr.as_ptr(), self.len, self.cap, off),
            original_capacity_repr,
            ref_count: AtomicUsize::new(ref_cnt),
        })
    }

    #[inline]
    fn original_capacity_to_repr(cap: usize) -> usize {
        let width = PTR_WIDTH - ((cap >> MIN_ORIGINAL_CAPACITY_WIDTH).leading_zeros() as usize);
        cmp::min(
            width,
            MAX_ORIGINAL_CAPACITY_WIDTH - MIN_ORIGINAL_CAPACITY_WIDTH,
        )
    }

    fn original_capacity_from_repr(repr: usize) -> usize {
        if repr == 0 {
            return 0;
        }

        1 << (repr + (MIN_ORIGINAL_CAPACITY_WIDTH - 1))
    }

    #[inline]
    fn reserve(&mut self, additional: usize) {
    }

    fn reserve_inner(&mut self, additional: usize) {
    }
}


unsafe impl RefCountBuf for VecMut {
    fn slice(&self) -> (*const u8, usize) {
        (self.ptr.as_ptr(), self.len)
    }

    unsafe fn clone(&self, ptr: *const u8, len: usize) -> (Option<Box<dyn RefCountBuf>>, *const u8, usize) {
        self.promote_to_shared(/*ref_count = */ 2);
        let new_inst = ptr::read(self)

    }

    unsafe fn freeze(&self, ptr: *const u8, len: usize) -> Option<Box<dyn RefCountBuf>> {
        let (off, _) = self.get_vec_pos();
        let vec = VecMut::rebuild_vec(self.ptr.as_ptr(), self.len, self.cap, off);
        mem::forget(self);
        let mut b: Bytes = vec.into();
        b.advance(off);
        b
    }

    unsafe fn into_vec(&mut self, ptr: *const u8, len: usize) -> Vec<u8> {
        if self.shared.is_unique() {
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
            self.shared.release();
            v
        }
    }

    unsafe fn drop(&mut self, _ptr: *const u8, _len: usize) {
        self.data.with_mut_ref(|shared| {
            shared.release()
        });
    }
}
