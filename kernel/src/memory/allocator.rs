use super::{align_page_up, sections, PAGE_SIZE, TOTAL_PAGES};
use crate::spinlock::SpinLock;
use core::{
    alloc::{GlobalAlloc, Layout},
    ptr,
};

#[global_allocator]
pub static ALLOCATOR: SpinLock<Allocator> = SpinLock::new(Allocator::new());

unsafe impl GlobalAlloc for SpinLock<Allocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.lock()
            .allocate(layout.size())
            .unwrap_or(ptr::null_mut())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        self.lock().deallocate(ptr);
    }
}

pub struct Allocator {
    /// Keeps track of which pages are free.
    pages: [usize; TOTAL_PAGES],
    /// The base address of the heap.
    base_addr: usize,
    /// Whether we are allowed to allocate or deallocate memory.
    active: bool,
}

impl Allocator {
    const fn new() -> Self {
        Self {
            pages: [0; TOTAL_PAGES],
            active: false,
            // Must be initialized later as we cannot access the heap symbols from a const fn.
            base_addr: 0,
        }
    }

    const fn offset_addr_of(&self, page: usize) -> usize {
        self.base_addr + (page * PAGE_SIZE)
    }

    const unsafe fn offset_id_of(&self, page: usize) -> *mut u8 {
        self.offset_addr_of(page) as *mut u8
    }

    pub fn disable(&mut self) {
        self.active = false;
    }

    pub fn enable(&mut self) {
        self.active = true;
    }

    fn offset_page_of(&self, ptr: *mut u8) -> usize {
        (ptr as usize).saturating_sub(self.base_addr) / PAGE_SIZE
    }

    pub fn allocate(&mut self, size: usize) -> Option<*mut u8> {
        assert!(self.active, "allocator is inactive but allocate was called");
        let pages_needed = align_page_up(size) / PAGE_SIZE;
        for i in 0..TOTAL_PAGES {
            if self.pages[i] != 0 {
                continue;
            }

            // Check if the gap is big enough
            let mut found = true;
            for j in 0..pages_needed {
                if *self.pages.get(i + j)? != 0 {
                    found = false;
                    break;
                }
            }

            if found {
                for j in 0..pages_needed {
                    self.pages[i + j] = pages_needed - j;
                }

                return Some(unsafe { self.offset_id_of(i) });
            }
        }

        // Not enough free pages were found.
        None
    }

    /// Deallocates a pointer.
    pub fn deallocate(&mut self, ptr: *mut u8) {
        assert!(self.active, "allocator is inactive but deallocate was called");
        let id = self.offset_page_of(ptr);
        let page_stride = self.pages.get(id).cloned().unwrap();
        for i in 0..page_stride {
            self.pages[id + i] = 0;
        }
    }

    // pub fn size_of(&self, ptr: *mut u8) -> usize {
    //     let id = self.offset_page_of(ptr);
    //     self.pages[id] * PAGE_SIZE
    // }
}

// TODO: use core::cell::OnceCell once it is stabilized.
pub unsafe fn init() {
    ALLOCATOR.lock_with(|alloc| {
        alloc.base_addr = align_page_up(sections::heap_start());
        alloc.enable();
    });
}
