#![no_std]
#![no_main]

use core::panic::PanicInfo;

use crate::vga_buffer::{Color, ColorCode, VgaBuffer, VgaWriter};

mod vga_buffer;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    let mut writer = VgaWriter {
        column_position: 0,
        color_code: ColorCode::new(Color::Red, Color::White),
        buffer: unsafe { &mut *(0xb8000 as *mut VgaBuffer) },
    };
    
    writer.write_string("Ahoj jak se mas?");
    
    loop {}
}
