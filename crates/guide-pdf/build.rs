use std::{env, path::PathBuf, process::Command};

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    let (lib_name, url, inner_path) = if target_os == "windows" {
        (
            "pdfium.dll",
            "https://github.com/bblanchon/pdfium-binaries/releases/download/chromium%2F7713/pdfium-win-x64.tgz",
            "bin/pdfium.dll",
        )
    } else {
        (
            "libpdfium.so",
            "https://github.com/bblanchon/pdfium-binaries/releases/download/chromium%2F7713/pdfium-linux-x64.tgz",
            "lib/libpdfium.so",
        )
    };

    let lib_path = out_dir.join(lib_name);

    if !lib_path.exists() {
        println!("cargo:warning=Downloading {lib_name}...");

        let tgz_path = out_dir.join("pdfium.tgz");

        let dl_ok = Command::new("curl")
            .args(["-fsSL", "-o"])
            .arg(&tgz_path)
            .arg(url)
            .status()
            .expect("failed to invoke curl — ensure it is installed")
            .success();

        if !dl_ok {
            panic!("Failed to download pdfium binary from {url}. Ensure curl is installed.");
        }

        let extract_ok = Command::new("tar")
            .arg("-xzf")
            .arg(&tgz_path)
            .arg("--strip-components=1")
            .arg("-C")
            .arg(&out_dir)
            .arg(inner_path)
            .status()
            .expect("failed to invoke tar — ensure it is installed")
            .success();

        let _ = std::fs::remove_file(&tgz_path);

        if !extract_ok {
            panic!(
                "Failed to extract {inner_path} from pdfium archive. \
                 Ensure tar is available and the download succeeded."
            );
        }
    }

    println!("cargo:rustc-env=PDFIUM_LIB_PATH={}", lib_path.display());
    println!("cargo:rustc-env=PDFIUM_LIB_NAME={lib_name}");
    println!("cargo:rerun-if-changed=build.rs");
}
