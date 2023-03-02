fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=link.ld");
    println!("cargo:rustc-link-arg=-T{manifest_dir}/link.ld");
}
