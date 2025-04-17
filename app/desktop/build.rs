fn main() {
    println!("cargo:rerun-if-changed=src/patch");

    #[cfg(target_os = "macos")]
    cc::Build::new().file("src/patch/mac.m").compile("patch");
}
