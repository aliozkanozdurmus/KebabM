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
        // rfd imports TaskDialogIndirect, which requires Common Controls v6.
        // Embed through the linker so Rust's separate test executables get it too.
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTDEPENDENCY:type='win32' name='Microsoft.Windows.Common-Controls' version='6.0.0.0' processorArchitecture='*' publicKeyToken='6595b64144ccf1df' language='*'");
        // Remove static CRT, add dynamic CRT to resolve __imp_* symbols
        println!("cargo:rustc-link-arg=/NODEFAULTLIB:libucrt.lib");
        println!("cargo:rustc-link-arg=/DEFAULTLIB:ucrt.lib");
    }
    #[cfg(target_os = "windows")]
    tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(
        // The linker embeds the dependency above; a second manifest in the
        // application's .res would produce a duplicate resource (CVT1100).
        tauri_build::WindowsAttributes::new_without_app_manifest(),
    ))
    .expect("failed to run tauri-build");
    #[cfg(not(target_os = "windows"))]
    tauri_build::build();
}
