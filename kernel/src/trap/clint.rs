use core::arch::asm;

const INTERVAL: u64 = 10000;

const BASE_ADDR: usize = 0x0200_0000;
const MTIME: usize = BASE_ADDR + 0xBFF8;
const MTIMECMP: usize = BASE_ADDR + 0x4000;

/// Information saved between traps, assuming one HART. Layout must match `vector.s`.
///     0: `mtime` pointer
///     1: `mtimecmp` pointer
///     2: `mtimecmp` interval
static mut MSCRATCH: [u64; 3] = [0; 3];

/// Initializes the machine-mode timer. This has to be called before we enter supervisor mode.
#[no_mangle]
unsafe extern "C" fn machine_timer_init() {
    let mtime: *mut u64 = MTIME as _;
    let mtimecmp: *mut u64 = MTIMECMP as _;

    println!("initializing machine timer...");

    // Set the machine timer to go off after the specified interval
    mtimecmp.write_volatile(mtime.read_volatile() + INTERVAL);

    // Save our context
    MSCRATCH[0] = mtime as _;
    MSCRATCH[1] = mtimecmp as _;
    MSCRATCH[2] = INTERVAL;
    asm!("csrw mscratch, {}", in(reg) MSCRATCH.as_mut_ptr() as usize);

    // Enable machine interrupts
    asm!("csrs mstatus, {}", in(reg) 1 << 3);

    // Enable the machine timer interrupts
    asm!("csrs mie, {}", in(reg) 1 << 7);

    println!("machine timer initialized");
}
