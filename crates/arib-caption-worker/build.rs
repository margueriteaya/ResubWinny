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
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux") {
        // aribcaption is a static C++ archive. CMake's PRIVATE dependency
        // graph is not propagated through the cmake crate, so declare the
        // platform runtime and renderer libraries on the Rust link line.
        for library in [
            "stdc++",
            "fontconfig",
            "freetype",
            "expat",
            "pthread",
            "dl",
            "m",
        ] {
            println!("cargo:rustc-link-lib={library}");
        }
    }
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rustc-link-lib=c++");
        for framework in ["CoreFoundation", "CoreGraphics", "CoreText"] {
            println!("cargo:rustc-link-lib=framework={framework}");
        }
    }
    println!("cargo:rerun-if-changed=../../native/aribcaption-bridge");
    println!("cargo:rerun-if-changed=../../third_party/libaribcaption");
}
