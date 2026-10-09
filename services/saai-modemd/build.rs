use std::path::PathBuf;
use std::process::Command;

fn main() {
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    if arch != "aarch64" {
        return;
    }
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let source = manifest.join("../../os/targets/panther/diagnostics/modem-rfs-full-quarantine-owner.c");
    let source = source.canonicalize().expect("camp owner source");
    println!("cargo:rerun-if-changed={}", source.display());
    let include = source.parent().expect("diagnostics dir");
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("out dir")).join("camp_owner.o");
    let zig = std::env::var("ZIG").unwrap_or_else(|_| "zig".to_string());
    let status = Command::new(&zig)
        .arg("cc")
        .args([
            "-target",
            "aarch64-linux-musl",
            "-std=c11",
            "-O2",
            "-Wall",
            "-Wextra",
            "-Wno-unused-function",
        ])
        .arg("-DSAAIOS_RFS_CAMP")
        .arg("-DSAAIOS_EMBEDDED_OWNER")
        .arg("-I")
        .arg(include)
        .arg("-c")
        .arg(&source)
        .arg("-o")
        .arg(&out)
        .status()
        .unwrap_or_else(|error| panic!("starting {zig} cc: {error}"));
    if !status.success() {
        panic!("camp owner compile failed");
    }
    println!("cargo:rustc-link-arg={}", out.display());
}
