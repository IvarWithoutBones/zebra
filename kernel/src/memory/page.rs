use {
    super::{align_page_down, align_page_up, PAGE_SIZE},
    core::ptr::read_volatile,
};

const TABLE_LEN: usize = 512;

#[derive(Copy, Clone)] // For initializer
#[repr(transparent)]
pub struct Entry(usize);

#[repr(transparent)]
pub struct VPN(usize);

#[repr(transparent)]
pub struct PPN(usize);

#[repr(C, align(4096))]
pub struct Table {
    pub entries: [Entry; TABLE_LEN],
}

#[repr(C, align(4096))]
struct Page {
    data: [u8; PAGE_SIZE],
}

/// https://five-embeddev.com/riscv-isa-manual/latest/supervisor.html#sec:translation
pub enum EntryAttributes {
    Valid = 1 << 0,
    Readable = 1 << 1,
    Writable = 1 << 2,
    Executable = 1 << 3,
    User = 1 << 4,
    Global = 1 << 5,
    Accessed = 1 << 6,
    Dirty = 1 << 7,

    RW = 0b11 << 1,
    RX = 0b101 << 1,
    UR = 0b10010,
    URW = 0b10110,
    URX = 0b11010,
}

impl EntryAttributes {
    const fn is_set(self, data: usize) -> bool {
        data & (self as usize) != 0
    }
}

impl Entry {
    const fn is_valid(&self) -> bool {
        EntryAttributes::Valid.is_set(self.0)
    }

    const fn is_readable(&self) -> bool {
        EntryAttributes::Readable.is_set(self.0)
    }

    const fn is_writable(&self) -> bool {
        EntryAttributes::Writable.is_set(self.0)
    }

    const fn is_executable(&self) -> bool {
        EntryAttributes::Executable.is_set(self.0)
    }

    const fn is_user(&self) -> bool {
        EntryAttributes::User.is_set(self.0)
    }

    const fn is_global(&self) -> bool {
        EntryAttributes::Global.is_set(self.0)
    }

    const fn is_accessed(&self) -> bool {
        EntryAttributes::Accessed.is_set(self.0)
    }

    const fn is_dirty(&self) -> bool {
        EntryAttributes::Dirty.is_set(self.0)
    }

    const fn is_leaf(&self) -> bool {
        self.0 & 0xe != 0
    }

    const fn paddr(&self) -> PPN {
        PPN((self.0 & !0x3ff) << 2)
    }

    const fn flags(&self) -> usize {
        self.0 & 0x3ff
    }

    const fn new(ppn: usize, flags: usize) -> Self {
        Self(((ppn & !0xfff) >> 2) | flags)
    }
}

impl PPN {
    const fn ppn0(&self) -> usize {
        (self.0 >> 12) & 0x1ff
    }

    const fn ppn1(&self) -> usize {
        (self.0 >> 21) & 0x1ff
    }

    const fn ppn2(&self) -> usize {
        (self.0 >> 30) & 0x3ff_ffff
    }

    const fn idx(&self, id: usize) -> usize {
        match id {
            0 => self.ppn0(),
            1 => self.ppn1(),
            2 => self.ppn2(),
            _ => unreachable!(),
        }
    }
}

impl VPN {
    const fn vpn0(&self) -> usize {
        (self.0 >> 12) & 0x1ff
    }

    const fn vpn1(&self) -> usize {
        (self.0 >> 21) & 0x1ff
    }

    const fn vpn2(&self) -> usize {
        (self.0 >> 30) & 0x1ff
    }

    const fn idx(&self, id: usize) -> usize {
        match id {
            0 => self.vpn0(),
            1 => self.vpn1(),
            2 => self.vpn2(),
            _ => unreachable!(),
        }
    }
}

impl Table {
    const fn new() -> Self {
        Self {
            entries: [Entry(0); TABLE_LEN],
        }
    }

    const fn len() -> usize {
        TABLE_LEN
    }

    fn map(&mut self, vaddr: usize, paddr: usize, flags: usize) {
        assert!(EntryAttributes::User.is_set(flags));
        self.map_addr(vaddr, paddr, flags, 0);
    }

    pub fn kernel_map(&mut self, vaddr: usize, paddr: usize, flags: usize) {
        assert!(!EntryAttributes::User.is_set(flags));
        self.map_addr(vaddr, paddr, flags, 0);
    }

    fn map_addr(&mut self, vaddr: usize, paddr: usize, flags: usize, level: usize) {
        assert!(paddr % PAGE_SIZE == 0);
        assert!(vaddr % PAGE_SIZE == 0);
        assert!(level <= 2);

        let vpn = VPN(vaddr);
        let mut v = &mut self.entries[vpn.vpn2()];

        // Traverse the page table to a leaf
        for lvl in (level..2).rev() {
            if !v.is_valid() {
                // Allocate a new page table if not already present at this level
                *v = Entry::new(
                    &mut Table::new() as *mut _ as usize,
                    EntryAttributes::Valid as usize,
                );

                v = unsafe {
                    // We need volatile as this gets optimized out otherwise
                    let entry: *mut Entry = read_volatile(&v.paddr().0 as *const _) as _;
                    entry.add(vpn.idx(lvl)).as_mut().unwrap()
                };
            }
        }

        // Map the requested address
        *v = Entry::new(paddr, flags | EntryAttributes::Valid as usize);
    }

    fn paddr_of(&self, vaddr: usize) -> Option<usize> {
        assert!(vaddr % PAGE_SIZE == 0);

        let vpn = VPN(vaddr);
        let mut v = &self.entries[vpn.vpn2()];

        for lvl in (0..2).rev() {
            if !v.is_valid() {
                return None;
            }

            v = unsafe {
                // We need volatile as this gets optimized out otherwise
                let entry: *mut Entry = read_volatile(&v.paddr().0 as *const _) as _;
                entry.add(vpn.idx(lvl)).as_mut().unwrap()
            };
        }

        assert!(v.is_valid() && v.is_leaf());
        Some(v.paddr().0)
    }

    pub fn id_map_range(&mut self, start: usize, end: usize, flags: usize) {
        let mut addr = align_page_down(start);
        let num_kb_pages = (align_page_up(end) - addr) / PAGE_SIZE;
        for _ in 0..num_kb_pages {
            self.map_addr(addr, addr, flags, 0);
            addr += PAGE_SIZE;
        }
    }

    pub fn map_range(&mut self, start: usize, end: usize, vaddr_start: usize, bits: usize) {
        let mut memaddr = start & !(PAGE_SIZE - 1);
        let mut vaddr_start = vaddr_start & !(PAGE_SIZE - 1);
        let num_kb_pages = (align_page_up(end) - memaddr) / PAGE_SIZE;

        for _ in 0..num_kb_pages {
            self.map_addr(vaddr_start, memaddr, bits, 0);
            memaddr += PAGE_SIZE;
            vaddr_start += PAGE_SIZE;
        }
    }
}

pub static KERNEL_PAGE_TABLE: Table = Table::new();
