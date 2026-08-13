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

// Publish the directory holding vmlinux.h / vmlinux_601.h to dependents' build
// scripts. Because this crate sets `links`, cargo forwards `include` to each
// direct dependent's build script as `DEP_BELOW_VMLINUX_INCLUDE`; cgroupfs and
// below read it and pass it as `-I` to the BPF skeleton builder.
fn main() {
    let dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set");
    println!("cargo:include={dir}");
    println!("cargo:rerun-if-changed=vmlinux.h");
    println!("cargo:rerun-if-changed=vmlinux_601.h");
}
