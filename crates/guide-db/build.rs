fn main() {
    // DuckDB 1.1+ on Windows uses the Restart Manager API (RmStartSession etc.)
    // from rstrtmgr.lib, but libduckdb-sys doesn't emit this link directive.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rustc-link-lib=rstrtmgr");
    }
    println!("cargo:rerun-if-changed=build.rs");
}
