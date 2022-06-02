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

use crate::SampleTree;

#[test]
fn sample_tree_create_test() {
    let st = SampleTree::new();
    assert_eq!(st.total, 0);
}

#[test]
fn sample_tree_add_empty_test() {
    let mut st = SampleTree::new();
    st.add(std::iter::empty());
    assert_eq!(st.total, 0);
}

#[test]
fn sample_tree_add_path_test() {
    let mut st = SampleTree::new();
    st.add("path/to/split".split('/'));
    st.add("another/path/to/split".split('/'));
    assert_eq!(st.total, 2);
    assert_eq!(st.children["path"].total, 1);
    assert_eq!(st.children["another"].total, 1);
}
