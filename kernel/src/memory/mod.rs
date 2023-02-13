#![allow(dead_code)]

pub mod allocator;
mod page;

const PAGE_ORDER: usize = 12;
const PAGE_SIZE: usize = 1 << PAGE_ORDER; // 4 KiB

// TODO: Assuming 128 MiB of memory as qemu uses that.
const TOTAL_PAGES: usize = (128 * (1024 * 1024)) / PAGE_SIZE;

/// Align an address to upper bound according to specified order.
pub const fn align_up(val: usize, order: usize) -> usize {
    let o = (1 << order) - 1;
    (val + o) & !o
}

/// Align an address to lower bound according to specified order.
pub const fn align_down(val: usize, order: usize) -> usize {
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

/// Helpers to access symbols defined in the linker script (`link.ld`).
#[allow(non_snake_case)]
mod symbols {
    extern "C" {
        static _heap_start: usize;
        static _heap_size: usize; // TODO: inconsistent

        static _text_start: usize;
        static _text_end: usize;

        static _rodata_start: usize;
        static _rodata_end: usize;

        static _data_start: usize;
        static _data_end: usize;

        static _bss_start: usize;
        static _bss_end: usize;

        static _stack_start: usize;
        static _stack_end: usize;
    }

    #[inline(always)]
    pub fn HEAP_START() -> usize {
        unsafe { &_heap_start as *const _ as _ }
    }

    #[inline(always)]
    pub fn HEAP_END() -> usize {
        HEAP_START() + unsafe { &_heap_size as *const _ as usize }
    }

    #[inline(always)]
    pub fn TEXT_START() -> usize {
        unsafe { &_text_start as *const _ as _ }
    }

    #[inline(always)]
    pub fn TEXT_END() -> usize {
        unsafe { &_text_end as *const _ as _ }
    }

    #[inline(always)]
    pub fn RODATA_START() -> usize {
        unsafe { &_rodata_start as *const _ as _ }
    }

    pub fn RODATA_END() -> usize {
        unsafe { &_rodata_end as *const _ as _ }
    }

    #[inline(always)]
    pub fn DATA_START() -> usize {
        unsafe { &_data_start as *const _ as _ }
    }

    #[inline(always)]
    pub fn DATA_END() -> usize {
        unsafe { &_data_end as *const _ as _ }
    }

    #[inline(always)]
    pub fn BSS_START() -> usize {
        unsafe { &_bss_start as *const _ as _ }
    }

    #[inline(always)]
    pub fn BSS_END() -> usize {
        unsafe { &_bss_end as *const _ as _ }
    }

    #[inline(always)]
    pub fn STACK_START() -> usize {
        unsafe { &_stack_start as *const _ as _ }
    }

    #[inline(always)]
    pub fn STACK_END() -> usize {
        unsafe { &_stack_end as *const _ as _ }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Unknown symbols are ignored if the functions are not used, this gives a linker error instead.
    #[test_case]
    fn symbols_exist() {
        assert!(symbols::HEAP_START() > 0);
        assert!(symbols::HEAP_END() > 0);
        assert!(symbols::TEXT_START() > 0);
        assert!(symbols::TEXT_END() > 0);
        assert!(symbols::RODATA_START() > 0);
        assert!(symbols::RODATA_END() > 0);
        assert!(symbols::DATA_START() > 0);
        assert!(symbols::DATA_END() > 0);
        assert!(symbols::BSS_START() > 0);
        assert!(symbols::BSS_END() > 0);
        assert!(symbols::STACK_START() > 0);
        assert!(symbols::STACK_END() > 0);
    }

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
