# Framebuffer + Gradient Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Migrate the `dsos` kernel from `bootloader` 0.9 + `bootimage` to `bootloader` 0.11, get a linear framebuffer, and draw a full-screen 2D color gradient.

**Architecture:** Convert the single-crate project into a Cargo workspace with a `kernel/` crate (no_std, `bootloader_api`) and a `runner/` crate (host-side, depends on `bootloader` via an artifact dependency on the kernel binary). The runner builds a BIOS disk image and launches QEMU. The kernel uses bootloader 0.11's `entry_point!` macro, receives a `&'static mut BootInfo`, and writes pixels directly into the framebuffer slice.

**Tech Stack:** Rust 2024 (nightly, `build-std`), `bootloader_api` 0.11 (kernel side), `bootloader` 0.11 (host side, build-deps), Cargo `bindeps` artifact dependencies (unstable), QEMU.

**Spec:** `docs/superpowers/specs/2026-05-12-framebuffer-gradient-design.md`

**Note on verification:** This is a `no_std` kernel — there is no `cargo test` here. Each task's verification is **visual confirmation in the QEMU window**. The user runs `cargo run`, looks at the screen, and confirms the expected output. If the screen is wrong or QEMU triple-faults (the window shows a reboot loop), the task is not done.

---

## File Structure

After this plan, the repo looks like:

```
rust_os/
├── Cargo.toml                  # workspace root (NEW; replaces old single-crate root)
├── .cargo/
│   └── config.toml             # MODIFIED: drop custom target + bootimage, enable bindeps
├── kernel/
│   ├── Cargo.toml              # NEW: kernel crate manifest, depends on bootloader_api
│   └── src/
│       ├── main.rs             # MOVED + REWRITTEN: entry_point!, calls framebuffer
│       └── framebuffer.rs      # NEW: FrameBufferInfo wrapper, put_pixel, draw_gradient
├── runner/
│   ├── Cargo.toml              # NEW: host crate, artifact dep on kernel
│   └── src/
│       └── main.rs             # NEW: builds disk image, launches QEMU
├── docs/                       # unchanged
├── README.md                   # MODIFIED: new build instructions
└── (deleted) src/, x86_64-llvm-target.json, root Cargo.toml's old contents
```

Responsibilities:
- `kernel/src/main.rs` — entry point, top-level orchestration, panic handler, halt loop.
- `kernel/src/framebuffer.rs` — everything about writing pixels to the framebuffer.
- `runner/src/main.rs` — at build/run time, produces a `.img` and launches QEMU.

---

## Task 1: Restructure into a workspace, switch to bootloader 0.11, boot to a black screen

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `kernel/Cargo.toml`
- Create: `kernel/src/main.rs`
- Create: `runner/Cargo.toml`
- Create: `runner/src/main.rs`
- Modify: `.cargo/config.toml`
- Move + replace: `src/main.rs` → `kernel/src/main.rs` (contents fully rewritten)
- Delete: `src/main.rs`, `src/vga_buffer.rs`, `src/` directory, old root `Cargo.toml` content (replaced), `Cargo.lock` (will be regenerated), `x86_64-llvm-target.json`

**Goal of this task:** End up with a project that boots in QEMU under bootloader 0.11 and shows a blank/black screen (or the bootloader's brief output then black). No panic, no triple-fault reboot loop. This proves the build system, workspace, runner, and new entry point all work — before we add any pixel drawing.

- [ ] **Step 1: Stage existing files for deletion / move**

```bash
rm -rf src target x86_64-llvm-target.json Cargo.lock
```

The current `Cargo.toml` content gets replaced wholesale in step 3, so don't delete it yet — just overwrite it.

- [ ] **Step 2: Create the workspace root `Cargo.toml`**

Replace the contents of `/Users/vojtechrichter/projects/rust_os/Cargo.toml` with:

```toml
[workspace]
resolver = "2"
members = ["kernel", "runner"]
default-members = ["runner"]

[profile.dev]
panic = "abort"

[profile.release]
panic = "abort"
```

`default-members = ["runner"]` makes plain `cargo run` invoke the runner. `panic = "abort"` lives at workspace-root so it applies to both crates.

- [ ] **Step 3: Create `kernel/Cargo.toml`**

Create `/Users/vojtechrichter/projects/rust_os/kernel/Cargo.toml`:

```toml
[package]
name = "dsos"
version = "0.1.0"
edition = "2024"

