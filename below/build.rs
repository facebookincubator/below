use std::env;
use std::path::Path;

use libbpf_cargo::SkeletonBuilder;

const SRC: &str = "./src/bpf/exitstat.bpf.c";

fn main() {
    let skel = Path::new(&env::var("OUT_DIR").unwrap()).join("exitstat.skel.rs");
    SkeletonBuilder::new(SRC).generate(&skel).unwrap();
    println!("cargo:rerun-if-changed={}", SRC);
}
