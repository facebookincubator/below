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
use std::ffi::OsString;
use std::path::PathBuf;

use libbpf_cargo::SkeletonBuilder;

const CGROUP_BPF_SRC: &str = "./src/bpf/cgroup_bpf.bpf.c";
const CGROUP_BPF_RECORD_HEADER: &str = "./src/bpf/cgroup_bpf_record.h";

fn main() {
    let out_dir =
        PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR must be set in build script"));

    // BPF skeleton for cgroup_bpf.bpf.c.
    let mut skel_out = out_dir.clone();
    skel_out.push("cgroup_bpf.skel.rs");

    // The BPF program includes <vmlinux.h> from the shared below-vmlinux crate,
    // which publishes its header directory as DEP_BELOW_VMLINUX_INCLUDE (see its
    // build.rs); add it to the include path.
    let vmlinux_include = env::var_os("DEP_BELOW_VMLINUX_INCLUDE")
        .expect("DEP_BELOW_VMLINUX_INCLUDE must be set by the below-vmlinux build dependency");

    // Rebuild the skeleton when the shared vmlinux headers change: the .bpf.c
    // includes <vmlinux.h> (which pulls in vmlinux_601.h) from this include dir.
    // DEP_BELOW_VMLINUX_INCLUDE's value is stable, so without these Cargo would
    // not re-run this script when the vendored headers are edited.
    let vmlinux_dir = PathBuf::from(&vmlinux_include);
    for header in ["vmlinux.h", "vmlinux_601.h"] {
        println!(
            "cargo:rerun-if-changed={}",
            vmlinux_dir.join(header).display()
        );
    }

    // -Wno-incompatible-pointer-types: the BPF program uses the CO-RE
    // "versioned struct" pattern (e.g. `struct mm_struct___pre62`) and assigns a
    // real kernel pointer to a versioned type. clang >= 15 promotes
    // -Wincompatible-pointer-types to a hard error, so downgrade it (matches
    // libbpf/kernel selftests).
    let clang_args: [OsString; 3] = [
        OsString::from("-Wno-incompatible-pointer-types"),
        OsString::from("-I"),
        vmlinux_include,
    ];

    let mut builder = SkeletonBuilder::new();
    builder.source(CGROUP_BPF_SRC);
    builder.clang_args(clang_args);
    if let Some(clang) = option_env!("CLANG") {
        builder.clang(clang);
    }
    builder
        .build_and_generate(skel_out)
        .expect("should build and generate the cgroup_bpf skeleton");
    println!("cargo:rerun-if-changed={CGROUP_BPF_SRC}");

    // Rust bindings for `struct cgroup_bpf_record`, generated from the same
    // header the BPF program includes and the fbcode `rust_bindgen_library` uses,
    // so the wire format never drifts. Generating here (rather than checking in a
    // copy) keeps the open-source bindings in lockstep with the header. The header
    // is self-contained (plain integer types), so it needs no include path; the
    // flags mirror the buck bindgen target.
    let mut record_out = out_dir;
    record_out.push("cgroup_bpf_record.rs");
    bindgen::Builder::default()
        .header(CGROUP_BPF_RECORD_HEADER)
        .derive_default(true)
        .prepend_enum_name(false)
        .generate()
        .expect("Failed to generate cgroup_bpf_record bindings")
        .write_to_file(&record_out)
        .expect("Failed to write cgroup_bpf_record bindings");
    println!("cargo:rerun-if-changed={CGROUP_BPF_RECORD_HEADER}");
}
