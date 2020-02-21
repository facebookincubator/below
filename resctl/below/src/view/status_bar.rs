use chrono::{DateTime, Local};
use cursive::utils::markup::StyledString;
use cursive::view::{Identifiable, View};
use cursive::views::TextView;
use cursive::Cursive;

use crate::view::ViewState;

fn get_content(c: &mut Cursive) -> impl Into<StyledString> {
    let model = &c
        .user_data::<ViewState>()
        .expect("No data stored in Cursive object!")
        .model;
    let datetime = DateTime::<Local>::from(model.timestamp);
    let mut header_str = datetime.format("%m/%d/%Y %H:%M:%S").to_string();
    header_str += format!("      {}", &model.system.hostname).as_str();
    header_str
}

pub fn refresh(c: &mut Cursive) {
    let content = get_content(c);
    let mut v = c
        .find_id::<TextView>("status_bar")
        .expect("No status_bar view found!");
    v.set_content(content);
}

pub fn new(c: &mut Cursive) -> impl View {
    TextView::new(get_content(c)).with_id("status_bar")
}
