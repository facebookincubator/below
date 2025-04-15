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

//! Controllers module is used for how below will react on a user's input.
//!
//! # Terms
//! ## Command
//! A command is the name or id of a certain below view behavior. Each command will be
//! mapped to a uniq EventController and can be explicitly called from CommandPalette.
//!
//! ## Event
//! An Event is a trigger of an EventController. Each EventController has a default
//! Event associate with it and can be customized by an cmdrc file.
//!
//! ## EventController
//! An EventController is the handler of a "Command". Each EventController will have the following 4 pieces:
//! * command: The command of this EventController
//! * default_events: An array of default event triggers for this EventController
//! * handle: How should below handle such event given the currst StatsView<T>
//! * callback: How should below handle such event with a cursive object.
//! EventController is a interface ONLY struct.
//!
//! ## Controllers
//! The Controllers is a enum of controllers and each enum value will be mapped to a uniq EventController. This
//! will help us to unify EventController types. Controllers will provide similar interface as EventController
//! except it will take a reference of self object. The impl of Controllers does nothing but call the corresponding
//! fn in the associated EventController.
//!
//! # Construction
//! ## make_event_controller
//! Convenience macro of making an EventController.
//!
//! ## make_controllers
//! make_controllers macro will take a series of (enum value: EventController) pairs and generate the Controllers
//! enum. Besides that, it will also generate HashMap construction fns: make_event_controller_map and
//! make_cmd_controller_map
//!
//! ## make_event_controller_map && make_cmd_controller_map
//! On constructing global ViewState, we will have two HashMap refcells constructed: event_controller_map and
//! cmd_controller_map. As their name indicated, they are maps between "Event" or "Command" to their corresponding
//! Controllers enum values. The event_controller_map will be referenced by StatsView<T> and cmd_controller_map
//! will be referenced by CommandPalette. While generating the event_controller_map, we will read the cmdrc file
//! to replace default Event with user specified one.
//!
//! # Calling flow
//! ## Event to EventController
//! 1. User typed something when not in the "command mode". For example "c".
//! 2. StatsView<T> capture the cursive event. For example Event::char('c').
//! 3. StatsView<T> trys to find the corresponding Controllers value in event_controller_map
//!   3.1 if not found, send the event to its parent and return.
//!   3.2 if found, get the Controllers value. For example Controllers::Cgroup.
//! 4. Invoke the handle function. For example Controllers::Cgroup.hanle()
//! 5. Invoke the callback function. For example Controllers::Cgroup.callback()
//! 6. Mark the event as consumed.
//!
//! ## Command to EventController
//! 1. User typed something in "command mode" and hit enter. For example: "cgroup".
//! 2. CommandPalette capture the input and try to find the corresponding Controllers value in cmd_controller_map
//!   2.1 if not found, raise error message
//!   2.2 if found, get the Controllers value. For example Controllers::Cgroup
//! 3. Invoke the handle function. For example Controllers::Cgroup.hanle()
//! 4. Invoke the callback function. For example Controllers::Cgroup.callback()
use cursive::Cursive;
use cursive::event::Event;
use cursive::event::EventTrigger;
use cursive::event::Key;
use toml::value::Value;

#[macro_use]
mod controller_infra;
mod content_controllers;
mod sample_controllers;
mod view_controllers;

#[cfg(test)]
mod test;

use common::open_source_shim;
use content_controllers::*;
use controller_infra::*;
use sample_controllers::*;
use view_controllers::*;

use crate::ViewState;
use crate::refresh;
use crate::stats_view::StateCommon;
use crate::stats_view::StatsView;
use crate::stats_view::ViewBridge;

open_source_shim!();

use std::collections::HashMap;

pub use controller_infra::event_to_string;
pub use controller_infra::str_to_event;

make_controllers!(
    CmdPalette: InvokeCmdPalette,
    NextTab: NextTabImpl,
    PrevTab: PrevTabImpl,
    NextCol: NextColImpl,
    PrevCol: PrevColImpl,
    Right: RightImpl,
    Left: LeftImpl,
    SortCol: SortByColumn,
    Filter: FilterPopup,
    CFilter: ClearFilter,
    JForward: JumpForward,
    JBackward: JumpBackward,
    NSample: NextSample,
    PSample: PrevSample,
    Pause: PauseImpl,
    Quit: QuitImpl,
    Help: HelpMenu,
    Process: ProcessView,
    Cgroup: CgroupView,
    System: SystemView,
    Gpu: GpuView,
    GpuProcess: GpuProcessView,
    GpuZoom: GpuZoomView,
    Zoom: ZoomView,
    Fold: FoldProcessView,
    NextPage: NextPageImpl,
    PrevPage: PrevPageImpl,
    NextSelection: NextSelectionImpl,
    PrevSelection: PrevSelectionImpl,
);
