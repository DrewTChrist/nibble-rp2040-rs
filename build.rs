use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

fn main() {
    // Put `memory.x` in our output directory and ensure it's
    // on the linker search path.
    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());
    File::create(out.join("memory.x"))
        .unwrap()
        .write_all(include_bytes!("memory.x"))
        .unwrap();
    println!("cargo:rustc-link-search={}", out.display());

    // By default, Cargo will re-run a build script whenever
    // any file in the project changes. By specifying `memory.x`
    // here, we ensure the build script is only re-run when
    // `memory.x` is changed.
    println!("cargo:rerun-if-changed=memory.x");

    let rp2040_pro_micro = env::var_os("CARGO_FEATURE_RP2040_PRO_MICRO");
    let bit_c_rp2040 = env::var_os("CARGO_FEATURE_BIT_C_RP2040");

    if rp2040_pro_micro.is_some() {
        println!("cargo:rustc-cfg=feature = \"rp2040-pro-micro\"")
    } else if bit_c_rp2040.is_some() {
        println!("cargo:rustc-cfg=feature = \"bit-c-rp2040\"")
    } else {
        // Build for the kb2040 by default
        println!("cargo:rustc-cfg=feature = \"kb2040\"")
    }
}
