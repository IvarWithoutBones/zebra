// TODO: remove
#![allow(dead_code)]
const BINARY: &[u8] =
    include_bytes!("../../../target/riscv64gc-unknown-none-elf/debug/zebra-kernel");

mod header;
mod program;
mod section;
mod symbol;

// Resources:
// https://en.wikipedia.org/wiki/Executable_and_Linkable_Format
// https://upload.wikimedia.org/wikipedia/commons/e/e4/ELF_Executable_and_Linkable_Format_diagram_by_Ange_Albertini.png
// https://man7.org/linux/man-pages/man5/elf.5.html
// https://docs.oracle.com/cd/E23824_01/html/819-0690/chapter6-46512.html#scrolltoc
// https://wiki.osdev.org/ELF_Tutorialhttps://wiki.osdev.org/ELF_Tutorial
// $ cargo readobj -- --headers

#[cfg(test)]
mod tests {
    use super::*;
    use binrw::BinRead;

    // Quick & dirty way to avoid having to modify main
    #[test_case]
    fn testing() {
        let mut cursor = binrw::io::Cursor::new(BINARY);
        let header = header::Header::try_from(&mut cursor).unwrap();
        assert_eq!(header.identifier.class, header::Class::Bits64);
        println!("\n\n{header:#?}\n");

        cursor.set_position(header.primary.program_header_start_64.unwrap() as _);
        for _ in 0..header.primary.program_header_entry_count {
            let program =
                program::ProgramHeader::read_options(&mut cursor, header.endianness(), ()).unwrap();
            println!("{program:#?}\n");
        }

        // cursor.set_position(header.primary.section_header_start_64.unwrap() as _);
        // let sections = section::SectionTable::new(&mut cursor, &header).unwrap();

        // let symbols = symbol::SymbolTable::new(&sections, header.endianness()).unwrap();
        //
        // for sym in symbols.iter() {
        //     if let Some(name) = sym.name {
        //         println!("{name}");
        //     }
        // }

        println!();
        panic!();
    }
}
