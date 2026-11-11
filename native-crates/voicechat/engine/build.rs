//! iOS: libopus is linked in, not loaded (an iOS app can't load a library it doesn't carry
//! signed). The iOS workflow builds it and names its folder in `HORIZON_OPUS_DIR`
//! (.github/workflows/ios.yml). Elsewhere the system's is loaded at run time (opus.rs).
fn main() {
    println!("cargo:rerun-if-env-changed=HORIZON_OPUS_DIR");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("ios") {
        return;
    }
    match std::env::var("HORIZON_OPUS_DIR") {
        Ok(dir) => {
            println!("cargo:rustc-link-search=native={dir}");
            println!("cargo:rustc-link-lib=static=opus");
            println!("cargo:rustc-cfg=opus_linked");
        }
        // A check without the library (`cargo check` off a Mac): the voice chat says it has no
        // Opus instead of linking one.
        Err(_) => println!("cargo:warning=HORIZON_OPUS_DIR is not set: the iOS voice chat has no Opus"),
    }
}
