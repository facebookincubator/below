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

//! Vendored kernel BTF header (`vmlinux.h`) shared by below's BPF programs.
//!
//! This crate has no Rust API. It carries the version-pinned `vmlinux_601.h`
//! (and the stable `vmlinux.h` include name) so `cgroupfs` and the `below`
//! binary compile their BPF against one copy in the open-source/cargo build. Its
//! `build.rs` exposes the header directory to dependents' build scripts as
//! `DEP_BELOW_VMLINUX_INCLUDE`, which they add to the BPF compiler's include
//! path. The buck build gets the header via the `bpf_header_library` targets in
//! BUCK instead.
