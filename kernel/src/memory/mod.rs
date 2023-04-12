mod allocator;
pub mod page;
pub mod sections;

use crate::spinlock::SpinlockGuard;

const PAGE_ORDER: usize = 12;
pub const PAGE_SIZE: usize = 1 << PAGE_ORDER; // 4 KiB

// TODO: Assuming 128 MiB of memory as qemu uses that.
const TOTAL_PAGES: usize = (128 * (1024 * 1024)) / PAGE_SIZE;

pub fn allocator() -> SpinlockGuard<'static, allocator::Allocator> {
    allocator::ALLOCATOR.lock()
}

pub unsafe fn init() {
    println!("initializing allocator...");
    allocator::init();
    println!("allocator initialized");

    println!("mapping kernel sections...");
    // Some funky unsafe magic to get around the borrow checker
    let mut root_table = page::root_table();
    sections::map_kernel(&mut root_table);
    println!("succesfully mapped kernel sections");

    // TODO: This must be called for every hart, will need to be moved later
    println!("enabling paging...");
    page::init(&root_table);
    println!("paging enabled");
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
pub const fn align_page_up(val: usize) -> usize {
    align_up(val, PAGE_ORDER)
}

/// Align an address to the begin of a page.
pub const fn align_page_down(val: usize) -> usize {
    align_down(val, PAGE_ORDER)
}

pub const fn pages_needed(size: usize) -> usize {
    align_page_up(size) / PAGE_SIZE
}

pub fn page_offsets(size: usize) -> impl Iterator<Item = usize> {
    (0..pages_needed(size)).map(|i| i * PAGE_SIZE)
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
