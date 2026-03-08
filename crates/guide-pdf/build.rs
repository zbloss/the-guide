use std::{env, path::PathBuf, process::Command};

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let lib_path = out_dir.join("libpdfium.so");

    if !lib_path.exists() {
        println!("cargo:warning=Downloading libpdfium.so...");

        let status = Command::new("bash")
            .args([
                "-c",
                &format!(
                    "set -eo pipefail; curl -fsSL \
                     'https://github.com/bblanchon/pdfium-binaries/releases/download/chromium%2F7713/pdfium-linux-x64.tgz' \
                     | tar -xz --strip-components=1 -C '{}' lib/libpdfium.so",
                    out_dir.display()
                ),
            ])
            .status()
            .expect("failed to invoke curl/tar — ensure both are installed");

        if !status.success() {
            panic!(
                "Failed to download libpdfium.so. \
                 Run scripts/get-pdfium.sh manually or ensure curl and tar are available."
            );
        }
    }

    println!("cargo:rustc-env=PDFIUM_LIB_PATH={}", lib_path.display());
    println!("cargo:rerun-if-changed=build.rs");
}
