fn main() {
    // Only pass the linker script when targeting bare metal; the macOS/Linux
    // host linker (used by `cargo test`) does not understand -T.
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "none" {
        let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        println!("cargo:rustc-link-arg=-T{dir}/linker.ld");
    }
    println!("cargo:rerun-if-changed=linker.ld");
    println!("cargo:rerun-if-changed=src/arch/x86_64/boot.rs");
}
