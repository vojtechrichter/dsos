use bootloader_api::info::{FrameBuffer, PixelFormat};

#[allow(dead_code)]
pub fn put_pixel(fb: &mut FrameBuffer, x: usize, y: usize, r: u8, g: u8, b: u8) {
    let info = fb.info();
    if x >= info.width || y >= info.height {
        return;
    }

    let bytes_per_pixel = info.bytes_per_pixel;
    let offset = (y * info.stride + x) * bytes_per_pixel;
    let pixel_format = info.pixel_format;
    let buffer = fb.buffer_mut();

    let (b0, b1, b2) = match pixel_format {
        PixelFormat::Rgb => (r, g, b),
        PixelFormat::Bgr => (b, g, r),
        _ => panic!("unsupported pixel format"),
    };

    buffer[offset] = b0;
    buffer[offset + 1] = b1;
    buffer[offset + 2] = b2;
    // bytes_per_pixel is typically 4; the 4th byte is reserved/alpha — leave it.
}

pub fn draw_gradient(fb: &mut FrameBuffer) {
    let info = fb.info();
    let width = info.width;
    let height = info.height;
    let stride = info.stride;
    let bytes_per_pixel = info.bytes_per_pixel;
    let pixel_format = info.pixel_format;
    let buffer = fb.buffer_mut();

    for y in 0..height {
        for x in 0..width {
            let r = ((x * 255) / width.max(1)) as u8;
            let g = ((y * 255) / height.max(1)) as u8;
            let b = (((x + y) * 255) / (width + height).max(1)) as u8;

            let offset = (y * stride + x) * bytes_per_pixel;
            let (b0, b1, b2) = match pixel_format {
                PixelFormat::Rgb => (r, g, b),
                PixelFormat::Bgr => (b, g, r),
                _ => panic!("unsupported pixel format"),
            };
            buffer[offset] = b0;
            buffer[offset + 1] = b1;
            buffer[offset + 2] = b2;
        }
    }
}
