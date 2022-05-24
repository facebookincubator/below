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

/// All available field id representations of the base `Model` struct.
///
/// Most can be parsed as `ModelFieldId` and used to query a Model. Some are
/// parameterized field ids (with angle bracketed variable like <idx>) that must
/// have the variable substituted with specific values to be parsed correctly.
///
/// For example, <idx> must be some zero-indexed Vec index, <cgroup_path> must
/// be a path and <key> could be pid, disk name, iface name etc.
///
/// This list also servers as documentation for available field ids that could
/// be used in other below crates. A test ensures that this list is up-to-date.
pub const MODEL_FIELD_IDS: &[&str] = &crate::common_field_ids::COMMON_MODEL_FIELD_IDS;
