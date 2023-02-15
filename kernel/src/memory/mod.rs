mod allocator;
pub mod page;

use crate::sections;

const PAGE_ORDER: usize = 12;
const PAGE_SIZE: usize = 1 << PAGE_ORDER; // 4 KiB

// TODO: Assuming 128 MiB of memory as qemu uses that.
const TOTAL_PAGES: usize = (128 * (1024 * 1024)) / PAGE_SIZE;

pub unsafe fn init() {
    println!("initializing allocator...");
    allocator::init();
    println!("allocator initialized");

    println!("mapping kernel sections...");
    map_kernel_sections();
    println!("succesfully mapped kernel sections");

    // TODO: This must be called for every hart, will need to be moved later
    println!("starting paging...");
    page::init();
    println!("paging enabled");
}

unsafe fn map_kernel_sections() {
    // TODO: move
    const UART_ADDR: usize = 0x1000_0000;
    const SIFIVE_TEST_REG: usize = 0x100000;

    // Some funky unsafe syntax to bypass the borrow checker
    let root_table: &mut page::Table = &mut *(page::root_table() as *mut _);

    // Map all of our sections
    root_table.identity_map(
        sections::TEXT_START(),
        sections::TEXT_END(),
        page::EntryAttributes::RX as usize,
    );

    root_table.identity_map(
        sections::RODATA_START(),
        sections::RODATA_END(),
        page::EntryAttributes::RX as usize,
    );

    root_table.identity_map(
        sections::DATA_START(),
        sections::DATA_END(),
        page::EntryAttributes::RW as usize,
    );

    root_table.identity_map(
        sections::BSS_START(),
        sections::BSS_END(),
        page::EntryAttributes::RW as usize,
    );

    root_table.identity_map(
        sections::STACK_START(),
        sections::STACK_END(),
        page::EntryAttributes::RW as usize,
    );

    root_table.identity_map(
        sections::HEAP_START(),
        sections::HEAP_END(),
        page::EntryAttributes::RW as usize,
    );

    root_table.kernel_map(UART_ADDR, UART_ADDR, page::EntryAttributes::RW as usize);

    root_table.kernel_map(
        SIFIVE_TEST_REG,
        SIFIVE_TEST_REG,
        page::EntryAttributes::RW as usize,
    );
}

/// Align an address to upper bound according to specified order.
const fn align_up(val: usize, order: usize) -> usize {
    let o = (1 << order) - 1;
    (val + o) & !o
}

/// Align an address to lower bound according to specified order.
const fn align_down(val: usize, order: usize) -> usize {
    val & !((1 << order) - 1)
}

/// Align an address to the end of a page.
const fn align_page_up(val: usize) -> usize {
    align_up(val, PAGE_ORDER)
}

/// Align an address to the begin of a page.
const fn align_page_down(val: usize) -> usize {
    align_down(val, PAGE_ORDER)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn page_align_up() {
        assert_eq!(align_page_up(0), 0);
        assert_eq!(align_page_up(1), PAGE_SIZE);
        assert_eq!(align_page_up(PAGE_SIZE), PAGE_SIZE);
        assert_eq!(align_page_up(PAGE_SIZE + 1), PAGE_SIZE * 2);
        assert_eq!(align_page_up(PAGE_SIZE * 2), PAGE_SIZE * 2);
        assert_eq!(align_page_up((PAGE_SIZE * 2) - 1), PAGE_SIZE * 2);
    }

    #[test_case]
    fn page_align_down() {
        assert_eq!(align_page_down(0), 0);
        assert_eq!(align_page_down(1), 0);
        assert_eq!(align_page_down(PAGE_SIZE - 1), 0);
        assert_eq!(align_page_down(PAGE_SIZE), PAGE_SIZE);
        assert_eq!(align_page_down(PAGE_SIZE + 512), PAGE_SIZE);
        assert_eq!(align_page_down(PAGE_SIZE * 2), PAGE_SIZE * 2);
        assert_eq!(align_page_down((PAGE_SIZE * 2) - 1), PAGE_SIZE);
    }
}
