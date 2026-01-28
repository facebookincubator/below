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

#![deny(clippy::all)]

pub mod cliutil;
pub mod cpuset;
pub mod dateutil;
pub mod fileutil;
pub mod logutil;
pub mod util;

pub use cpuset::Cpuset;

// Shim between facebook types and open source types.
//
// The type interfaces and module hierarchy should be identical on
// both "branches". And since we glob import, all the submodules in
// this crate will inherit our name bindings and can use generic paths,
// eg `crate::logging::setup(..)`.
#[macro_export]
macro_rules! open_source_shim {
    ($v:vis) => {
        #[cfg(fbcode_build)]
        mod facebook;
        #[cfg(fbcode_build)]
        $v use facebook::*;
        #[cfg(not(fbcode_build))]
        mod open_source;
        #[cfg(not(fbcode_build))]
        $v use open_source::*;
    };
    () => {
        open_source_shim!(pub(self));
    };
}
