fn main() {
    napi_build::setup();
    // `openpty` lives in libutil on Linux. The reference guest uses it for TTY exec.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux") {
        println!("cargo:rustc-link-lib=util");
    }
}
