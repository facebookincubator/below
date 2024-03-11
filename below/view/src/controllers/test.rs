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

use std::io::prelude::*;

use tempfile::TempDir;

use super::*;
use crate::fake_view::FakeView;
use crate::View;

#[test]
fn test_event_controller_override() {
    let mut fake_view = FakeView::new();
    fake_view.add_cgroup_view();

    let cmdrc_str = b"invoke_cmd_palette = 'a'
next_tab = 'b'
cgroup = 'k'
prev_tab = 'c'
next_col = 'd'
prev_col = 'e'
right = 'f'
left = 'w'
quit = 'h'
help = 'i'
process = 'j'
system = 'l'
zoom = 'm'
fold = 'f'
jump_forward = 'o'
jump_backward = 'p'
next_sample = 'q'
prev_sample = 'r'
pause_resume = 's'
sort = 't'
filter = 'u'
clear_filter = 'v'
next_page = 'Y'
prev_page = 'y'
";
    let cmdrc_val = std::str::from_utf8(cmdrc_str)
        .expect("Failed to parse [u8] to str")
        .parse::<Value>()
        .expect("Failed to parse test cmdrc");
    let event_controllers = make_event_controller_map(&mut fake_view.inner, &Some(cmdrc_val));

    assert_eq!(
        event_controllers.get(&Event::Char('a')),
        Some(&Controllers::CmdPalette)
    );
    assert_eq!(
        event_controllers.get(&Event::Char('b')),
        Some(&Controllers::NextTab)
    );
    assert_eq!(
        event_controllers.get(&Event::Char('c')),
        Some(&Controllers::PrevTab)
    );
    assert_eq!(
        event_controllers.get(&Event::Char('d')),
        Some(&Controllers::NextCol)
    );
    assert_eq!(
        event_controllers.get(&Event::Char('e')),
        Some(&Controllers::PrevCol)
    );
    assert_eq!(
        event_controllers.get(&Event::Char('f')),
        Some(&Controllers::Right)
    );
    assert_eq!(
        event_controllers.get(&Event::Char('w')),
        Some(&Controllers::Left)
    );
    assert_eq!(
        event_controllers.get(&Event::Char('t')),
        Some(&Controllers::SortCol)
    );
    assert_eq!(
        event_controllers.get(&Event::Char('u')),
        Some(&Controllers::Filter)
    );
    assert_eq!(
        event_controllers.get(&Event::Char('v')),
        Some(&Controllers::CFilter)
    );
    assert_eq!(
        event_controllers.get(&Event::Char('o')),
        Some(&Controllers::JForward)
    );
    assert_eq!(
        event_controllers.get(&Event::Char('p')),
        Some(&Controllers::JBackward)
    );
    assert_eq!(
        event_controllers.get(&Event::Char('q')),
        Some(&Controllers::NSample)
    );
    assert_eq!(
        event_controllers.get(&Event::Char('r')),
        Some(&Controllers::PSample)
    );
    assert_eq!(
        event_controllers.get(&Event::Char('s')),
        Some(&Controllers::Pause)
    );
    assert_eq!(
        event_controllers.get(&Event::Char('h')),
        Some(&Controllers::Quit)
    );
    assert_eq!(
        event_controllers.get(&Event::Char('i')),
        Some(&Controllers::Help)
    );
    assert_eq!(
        event_controllers.get(&Event::Char('j')),
        Some(&Controllers::Process)
    );
    assert_eq!(
        event_controllers.get(&Event::Char('k')),
        Some(&Controllers::Cgroup)
    );
    assert_eq!(
        event_controllers.get(&Event::Char('l')),
        Some(&Controllers::System)
    );
    assert_eq!(
        event_controllers.get(&Event::Char('m')),
        Some(&Controllers::Zoom)
    );
    assert_eq!(
        event_controllers.get(&Event::Char('Y')),
        Some(&Controllers::NextPage)
    );
    assert_eq!(
        event_controllers.get(&Event::Char('y')),
        Some(&Controllers::PrevPage)
    );
}

