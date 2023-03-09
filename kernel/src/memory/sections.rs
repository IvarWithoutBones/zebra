use {
    super::{page, PAGE_SIZE},
    crate::{power, trap::plic, uart},
};

/// Generate a safe wrapper to access a linker section.
macro_rules! section {
    ($fn_name: ident, $link_name: ident) => {
        extern "C" {
            static $link_name: usize;
        }

        /// Returns the address of the corresponding linker section.
        #[inline(always)]
        pub fn $fn_name() -> usize {
            unsafe { &$link_name as *const _ as _ }
        }
    };
}

section!(heap_start, _heap_start);
section!(heap_end, _heap_end);

section!(text_start, _text_start);
section!(text_end, _text_end);

section!(rodata_start, _rodata_start);
section!(rodata_end, _rodata_end);

section!(data_start, _data_start);
section!(data_end, _data_end);

section!(bss_start, _bss_start);
section!(bss_end, _bss_end);

section!(stack_start, _stack_start);
section!(stack_end, _stack_end);

section!(trampoline_start, _trampoline_start);
section!(trampoline_end, _trampoline_end);

/// Map the trampoline section into the given page table.
pub fn map_trampoline(page_table: &mut page::Table) {
    assert!(trampoline_end() - trampoline_start() <= PAGE_SIZE);

    // TODO: really should not be identity mapped
    page_table.map_page(
        trampoline_start(),
        trampoline_start(),
        page::EntryAttributes::ReadExecute,
    );
}

/// Map the kernel sections into the given page table.
pub fn map_kernel(page_table: &mut page::Table) {
    // Map the linker sections
    map_trampoline(page_table);
    page_table.identity_map(
        rodata_start(),
        rodata_end(),
        page::EntryAttributes::ReadExecute,
    );

    page_table.identity_map(text_start(), text_end(), page::EntryAttributes::ReadExecute);
    page_table.identity_map(data_start(), data_end(), page::EntryAttributes::ReadWrite);
    page_table.identity_map(bss_start(), bss_end(), page::EntryAttributes::ReadWrite);
    page_table.identity_map(stack_start(), stack_end(), page::EntryAttributes::ReadWrite);
    page_table.identity_map(heap_start(), heap_end(), page::EntryAttributes::ReadWrite);

    // Map peripherals devices. TODO: Could be prettier.
    page_table.identity_map(
        plic::BASE_ADDR,
        plic::BASE_ADDR + 0x400000,
        page::EntryAttributes::ReadWrite,
    );

    page_table.map_page(
        uart::BASE_ADDR,
        uart::BASE_ADDR,
        page::EntryAttributes::ReadWrite,
    );

    page_table.map_page(
        power::BASE_ADDR,
        power::BASE_ADDR,
        page::EntryAttributes::ReadWrite,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    // Unknown symbols are ignored if the functions are not used, this gives a linker error instead.
    #[test_case]
    fn symbols_exist() {
        assert!(heap_start() > 0);
        assert!(heap_end() > 0);
        assert!(text_start() > 0);
        assert!(text_end() > 0);
        assert!(rodata_start() > 0);
        assert!(rodata_end() > 0);
        assert!(data_start() > 0);
        assert!(data_end() > 0);
        assert!(bss_start() > 0);
        assert!(bss_end() > 0);
        assert!(stack_start() > 0);
        assert!(stack_end() > 0);
        assert!(trampoline_start() > 0);
        assert!(trampoline_end() > 0);
    }
}
