#![allow(dead_code)]

use {
    arbitrary_int::{u12, u26, u9},
    bitbybit::bitfield,
    core::{mem::size_of, ops::Range},
};

extern "C" {
    static HEAP_START: usize;
    static HEAP_SIZE: usize;
}

const PAGE_SIZE: usize = 0x1000;

#[inline(always)]
fn heap_start() -> usize {
    unsafe { HEAP_START }
}

#[inline(always)]
fn heap_size() -> usize {
    unsafe { HEAP_SIZE }
}

#[inline(always)]
fn total_pages() -> usize {
    heap_size() / PAGE_SIZE
}

fn allocation_start() -> usize {
    let result = (heap_start() + total_pages()) * size_of::<PageHeader>();
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

pub fn allocate(size: usize) -> Option<*mut u8> {
    if size == 0 {
        return None;
    }

    let ptr = heap_start() as *mut PageHeader;
    for i in 0..(total_pages().saturating_sub(size)) {
        let mut found = false;

        if unsafe { (*ptr.add(i)).is_empty() } {
            found = true;
            for next_pages in i..(size + i) {
                // Check if there is enough space
                if unsafe { !(*ptr.add(next_pages)).is_empty() } {
                    found = false;
                    break;
                }
            }
        }

        if found {
            for j in i..(size + i) {
                unsafe { *ptr.add(j) = PageHeader::Allocated };
            }
            unsafe { *ptr.add(i + (size - 1)) = PageHeader::Last };

            return Some((allocation_start() + (i * PAGE_SIZE)) as *mut u8);
        }
    }
    None
}

pub fn zero_allocate(pages: usize) -> Option<*mut u8> {
    let ptr = allocate(pages)?;
    let size = PAGE_SIZE * pages;
    unsafe {
        core::ptr::write_bytes(ptr, 0, size);
    }
    Some(ptr)
}

pub fn deallocate(ptr: *mut u8) {
    assert!(!ptr.is_null());

    let addr = heap_start() + ((ptr as usize - allocation_start()) / PAGE_SIZE);
    assert!(addr >= heap_start() && addr < (heap_start() + heap_size()));
    let mut header = addr as *mut PageHeader;

    unsafe {
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

struct Table {
    entries: [Entry; 512],
}

impl Default for Table {
    fn default() -> Self {
        Self {
            entries: [Entry::default(); 512],
        }
    }
}

#[bitfield(u64, default: 0)]
struct Entry {
    #[bit(0, rw)]
    valid: bool,
    #[bit(1, rw)]
    read: bool,
    #[bit(2, rw)]
    write: bool,
    #[bit(3, rw)]
    execute: bool,
    #[bit(4, rw)]
    user: bool,
    #[bit(5, rw)]
    global: bool,
    #[bit(6, rw)]
    accessed: bool,
    #[bit(7, rw)]
    dirty: bool,
}

impl Entry {
    const fn new_valid(ptr: u64) -> Self {
        Self::new_with_raw_value(ptr).with_valid(true)
    }

    const fn is_leaf(&self) -> bool {
        self.read() || self.write() || self.execute()
    }

    const fn is_branch(&self) -> bool {
        !self.is_leaf()
    }

    // TODO: this is ugly
    const fn physical_address(&self) -> usize {
        (self.raw_value() & !0x3ff) as usize
    }
}

#[bitfield(u64, default: 0)]
struct VirtualPageNumber {
    #[bits(0..=11, r)]
    offset: u12,
    #[bits(12..=20, r)]
    level_0: u9,
    #[bits(21..=29, r)]
    level_1: u9,
    #[bits(30..=38, r)]
    level_2: u9,
}

impl VirtualPageNumber {
    const fn entry_at(&self, level: usize) -> usize {
        match level {
            0 => self.level_0().value() as _,
            1 => self.level_1().value() as _,
            2 => self.level_2().value() as _,
            _ => unreachable!(),
        }
    }
}

#[bitfield(u64, default: 0)]
struct PhysicalPageNumber {
    #[bits(12..=20, r)]
    level_0: u9,
    #[bits(21..=29, r)]
    level_1: u9,
    #[bits(30..=55, r)]
    level_2: u26,
}

impl PhysicalPageNumber {
    const fn entry_at(&self, level: usize) -> usize {
        match level {
            0 => self.level_0().value() as _,
            1 => self.level_1().value() as _,
            2 => self.level_2().value() as _,
            _ => unreachable!(),
        }
    }
}

fn map(root: &mut Table, vaddr: u64, paddr: u64, flags: u64, level: usize) {
    assert!(level <= 2);
    assert!(flags & 0xe != 0);
    assert!(vaddr % PAGE_SIZE as u64 == 0);
    assert!(paddr % PAGE_SIZE as u64 == 0);

    let vpn = VirtualPageNumber::new_with_raw_value(vaddr);
    let ppn = PhysicalPageNumber::new_with_raw_value(paddr);

    let mut virt = &mut root.entries[vpn.entry_at(level)];

    // Search for a leaf, allocating pages as needed
    for i in (level..2).rev() {
        if !virt.valid() {
            let page = zero_allocate(1).unwrap();
            *virt = Entry::new_valid(page as u64);
        }

        let entry = virt.physical_address() as *mut Entry;
        virt = unsafe { entry.add(vpn.entry_at(i) as usize).as_mut().unwrap() };
    }

    *virt = Entry::new_valid(ppn.raw_value() | flags);
}

fn unmap(root: &mut Table) {
    for level_2 in root.entries.iter() {
        if level_2.valid() && level_2.is_branch() {
            let table_level_1 = {
                let table = level_2.physical_address() as *mut Table;
                unsafe { &mut *table }
            };

            for level_1 in table_level_1.entries.iter() {
                if level_1.valid() && level_1.is_branch() {
                    let addr = level_1.physical_address();
                    deallocate(addr as _);
                }
            }

            deallocate(table_level_1 as *mut Table as _);
        }
    }
}

fn virtual_to_physical(root: &Table, vaddr: usize) -> Option<usize> {
    let vpn = VirtualPageNumber::new_with_raw_value(vaddr as u64);
    let mut virt = &root.entries[vpn.entry_at(0)];

    for i in (0..=2).rev() {
        if !virt.valid() {
            // Page fault
            return None;
        } else if virt.is_leaf() {
            let ppn = PhysicalPageNumber::new_with_raw_value(virt.raw_value());
            return Some((ppn.entry_at(i) * PAGE_SIZE) + vpn.offset().value() as usize);
        }

        let entry = virt.physical_address() as *mut Entry;
        virt = unsafe { entry.add(vpn.entry_at(i - 1) as usize).as_ref().unwrap() };
    }
    None
}

fn identity_map(root: &mut Table, range: Range<usize>, flags: u64) {
    let total_pages = (range.end - range.start) / PAGE_SIZE;
    for i in 0..total_pages {
        let addr: u64 = (range.start + (i * PAGE_SIZE)) as _;
        map(root, addr, addr, flags, 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn virt_to_phys() {
        let mut table = Table::default();
        map(&mut table, 0x1000, 0x2000, 0x7, 0);
        let phys = virtual_to_physical(&table, 0x1000).unwrap();
        assert_eq!(phys, 0x2000);
        unmap(&mut table);
    }

    #[test_case]
    fn id_map() {
        let mut table = Table::default();
        identity_map(&mut table, 0x1000..0x2000, 0x7);

        let phys = virtual_to_physical(&table, 0x1000).unwrap();
        assert_eq!(phys, 0x1000);

        assert_eq!(virtual_to_physical(&table, 0x1999).unwrap(), 0x1999);
        assert!(virtual_to_physical(&table, 0x2000).is_none());

        unmap(&mut table);
    }

    #[test_case]
    fn zero_alloc() {
        let full = allocate(40).unwrap();
        for i in 0..(40 * PAGE_SIZE) {
            unsafe { *full.add(i) = 0xFF };
        }

        assert!(unsafe { *full } == 0xFF);
        assert!(unsafe { *full.add(40 * PAGE_SIZE - 1) } == 0xFF);
        deallocate(full);

        let a = zero_allocate(20).unwrap();
        let b = zero_allocate(20).unwrap();

        for i in 0..(20 * PAGE_SIZE) {
            assert_eq!(unsafe { *a.add(i) }, 0);
            assert_eq!(unsafe { *b.add(i) }, 0);
        }

        deallocate(a);
        deallocate(b);
    }

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
