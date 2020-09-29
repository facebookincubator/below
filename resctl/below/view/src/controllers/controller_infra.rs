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
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_macros)]

use super::*;

/// Convert a string to cursive Key enum
fn str_to_key(cmd: &str) -> Option<Key> {
    match cmd.trim().to_lowercase().as_str() {
        "tab" => Some(Key::Tab),
        "enter" => Some(Key::Enter),
        "backspace" => Some(Key::Backspace),
        "left" => Some(Key::Left),
        "right" => Some(Key::Right),
        "up" => Some(Key::Up),
        "down" => Some(Key::Down),
        "ins" => Some(Key::Ins),
        "del" => Some(Key::Del),
        "home" => Some(Key::Home),
        "end" => Some(Key::End),
        "page_up" => Some(Key::PageUp),
        "page_down" => Some(Key::PageDown),
        "pause_break" => Some(Key::PauseBreak),
        "esc" => Some(Key::Esc),
        _ => None,
    }
}

/// Convert a command to Cursive event.
// This fn is used while parsing the user's cmdrc file and generate
// a customized command-event map
pub fn str_to_event(cmd: &str) -> Option<Event> {
    if cmd.len() == 1 {
        return Some(Event::Char(
            cmd.chars()
                .next()
                .expect("Failed to parse first char from command"),
        ));
    }

    let cmd_vec = cmd.split('-').collect::<Vec<&str>>();
    match cmd_vec.len() {
        1 => {
            if let Some(k) = str_to_key(cmd_vec[0]) {
                Some(Event::Key(k))
            } else {
                None
            }
        }
        2 => match cmd_vec[0].trim().to_lowercase().as_str() {
            "ctrl" if cmd_vec[1].len() == 1 => Some(Event::CtrlChar(
                cmd_vec[1]
                    .chars()
                    .next()
                    .expect("Failed to parse first char from ctrl-command"),
            )),
            "alt" if cmd_vec[1].len() == 1 => Some(Event::AltChar(
                cmd_vec[1]
                    .chars()
                    .next()
                    .expect("Failed to parse first char from alt-command"),
            )),
            "shift" => {
                if let Some(k) = str_to_key(cmd_vec[1]) {
                    Some(Event::Shift(k))
                } else {
                    None
                }
            }
            "alt" => {
                if let Some(k) = str_to_key(cmd_vec[1]) {
                    Some(Event::Alt(k))
                } else {
                    None
                }
            }
            "altshift" => {
                if let Some(k) = str_to_key(cmd_vec[1]) {
                    Some(Event::AltShift(k))
                } else {
                    None
                }
            }
            "ctrl" => {
                if let Some(k) = str_to_key(cmd_vec[1]) {
                    Some(Event::Ctrl(k))
                } else {
                    None
                }
            }
            "ctrlshift" => {
                if let Some(k) = str_to_key(cmd_vec[1]) {
                    Some(Event::CtrlShift(k))
                } else {
                    None
                }
            }
            "ctrlalt" => {
                if let Some(k) = str_to_key(cmd_vec[1]) {
                    Some(Event::CtrlAlt(k))
                } else {
                    None
                }
            }
            _ => None,
        },
        _ => None,
    }
}

/// Common trait that each controller should implement, more details in the module
/// level doc.
pub trait EventController {
    /// Return the command for this controller
    fn command() -> &'static str;

    /// Return the Event trigger for this controller
    fn default_event() -> Event;

    /// Handler for event, for event that don't need a cursive object
    fn handle<T: 'static + ViewBridge>(_view: &mut StatsView<T>, _cmd_vec: &[&str]) {}

    /// Callback for event, for event that need a cursive object
    fn callback<T: 'static + ViewBridge>(_c: &mut Cursive, _cmd_vec: &[&str]) {}
}

