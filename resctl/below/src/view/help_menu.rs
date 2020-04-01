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

use cursive::view::{Identifiable, View};
use cursive::views::{LinearLayout, TextView};

fn fill_content(v: &mut LinearLayout) {
    let lines = vec![
        "'?'     - toggle help menu\n",
        "\n",
        "<DOWN>  - scroll down primary display\n",
        "<UP>    - scroll up primary display\n",
        "<PgDn>  - scroll down 15 lines primary display\n",
        "<PgUp>  - scroll up 15 lines primary display\n",
        "<Home>  - scroll to top of primary display\n",
        "<End>   - scroll to end of primary display\n",
        "<Enter> - collapse/expand cgroup tree\n",
        "'t'     - show next sample (replay mode)\n",
        "'T'     - show previous sample (replay mode)\n",
        "'c'     - show cgroup view\n",
        "'p'     - show process view\n",
        "'q'     - quit or exit help\n",
        "'P'     - sort by pid (process view only)\n",
        "'N'     - sort by name\n",
        "'C'     - sort by cpu\n",
        "'M'     - sort by memory\n",
        "'D'     - sort by total disk activity\n",
        "'z'     - zoom into process view filtered by selected cgroup\n",
    ];

    for line in lines {
        v.add_child(TextView::new(line));
    }
}

pub fn new() -> impl View {
    let mut view = LinearLayout::vertical();
    fill_content(&mut view);
    view.with_name("help_menu")
}
