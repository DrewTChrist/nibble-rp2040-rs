use std::env;

fn main() {
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
