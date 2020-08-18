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
        "'?'            - toggle help menu\n",
        "\n",
        "<DOWN>         - scroll down primary display\n",
        "<UP>           - scroll up primary display\n",
        "<PgDn>         - scroll down 15 lines primary display\n",
        "<PgUp>         - scroll up 15 lines primary display\n",
        "<Home>         - scroll to top of primary display\n",
        "<End>          - scroll to end of primary display\n",
        "<Enter>        - collapse/expand cgroup tree\n",
        "<Tab>          - cycle topic tabs\n",
        "<Shift><Tab>   - reverse cycle topic tabs\n",
        "<Space>        - pause/resume the live mode. While pausing, use t/T to iterate sample forward and backward",
        "','            - reverse cycle columns.\n",
        "'.'            - cycle columns.\n",
        "'S'            - sort by the selected title, reverse the result by hit 'S' again(cgroup view and process view only)\n",
        "'t'            - show next sample (replay and live-paused mode)\n",
        "'T'            - show previous sample (replay and live-paused mode)\n",
        "'j'            - jump time by a specific amount forward or to a specific timestamp (replay and live-paused mode)\n",
        "'J'            - jump time by a specific amount backward or to a specific timestamp (replay and live-paused mode)\n",
        "'c'            - show cgroup view\n",
        "'p'            - show process view\n",
        "'s'            - show system core view\n",
        "'q'            - quit or exit help\n",
        "'P'            - sort by pid (process view only)\n",
        "'N'            - sort by name (process view only)\n",
        "'C'            - sort by cpu (cgroup view and process view only)\n",
        "'M'            - sort by memory (cgroup view and process view only)\n",
        "'D'            - sort by total disk activity(cgroup view and process view only)\n",
        "'z'            - zoom into process view filtered by selected cgroup\n",
        "'/'            - filter by name\n",
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
