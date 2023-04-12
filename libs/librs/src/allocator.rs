use core::alloc::{GlobalAlloc, Layout};
use crate::syscall;

struct Allocator;

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        syscall::allocate(layout.size())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        syscall::deallocate(ptr);
    }
}

#[global_allocator]
static mut ALLOCATOR: Allocator = Allocator;
