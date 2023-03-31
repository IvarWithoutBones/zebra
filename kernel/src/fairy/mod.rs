// TODO: remove
#![allow(dead_code)]

mod header;
mod program;
mod section;
mod symbol;

use crate::memory;
use alloc::boxed::Box;
use binrw::BinRead;

// Resources:
// https://en.wikipedia.org/wiki/Executable_and_Linkable_Format
// https://upload.wikimedia.org/wikipedia/commons/e/e4/ELF_Executable_and_Linkable_Format_diagram_by_Ange_Albertini.png
// https://man7.org/linux/man-pages/man5/elf.5.html
// https://docs.oracle.com/cd/E23824_01/html/819-0690/chapter6-46512.html#scrolltoc
// https://wiki.osdev.org/ELF_Tutorialhttps://wiki.osdev.org/ELF_Tutorial
// $ cargo readobj -- --headers

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
                    let offset = program.offset as usize + (page * memory::PAGE_SIZE);
                    let mut data = Box::new([0u8; memory::PAGE_SIZE]);
                    data[..len].copy_from_slice(&elf[offset..offset + len]);
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
                    program.flags.as_page_attributes(),
                );
            }
        }
    }

    header.primary.entry_point_64.unwrap()
}