[dependencies]
bootloader_api = "0.11"

[[bin]]
name = "dsos"
test = false
bench = false
```

Note: no `volatile`, no `spin`, no `bootloader` here — those are gone. `bootloader_api` is the kernel-side half of the new bootloader crate.

- [ ] **Step 4: Create `kernel/src/main.rs`**

Create `/Users/vojtechrichter/projects/rust_os/kernel/src/main.rs`:

```rust
#![no_std]
#![no_main]

use bootloader_api::{entry_point, BootInfo, BootloaderConfig};
use bootloader_api::config::Mapping;
use core::panic::PanicInfo;

static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::Dynamic);
    config.frame_buffer.minimum_framebuffer_width = Some(1280);
    config.frame_buffer.minimum_framebuffer_height = Some(720);
    config
};

entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

fn kernel_main(_boot_info: &'static mut BootInfo) -> ! {
    halt_loop();
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    halt_loop();
}

fn halt_loop() -> ! {
    loop {
        unsafe {
            core::arch::asm!("hlt", options(nomem, nostack, preserves_flags));
        }
    }
}
```

This is enough kernel to boot, get handed a framebuffer (which we ignore for now), and idle.

- [ ] **Step 5: Create `runner/Cargo.toml`**

Create `/Users/vojtechrichter/projects/rust_os/runner/Cargo.toml`:

```toml
[package]
name = "runner"
version = "0.1.0"
edition = "2024"

[dependencies]
bootloader = "0.11"
dsos = { path = "../kernel", artifact = "bin", target = "x86_64-unknown-none" }
```

The `artifact = "bin"` dependency is the Cargo `bindeps` feature — when you build `runner`, Cargo builds the `dsos` kernel binary first and exposes its path via the `CARGO_BIN_FILE_DSOS_dsos` env var. This requires nightly + `-Z bindeps`.

- [ ] **Step 6: Create `runner/src/main.rs`**

Create `/Users/vojtechrichter/projects/rust_os/runner/src/main.rs`:

```rust
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let kernel_path = PathBuf::from(env!("CARGO_BIN_FILE_DSOS_dsos"));

    let out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("target");
    std::fs::create_dir_all(&out_dir).unwrap();
    let bios_image = out_dir.join("dsos-bios.img");

    bootloader::DiskImageBuilder::new(kernel_path)
        .create_bios_image(&bios_image)
        .expect("failed to build BIOS disk image");

    let status = Command::new("qemu-system-x86_64")
        .arg("-drive")
        .arg(format!("format=raw,file={}", bios_image.display()))
        .status()
        .expect("failed to launch qemu-system-x86_64 — is QEMU installed and on PATH?");

    std::process::exit(status.code().unwrap_or(1));
}
```

- [ ] **Step 7: Rewrite `.cargo/config.toml`**

Replace `/Users/vojtechrichter/projects/rust_os/.cargo/config.toml` with:

```toml
[unstable]
bindeps = true
build-std = ["core", "compiler_builtins"]
build-std-features = ["compiler-builtins-mem"]

[build]
# No global target — the runner builds host-native, the kernel builds via artifact dep.

[target.x86_64-unknown-none]
# Kernel build target; build-std applies here via [unstable] above.
```

Why no `[build] target = ...`: the runner is a host binary (must build for the host triple), and the kernel crate's target is specified by the artifact dependency itself (`target = "x86_64-unknown-none"` in `runner/Cargo.toml`). Setting a global `[build] target` would force the runner to also build for `x86_64-unknown-none`, which won't link.

- [ ] **Step 8: Verify it builds**

Run:

```bash
cargo +nightly build
```

Expected: builds successfully. You'll see Cargo build the kernel (`dsos`) for `x86_64-unknown-none`, then the runner for the host target. No errors.

If you get `error: the package 'runner' depends on 'dsos', with features: ... but 'dsos' does not have these features` or `feature 'bindeps' is required` — confirm `[unstable] bindeps = true` is in `.cargo/config.toml` and you're on nightly.

- [ ] **Step 9: Verify it runs**

Run:

```bash
cargo +nightly run
```

Expected:
1. Cargo builds kernel + runner.
2. The runner writes `target/dsos-bios.img`.
3. A QEMU window opens.
4. You see a brief bootloader splash / firmware output, then a **black screen**. The QEMU window stays open and does not enter a reboot loop. Close it with the QEMU menu or Ctrl-C in the terminal.

If the QEMU window cycles (firmware → black → firmware → black repeatedly), the kernel is triple-faulting. Re-check Step 4's `main.rs` and that `panic = "abort"` is set in the workspace `Cargo.toml`.

If you get `failed to launch qemu-system-x86_64`, install QEMU (`brew install qemu` on macOS).

- [ ] **Step 10: Commit**

```bash
git add Cargo.toml kernel runner .cargo
git add -u  # stages deletions of src/, x86_64-llvm-target.json, old Cargo.lock
git commit -m "Migrate to bootloader 0.11 with workspace + runner crate

