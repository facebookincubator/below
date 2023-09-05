/// This build script generates Rust bindings for the ethtool API.
fn main() {
    let api_dir = "src/api";

    let bindings = bindgen::Builder::default()
        .header(format!("{}/ethtool.h", api_dir))
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("ethtool_bindings.rs"))
        .expect("Couldn't write bindings!");
}
