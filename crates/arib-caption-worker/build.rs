fn main() {
    let bridge = cmake::Config::new("../../native/aribcaption-bridge")
        .profile("Release")
        .build();
    println!(
        "cargo:rustc-link-search=native={}",
        bridge.join("lib").display()
    );
    println!("cargo:rustc-link-lib=static=arib_caption_bridge");
    println!("cargo:rustc-link-lib=static=aribcaption");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        for library in ["ole32", "d2d1", "dwrite", "windowscodecs"] {
            println!("cargo:rustc-link-lib={library}");
        }
    }
    println!("cargo:rerun-if-changed=../../native/aribcaption-bridge");
    println!("cargo:rerun-if-changed=../../third_party/libaribcaption");
}
