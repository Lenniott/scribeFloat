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
    let profile = std::env::var("PROFILE").unwrap_or_default();
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

    // Tauri externalBin expects triple-suffixed binaries here at bundle time only.
    // Writing this during `tauri dev` retriggers the file watcher in an infinite loop.
    if profile == "release" {
        let bundle_dir = manifest_dir.join("binaries");
        std::fs::create_dir_all(&bundle_dir).expect("create binaries dir");
        let bundle_dest = bundle_dir.join(format!("set-default-output-{target}"));
        std::fs::copy(&dest, &bundle_dest).expect("copy set-default-output for bundle");
    }
}
