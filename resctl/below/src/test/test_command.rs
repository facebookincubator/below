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

use cursive::event::{Event, Key};
use tempfile::NamedTempFile;

use std::io::Write;

use view::controllers::*;

#[test]
fn test_str_to_event_valid() {
    assert_eq!(str_to_event("c").unwrap(), Event::Char('c'));
    assert_eq!(str_to_event("tab").unwrap(), Event::Key(Key::Tab));
    assert_eq!(str_to_event("enter").unwrap(), Event::Key(Key::Enter));
    assert_eq!(
        str_to_event("backspace").unwrap(),
        Event::Key(Key::Backspace)
    );
    assert_eq!(str_to_event("left").unwrap(), Event::Key(Key::Left));
    assert_eq!(str_to_event("right").unwrap(), Event::Key(Key::Right));
    assert_eq!(str_to_event("up").unwrap(), Event::Key(Key::Up));
    assert_eq!(str_to_event("down").unwrap(), Event::Key(Key::Down));
    assert_eq!(str_to_event("ins").unwrap(), Event::Key(Key::Ins));
    assert_eq!(str_to_event("del").unwrap(), Event::Key(Key::Del));
    assert_eq!(str_to_event("home").unwrap(), Event::Key(Key::Home));
    assert_eq!(str_to_event("end").unwrap(), Event::Key(Key::End));
    assert_eq!(str_to_event("page_up").unwrap(), Event::Key(Key::PageUp));
    assert_eq!(
        str_to_event("page_down").unwrap(),
        Event::Key(Key::PageDown)
    );
    assert_eq!(
        str_to_event("pause_break").unwrap(),
        Event::Key(Key::PauseBreak)
    );
    assert_eq!(str_to_event("esc").unwrap(), Event::Key(Key::Esc));
    assert_eq!(str_to_event("ctrl-c").unwrap(), Event::CtrlChar('c'));
    assert_eq!(str_to_event("ctrl-enter").unwrap(), Event::Ctrl(Key::Enter));
    assert_eq!(str_to_event("alt-c").unwrap(), Event::AltChar('c'));
    assert_eq!(str_to_event("alt-enter").unwrap(), Event::Alt(Key::Enter));
    assert_eq!(
        str_to_event("shift-enter").unwrap(),
        Event::Shift(Key::Enter)
    );
    assert_eq!(
        str_to_event("altshift-enter").unwrap(),
        Event::AltShift(Key::Enter)
    );
    assert_eq!(
        str_to_event("ctrlshift-enter").unwrap(),
        Event::CtrlShift(Key::Enter)
    );
    assert_eq!(
        str_to_event("ctrlalt-enter").unwrap(),
        Event::CtrlAlt(Key::Enter)
    );
}

#[test]
fn test_str_to_event_invalid() {
    assert_eq!(str_to_event("cd"), None);
    assert_eq!(str_to_event("ctrl-lll"), None);
    assert_eq!(str_to_event("ctrl-shift-enter"), None);
}
