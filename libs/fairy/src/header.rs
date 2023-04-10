use binrw::{binrw, io::Cursor, BinRead, Endian};

/// The initial section of a ELF header. (`e_ident`)
#[derive(Debug)]
#[binrw]
#[br(magic(b"\x7fELF"))]
pub struct IdentifierHeader {
    /// `e_ident[EI_CLASS]`
    pub class: Class,
    /// `e_ident[EI_DATA]`
    pub data: Data,
    /// `e_ident[EI_VERSION]`
    pub version: Version,
    /// `e_ident[EI_OSABI]`
    pub os_abi: OsAbi,
    /// `e_ident[EI_ABIVERSION]`
    #[br(pad_after = 7)]
    pub abi_version: u8,
}

/// The main ELF header, missing the `e_ident` section as that is parsed separately in `Identifier`.
#[derive(Debug)]
#[binrw]
#[br(import { class: Class } )]
pub struct PrimaryHeader {
    /// `e_type`
    pub object_type: ObjectType,
    /// `e_machine`
    pub machine: Machine,
    /// `e_version`
    pub version: u32,
    /// `e_entry`
    #[br(if(class == Class::Bits32))]
    pub entry_point_32: Option<u32>,
    /// `e_entry`
    #[br(if(class == Class::Bits64))]
    pub entry_point_64: Option<u64>,
    /// `e_phoff`
    #[br(if(class == Class::Bits32))]
    pub program_header_start_32: Option<u32>,
    /// `e_phoff`
    #[br(if(class == Class::Bits64))]
    pub program_header_start_64: Option<u64>,
    /// `e_shoff`
    #[br(if(class == Class::Bits32))]
    pub section_header_start_32: Option<u32>,
    /// `e_shoff`
    #[br(if(class == Class::Bits64))]
    pub section_header_start_64: Option<u64>,
    /// `e_flags`
    pub flags: u32,
    /// `e_ehsize`
    pub header_size: u16,
    /// `e_phentsize`
    pub program_header_entry_size: u16,
    /// `e_phnum`
    pub program_header_entry_count: u16,
    /// `e_shentsize`
    pub section_header_entry_size: u16,
    /// `e_shnum`
    pub section_header_entry_count: u16,
    /// `e_shstrndx`
    pub section_header_string_table_index: u16,
}

#[derive(Debug)]
pub struct Header {
    pub identifier: IdentifierHeader,
    pub primary: PrimaryHeader,
}

impl Header {
    fn new(identifier: IdentifierHeader, primary: PrimaryHeader) -> Self {
        Self {
            identifier,
            primary,
        }
    }

    pub fn endianness(&self) -> Endian {
        match self.identifier.data {
            Data::LittleEndian => Endian::Little,
            Data::BigEndian => Endian::Big,
        }
    }
}

impl TryFrom<&mut Cursor<&[u8]>> for Header {
    type Error = binrw::error::Error;

    fn try_from(cursor: &mut Cursor<&[u8]>) -> Result<Self, Self::Error> {
        let endianness = Endian::Little;
        let identifier = IdentifierHeader::read_options(cursor, endianness, ())?;

        let endianness = match identifier.data {
            Data::LittleEndian => binrw::Endian::Little,
            Data::BigEndian => binrw::Endian::Big,
        };

        let header = PrimaryHeader::read_options(
            cursor,
            endianness,
            binrw::args! { class: identifier.class.clone() },
        )?;

        Ok(Self::new(identifier, header))
    }
}

/// The class of the object file. (`e_ident[EI_CLASS]`)
#[derive(Debug, Clone, PartialEq, Eq)]
#[binrw]
#[brw(repr(u8))]
pub enum Class {
    Bits32 = 1,
    Bits64 = 2,
}

/// The endianness of the object file. (`e_ident[EI_DATA]`)
#[derive(Debug)]
#[binrw]
#[brw(repr(u8))]
pub enum Data {
    LittleEndian = 1,
    BigEndian = 2,
}

/// The version of the ELF specification. (`e_ident[EI_VERSION]`)
#[derive(Debug)]
#[binrw]
#[brw(repr(u8))]
pub enum Version {
    None = 0,
    Current = 1,
}

/// The target OS ABI. (`e_ident[EI_OSABI]`)
#[derive(Debug)]
#[binrw]
#[brw(repr(u8))]
pub enum OsAbi {
    SystemV = 0x0,
    HpUx = 0x1,
    NetBSD = 0x2,
    Linux = 0x3,
    GnuHurd = 0x4,
    Solaris = 0x6,
    Aix = 0x7,
    Irix = 0x8,
    FreeBSD = 0x9,
    Tru64 = 0xA,
    NovellModesto = 0xB,
    OpenBSD = 0xC,
    OpenVMS = 0xD,
    NonStopKernel = 0xE,
    Aros = 0xF,
    FenixOS = 0x10,
    NuxiCloudABI = 0x11,
    OpenVOS = 0x12,
}

/// The type of object. (`e_type`)
#[derive(Debug)]
#[binrw]
#[brw(repr(u16))]
pub enum ObjectType {
    /// Unknown. (`ET_NONE`)
    None = 0,
    /// Relocatable file. (`ET_REL`)
    Relocatable = 1,
    /// Executable file. (`ET_EXEC`)
    Executable = 2,
    /// Shared object. (`ET_DYN`)
    SharedObject = 3,
    /// Core file. (`ET_CORE`)
    Core = 4,
    // Ignoring ET_LOOS..ET_HIOS and ET_LOPROC..ET_HIPROC
}

/// The target instruction set architecture. (`e_machine`)
#[derive(Debug)]
#[binrw]
#[brw(repr(u16))]
pub enum Machine {
    // Ignoring many (mostly ancient) variants
    None = 0x0,
    X86 = 0x3,
    Mips = 0x8,
    PowerPC = 0x14,
    PowerPC64 = 0x15,
    Aarch32 = 0x28,
    X86_64 = 0x3E,
    Aarch64 = 0xB7,
    RiscV = 0xF3,
}
