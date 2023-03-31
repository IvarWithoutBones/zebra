use core::alloc::{GlobalAlloc, Layout};

struct Allocator;

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        crate::syscall::allocate(layout.size())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        crate::syscall::deallocate(ptr);
    }
}

#[global_allocator]
static mut ALLOCATOR: Allocator = Allocator;
