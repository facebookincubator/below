// Copyright (c) Facebook, Inc. and its affiliates.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::env;
use std::path::PathBuf;

use libbpf_cargo::SkeletonBuilder;

const SRC: &str = "./src/bpf/exitstat.bpf.c";

fn main() {
    let mut out =
        PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR must be set in build script"));
    out.push("exitstat.skel.rs");

    let mut builder = SkeletonBuilder::new();
    builder.source(SRC);
    if let Some(clang) = option_env!("CLANG") {
        builder.clang(clang);
    }
    builder.build_and_generate(out).unwrap();
    println!("cargo:rerun-if-changed={}", SRC);

    #[cfg(all(feature = "no-vendor", feature = "default"))]
    compile_error!(
        "In order to build without vendored dependencies please disable default features"
    );
}
