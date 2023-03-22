use super::section::SectionTable;
use alloc::vec::Vec;
use binrw::{binrw, io::Cursor, BinRead, BinResult, Endian};
use core::ops::{Index, IndexMut};

const SECTION_ABSOLUTE: u16 = 0xfff1;

#[derive(Debug)]
pub struct SymbolTable<'a> {
    symbols: Vec<Symbol<'a>>,
}

impl<'a> SymbolTable<'a> {
    pub fn new(sections: &'a SectionTable, endianness: Endian) -> Option<Self> {
        let symtab = sections.get(".symtab")?;
        let strtab = sections.get(".strtab")?;

        let symbol_entries = symtab.header.size / symtab.header.entry_size;
        let mut symbol_cursor = Cursor::new(symtab.data);

        // Read all the symbol entries and resolve their names
        let mut entries = Vec::with_capacity(symbol_entries as usize);
        for _ in 0..symbol_entries {
            let entry = SymbolEntry::read_options(&mut symbol_cursor, endianness, ()).ok()?;
            let name = if entry.name_offset != 0 {
                Some(strtab.read_string_table(entry.name_offset as _)?)
            } else {
                None
            };

            entries.push(Symbol { name, entry });
        }

        Some(Self { symbols: entries })
    }

    pub fn get(&self, name: &str) -> Option<&Symbol<'a>> {
        self.symbols.iter().find(|s| s.name == Some(name))
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut Symbol<'a>> {
        self.symbols.iter_mut().find(|s| s.name == Some(name))
    }

    pub fn iter(&self) -> impl Iterator<Item = &Symbol<'a>> {
        self.symbols.iter()
    }
}

impl<'a> Index<usize> for SymbolTable<'a> {
    type Output = Symbol<'a>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.symbols[index]
    }
}

impl<'a> IndexMut<usize> for SymbolTable<'a> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.symbols[index]
    }
}

#[derive(Debug)]
pub struct Symbol<'a> {
    pub name: Option<&'a str>,
    pub entry: SymbolEntry,
}

#[derive(Debug)]
#[binrw]
pub struct SymbolEntry {
    /// An index into the object file's symbol string table, which holds the character representations of the symbol names. (`st_name`)
    pub name_offset: u32,
    /// The symbol's type and binding attributes. (`st_info`)
    #[br(parse_with = Info::try_parse)]
    pub info: Info,
    /// The symbol's visibility. (`st_other`)
    #[br(parse_with = Visibility::try_parse)]
    pub visibility: Visibility,
    /// The index of the section header table entry associated with this symbol. (`st_shndx`)
    pub section_index: u16,
    /// The value of the symbol. (`st_value`)
    pub value: u64,
    /// The size of the symbol. (`st_size`)
    pub size: u64,
}

#[derive(Debug, PartialEq, Eq)]
#[binrw]
#[brw(repr(u8))]
pub enum Binding {
    Local = 0,
    Global = 1,
    Weak = 2,
    // Ignoring STB_LOOS..STB_HIOS and STB_LOPROC..STB_HIPROC
}

#[derive(Debug, PartialEq, Eq)]
#[binrw]
#[brw(repr(u8))]
pub enum SymbolType {
    NoType = 0,
    Object = 1,
    Function = 2,
    Section = 3,
    File = 4,
    Common = 5,
    ThreadLocalStorage = 6,
    SparcRegister = 13,
    // Ignoring STT_LOOS..STT_HIOS and STT_LOPROC..STT_HIPROC
}

#[derive(Debug)]
#[binrw]
pub struct Info {
    pub binding: Binding,
    pub symbol_type: SymbolType,
}

impl Info {
    #[binrw::parser(reader, endian)]
    fn try_parse() -> BinResult<Self> {
        let input = {
            let input = u8::read_options(reader, endian, ())?;
            &[input >> 4, input & 0xF]
        };

        // Having to create a new reader here sucks, but I have no idea how else to replace the input
        let mut new_reader = Cursor::new(input);
        let binding = Binding::read_options(&mut new_reader, endian, ())?;
        let symbol_type = SymbolType::read_options(&mut new_reader, endian, ())?;

        Ok(Self {
            binding,
            symbol_type,
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
#[binrw]
#[brw(repr(u8))]
pub enum Visibility {
    Default = 0,
    Internal = 1,
    Hidden = 2,
    Protected = 3,
    Exported = 4,
    Singleton = 5,
    Eliminate = 6,
}

impl Visibility {
    #[binrw::parser(reader, endian)]
    fn try_parse() -> BinResult<Self> {
        let input = {
            let input = u8::read_options(reader, endian, ())?;
            &[input & 0x3]
        };

        // Having to create a new reader here sucks, but I have no idea how else to replace the input
        let mut new_reader = Cursor::new(input);
        Self::read_options(&mut new_reader, endian, ())
    }
}