/// Macro to make view event controller
/// # Argument
/// * name - Struct name
/// * cmd - command string
/// * event - event trigger. Will be replaced with custom command from cmdrc
/// * handle - handler closure for view level processing
/// * callback - callback closure for cursive level processing
macro_rules! make_event_controller {
    ($name:ident, $cmd:expr, $event:expr, $handle:expr) => {
        pub struct $name;

        impl EventController for $name {
            fn command() -> &'static str {
                $cmd
            }

            fn default_event() -> Event {
                $event
            }

            fn handle<T: 'static + ViewBridge>(view: &mut StatsView<T>, cmd_vec: &[&str]) {
                $handle(view, cmd_vec)
            }
        }
    };

    ($name:ident, $cmd:expr, $event:expr, $handle:expr, $callback:expr) => {
        pub struct $name;

        impl EventController for $name {
            fn command() -> &'static str {
                $cmd
            }

            fn default_event() -> Event {
                $event
            }

            fn handle<T: 'static + ViewBridge>(view: &mut StatsView<T>, cmd_vec: &[&str]) {
                $handle(view, cmd_vec)
            }

            fn callback<T: 'static + ViewBridge>(c: &mut Cursive, cmd_vec: &[&str]) {
                $callback(c, cmd_vec)
            }
        }
    };
}

/// Generate controller enums with its building functions.
// The controllers enum will map the enum member to the actual controller
// struct that implement the EventController trait.
macro_rules! make_controllers {
    ($($enum_item:tt: $struct_item:tt,)*) => {
        #[derive(Clone, PartialEq, Debug)]
        pub enum Controllers {
            Unknown,
            $($enum_item,)*
        }

        impl Controllers {
            pub fn command(&self) -> &'static str {
                match self {
                    Controllers::Unknown => "",
                    $(Controllers::$enum_item => $struct_item::command(),)*
                }
            }

            pub fn default_event(&self) -> Event {
                match self {
                    Controllers::Unknown => Event::Unknown(vec![]),
                    $(Controllers::$enum_item => $struct_item::default_event(),)*
                }
            }

            pub fn handle<T: 'static + ViewBridge>(&self, view: &mut StatsView<T>, cmd_vec: &[&str]) {
                match self {
                    Controllers::Unknown => (),
                    $(Controllers::$enum_item => $struct_item::handle(view, cmd_vec),)*
                }
            }

            pub fn callback<T: 'static + ViewBridge>(&self, c: &mut Cursive, cmd_vec: &[&str]) {
                match self {
                    Controllers::Unknown => (),
                    $(Controllers::$enum_item => $struct_item::callback::<T>(c, cmd_vec),)*
                }
            }
        }

        /// Map the controller enum to event trigger
        pub fn make_event_controller_map(c: &mut Cursive, file_name: &str) -> HashMap<Event, Controllers> {
            let cmdrc = match std::fs::read_to_string(file_name) {
                Ok(cmdrc_str) => cmdrc_str.parse::<Value>().unwrap_or(Value::Integer(1)),
                _ => Value::Integer(0)
            };

            let mut res = HashMap::new();
            $(
                res.insert(
                    cmdrc.get($struct_item::command()).map_or($struct_item::default_event(), |v| {
                        v.as_str().map_or(
                            $struct_item::default_event(),
                            |v| {
                                if let Some(evt) = str_to_event(v) {
                                    evt
                                } else {
                                    view_warn!(
                                        c,
                                        "Fail to parse command from cmdrc: {} --> {}",
                                        $struct_item::command(),
                                        v
                                    );
                                    $struct_item::default_event()
                                }
                            })
                    }),
                    Controllers::$enum_item
                );
            )*

            res
        }

        /// Map the controller enum to cmd string
        pub fn make_cmd_controller_map() -> HashMap<&'static str, Controllers> {
            let mut res = HashMap::new();
            $(
                res.insert(
                    $struct_item::command(),
                    Controllers::$enum_item
                );
            )*
            res
        }
    }
}
