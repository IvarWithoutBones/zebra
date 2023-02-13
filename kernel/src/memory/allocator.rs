use {
    super::{align_page_up, PAGE_SIZE, TOTAL_PAGES},
    crate::spinlock::Spinlock,
};

struct Allocator {
    /// Keeps track of which pages are free.
    pages: [usize; TOTAL_PAGES],
    /// The base address of the heap.
    base_addr: usize,
}

impl Allocator {
    const fn new() -> Self {
        Self {
            pages: [0; TOTAL_PAGES],
            // Must be initialized later.
            base_addr: 0,
        }
    }

    const fn offset_addr_of(&self, page: usize) -> usize {
        self.base_addr + (page * PAGE_SIZE)
    }

    const unsafe fn offset_id_of(&self, page: usize) -> *mut u8 {
        self.offset_addr_of(page) as *mut u8
    }

    fn offset_page_of(&self, ptr: *mut u8) -> usize {
        (ptr as usize - self.base_addr) / PAGE_SIZE
    }

    fn allocate(&mut self, size: usize) -> Option<*mut u8> {
        let pages_needed = align_page_up(size) / PAGE_SIZE;
        for i in 0..TOTAL_PAGES {
            if self.pages[i] != 0 {
                continue;
            }

            // Check if the gap is big enough
            let mut found = true;
            for j in 0..pages_needed {
                if self.pages[i + j] != 0 {
                    found = false;
                    break;
                }
            }

            if found {
                for j in 0..pages_needed {
                    // TODO: would `pages_needed - j` make more sense?
                    self.pages[i + j] = pages_needed;
                }

                return Some(unsafe { self.offset_id_of(i) });
            }
        }

        // Not enough free pages were found.
        None
    }

    fn deallocate(&mut self, ptr: *mut u8) {
        let id = self.offset_page_of(ptr);
        let page_stride = self.pages[id];
        for i in 0..page_stride {
            self.pages[id + i] = 0;
        }
    }
}

static ALLOCATOR: Spinlock<Allocator> = Spinlock::new(Allocator::new());

pub unsafe fn init() {
    use super::{
        page::{EntryAttributes, Table, KERNEL_PAGE_TABLE},
        symbols,
    };

    // TODO: move
    const UART_ADDR: usize = 0x1000_0000;

    // Initialize the heap.
    let mut alloc = ALLOCATOR.lock();
    alloc.base_addr = align_page_up(symbols::HEAP_START());
    for page in alloc.pages.iter_mut() {
        *page = 0;
    }

    // Some funky unsafe syntax to bypass the borrow checker
    let page_table: &mut Table = &mut *(&KERNEL_PAGE_TABLE as *const _ as *mut _);

    // Map all of our sections
    page_table.id_map_range(
        symbols::TEXT_START(),
        symbols::TEXT_END(),
        EntryAttributes::RX as usize,
    );

    page_table.id_map_range(
        symbols::RODATA_START(),
        symbols::RODATA_END(),
        EntryAttributes::RX as usize,
    );

    page_table.id_map_range(
        symbols::DATA_START(),
        symbols::DATA_END(),
        EntryAttributes::RW as usize,
    );

    page_table.id_map_range(
        symbols::BSS_START(),
        symbols::BSS_END(),
        EntryAttributes::RW as usize,
    );

    page_table.id_map_range(
        symbols::STACK_START(),
        symbols::STACK_END(),
        EntryAttributes::RW as usize,
    );

    page_table.id_map_range(
        symbols::HEAP_START(),
        symbols::HEAP_END(),
        EntryAttributes::RW as usize,
    );

    // Map the UART
    page_table.kernel_map(UART_ADDR, UART_ADDR, EntryAttributes::RW as usize);
}
