fn main() {
    #[cfg(target_os = "macos")]
    build_set_default_output_helper();
    tauri_build::build();
}

#[cfg(target_os = "macos")]
fn build_set_default_output_helper() {
    use std::path::PathBuf;
    use std::process::Command;

    let target = std::env::var("TARGET").expect("TARGET");
    println!("cargo:rustc-env=SCRIBEFLOAT_TARGET_TRIPLE={target}");
    println!("cargo:rerun-if-changed=Swift/SetDefaultOutput/main.swift");

    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let src = manifest_dir.join("Swift/SetDefaultOutput/main.swift");
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    let dest = out_dir.join("set-default-output");

    let status = Command::new("swiftc")
        .args(["-O", "-o"])
        .arg(&dest)
        .arg(&src)
        .status()
        .expect("run swiftc");
    if !status.success() {
        panic!(
            "failed to compile set-default-output helper (is Xcode command line tools installed?)"
        );
    }

    // Runtime path for dev/debug builds (under target/, not src-tauri/binaries/).
    println!(
        "cargo:rustc-env=SCRIBEFLOAT_SET_DEFAULT_OUTPUT_HELPER={}",
        dest.display()
    );

    // Tauri `externalBin` validates triple-suffixed binaries at build time (dev and release).
    // Copy only when missing or stale so `tauri dev` does not rewrite an unchanged file every
    // rebuild and retrigger the file watcher in a loop.
    let bundle_dir = manifest_dir.join("binaries");
    std::fs::create_dir_all(&bundle_dir).expect("create binaries dir");
    let bundle_dest = bundle_dir.join(format!("set-default-output-{target}"));
    copy_helper_if_changed(&dest, &bundle_dest);
}

#[cfg(target_os = "macos")]
fn copy_helper_if_changed(src: &std::path::Path, dest: &std::path::Path) {
    use std::io::Read;

    let src_bytes = std::fs::read(src).expect("read compiled set-default-output helper");
    if dest.is_file() {
        let mut dest_bytes = Vec::new();
        std::fs::File::open(dest)
            .and_then(|mut f| f.read_to_end(&mut dest_bytes))
            .expect("read existing set-default-output bundle binary");
        if dest_bytes == src_bytes {
            return;
        }
    }
    std::fs::write(dest, src_bytes).expect("write set-default-output bundle binary");
}