Kernel now uses bootloader_api's entry_point! macro and receives a
BootInfo with a framebuffer. The old VGA text mode path is gone; the
kernel currently boots and idles on a black screen, framebuffer
unused. Pixel drawing comes in the next task."
```

---

## Task 2: Add `framebuffer` module, draw a single red test pixel

**Files:**
- Create: `kernel/src/framebuffer.rs`
- Modify: `kernel/src/main.rs`

**Goal:** Prove the framebuffer pipeline end-to-end with the minimum visible output — one bright red pixel at (100, 100). If you can see it, every layer is working: BootInfo extraction, pixel format handling, byte offset math.

- [ ] **Step 1: Create `kernel/src/framebuffer.rs`**

Create `/Users/vojtechrichter/projects/rust_os/kernel/src/framebuffer.rs`:

```rust
use bootloader_api::info::{FrameBuffer, FrameBufferInfo, PixelFormat};

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
```

Key details:
- `stride` is in **pixels**, not bytes, on bootloader 0.11's `FrameBufferInfo`. The byte offset multiplies by `bytes_per_pixel` once.
- We panic on `PixelFormat::U8` / `Unknown` — won't happen on QEMU, would need extending for real hardware.
- We re-fetch `info` before `buffer_mut()` because both methods take `&mut self` / `&self`; ordering matters for the borrow checker.

- [ ] **Step 2: Wire the module into `main.rs` and draw the pixel**

Modify `/Users/vojtechrichter/projects/rust_os/kernel/src/main.rs`. Add `mod framebuffer;` and update `kernel_main`:

```rust
#![no_std]
#![no_main]

use bootloader_api::{entry_point, BootInfo, BootloaderConfig};
use bootloader_api::config::Mapping;
use core::panic::PanicInfo;

mod framebuffer;

static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::Dynamic);
    config.frame_buffer.minimum_framebuffer_width = Some(1280);
    config.frame_buffer.minimum_framebuffer_height = Some(720);
    config
};

entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    let fb = boot_info.framebuffer.as_mut().expect("no framebuffer");
    framebuffer::put_pixel(fb, 100, 100, 255, 0, 0);
    halt_loop();
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    halt_loop();
}

fn halt_loop() -> ! {
    loop {
        unsafe {
            core::arch::asm!("hlt", options(nomem, nostack, preserves_flags));
        }
    }
}
```

- [ ] **Step 3: Run and verify**

Run:

```bash
cargo +nightly run
```

Expected: QEMU window opens. **You see exactly one bright red pixel at roughly (100, 100) from the top-left corner.** The rest of the screen is black. The window stays open (no reboot loop).

If the pixel is the wrong color (e.g. blue instead of red), the BGR/RGB branch is wrong — re-check `put_pixel`'s match arms. If you see nothing at all, try widening to `put_pixel(fb, 0, 0, 255, 255, 255)` to draw a white pixel at the corner and see if anything appears.

- [ ] **Step 4: Commit**

```bash
git add kernel/src/framebuffer.rs kernel/src/main.rs
git commit -m "Add framebuffer module with put_pixel, draw a test pixel

