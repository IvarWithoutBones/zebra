use {
    super::{align_page_down, align_page_up, PAGE_SIZE},
    alloc::boxed::Box,
    core::{arch::asm, ptr::read_volatile},
};

static KERNEL_PAGE_TABLE: Table = Table::new();

pub fn init() {
    let satp = {
        let mode = 8; // Sv39
        (root_table() as usize / PAGE_SIZE) | (mode << 60)
    };

    unsafe {
        // NOTE: `sfence.vma` is not required, the TLB will be freshly populated on the next memory access
        asm!("csrw satp, {}", in(reg) satp);
    }
}

pub fn root_table() -> *const Table {
    &KERNEL_PAGE_TABLE as _
}

const TABLE_LEN: usize = 512;

#[derive(Debug, Copy, Clone)] // For initializer
#[repr(transparent)]
pub struct Entry(usize);

#[repr(transparent)]
pub struct VirtualPageNumber(usize);

#[derive(Debug)]
#[repr(C, align(4096))]
pub struct Table {
    pub entries: [Entry; TABLE_LEN],
}

/// <https://five-embeddev.com/riscv-isa-manual/latest/supervisor.html#sec:translation>
#[allow(dead_code)]
pub enum EntryAttributes {
    Valid = 1 << 0,
    Readable = 1 << 1,
    Writable = 1 << 2,
    Executable = 1 << 3,
    User = 1 << 4,
    Global = 1 << 5,
    Accessed = 1 << 6,
    Dirty = 1 << 7,

    ReadWrite = 0b11 << 1,
    ReadExecute = 0b101 << 1,
    UserRead = 0b10010,
    UserReadWrite = 0b10110,
    UserReadExecute = 0b11010,
}

impl EntryAttributes {
    const fn contains(self, data: usize) -> bool {
        data & (self as usize) != 0
    }
}

impl Entry {
    const fn is_valid(&self) -> bool {
        EntryAttributes::Valid.contains(self.0)
    }

    const fn is_user(&self) -> bool {
        EntryAttributes::User.contains(self.0)
    }

    const fn is_leaf(&self) -> bool {
        // TODO: prettify
        self.0 & 0xe != 0
    }

    const fn paddr(&self) -> usize {
        (self.0 & !0x3ff) << 2
    }

    const fn new(ppn: usize, flags: usize) -> Self {
        Self(((ppn & !0xfff) >> 2) | flags)
    }
}

impl VirtualPageNumber {
    const fn vpn0(&self) -> usize {
        (self.0 >> 12) & 0x1ff
    }

    const fn vpn1(&self) -> usize {
        (self.0 >> 21) & 0x1ff
    }

    const fn vpn2(&self) -> usize {
        (self.0 >> 30) & 0x1ff
    }

    const fn index(&self, id: usize) -> usize {
        match id {
            0 => self.vpn0(),
            1 => self.vpn1(),
            2 => self.vpn2(),
            _ => unreachable!(),
        }
    }
}

impl Table {
    pub const fn new() -> Self {
        Self {
            entries: [Entry(0); TABLE_LEN],
        }
    }

    fn map_addr(&mut self, vaddr: usize, paddr: usize, flags: usize, level: usize) {
        assert!(
            paddr % PAGE_SIZE == 0,
            "physical address unaligned: {paddr:#x}"
        );
        assert!(
            vaddr % PAGE_SIZE == 0,
            "virtual address unaligned: {vaddr:#x}"
        );
        assert!(level <= 2, "invalid level: {level}");

        let vpn = VirtualPageNumber(vaddr);
        let mut v = &mut self.entries[vpn.vpn2()];

        // Traverse the page table to a leaf
        for lvl in (level..2).rev() {
            if !v.is_valid() {
                let table = Box::new(Table::new());
                *v = Entry::new(
                    Box::into_raw(table) as usize,
                    EntryAttributes::Valid as usize,
                );
            }

            // Get the next level
            v = unsafe {
                // We need volatile as this gets optimized out otherwise
                let entry: *mut Entry = read_volatile(&v.paddr() as *const _) as _;
                entry.add(vpn.index(lvl)).as_mut().unwrap()
            };
        }

        // Map the requested address
        *v = Entry::new(paddr, flags | EntryAttributes::Valid as usize);
    }

    pub fn kernel_map(&mut self, vaddr: usize, paddr: usize, flags: usize) {
        assert!(
            !EntryAttributes::User.contains(flags),
            "User pages are not supported"
        );
        self.map_addr(vaddr, paddr, flags, 0);
    }

    pub fn identity_map(&mut self, start: usize, end: usize, flags: usize) {
        let mut addr = align_page_down(start);
        let num_kb_pages = (align_page_up(end) - addr) / PAGE_SIZE;
        for _ in 0..num_kb_pages {
            self.map_addr(addr, addr, flags, 0);
            addr += PAGE_SIZE;
        }
    }

    pub fn physical_addr(&self, mut vaddr: usize) -> Option<usize> {
        vaddr = align_page_down(vaddr);
        let vpn = VirtualPageNumber(vaddr);
        let mut v = &self.entries[vpn.vpn2()];

        for lvl in (0..2).rev() {
            if !v.is_valid() {
                return None;
            }

            // Get the next level
            v = unsafe {
                // We need volatile as this gets optimized out otherwise
                let entry: *const Entry = read_volatile(&v.paddr() as *const _) as _;
                entry.add(vpn.index(lvl)).as_ref().unwrap()
            };
        }

        Some(v.paddr())
    }
}

impl Drop for Table {
    fn drop(&mut self) {
        for entry in self.entries.iter_mut() {
            if entry.is_valid() {
                if entry.is_leaf() {
                    if entry.is_user() {
                        drop(unsafe { Box::from_raw(entry.paddr() as *mut [u8; PAGE_SIZE]) });
                    }
                } else {
                    drop(unsafe { Box::from_raw(entry.paddr() as *mut Table) });
                }
            }
        }
    }
}
