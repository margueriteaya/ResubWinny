fn main() {
    // Tauri validates bundle resources while compiling its build script. A
    // normal debug check should not require a prebuilt release Worker; release
    // builds still validate and bundle every resource from tauri.conf.json.
    if std::env::var("PROFILE").as_deref() != Ok("release") {
        // SAFETY: build scripts run as a single process before tauri-build is
        // invoked, so no other thread can observe this environment mutation.
        unsafe {
            std::env::set_var("TAURI_CONFIG", r#"{"bundle":{"resources":[]}}"#);
        }
    }
    tauri_build::build()
}
