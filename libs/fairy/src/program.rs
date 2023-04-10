use binrw::{binrw, io::SeekFrom, BinRead, BinResult};
use bitbybit::bitfield;
use core::{fmt, mem::size_of, ops::RangeInclusive};

#[derive(Debug)]
#[binrw]
pub struct ProgramHeader {
    /// Identifies the type of the segment. (`p_type`)
    #[br(parse_with = ProgramType::try_parse)]
    pub program_type: ProgramType,
    /// Segment-dependent flags. (`p_flags`)
    pub flags: ProgramFlags,
    /// The file offset of the segment. (`p_offset`)
    pub offset: u64,
    /// The virtual address of the segment in memory. (`p_vaddr`)
    pub virtual_address: u64,
    /// The physical address of the segment in memory. (`p_paddr`)
    pub physical_address: u64,
    /// Size in bytes of the segment in the file image. (`p_filesz`)
    pub file_size: u64,
    /// Size in bytes of the segment in memory. (`p_memsz`)
    pub memory_size: u64,
    /// Alignment of the segment in memory and file. (`p_align`)
    #[br(parse_with = Alignment::try_parse)]
    pub alignment: Alignment,
}

#[derive(Debug, PartialEq, Eq)]
#[binrw]
#[brw(repr(u32))]
pub enum ProgramType {
    /// Program header table entry unused. (`NULL`)
    Null = 0,
    /// Loadable segment. (`LOAD`)
    Loadable = 1,
    /// Dynamic linking information. (`DYNAMIC`)
    Dynamic = 2,
    /// Interpreter information. (`INTERP`)
    Interpreter = 3,
    /// Auxiliary information. (`NOTE`)
    Note = 4,
    /// Reserved. (`SHLIB`)
    ShLib = 5,
    /// Segment containing program header table itself. (`PHDR`)
    ProgramHeaderTable = 6,
    /// Thread-Local Storage template. (`TLS`)
    ThreadLocalStorage = 7,
    /// Reserved inclusive range. Operating system specific. (`LOOS` and `HIOS`)
    OperatingSystemSpecific = 0x60000000, // ..=0x6FFFFFFF
    /// Reserved inclusive range. Processor specific. (`LOPROC` and `HIPROC`)
    ProcessorSpecific = 0x70000000, // ..=0x7FFFFFFF
}

#[bitfield(u32, default: 0)]
#[binrw]
#[br(map = Self::new_with_raw_value)]
pub struct ProgramFlags {
    /// Execute permission. (`PF_X`)
    #[bit(0, rw)]
    execute: bool,
    /// Write permission. (`PF_W`)
    #[bit(1, rw)]
    write: bool,
    /// Read permission. (`PF_R`)
    #[bit(2, rw)]
    read: bool,
}

impl fmt::Debug for ProgramFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProgramFlags")
            .field("execute", &self.execute())
            .field("write", &self.write())
            .field("read", &self.read())
            .finish()
    }
}

impl ProgramType {
    const OPERATING_SYSTEM_SPECIFIC: RangeInclusive<u32> = 0x60000000..=0x6FFFFFFF;
    const PROCESSOR_SPECIFIC: RangeInclusive<u32> = 0x70000000..=0x7FFFFFFF;

    #[binrw::parser(reader, endian)]
    fn try_parse() -> BinResult<Self> {
        let value = u32::read_options(reader, endian, ())?;

        if Self::OPERATING_SYSTEM_SPECIFIC.contains(&value) {
            Ok(Self::OperatingSystemSpecific)
        } else if Self::PROCESSOR_SPECIFIC.contains(&value) {
            Ok(Self::ProcessorSpecific)
        } else {
            // Rewind so that we dont advance the reader twice
            reader.seek(SeekFrom::Current(-(size_of::<u32>() as i64)))?;
            Self::read_options(reader, endian, ())
        }
    }
}

#[derive(Debug)]
#[binrw]
pub enum Alignment {
    /// No alignment required. (`0` or `1`)
    None,
    /// Alignment that is  a power of two.
    PowerOfTwo(u64),
}

impl Alignment {
    #[binrw::parser(reader, endian)]
    fn try_parse() -> BinResult<Self> {
        let value = u64::read_options(reader, endian, ())?;
        match value {
            0 | 1 => Ok(Self::None),
            _ if value.is_power_of_two() => Ok(Self::PowerOfTwo(value)),
            _ => Err(binrw::Error::NoVariantMatch {
                pos: reader.seek(SeekFrom::Current(0))?,
            }),
        }
    }
}
