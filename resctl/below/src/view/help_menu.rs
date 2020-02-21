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
    ];

    for line in lines {
        v.add_child(TextView::new(line));
    }
}

pub fn new() -> impl View {
    let mut view = LinearLayout::vertical();
    fill_content(&mut view);
    view.with_id("help_menu")
}
