fn main() {
    let linker_script = std::env::var("DEP_ZEBRA_LIBRS_LINKER_SCRIPT").unwrap();
    println!("cargo:rustc-link-arg=-T{linker_script}");
    println!("cargo:rerun-if-changed=build.rs");
}
