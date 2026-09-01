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

    if std::env::var_os("CARGO_FEATURE_LIBARIBTLV").is_some() {
        let out_dir = std::path::PathBuf::from(
            std::env::var_os("OUT_DIR").expect("Cargo must provide OUT_DIR"),
        )
        .join("libaribtlv");
        let tlv_bridge = cmake::Config::new("../../native/aribtlv-bridge")
            .profile("Release")
            .out_dir(out_dir)
            .build();
        println!(
            "cargo:rustc-link-search=native={}",
            tlv_bridge.join("lib").display()
        );
        println!("cargo:rustc-link-lib=static=resub_aribtlv_bridge");
        println!("cargo:rustc-link-lib=static=aribtlv");
        println!("cargo:rustc-link-lib=static=resub_zlib");
        println!("cargo:rerun-if-changed=../../native/aribtlv-bridge");
        println!("cargo:rerun-if-changed=../../third_party/libaribtlv");
        println!("cargo:rerun-if-changed=../../third_party/zlib");
    }
}
