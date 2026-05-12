# Framebuffer + Gradient on bootloader 0.11

**Date:** 2026-05-12
**Status:** Approved, ready for implementation plan

## Goal

Replace the current VGA text-mode "hello world" with a linear-framebuffer kernel that draws a 2D color gradient. This is the first visible milestone on the path to full graphics: it proves the boot path, the framebuffer abstraction, and the pixel-plotting primitive end-to-end. No font, no animation, no input — just pixels.

**Success criterion:** `cargo run` boots the kernel in QEMU and shows a full-screen gradient (red horizontal, green vertical, blue = `(x+y)/2`).

## Scope

### In scope
- Migrate from `bootloader` 0.9 + `bootimage` to `bootloader` 0.11.
- New kernel entry point using `entry_point!` and `&'static mut BootInfo`.
- A `framebuffer` module with a thin wrapper providing `put_pixel` and `draw_gradient`.
- A working `cargo run` flow that builds the kernel, builds a BIOS disk image, and launches QEMU.
- Delete `src/vga_buffer.rs` and the `println!`/`print!` macros, plus dependencies that become unused (`volatile`, `spin`).

### Out of scope (deferred)
- Bitmap font and `println!` rewritten for the framebuffer.
- Animation, double-buffering, locking around the framebuffer.
- UEFI build target (BIOS only for now).
- Interrupts, GDT, IDT, allocator.

## Defaults / Decisions

| Decision | Choice | Rationale |
|---|---|---|
| Boot mode | BIOS only | Matches current setup; smallest delta. UEFI added in a later milestone. |
| Resolution preference | Request 1280×720; accept what firmware returns | Common, looks good in QEMU, but code must not assume it. |
| Pixel format support | BGR and RGB at runtime | Both are common on real hardware/QEMU. Panic on `U8`/grayscale/unknown — won't happen on QEMU. |
| Existing `vga_buffer.rs` | Delete | The bootloader 0.11 framebuffer takeover means the 0xb8000 text buffer is no longer guaranteed; dead code violates surgical-changes. |
| Hardware idle | Inline `asm!("hlt")` in the final `loop` | Avoids pulling in the `x86_64` crate just for one instruction. |
| Custom target file | Try replacing `x86_64-llvm-target.json` with stock `x86_64-unknown-none` | Bootloader 0.11 examples use the stock triple; only re-add a custom file if something specific is needed. |

## Architecture

### Module layout

```
src/
  main.rs           # entry_point!(kernel_main), top-level orchestration
  framebuffer.rs    # FrameBuffer wrapper, pixel + gradient routines
build.rs            # builds the BIOS disk image via bootloader::DiskImageBuilder
```

`main.rs` and `framebuffer.rs` are the only kernel-side source files. `build.rs` is host-side and runs at compile time.

### Boot path

1. Cargo builds the kernel binary for `x86_64-unknown-none` (no_std, no_main).
2. `build.rs` uses `bootloader::DiskImageBuilder` to produce a bootable BIOS image, writing the path to an env var (`BIOS_IMAGE`) that a runner picks up.
3. A `cargo run` runner (either via `runner = "..."` in `.cargo/config.toml` or a small `bin/qemu_bios.rs`) launches QEMU pointing at that image.
4. Firmware → bootloader → bootloader's stage code → calls `kernel_main(boot_info: &'static mut BootInfo)`.
5. `kernel_main` extracts the framebuffer, draws the gradient, halts.

### Kernel entry point

```rust
#![no_std]
#![no_main]

use bootloader_api::{entry_point, BootInfo};
use core::panic::PanicInfo;

mod framebuffer;

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    let fb = boot_info.framebuffer.as_mut().expect("no framebuffer");
    framebuffer::draw_gradient(fb);
    halt_loop();
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    halt_loop();
}

fn halt_loop() -> ! {
    loop {
        unsafe { core::arch::asm!("hlt", options(nomem, nostack, preserves_flags)); }
    }
}
```

A bootloader-config block (passed to `entry_point!`) requests the 1280×720 preference.

### Framebuffer abstraction

`framebuffer.rs` exposes a single free function for this milestone:

```rust
pub fn draw_gradient(fb: &mut bootloader_api::info::FrameBuffer) { ... }
```

Internally it reads `FrameBufferInfo` once (`width`, `height`, `stride`, `bytes_per_pixel`, `pixel_format`) and then iterates `y in 0..height`, `x in 0..width`, computing:

- `r = (x * 255 / width) as u8`
- `g = (y * 255 / height) as u8`
- `b = ((x + y) * 255 / (width + height)) as u8`

A small helper `put_pixel(buffer: &mut [u8], info: &FrameBufferInfo, x, y, r, g, b)` handles the BGR/RGB swap and the byte offset:

```
offset = y * stride * bytes_per_pixel + x * bytes_per_pixel
```

No globals, no locking. `kernel_main` owns the framebuffer exclusively for this milestone.

### Cargo / build changes

**`Cargo.toml`:**
- Remove `bootloader = "0.9"`, `volatile = "0.2.6"`, `spin = "0.9"`.
- Add `bootloader_api = "0.11"` under `[dependencies]`.
- Add `bootloader = "0.11"` under a new `[build-dependencies]` table.
- Keep `panic = "abort"` for both profiles.

**`build.rs` (new):**
- Use `bootloader::BiosBoot` (or `DiskImageBuilder`, whichever the crate's current API is — verify at implementation time) to create a `.img` from `env!("CARGO_BIN_FILE_DSOS_dsos")`.
- Print `cargo:rustc-env=BIOS_IMAGE=<path>` so the runner can find it.

**Runner:**
- Either `.cargo/config.toml` `[target.x86_64-unknown-none] runner = "..."` invoking QEMU directly, or a tiny `src/bin/qemu_bios.rs` that reads `env!("BIOS_IMAGE")` and `Command::new("qemu-system-x86_64")...`. Going with `.cargo/config.toml` if it works with one line; otherwise the runner binary.

**Target triple:** try `x86_64-unknown-none` first; only keep `x86_64-llvm-target.json` if the stock triple breaks something.

## Verification

1. `cargo run` exits the build cleanly and opens a QEMU window.
2. The QEMU window shows a smooth 2D gradient covering the full screen — red increasing left-to-right, green increasing top-to-bottom, blue increasing diagonally.
3. Resize the QEMU window or change the resolution preference; the gradient still spans the full reported framebuffer with no tearing or stray pixels.
4. No panics in QEMU's serial / debug log (we don't read it yet but absence of a triple-fault reboot loop is the signal).

## Risks / unknowns

- **Bootloader 0.11 build flow on macOS.** The user's host is darwin; `bootloader` 0.11's disk-image building should be host-agnostic, but verify before deep work. If broken, fall back to a host-side runner binary that shells out.
- **Custom target file removal.** `x86_64-llvm-target.json` may exist for a specific reason (panic strategy, code model). If `x86_64-unknown-none` causes link errors, restore the custom file.
- **Pixel format on real hardware.** We panic on unsupported formats; this is fine for QEMU but would need extension before booting on metal.

## Next milestones (not part of this spec)

1. Bitmap font + framebuffer `println!` to replace the deleted VGA text writer.
2. Smooth Mandelbrot at native framebuffer resolution.
3. PIT timer + animation loop (plasma, fire).
4. GDT/IDT, PS/2 keyboard, interactive shell.
