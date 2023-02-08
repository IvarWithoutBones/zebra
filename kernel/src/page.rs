#![allow(dead_code)]

extern "C" {
    static HEAP_START: usize;
    static HEAP_SIZE: usize;
}

const PAGE_SIZE: usize = 0x1000;

fn heap_start() -> usize {
    unsafe { HEAP_START }
}

fn total_pages() -> usize {
    unsafe { HEAP_SIZE / PAGE_SIZE }
}

fn allocation_start() -> usize {
    let result = (heap_start() + total_pages()) * core::mem::size_of::<PageHeader>();
    align(result, 12)
}

pub const fn align(val: usize, order: usize) -> usize {
    let tmp = (1 << order) - 1;
    (val + tmp) & !tmp
}

#[derive(PartialEq, Eq)]
#[repr(u8)]
enum PageHeader {
    Empty,
    Allocated,
    Last,
}

impl PageHeader {
    fn is_empty(&self) -> bool {
        matches!(self, PageHeader::Empty)
    }
}

pub fn allocate(pages: usize) -> Option<*mut u8> {
    if pages == 0 {
        return None;
    }

    let ptr = heap_start() as *mut PageHeader;
    for idx in 0..(total_pages().saturating_sub(pages)) {
        let mut found = false;

        unsafe {
            if (*ptr.add(idx)).is_empty() {
                found = true;
                for next_pages in idx..(pages + idx) {
                    if !(*ptr.add(next_pages)).is_empty() {
                        found = false;
                        break;
                    }
                }
            }
        }

        if found {
            unsafe {
                for page_idx in idx..(pages + idx) {
                    *ptr.add(page_idx) = PageHeader::Allocated;
                }
                *ptr.add(idx + (pages - 1)) = PageHeader::Last;
            }

            return Some((allocation_start() + (idx * PAGE_SIZE)) as *mut u8);
        }
    }
    None
}

pub fn deallocate(ptr: *mut u8) {
    assert!(!ptr.is_null());

    unsafe {
        let addr = HEAP_START + ((ptr as usize - allocation_start()) / PAGE_SIZE);
        assert!(addr >= HEAP_START && addr < (HEAP_START + HEAP_SIZE));

        let mut header = addr as *mut PageHeader;
        while *header == PageHeader::Allocated {
            *header = PageHeader::Empty;
            header = header.add(1);
        }

        assert!(*header == PageHeader::Last);
        *header = PageHeader::Empty;
    }
}

pub fn print() {
    unsafe {
        let page_start = HEAP_START as *mut PageHeader;
        let page_end = page_start.add(total_pages());

        let alloc_start = allocation_start();
        let alloc_end = alloc_start + (total_pages() * PAGE_SIZE);

        println!(
            "table:\t{:#X} -> {:#X}",
            page_start as usize, page_end as usize
        );
        println!("mem:\t{:#X} -> {:#X}\n", alloc_start, alloc_end);

        let mut pages_in_alloc = 0;
        let mut pages = 0;
        for i in 0..total_pages() {
            let page = page_start.add(i);
            let page_addr = alloc_start + (i * PAGE_SIZE);
            let page_end = page_addr + PAGE_SIZE;

            match *page {
                PageHeader::Allocated => {
                    pages_in_alloc += 1;
                    pages += 1;
                }
                PageHeader::Last => {
                    println!(
                        "{:#X} -> {:#X}:\t{} page(s)",
                        page_addr,
                        page_end,
                        pages_in_alloc + 1
                    );
                    pages_in_alloc = 0;
                    pages += 1;
                }
                PageHeader::Empty => {}
            }
        }

        println!("allocated {} page(s)", pages);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn alloc() {
        let a = allocate(20).unwrap();
        let b = allocate(20).unwrap();
        assert_ne!(a, b);
        deallocate(a);
        deallocate(b);
    }

    #[test_case]
    fn dealloc() {
        let a = allocate(20).unwrap();
        allocate(10).unwrap();
        deallocate(a);
        let b = allocate(20).unwrap();
        assert_eq!(a, b);
        deallocate(b);
    }

    #[test_case]
    fn alloc_dealloc() {
        let a = allocate(20).unwrap();
        let b = allocate(20).unwrap();
        deallocate(a);
        deallocate(b);
        let c = allocate(20).unwrap();
        let d = allocate(20).unwrap();
        assert_eq!(a, c);
        assert_eq!(b, d);
        deallocate(c);
        deallocate(d);
    }

    #[test_case]
    fn out_of_memory() {
        let ptr = allocate(total_pages() + 1);
        assert!(ptr.is_none());
    }

    #[test_case]
    fn zero_pages() {
        let ptr = allocate(0);
        assert!(ptr.is_none());
    }

    #[test_case]
    fn one_page() {
        let ptr = allocate(1).unwrap();
        assert_eq!(ptr as usize, allocation_start());
        deallocate(ptr);
    }
}
