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
use cursive::Cursive;
use cursive::event::Event;

use super::*;
use crate::stats_view::StatsView;
use crate::stats_view::ViewBridge;

make_event_controller!(
    GpuView,
    "__unused_gpu",
    "",
    vec![Event::Char('g')],
    |_, _| {},
    |_, _| {}
);

make_event_controller!(
    GpuProcessView,
    "__unused_gpu_process",
    "",
    vec![Event::Char('G')],
    |_, _| {},
    |_, _| {}
);

make_event_controller!(
    GpuZoomView,
    "__unused_gpu_zoom",
    "",
    vec![Event::Char('x')],
    |_, _| {},
    |_, _| {}
);
