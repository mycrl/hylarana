fn main() {
    println!("cargo:rerun-if-changed=src/delegate");

    #[cfg(target_os = "macos")]
    cc::Build::new()
        .file("src/delegate/delegate.m")
        .compile("delegate");
}
