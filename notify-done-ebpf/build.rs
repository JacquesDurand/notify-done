fn main() {
    // Bindings are generated based on the target architecture
    println!("cargo:rerun-if-env-changed=CARGO_CFG_BPF_TARGET_ARCH");
}
