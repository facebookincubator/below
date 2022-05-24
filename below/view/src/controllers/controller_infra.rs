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

fn key_to_str(key: &Key) -> &'static str {
    match key {
        Key::Tab => "Tab",
        Key::Enter => "Enter",
        Key::Backspace => "Backspace",
        Key::Left => "Left",
        Key::Right => "Right",
        Key::Up => "Up",
        Key::Down => "Down",
        Key::Ins => "Ins",
        Key::Del => "Del",
        Key::Home => "Home",
        Key::End => "End",
        Key::PageUp => "PageUp",
        Key::PageDown => "PageDown",
        Key::PauseBreak => "PauseBreak",
        Key::Esc => "Esc",
        _ => "Unknown",
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

pub fn event_to_string(event: &Event) -> String {
    match event {
        Event::Char(c) => format!("'{}'", c),
        Event::CtrlChar(c) => format!("<Ctrl> '{}'", c),
        Event::AltChar(c) => format!("<Alt> '{}'", c),
        Event::Key(key) => format!("<{}>", key_to_str(key)),
        Event::Shift(key) => format!("<Shift><{}>", key_to_str(key)),
        Event::Alt(key) => format!("<Alt><{}>", key_to_str(key)),
        Event::AltShift(key) => format!("<Alt><Shift><{}>", key_to_str(key)),
        Event::Ctrl(key) => format!("<Ctrl><{}>", key_to_str(key)),
        Event::CtrlShift(key) => format!("<Ctrl><Shift><{}>", key_to_str(key)),
        Event::CtrlAlt(key) => format!("<Ctrl><Alt><{}>", key_to_str(key)),
        _ => "Unknown".into(),
    }
}

/// Common trait that each controller should implement, more details in the module
/// level doc.
pub trait EventController {
    /// Return the command for this controller
    fn command() -> &'static str;

    // A short version of cmd
    fn cmd_shortcut() -> &'static str;

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
/// * cmd_short - command shortcut string, empty string "" means no need for short cut
/// * event - event trigger. Will be replaced with custom command from cmdrc
/// * handle - handler closure for view level processing
/// * callback - callback closure for cursive level processing
macro_rules! make_event_controller {
    ($name:ident, $cmd:expr, $cmd_short:expr, $event:expr, $handle:expr) => {
        pub struct $name;

        impl EventController for $name {
            fn command() -> &'static str {
                $cmd
            }

            fn cmd_shortcut() -> &'static str {
                $cmd_short
            }

            fn default_event() -> Event {
                $event
            }

            fn handle<T: 'static + ViewBridge>(view: &mut StatsView<T>, cmd_vec: &[&str]) {
                $handle(view, cmd_vec)
            }
        }
    };

    ($name:ident, $cmd:expr, $cmd_short:expr, $event:expr, $handle:expr, $callback:expr) => {
        pub struct $name;

        impl EventController for $name {
            fn command() -> &'static str {
                $cmd
            }

            fn cmd_shortcut() -> &'static str {
                $cmd_short
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
    ($($(#[$attr:meta])* $enum_item:ident: $struct_item:ident,)*) => {
        #[derive(Clone, PartialEq, Debug, Hash, Eq)]
        pub enum Controllers {
            Unknown,
            $(
                $(#[$attr])*
                $enum_item,
            )*
        }

        impl Controllers {
            pub fn command(&self) -> &'static str {
                match self {
                    Controllers::Unknown => "",
                    $(
                        $(#[$attr])*
                        Controllers::$enum_item => $struct_item::command(),
                    )*
                }
            }

            pub fn cmd_shortcut(&self) -> &'static str {
                match self {
                    Controllers::Unknown => "",
                    $(
                        $(#[$attr])*
                        Controllers::$enum_item => $struct_item::cmd_shortcut(),
                    )*
                }
            }

            pub fn default_event(&self) -> Event {
                match self {
                    Controllers::Unknown => Event::Unknown(vec![]),
                    $(
                        $(#[$attr])*
                        Controllers::$enum_item => $struct_item::default_event(),
                    )*
                }
            }

            pub fn handle<T: 'static + ViewBridge>(&self, view: &mut StatsView<T>, cmd_vec: &[&str]) {
                match self {
                    Controllers::Unknown => (),
                    $(
                        $(#[$attr])*
                        Controllers::$enum_item => $struct_item::handle(view, cmd_vec),
                    )*
                }
            }

            pub fn callback<T: 'static + ViewBridge>(&self, c: &mut Cursive, cmd_vec: &[&str]) {
                match self {
                    Controllers::Unknown => (),
                    $(
                        $(#[$attr])*
                        Controllers::$enum_item => $struct_item::callback::<T>(c, cmd_vec),
                    )*
                }
            }
        }

        /// Map the controller enum to event trigger
        pub fn make_event_controller_map(c: &mut Cursive, cmdrc: &Option<Value>) -> HashMap<Event, Controllers> {
            let mut res: HashMap<Event, Controllers> = HashMap::new();

            // Generate default hashmap
            $(
                $(#[$attr])*
                res.insert(
                    $struct_item::default_event(),
                    Controllers::$enum_item
                );
            )*

            // Replace value with cmdrc
            cmdrc.as_ref().map(|value| {
                let cmd_controllers = c
                    .user_data::<crate::ViewState>()
                    .expect("No user data set")
                    .cmd_controllers
                    .clone();

                value.as_table().map(|table| table.iter().for_each(|(k, v)| {
                    if let Some(v_str) = v.as_str() {
                        match (cmd_controllers.borrow().get::<str>(k), str_to_event(v_str)) {
                            (Some(controller), Some(event)) => {
                                match res.get(&event) {
                                    // If the controller which using such event will not be replaced,
                                    // we raise warning
                                    Some(ctrller) if !table.contains_key(ctrller.command()) => {
                                        view_warn!(
                                            c,
                                            "Event {} has been used by: {}",
                                            v_str,
                                            ctrller.command()
                                        );
                                    }
                                    _ => {
                                        res.insert(event, controller.clone());
                                    }
                                }
                            },
                            (None, _) => {
                                view_warn!(c, "Unrecogonized command: {}", k);
                            },
                            (_, None) => {
                                view_warn!(
                                    c,
                                    "Fail to parse command from cmdrc: {} --> {}",
                                    k,
                                    v_str
                                );
                            }
                        }
                    } else {
                        view_warn!(c, "Failed to parse the value of {} in str", k);
                    }
                }))
            });

            res
        }

        /// Map the controller enum to cmd string
        pub fn make_cmd_controller_map() -> HashMap<&'static str, Controllers> {
            let mut res = HashMap::new();
            $(
                $(#[$attr])*
                res.insert(
                    $struct_item::command(),
                    Controllers::$enum_item
                );

                $(#[$attr])*
                if !$struct_item::cmd_shortcut().is_empty() {
                    res.insert(
                        $struct_item::cmd_shortcut(),
                        Controllers::$enum_item
                    );
                }
            )*
            res
        }
    }
}