Single-pixel test proves BootInfo framebuffer extraction, pixel format
handling (RGB/BGR), and byte offset math all work end-to-end."
```

---

## Task 3: Implement `draw_gradient`, fill the screen

**Files:**
- Modify: `kernel/src/framebuffer.rs`
- Modify: `kernel/src/main.rs`

**Goal:** The actual milestone — a full-screen 2D color gradient.

- [ ] **Step 1: Add `draw_gradient` to `framebuffer.rs`**

Append to `/Users/vojtechrichter/projects/rust_os/kernel/src/framebuffer.rs`:

```rust
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
```

We don't call `put_pixel` in a hot loop because each call re-fetches `info` and re-matches `pixel_format`. Inlining is fine for a one-shot fill and keeps the hot path obvious.

The `.max(1)` guards are paranoia against a zero-dimension framebuffer; they cost nothing.

- [ ] **Step 2: Call `draw_gradient` from `kernel_main`**

Modify `kernel_main` in `/Users/vojtechrichter/projects/rust_os/kernel/src/main.rs`:

```rust
fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    let fb = boot_info.framebuffer.as_mut().expect("no framebuffer");
    framebuffer::draw_gradient(fb);
    halt_loop();
}
```

Remove the `put_pixel` test call from Task 2.

- [ ] **Step 3: Run and verify**

Run:

```bash
cargo +nightly run
```

Expected: QEMU window opens to a **full-screen smooth color gradient**:
- Red increases left-to-right (left edge red ≈ 0, right edge red ≈ 255).
- Green increases top-to-bottom (top edge green ≈ 0, bottom edge green ≈ 255).
- Blue increases along the diagonal (top-left corner blue ≈ 0, bottom-right corner blue ≈ 255).

The top-left corner is near-black, the top-right is bright red-magenta, the bottom-left is yellow-green, and the bottom-right is white.

The gradient covers the entire framebuffer with no black borders, no tearing, no diagonal cuts, no repeating bands.

- [ ] **Step 4: Commit**

```bash
git add kernel/src/framebuffer.rs kernel/src/main.rs
git commit -m "Draw full-screen 2D gradient on the framebuffer

Red horizontal, green vertical, blue diagonal. First real visible
milestone: the kernel is now doing graphics."
```

---

## Task 4: Update README with new build instructions

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Rewrite `README.md`**

Replace `/Users/vojtechrichter/projects/rust_os/README.md` with:

```markdown
# DSOS (Dead Simple Operating System)

A hobby x86_64 kernel written in Rust.

## Requirements

- Rust nightly toolchain (the project uses `build-std` and Cargo's unstable `bindeps`)
- QEMU (`brew install qemu` on macOS)

## Building and running

```sh
cargo +nightly run
```

This builds the kernel (`kernel/`) for `x86_64-unknown-none`, builds a BIOS disk image via the host-side `runner/` crate (using `bootloader` 0.11's `DiskImageBuilder`), and launches QEMU.

## Layout

- `kernel/` — the kernel itself (`no_std`, `no_main`). Uses `bootloader_api` 0.11.
- `runner/` — host-side binary that builds the bootable disk image and launches QEMU.
- `docs/superpowers/` — design specs and implementation plans.
```

- [ ] **Step 2: Commit**

```bash
git add README.md
git commit -m "Update README for bootloader 0.11 workspace layout"
```

---

## Risks and what to do if things break

- **`bindeps` rejected:** Confirm you're on nightly (`rustc +nightly --version`) and that `.cargo/config.toml` has `[unstable] bindeps = true`.
- **Kernel triple-faults (QEMU reboots in a loop):** Most likely cause is a panic in `kernel_main` before reaching `halt_loop`. Check that `BootInfo`'s framebuffer is actually `Some` — the `expect("no framebuffer")` will trigger this on firmware that didn't honor our minimum-resolution request. Workaround: try lower minimums (e.g., 800×600) or remove the minimums entirely so the firmware picks anything.
- **Build can't find `qemu-system-x86_64`:** Install QEMU. The runner panics with a clear error message if QEMU is missing from `PATH`.
- **Stock `x86_64-unknown-none` target rejects the build:** Unlikely given that target ships with the disabled-redzone and soft-float defaults the old custom target file specified. If it does happen, restore the custom target file as `kernel/x86_64-unknown-none.json` and set `target = "../kernel/x86_64-unknown-none.json"` on the artifact dep. (Probably won't be needed.)
- **`put_pixel` shows the wrong color:** RGB/BGR branch is wrong; flip the match arms.
- **Gradient only fills part of the screen:** `stride` vs `width` confusion. Stride is in pixels and can exceed width (firmware padding). The byte offset MUST use `stride`, not `width`. The drawing loop bounds use `width` (visible area).

---

## Out of scope (next milestones)

- Bitmap font + framebuffer `println!` to restore text output.
- Smooth Mandelbrot at native framebuffer resolution.
- PIT timer + animation loop (plasma, fire, matrix rain).
- GDT/IDT, PS/2 keyboard, interactive shell.
- UEFI build target alongside BIOS.
