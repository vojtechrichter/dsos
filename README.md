# DSOS (Dead Simple Operating System)

A hobby x86_64 kernel written in Rust.

## Requirements

- Rust nightly toolchain (the project uses Cargo's unstable `bindeps`)
- The `x86_64-unknown-none` target installed: `rustup +nightly target add x86_64-unknown-none`
- QEMU (`brew install qemu` on macOS)

## Building and running

`cargo +nightly run`

This builds the kernel (`kernel/`) for `x86_64-unknown-none`, builds a BIOS disk image via the host-side `runner/` crate (using `bootloader` 0.11's `DiskImageBuilder`), and launches QEMU.

## Layout

- `kernel/` — the kernel itself (`no_std`, `no_main`). Uses `bootloader_api` 0.11.
- `runner/` — host-side binary that builds the bootable disk image and launches QEMU.
- `docs/superpowers/` — design specs and implementation plans.
