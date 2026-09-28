fn main() {
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");
        // Swift bridges also need the compatibility libraries when only CLT is installed.
        if let Ok(output) = std::process::Command::new("xcode-select").arg("-p").output() {
            let root = String::from_utf8_lossy(&output.stdout).trim().to_owned();
            for relative in ["usr/lib/swift/macosx", "Toolchains/XcodeDefault.xctoolchain/usr/lib/swift/macosx"] {
                let path = std::path::Path::new(&root).join(relative);
                if path.is_dir() { println!("cargo:rustc-link-search=native={}", path.display()); }
            }
        }
    }
    // Fix CRT linking: ort_sys and whisper_rs_sys are compiled with /MD (dynamic CRT)
    // but Rust uses /MT (static CRT). We need to swap static CRT for dynamic CRT.
    #[cfg(target_os = "windows")]
    {
        // Remove static CRT, add dynamic CRT to resolve __imp_* symbols
        println!("cargo:rustc-link-arg=/NODEFAULTLIB:libucrt.lib");
        println!("cargo:rustc-link-arg=/DEFAULTLIB:ucrt.lib");
    }
    tauri_build::build()
}
