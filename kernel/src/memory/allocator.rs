use {
    super::{
        align_page_up,
        page::{EntryAttributes, Table, KERNEL_PAGE_TABLE},
        symbols, PAGE_SIZE, TOTAL_PAGES,
    },
    crate::spinlock::Spinlock,
    core::{
        alloc::{GlobalAlloc, Layout},
        arch::asm,
        ptr,
    },
};

#[global_allocator]
static ALLOCATOR: Spinlock<Allocator> = Spinlock::new(Allocator::new());

unsafe impl GlobalAlloc for Spinlock<Allocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.lock()
            .allocate(layout.size())
            .unwrap_or(ptr::null_mut())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        self.lock().deallocate(ptr);
    }
}

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

// TODO: should probably move this to the parent module
// NOTE: Before this is called heap allocations will deadlock the kernel!
pub unsafe fn init() {
    // TODO: move
    const UART_ADDR: usize = 0x1000_0000;
    const SIFIVE_TEST_REG: usize = 0x100000;

    println!("initializing allocator...");

    ALLOCATOR.lock_with(|alloc| {
        alloc.base_addr = align_page_up(symbols::HEAP_START());
    });

    // Some funky unsafe syntax to bypass the borrow checker
    let page_table: &mut Table = &mut *(&KERNEL_PAGE_TABLE as *const _ as *mut _);

    println!("mapping kernel sections...");

    // Map all of our sections
    page_table.identity_map(
        symbols::TEXT_START(),
        symbols::TEXT_END(),
        EntryAttributes::RX as usize,
    );

    page_table.identity_map(
        symbols::RODATA_START(),
        symbols::RODATA_END(),
        EntryAttributes::RX as usize,
    );

    page_table.identity_map(
        symbols::DATA_START(),
        symbols::DATA_END(),
        EntryAttributes::RW as usize,
    );

    page_table.identity_map(
        symbols::BSS_START(),
        symbols::BSS_END(),
        EntryAttributes::RW as usize,
    );

    page_table.identity_map(
        symbols::STACK_START(),
        symbols::STACK_END(),
        EntryAttributes::RW as usize,
    );

    page_table.identity_map(
        symbols::HEAP_START(),
        symbols::HEAP_END(),
        EntryAttributes::RW as usize,
    );

    page_table.kernel_map(UART_ADDR, UART_ADDR, EntryAttributes::RW as usize);
    page_table.kernel_map(
        SIFIVE_TEST_REG,
        SIFIVE_TEST_REG,
        EntryAttributes::RW as usize,
    );

    println!("succesfully mapped kernel sections");
    init_paging();
    println!("allocator initialized");
}

pub fn init_paging() {
    println!("initializing paging...");
    let root = &KERNEL_PAGE_TABLE as *const Table as usize;
    let satp = {
        let mode = 8; // Sv39
        (root / PAGE_SIZE) | (mode << 60)
    };

    unsafe {
        // NOTE: `sfence.vma` is not required, the TLB will be freshly populated on the next memory access
        asm!("csrw satp, {}", in(reg) satp);
    }

    println!("paging enabled");
}
