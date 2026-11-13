//! iOS: links the SwiftUI launcher (packaging/ios/swift), when the build compiled it into the
//! static library `HORIZON_IOS_SWIFT_LIB` names, and sets `horizon_swift_ui` for
//! src/ios_launcher.rs. `HORIZON_IOS_SWIFT_RUNTIME` lists, `:`-separated, where the Swift
//! libraries it links to are (the SDK's and the toolchain's). Elsewhere it does nothing.
fn main() {
    println!("cargo:rustc-check-cfg=cfg(horizon_swift_ui)");
    println!("cargo:rerun-if-env-changed=HORIZON_IOS_SWIFT_LIB");
    println!("cargo:rerun-if-env-changed=HORIZON_IOS_SWIFT_RUNTIME");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("ios") {
        return;
    }
    let Ok(lib) = std::env::var("HORIZON_IOS_SWIFT_LIB") else {
        return;
    };
    let lib = std::path::Path::new(&lib);
    println!("cargo:rerun-if-changed={}", lib.display());
    println!("cargo:rustc-cfg=horizon_swift_ui");
    if let Some(dir) = lib.parent() {
        println!("cargo:rustc-link-search=native={}", dir.display());
    }
    let name = lib
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.strip_prefix("lib").unwrap_or(s))
        .unwrap_or("HorizonUI");
    println!("cargo:rustc-link-lib=static={name}");
    if let Ok(dirs) = std::env::var("HORIZON_IOS_SWIFT_RUNTIME") {
        for dir in dirs.split(':').filter(|d| !d.is_empty()) {
            println!("cargo:rustc-link-search=native={dir}");
        }
    }
    for framework in ["SwiftUI", "UIKit", "Foundation", "UserNotifications"] {
        println!("cargo:rustc-link-lib=framework={framework}");
    }
}
