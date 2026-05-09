#![no_std]
#![no_main]

use core::panic::PanicInfo;

mod vga_buffer;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    println!("Hello from {}!", "dsos");
    println!("The numbers are {} and {}", 42, 1.0 / 3.0);

    loop {}
}
