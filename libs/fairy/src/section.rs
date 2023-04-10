use super::header::Header;
use alloc::{boxed::Box, vec::Vec};
use binrw::{binrw, io::Cursor, BinRead};
use core::{
    fmt,
    ops::{Index, IndexMut},
};

#[derive(Debug, Clone)]
pub struct SectionTable<'a> {
    sections: Vec<Section<'a>>,
}

impl<'a> SectionTable<'a> {
    pub fn new(cursor: &mut Cursor<&'a [u8]>, header: &Header) -> Option<Self> {
        let entry_count = header.primary.section_header_entry_count as usize;
        let mut buffer = Vec::with_capacity(entry_count);

        // Read all the section headers
        for _ in 0..entry_count {
            let old_pos = cursor.position();
            let header = SectionHeader64::read_options(cursor, header.endianness(), ()).ok()?;
            assert_eq!(cursor.position() - old_pos, 64);
            buffer.push(header);
        }

        // Get the string table
        let string_table = &cursor.get_ref()
            [buffer[header.primary.section_header_string_table_index as usize].offset as usize..];

        // Populate the section names
        let mut result = Vec::with_capacity(entry_count);
        for section in buffer.iter().cloned() {
            let name = &string_table[section.name_offset as usize..]
                .split(|&b| b == 0)
                .next()?;
            let name = core::str::from_utf8(name).ok()?;
            result.push(Section::new(name, section, cursor.get_ref()));
        }

        Some(Self { sections: result })
    }

    pub fn get(&self, name: &str) -> Option<&Section<'a>> {
        self.sections.iter().find(|s| s.name == name)
    }

    fn get_mut(&mut self, name: &str) -> Option<&mut Section<'a>> {
        self.sections.iter_mut().find(|s| s.name == name)
    }
}

impl<'a> Index<usize> for SectionTable<'a> {
    type Output = Section<'a>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.sections[index]
    }
}

impl<'a> IndexMut<usize> for SectionTable<'a> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.sections[index]
    }
}

impl<'a> Index<&str> for SectionTable<'a> {
    type Output = Section<'a>;

    fn index(&self, index: &str) -> &Self::Output {
        self.get(index).unwrap()
    }
}

impl<'a> IndexMut<&str> for SectionTable<'a> {
    fn index_mut(&mut self, index: &str) -> &mut Self::Output {
        self.get_mut(index).unwrap()
    }
}

#[derive(Clone)]
pub struct Section<'a> {
    pub name: &'a str,
    pub header: SectionHeader64,
    pub data: &'a [u8],
}

impl<'a> Section<'a> {
    fn new(name: &'a str, header: SectionHeader64, file: &'a [u8]) -> Self {
        Self {
            data: &file[header.offset as usize..][..header.size as usize],
            name,
            header,
        }
    }

    pub fn read_string_table(&self, offset: usize) -> Option<&'a str> {
        match self.header.section_type {
            SectionType::StringTable => {
                let data = &self.data[offset..].split(|&b| b == 0).next()?;
                Some(core::str::from_utf8(data).ok()?)
            }
            _ => None,
        }
    }
}

impl<'a> fmt::Debug for Section<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Section")
            .field("name", &self.name)
            .field("header", &self.header)
            .field("data", &format_args!("[u8; {}]", self.data.len()))
            .finish()
    }
}

/// A section header table entry, for 64-bit objects.
#[derive(Debug, Clone)]
#[binrw]
pub struct SectionHeader64 {
    /// An offset to a string in the .shstrtab section that represents the name of this section. (`sh_name`)
    pub name_offset: u32,
    /// The type of this section header. (`sh_type`)
    #[br(try_map = SectionType::try_parse)]
    pub section_type: SectionType,
    /// The attributes of the section. (`sh_flags`)
    pub flags: u64, // TODO: enumify
    /// The virtual address of the section in memory. (`sh_addr`)
    pub address: u64,
    /// Offset of the section in the file image. (`sh_offset`)
    pub offset: u64,
    /// Size in bytes of the section in the file image. (`sh_size`)
    pub size: u64,
    /// The section index of an associated section. (`sh_link`)
    pub link: u32,
    /// Extra information about the section. (`sh_info`)
    pub info: u32,
    /// The required alignment of the section. Must be a power of two. (`sh_addralign`)
    pub address_align: u64,
    /// The size, in bytes, of each entry, for sections that contain fixed-size entries. (`sh_entsize`)
    pub entry_size: u64,
}

/// The type of section. (`sh_type`)
#[derive(Debug, PartialEq, Eq, Clone)]
#[binrw]
#[brw(repr(u32))]
pub enum SectionType {
    /// Section header table entry unused. (`SHT_NULL`)
    Unused = 0x0,
    /// Program data. (`SHT_PROGBITS`)
    ProgramBits = 0x1,
    /// Symbol table. (`SHT_SYMTAB`)
    SymbolTable = 0x2,
    /// String table. (`SHT_STRTAB`)
    StringTable = 0x3,
    /// Relocation entries with addends. (`SHT_RELA`)
    RelocationEntriesWithAddends = 0x4,
    /// Symbol hash table. (`SHT_HASH`)
    SymbolHashTable = 0x5,
    /// Dynamic linking information. (`SHT_DYNAMIC`)
    Dynamic = 0x6,
    /// Notes. (`SHT_NOTE`)
    Notes = 0x7,
    /// Program space with no data (bss) (`SHT_NOBITS`)
    ProgramSpaceNoData = 0x8,
    /// Relocation entries, no addends. (`SHT_REL`)
    RelocationEntries = 0x9,
    /// Reserved. (`SHT_SHLIB`)
    Reserved = 0x0A,
    /// Dynamic linker symbol table. (`SHT_DYNSYM`)
    DynamicLinkerSymbol = 0x0B,
    /// Array of constructors. (`SHT_INIT_ARRAY`)
    ArrayConstructors = 0x0E,
    /// Array of destructors. (`SHT_FINI_ARRAY`)
    ArrayDestructors = 0x0F,
    /// Array of pre-constructors. (`SHT_PREINIT_ARRAY`)
    ArrayPreConstructors = 0x10,
    /// Section group. (`SHT_GROUP`)
    SectionGroup = 0x11,
    /// Extended section indices. (`SHT_SYMTAB_SHNDX`)
    SymbolTableWithExtendedIndices = 0x12,
    /// Number of defined types. (`SHT_NUM`)
    DefinedTypes = 0x13,
    /// Start OS-specific. (`SHT_LOOS`)
    ProcessorSpecific = 0x60000000, // ..=u32::MAX, see `try_parse`.
}

impl SectionType {
    fn try_parse(data: u32) -> Result<Self, binrw::Error> {
        if data >= Self::ProcessorSpecific as u32 {
            Ok(Self::ProcessorSpecific)
        } else {
            Self::read_le(&mut Cursor::new(&data.to_le_bytes()))
        }
    }
}
