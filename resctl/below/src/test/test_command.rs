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
use cursive::Cursive;
use tempfile::NamedTempFile;

use std::io::Write;

use view::controllers::*;

#[test]
fn test_default_cmd_controllers() {
    let cmd_controllers = make_cmd_controller_map();
    assert_eq!(
        cmd_controllers.get("invoke_cmd_palette"),
        Some(&Controllers::CmdPalette)
    );
    assert_eq!(cmd_controllers.get("next_tab"), Some(&Controllers::NextTab));
    assert_eq!(cmd_controllers.get("prev_tab"), Some(&Controllers::PrevTab));
    assert_eq!(cmd_controllers.get("next_col"), Some(&Controllers::NextCol));
    assert_eq!(cmd_controllers.get("prev_col"), Some(&Controllers::PrevCol));
    assert_eq!(cmd_controllers.get("right"), Some(&Controllers::Right));
    assert_eq!(cmd_controllers.get("left"), Some(&Controllers::Left));
    assert_eq!(cmd_controllers.get("sort"), Some(&Controllers::SortCol));
    assert_eq!(cmd_controllers.get("filter"), Some(&Controllers::Filter));
    assert_eq!(
        cmd_controllers.get("clear_filter"),
        Some(&Controllers::CFilter)
    );
    assert_eq!(
        cmd_controllers.get("jump_forward"),
        Some(&Controllers::JForward)
    );
    assert_eq!(
        cmd_controllers.get("jump_backward"),
        Some(&Controllers::JBackward)
    );
    assert_eq!(
        cmd_controllers.get("next_sample"),
        Some(&Controllers::NSample)
    );
    assert_eq!(
        cmd_controllers.get("prev_sample"),
        Some(&Controllers::PSample)
    );
    assert_eq!(
        cmd_controllers.get("pause_resume"),
        Some(&Controllers::Pause)
    );
    assert_eq!(cmd_controllers.get("quit"), Some(&Controllers::Quit));
    assert_eq!(cmd_controllers.get("help"), Some(&Controllers::Help));
    assert_eq!(cmd_controllers.get("process"), Some(&Controllers::Process));
    assert_eq!(cmd_controllers.get("cgroup"), Some(&Controllers::Cgroup));
    assert_eq!(cmd_controllers.get("system"), Some(&Controllers::System));
    assert_eq!(cmd_controllers.get("zoom"), Some(&Controllers::Zoom));
}

#[test]
fn test_cmd_shortcut() {
    let cmd_controllers = make_cmd_controller_map();
    assert_eq!(cmd_controllers.get("nt"), Some(&Controllers::NextTab));
    assert_eq!(cmd_controllers.get("pt"), Some(&Controllers::PrevTab));
    assert_eq!(cmd_controllers.get("nc"), Some(&Controllers::NextCol));
    assert_eq!(cmd_controllers.get("pc"), Some(&Controllers::PrevCol));
    assert_eq!(cmd_controllers.get("s"), Some(&Controllers::SortCol));
    assert_eq!(cmd_controllers.get("f"), Some(&Controllers::Filter));
    assert_eq!(cmd_controllers.get("cf"), Some(&Controllers::CFilter));
    assert_eq!(cmd_controllers.get("jf"), Some(&Controllers::JForward));
    assert_eq!(cmd_controllers.get("jb"), Some(&Controllers::JBackward));
    assert_eq!(cmd_controllers.get("ns"), Some(&Controllers::NSample));
    assert_eq!(cmd_controllers.get("ps"), Some(&Controllers::PSample));
    assert_eq!(cmd_controllers.get("pr"), Some(&Controllers::Pause));
    assert_eq!(cmd_controllers.get("q"), Some(&Controllers::Quit));
    assert_eq!(cmd_controllers.get("h"), Some(&Controllers::Help));
}

#[test]
fn test_default_event_controllers() {
    let mut cursive = Cursive::dummy();
    let event_controllers = make_event_controller_map(&mut cursive, "");

    assert_eq!(
        event_controllers.get(&Event::Char(':')),
        Some(&Controllers::CmdPalette)
    );
    assert_eq!(
        event_controllers.get(&Event::Key(Key::Tab)),
        Some(&Controllers::NextTab)
    );
    assert_eq!(
        event_controllers.get(&Event::Shift(Key::Tab)),
        Some(&Controllers::PrevTab)
    );
    assert_eq!(
        event_controllers.get(&Event::Char('.')),
        Some(&Controllers::NextCol)
    );
    assert_eq!(
        event_controllers.get(&Event::Char(',')),
        Some(&Controllers::PrevCol)
    );
    assert_eq!(
        event_controllers.get(&Event::Key(Key::Right)),
        Some(&Controllers::Right)
    );
    assert_eq!(
        event_controllers.get(&Event::Key(Key::Left)),
        Some(&Controllers::Left)
    );
    assert_eq!(
        event_controllers.get(&Event::Char('S')),
        Some(&Controllers::SortCol)
    );
    assert_eq!(
        event_controllers.get(&Event::Char('/')),
        Some(&Controllers::Filter)
    );
    assert_eq!(
        event_controllers.get(&Event::CtrlChar('l')),
        Some(&Controllers::CFilter)
    );
    assert_eq!(
        event_controllers.get(&Event::Char('j')),
        Some(&Controllers::JForward)
    );
    assert_eq!(
        event_controllers.get(&Event::Char('J')),
        Some(&Controllers::JBackward)
    );
    assert_eq!(
        event_controllers.get(&Event::Char('t')),
        Some(&Controllers::NSample)
    );
    assert_eq!(
        event_controllers.get(&Event::Char('T')),
        Some(&Controllers::PSample)
    );
    assert_eq!(
        event_controllers.get(&Event::Char(' ')),
        Some(&Controllers::Pause)
    );
    assert_eq!(
        event_controllers.get(&Event::Char('q')),
        Some(&Controllers::Quit)
    );
    assert_eq!(
        event_controllers.get(&Event::Char('?')),
        Some(&Controllers::Help)
    );
    assert_eq!(
        event_controllers.get(&Event::Char('p')),
        Some(&Controllers::Process)
    );
    assert_eq!(
        event_controllers.get(&Event::Char('c')),
        Some(&Controllers::Cgroup)
    );
    assert_eq!(
        event_controllers.get(&Event::Char('s')),
        Some(&Controllers::System)
    );
    assert_eq!(
        event_controllers.get(&Event::Char('z')),
        Some(&Controllers::Zoom)
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
fn test_event_trigger_override() {
    let cmdrc_str = b"invoke_cmd_palette = 'a'
next_tab = 'b'
prev_tab = 'c'
next_col = 'd'
prev_col = 'e'
right = 'f'
left = 'g'
quit = 'h'
help = 'i'
process = 'j'
cgroup = 'k'
system = 'l'
zoom = 'm'
jump_forward = 'o'
jump_backward = 'p'
next_sample = 'q'
prev_sample = 'r'
pause_resume = 's'
sort = 't'
filter = 'u'
clear_filter = 'v'
";
    let mut cmdrc = NamedTempFile::new().expect("Fail to create tmp file for event controllers");
    cmdrc
        .write_all(cmdrc_str)
        .expect("Fail to write cmdrc file");
    let mut cursive = Cursive::dummy();
    let event_controllers = make_event_controller_map(
        &mut cursive,
        cmdrc
            .path()
            .to_str()
            .expect("Fail to convert path string for event controllers"),
    );

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
        event_controllers.get(&Event::Char('g')),
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
}
