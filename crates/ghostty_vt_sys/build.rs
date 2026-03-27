use std::path::PathBuf;
use std::process::Command;

fn main() {
    let manifest_dir = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let workspace_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("ghostty_vt_sys must live under crates/*");

    let ghostty_dir = workspace_root.join("vendor/ghostty");
    println!(
        "cargo:rerun-if-changed={}",
        ghostty_dir.join("build.zig.zon").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        ghostty_dir.join("build.zig").display()
    );

    if !ghostty_dir.exists() {
        panic!(
            "vendor/ghostty is missing; run `git submodule update --init --recursive` and retry"
        );
    }

    let zig = find_zig(workspace_root);
    if Command::new(&zig).arg("version").output().is_err() {
        panic!("`zig` 0.15.2+ is required on PATH or set ZIG env var");
    }

    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let prefix = out_dir.join("ghostty-vt-out");

    let status = Command::new(&zig)
        .current_dir(&ghostty_dir)
        .arg("build")
        .arg("-Demit-lib-vt=true")
        .arg("-Doptimize=ReleaseFast")
        .arg("--prefix")
        .arg(&prefix)
        .status()
        .expect("failed to invoke zig");
    if !status.success() {
        panic!("zig build failed");
    }

    println!(
        "cargo:rustc-link-search=native={}",
        prefix.join("lib").display()
    );
    println!("cargo:rustc-link-lib=static=ghostty-vt");
    println!("cargo:rustc-link-lib=c");
}

fn find_zig(workspace_root: &std::path::Path) -> PathBuf {
    if let Some(path) = std::env::var_os("ZIG") {
        return PathBuf::from(path);
    }

    if Command::new("zig").arg("version").output().is_ok() {
        return PathBuf::from("zig");
    }

    workspace_root.join(".context/zig/zig")
}
