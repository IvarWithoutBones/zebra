use crate::memory::{self, page};
use alloc::boxed::Box;
use binrw::BinRead;
use fairy::{
    header,
    program::{self, ProgramFlags},
};

const fn convert_flags(from: ProgramFlags) -> Option<page::EntryAttributes> {
    match (from.read(), from.write(), from.execute()) {
        (true, false, false) => Some(page::EntryAttributes::UserRead),
        (true, false, true) => Some(page::EntryAttributes::UserReadExecute),
        (true, true, false) => Some(page::EntryAttributes::UserReadWrite),
        _ => None,
    }
}

pub fn load_elf(elf: &[u8], page_table: &mut memory::page::Table) -> u64 {
    let mut cursor = binrw::io::Cursor::new(elf);
    let header = header::Header::try_from(&mut cursor).unwrap();
    assert_eq!(header.identifier.class, header::Class::Bits64);

    cursor.set_position(header.primary.program_header_start_64.unwrap() as _);
    for _ in 0..header.primary.program_header_entry_count {
        let program =
            program::ProgramHeader::read_options(&mut cursor, header.endianness(), ()).unwrap();

        if program.program_type == program::ProgramType::Loadable {
            let pages_needed = memory::align_page_up(program.file_size as _) / memory::PAGE_SIZE;
            let len = memory::PAGE_SIZE.min(program.file_size as usize);

            for page in 0..pages_needed {
                // New page filled with the program data
                let page_data = {
                    let elf_start = program.offset as usize + (page * memory::PAGE_SIZE);
                    let elf_end = (elf_start + len).min(elf.len());
                    let dst_len = elf_end - elf_start;

                    let mut data = Box::new([0u8; memory::PAGE_SIZE]);
                    data[..dst_len].copy_from_slice(&elf[elf_start..elf_end]);
                    data
                };

                // Calculate the virtual address of the page
                let vaddr = memory::align_page_down(
                    program.virtual_address as usize + (page * memory::PAGE_SIZE),
                );

                // Map the page to the virtual address.
                page_table.map_page(
                    vaddr,
                    Box::into_raw(page_data) as usize,
                    convert_flags(program.flags).unwrap(),
                );
            }
        }
    }

    header.primary.entry_point_64.unwrap()
}