#[test]
fn test_event_controller_override_failed() {
    let mut fake_view = FakeView::new();
    fake_view.add_cgroup_view();

    // Invalid command
    let cmdrc_str = "demacia = 'a'";
    let cmdrc_val = cmdrc_str
        .parse::<Value>()
        .expect("Failed to parse test cmdrc");
    make_event_controller_map(&mut fake_view.inner, &Some(cmdrc_val));
    assert_eq!(
        fake_view.get_cmd_palette("cgroup_view").get_content(),
        "WARN: Unrecogonized command: demacia"
    );

    // Duplicate event
    fake_view.get_cmd_palette("cgroup_view").set_info("");
    let cmdrc_str = "process = 'c'";
    let cmdrc_val = cmdrc_str
        .parse::<Value>()
        .expect("Failed to parse test cmdrc");
    make_event_controller_map(&mut fake_view.inner, &Some(cmdrc_val));
    assert_eq!(
        fake_view.get_cmd_palette("cgroup_view").get_content(),
        "WARN: Event c has been used by: cgroup"
    );

    // Failed to parse event
    fake_view.get_cmd_palette("cgroup_view").set_info("");
    let cmdrc_str = "process = 'ctrll-c'";
    let cmdrc_val = cmdrc_str
        .parse::<Value>()
        .expect("Failed to parse test cmdrc");
    make_event_controller_map(&mut fake_view.inner, &Some(cmdrc_val));
    assert_eq!(
        fake_view.get_cmd_palette("cgroup_view").get_content(),
        "WARN: Fail to parse command from cmdrc: process --> ctrll-c"
    );
}

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

#[test]
fn test_event_to_str() {
    assert_eq!(event_to_string(&Event::Char('c')), "'c'");
    assert_eq!(event_to_string(&Event::Key(Key::Tab)), "<Tab>");
    assert_eq!(event_to_string(&Event::Key(Key::Enter)), "<Enter>");
    assert_eq!(event_to_string(&Event::Key(Key::Backspace)), "<Backspace>");
    assert_eq!(event_to_string(&Event::Key(Key::Left)), "<Left>");
    assert_eq!(event_to_string(&Event::Key(Key::Right)), "<Right>");
    assert_eq!(event_to_string(&Event::Key(Key::Up)), "<Up>");
    assert_eq!(event_to_string(&Event::Key(Key::Down)), "<Down>");
    assert_eq!(event_to_string(&Event::Key(Key::Ins)), "<Ins>");
    assert_eq!(event_to_string(&Event::Key(Key::Del)), "<Del>");
    assert_eq!(event_to_string(&Event::Key(Key::Home)), "<Home>");
    assert_eq!(event_to_string(&Event::Key(Key::End)), "<End>");
    assert_eq!(event_to_string(&Event::Key(Key::PageUp)), "<PageUp>");
    assert_eq!(event_to_string(&Event::Key(Key::PageDown)), "<PageDown>");
    assert_eq!(
        event_to_string(&Event::Key(Key::PauseBreak)),
        "<PauseBreak>"
    );
    assert_eq!(event_to_string(&Event::Key(Key::Esc)), "<Esc>");
    assert_eq!(event_to_string(&Event::CtrlChar('c')), "<Ctrl> 'c'");
    assert_eq!(event_to_string(&Event::Ctrl(Key::Enter)), "<Ctrl><Enter>");
    assert_eq!(event_to_string(&Event::AltChar('c')), "<Alt> 'c'");
    assert_eq!(event_to_string(&Event::Alt(Key::Enter)), "<Alt><Enter>");
    assert_eq!(event_to_string(&Event::Shift(Key::Enter)), "<Shift><Enter>");
    assert_eq!(
        event_to_string(&Event::AltShift(Key::Enter)),
        "<Alt><Shift><Enter>"
    );
    assert_eq!(
        event_to_string(&Event::CtrlShift(Key::Enter)),
        "<Ctrl><Shift><Enter>"
    );
    assert_eq!(
        event_to_string(&Event::CtrlAlt(Key::Enter)),
        "<Ctrl><Alt><Enter>"
    );
}

#[test]
fn test_belowrc_to_event() {
    // Creating self cleaning test belowrc file
    let tempdir = TempDir::with_prefix("below_cmd_test.").expect("Failed to create temp dir");
    let path = tempdir.path().join("belowrc");

    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .truncate(true)
        .create(true)
        .open(&path)
        .expect("Fail to open belowrc in tempdir");
    let belowrc_str = r#"
[cmd]
next_tab = 'b'
cgroup = 'k'
prev_tab = 'c'
next_col = 'd'
"#;
    file.write_all(belowrc_str.as_bytes())
        .expect("Faild to write temp belowrc file during testing ignore");
    file.flush().expect("Failed to flush during testing ignore");

    //
    let mut fake_view = FakeView::new();
    View::generate_event_controller_map(&mut fake_view.inner, path.to_string_lossy().to_string());

    let event_controllers = fake_view
        .inner
        .user_data::<ViewState>()
        .expect("No data stored in Cursive object!")
        .event_controllers
        .borrow();
    assert_eq!(
        event_controllers.get(&Event::Char('b')),
        Some(&Controllers::NextTab)
    );
    assert_eq!(
        event_controllers.get(&Event::Char('c')),
        Some(&Controllers::PrevTab)
    );
    assert_eq!(
        event_controllers.get(&Event::Char('d')),
        Some(&Controllers::NextCol)
    );
    assert_eq!(
        event_controllers.get(&Event::Char('k')),
        Some(&Controllers::Cgroup)
    );
}
