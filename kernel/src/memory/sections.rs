#![allow(non_snake_case)]

extern "C" {
    static _heap_start: usize;
    static _heap_end: usize;

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

#[inline]
pub fn HEAP_START() -> usize {
    unsafe { &_heap_start as *const _ as _ }
}

#[inline]
pub fn HEAP_END() -> usize {
    unsafe { &_heap_end as *const _ as _ }
}

#[inline]
pub fn TEXT_START() -> usize {
    unsafe { &_text_start as *const _ as _ }
}

#[inline]
pub fn TEXT_END() -> usize {
    unsafe { &_text_end as *const _ as _ }
}

#[inline]
pub fn RODATA_START() -> usize {
    unsafe { &_rodata_start as *const _ as _ }
}

#[inline]
pub fn RODATA_END() -> usize {
    unsafe { &_rodata_end as *const _ as _ }
}

#[inline]
pub fn DATA_START() -> usize {
    unsafe { &_data_start as *const _ as _ }
}

#[inline]
pub fn DATA_END() -> usize {
    unsafe { &_data_end as *const _ as _ }
}

#[inline]
pub fn BSS_START() -> usize {
    unsafe { &_bss_start as *const _ as _ }
}

#[inline]
pub fn BSS_END() -> usize {
    unsafe { &_bss_end as *const _ as _ }
}

#[inline]
pub fn STACK_START() -> usize {
    unsafe { &_stack_start as *const _ as _ }
}

#[inline]
pub fn STACK_END() -> usize {
    unsafe { &_stack_end as *const _ as _ }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Unknown symbols are ignored if the functions are not used, this gives a linker error instead.
    #[test_case]
    fn symbols_exist() {
        assert!(HEAP_START() > 0);
        assert!(HEAP_END() > 0);
        assert!(TEXT_START() > 0);
        assert!(TEXT_END() > 0);
        assert!(RODATA_START() > 0);
        assert!(RODATA_END() > 0);
        assert!(DATA_START() > 0);
        assert!(DATA_END() > 0);
        assert!(BSS_START() > 0);
        assert!(BSS_END() > 0);
        assert!(STACK_START() > 0);
        assert!(STACK_END() > 0);
    }
}
