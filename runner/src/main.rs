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
