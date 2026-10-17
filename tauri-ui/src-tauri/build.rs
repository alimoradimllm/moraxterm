fn main() {
    // The embedded Unix runtime is a Windows-only MSYS2 archive (see provision_embedded_runtime,
    // which returns early on other platforms). Require it only when building for Windows, so a
    // clean Linux checkout builds without a 137 MB file that Linux never uses.
    let target_is_windows = std::env::var("CARGO_CFG_TARGET_OS").is_ok_and(|os| os == "windows");
    let resource = std::path::Path::new("resources/Moraxterm_EmbeddedUnix_x64_0.1.24.tar.xz");
    println!("cargo:rerun-if-changed={}", resource.display());
    if target_is_windows && !resource.is_file() {
        panic!(
            "embedded Unix runtime is missing: {}. Stage it with `python scripts/prepare_embedded_runtime.py --archive <archive.tar.xz>` before building.",
            resource.display()
        );
    }
    tauri_build::build();
}
