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

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;

use common::logutil::CPMsgRecord;
use common::logutil::get_last_log_to_display;
use cursive::Cursive;
use cursive::event::Event;
use cursive::event::EventResult;
use cursive::event::EventTrigger;
use cursive::utils::markup::StyledString;
use cursive::view::Nameable;
use cursive::view::Scrollable;
use cursive::view::View;
use cursive::view::ViewWrapper;
use cursive::views::LinearLayout;
use cursive::views::NamedView;
use cursive::views::OnEventView;
use cursive::views::Panel;
use cursive::views::ResizedView;
use cursive::views::ScrollView;
use cursive::views::SelectView;
use cursive::views::ViewRef;

use crate::command_palette::CommandPalette;
use crate::controllers::Controllers;
use crate::tab_view::TabView;

pub struct ColumnTitles {
    pub titles: Vec<String>,
    pub pinned_titles: usize, // the first `pinned_titles` titles are fixed
}

/// A trait that defines common state data querying or event handling.
///
/// This trait must be implemented by all view state. It will help to expose
/// state data to the StatsView for common behavior. On the other hand, it force
/// a view to have required data in order to fit itself inside the StatsView.
pub trait StateCommon: Send + Sync {
    type ModelType;
    type TagType: ToString;
    type KeyType: Clone + Send + Sync;

    /// Expose filter data for StatsView to set fields in filter popup
    fn get_filter_info(&self) -> &Option<(Self::TagType, String)>;

    /// Gets the FieldId associated with given tab and column index
    fn get_tag_from_tab_idx(&self, tab: &str, idx: usize) -> Self::TagType;

    /// Returns true iff filtering is supported for column
    fn is_filter_supported_from_tab_idx(&self, _tab: &str, _idx: usize) -> bool {
        false
    }
    /// Set the filter (current column and filter string)
    /// Return true on success, false on failure
    fn set_filter_from_tab_idx(
        &mut self,
        _tab: &str,
        _idx: usize,
        _filter: Option<String>,
    ) -> bool {
        false
    }

    /// Set the sorting tag to common state
    /// Return true on success, false if current tab doest support sorting.
    fn set_sort_tag(&mut self, _tag: Self::TagType, _reverse: &mut bool) -> bool {
        false
    }
    fn set_sort_string(&mut self, _selection: &str, _reverse: &mut bool) -> bool {
        false
    }
    fn set_sort_tag_from_tab_idx(&mut self, _tab: &str, _idx: usize, _reverse: &mut bool) -> bool {
        false
    }

    fn get_model(&self) -> MutexGuard<Self::ModelType>;
    fn get_model_mut(&self) -> MutexGuard<Self::ModelType>;
    fn new(model: Arc<Mutex<Self::ModelType>>) -> Self;
}

/// ViewBridge defines how a ConcreteView will relate to StatsView
pub trait ViewBridge: Send + Sync {
    type StateType: Default + StateCommon;
    /// Return the name of the view, this function will help StatsView to
    /// query view by name.
    fn get_view_name() -> &'static str;

    /// Return the column titles of the view
    fn get_titles(&self) -> ColumnTitles;

    /// The essential function that defines how a StatsView should fill
    /// the data. This function will iterate through the data, apply filter and sorting,
    /// return a Vec of (Stats String Line, Key) tuple.
    /// # Arguments
    /// * `state`: The concrete view state
    /// * `offset`: Indicates how many columns we should pass after the first column when generating a line.
    fn get_rows(
        &mut self,
        state: &Self::StateType,
        offset: Option<usize>,
    ) -> Vec<(
        StyledString,
        <<Self as ViewBridge>::StateType as StateCommon>::KeyType,
    )>;

    /// Optional callback called by on_select of inner SelectView and on
    /// refresh for updating state based on selected key.
    fn on_select_update_state(
        _state: &mut Self::StateType,
        _selected_key: Option<&<<Self as ViewBridge>::StateType as StateCommon>::KeyType>,
    ) {
    }

    /// Optional callback called by on_select of inner SelectView for
    /// updating command palette. Returns info String set on the palette.
    fn on_select_update_cmd_palette(
        _state: &Self::StateType,
        _selected_key: &<<Self as ViewBridge>::StateType as StateCommon>::KeyType,
        _current_tab: &str,
        _selected_column: usize,
    ) -> String {
        "".to_owned()
    }
}

/// StatsView is a view wrapper that wraps tabs, titles, and list of stats.
///
/// Terminology:
/// `title`: A title here means a column name in the stats table. We call it title to align with
///          below_derive's function `get_title_line`.
/// `tab` or `topic`: A tab or topic is the content of "Tabs" defined in the module level description.
///                   For example, "general", "cpu", "pressure", etc.
///
/// The `tab_titles_map` defines a hashmap between a tab name and a vector of its corresponding
/// string titles. This will help StatsView to switch stats headline(columns name) when a user switching
/// tabs. This hashmap will be automatically generated by `tab_view_map`.
///
/// `tab_view_map` defines a map relationship between a tab name and its concrete `V`
/// data structure. The `V` here represents "view" which, in implementation, is a enum of
/// "tab" data structure. And each "tab" defines stats data and will derive the BelowDecor
/// to generate all display functions. Please check the implementation of V for more details.
///
/// `detailed_view` here is our cursive object. I wrapped it with OnEventView in order to
/// let the concrete "view"s to define their customized handler. You can think the detailed_view
/// will be something like this
///
/// OnEventView
///    --> Panel
///        --> LinearLayout::Vertical
///          --> Child 0: A TabView that represent the topic tab
///          --> child 1: ScrollView
///            --> LinearLayout::Vertical
///            --> child 0: A TabView that represent the title header tab
///            --> child 1: A SelectView that represents the detail stats
///          --> child 2: Command palette
///
/// `state` defines the state of a view. Filters, sorting orders will be defined here.
pub struct StatsView<V: 'static + ViewBridge> {
    tab_titles_map: HashMap<String, ColumnTitles>,
    tab_view_map: HashMap<String, V>,
    detailed_view: OnEventView<Panel<LinearLayout>>,
    pub state: Arc<Mutex<V::StateType>>,
    pub reverse_sort: bool,
    pub event_controllers: Arc<Mutex<HashMap<Event, Controllers>>>,
}

impl<V: 'static + ViewBridge> ViewWrapper for StatsView<V> {
    cursive::wrap_impl!(self.detailed_view: OnEventView<Panel<LinearLayout>>);

    // We will handle common event in this wrapper. It will comsume the
    // event if there's a match. Otherwise, it will pass the event to the
    // concrete event handler.
    fn wrap_on_event(&mut self, ch: Event) -> EventResult {
        // Refresh event will be handled at root
        if ch == Event::Refresh {
            return EventResult::Ignored;
        }

        // if stats view is in cmd mode, pass all event to cmd_palette
        let cmd_mode = self.get_cmd_palette().is_cmd_mode();
        if cmd_mode {
            return self.get_cmd_palette().on_event(ch);
        }

        let controller = self
            .event_controllers
            .lock()
            .unwrap()
            .get(&ch)
            .unwrap_or(&Controllers::Unknown)
            .clone();

        // Unmapped event goes to the parent view.
        if controller == Controllers::Unknown {
            self.with_view_mut(|v| v.on_event(ch))
                .unwrap_or(EventResult::Ignored)
        } else {
            controller.handle(self, &[]);
            EventResult::with_cb(move |c| controller.callback::<V>(c, &[]))
        }
    }
}

impl<V: 'static + ViewBridge> StatsView<V> {
    #[allow(unused)]
    pub fn new(
        name: &'static str,
        tabs: Vec<String>,
        tab_view_map: HashMap<String, V>,
        select_view: SelectView<<V::StateType as StateCommon>::KeyType>,
        state: V::StateType,
        event_controllers: Arc<Mutex<HashMap<Event, Controllers>>>,
        cmd_controllers: Arc<Mutex<HashMap<&'static str, Controllers>>>,
    ) -> Self {
        let mut tab_titles_map = HashMap::new();
        for (tab, bridge) in &tab_view_map {
            let mut titles = bridge.get_titles();
            tab_titles_map.insert(tab.into(), titles);
        }

        let default_tab = tabs[0].clone();
        let tab_titles = tab_titles_map
            .get(&default_tab)
            .expect("Failed to query default tab");

        let select_view_with_cb =
            select_view.on_select(|c, selected_key: &<V::StateType as StateCommon>::KeyType| {
                c.call_on_name(V::get_view_name(), |view: &mut StatsView<V>| {
                    V::on_select_update_state(&mut view.state.lock().unwrap(), Some(selected_key));
                    let mut cmd_palette = view.get_cmd_palette();
                    let cur_tab = view.get_tab_view().get_cur_selected().to_string();
                    let selected_column = view.get_title_view().current_selected;
                    cmd_palette.set_info(V::on_select_update_cmd_palette(
                        &view.state.lock().unwrap(),
                        selected_key,
                        &cur_tab,
                        selected_column,
                    ));
                });
            });

        let detailed_view = OnEventView::new(Panel::new(
            LinearLayout::vertical()
                .child(
                    TabView::new(tabs, "   ", 0 /* pinned titles */)
                        .expect("Fail to construct tab")
                        .with_name(format!("{}_tab", &name)),
                )
                .child(
                    LinearLayout::vertical()
                        .child(
                            TabView::new(
                                tab_titles.titles.clone(),
                                " ",
                                tab_titles.pinned_titles, /* pinned titles */
                            )
                            .expect("Fail to construct title")
                            .with_name(format!("{}_title", &name)),
                        )
                        .child(ResizedView::with_full_screen(
                            select_view_with_cb
                                .with_name(format!("{}_detail", &name))
                                .scrollable(),
                        ))
                        .scrollable()
                        .scroll_x(true)
                        .scroll_y(false),
                )
                .child(
                    CommandPalette::new::<V>(name, "<root>", cmd_controllers)
                        .with_name(format!("{}_cmd_palette", &name)),
                ),
        ));

        Self {
            tab_titles_map,
            tab_view_map,
            detailed_view,
            state: Arc::new(Mutex::new(state)),
            reverse_sort: true,
            event_controllers,
        }
    }

    // When a user switch tab, we need to reset the title state.
    pub fn update_title(&mut self) {
        let cur_tab = self.get_tab_view().get_cur_selected().to_string();
        let mut title_view = self.get_title_view();
        let tabs = self
            .tab_titles_map
            .get(&cur_tab)
            .unwrap_or_else(|| panic!("Fail to query title from tab {}", cur_tab));
        title_view.tabs.clone_from(&tabs.titles);
        title_view.fixed_tabs = tabs.pinned_titles;
        title_view.current_selected = 0;
        title_view.current_offset_idx = 0;
        title_view.cur_offset = 0;
        title_view.total_length = title_view.tabs.iter().fold(0, |acc, x| acc + x.len() + 1);
        title_view.cur_length = title_view.tabs[0].len();
    }

    // Expose the OnEventView API.
    pub fn on_event<F, E>(mut self, trigger: E, cb: F) -> Self
    where
        E: Into<EventTrigger>,
        F: 'static + Fn(&mut Cursive) + Send + Sync,
    {
        self.detailed_view.set_on_event(trigger, cb);
        self
    }

    // A convenience function to get the topic tab view.
    pub fn get_tab_view(&mut self) -> ViewRef<TabView> {
        let tab_panel: &mut NamedView<TabView> = self
            .detailed_view // OnEventView
            .get_inner_mut() // PanelView
            .get_inner_mut() // LinearLayout
            .get_child_mut(0) // NamedView
            .expect("Fail to get tab panel, StatsView may not properly init")
            .downcast_mut()
            .expect("Fail to downcast to panel, StatsView may not properly init");

        tab_panel.get_mut()
    }

    // Helping method to downcast the scroll view.
    fn get_scroll_view(&mut self) -> &mut ScrollView<LinearLayout> {
        self.detailed_view // OnEventView
            .get_inner_mut() // PanelView
            .get_inner_mut() // LinearLayout
            .get_child_mut(1) // ScrollView
            .expect("Fail to get stats scrollable, StatsView may not properly init")
            .downcast_mut()
            .expect("Fail to downcast to stats scrollable, StatsView may not properly init")
    }

    // A convenience function to get the title tab view.
    pub fn get_title_view(&mut self) -> ViewRef<TabView> {
        let scroll_view = self.get_scroll_view();

        let title_named: &mut NamedView<TabView> = scroll_view
            .get_inner_mut() // LinearLayout
            .get_child_mut(0) //NamedView
            .expect("Fail to get title, StatsView may not properly init")
            .downcast_mut()
            .expect("Fail to downcast to title, StatsView may not properly init");

        title_named.get_mut()
    }

    // A convenience function to get the scroll view of the list
    pub fn get_list_scroll_view(
        &mut self,
    ) -> &mut ScrollView<NamedView<SelectView<<V::StateType as StateCommon>::KeyType>>> {
        let scroll_view = self.get_scroll_view();

        #[allow(clippy::type_complexity)]
        let select_named: &mut ResizedView<
            ScrollView<NamedView<SelectView<<V::StateType as StateCommon>::KeyType>>>,
        > = scroll_view
            .get_inner_mut() // LinearLayout
            .get_child_mut(1) // ResizedView
            .expect("Fail to get title, StatsView may not properly init")
            .downcast_mut()
            .expect("Fail to downcast to title, StatsView may not properly init");

        select_named.get_inner_mut()
    }

    // A convenience function to get the detail stats SelectView
    pub fn get_detail_view(
        &mut self,
    ) -> ViewRef<SelectView<<V::StateType as StateCommon>::KeyType>> {
        self.get_list_scroll_view().get_inner_mut().get_mut()
    }

    // A convenience function to get the command palette
    pub fn get_cmd_palette(&mut self) -> ViewRef<CommandPalette> {
        let cmd_palette: &mut NamedView<CommandPalette> = self
            .detailed_view // OnEventView
            .get_inner_mut() // PanelView
            .get_inner_mut() // LinearLayout
            .get_child_mut(2) // NamedView
            .expect("Fail to get cmd palette, StatsView may not properly init")
            .downcast_mut()
            .expect("Fail to downcast to cmd palette, StatsView may not properly init");

        cmd_palette.get_mut()
    }

    // convenience function to get screen width
    pub fn get_screen_width(&mut self) -> usize {
        self.get_scroll_view().content_viewport().width()
    }

    pub fn set_horizontal_offset(&mut self, x: usize) {
        let screen_width = self.get_screen_width();
        if screen_width < x {
            self.get_title_view()
                .scroll_to_offset(x - screen_width, screen_width);
        } else {
            self.get_title_view().scroll_to_offset(0, screen_width);
        }
    }

    // Function to refresh the view.
    // A potential optimize here is put the model of the cursive view_state as Rc<RefCell>
    // member of StatsView. In that case, we don't need to borrow the cursive object here.
    pub fn refresh(&mut self, c: &mut Cursive) {
        {
            let cur_tab = self.get_tab_view().get_cur_selected().to_string();
            let mut select_view = self.get_detail_view();

            let pos = select_view.selected_id().unwrap_or(0);
            select_view.clear();

            let horizontal_offset = self.get_title_view().current_offset_idx;

            let tab_detail = self
                .tab_view_map
                .get_mut(&cur_tab)
                .unwrap_or_else(|| panic!("Fail to query data from tab {}", cur_tab));
            select_view
                .add_all(tab_detail.get_rows(&self.state.lock().unwrap(), Some(horizontal_offset)));

            // This will trigger on_select handler, but handler will not be able to
            // find the current StatsView from cursive, presumably because we are
            // holding on a mutable reference to self at this moment.
            select_view.select_down(pos)(c);

            let mut cmd_palette = self.get_cmd_palette();
            if let Some(msg) = get_last_log_to_display() {
                cmd_palette.set_alert(msg);
            }

            let selection = select_view.selection().map(|rc| rc.as_ref().clone());
            let selected_column = self.get_title_view().current_selected;
            V::on_select_update_state(&mut self.state.lock().unwrap(), selection.as_ref());
            // We should not override alert on refresh. Only selection should
            // override alert.
            if let (false, Some(selection)) = (cmd_palette.is_alerting(), selection) {
                let info_msg = V::on_select_update_cmd_palette(
                    &self.state.lock().unwrap(),
                    &selection,
                    &cur_tab,
                    selected_column,
                );
                cmd_palette.set_info(info_msg);
            }
        }
        self.get_list_scroll_view().scroll_to_important_area();
    }

    // Chaining call. Use for construction to get initial data.
    pub fn feed_data(mut self, c: &mut Cursive) -> Self {
        self.refresh(c);
        self
    }

    /// Convenience function to get StatsView
    pub fn get_view(c: &mut Cursive) -> ViewRef<Self> {
        c.find_name::<Self>(V::get_view_name())
            .expect("Fail to find view with name")
    }

    // Locates the view with its defined name and refresh it.
    // This is a convenience function for refresh without need of anobject.
    pub fn refresh_myself(c: &mut Cursive) {
        Self::get_view(c).refresh(c)
    }

    pub fn set_alert(&mut self, msg: &str) {
        self.get_cmd_palette()
            .set_alert(CPMsgRecord::construct_msg(slog::Level::Warning, msg));
    }

    /// Convenience function to raise warning. Only to CommandPalette.
    pub fn cp_warn(c: &mut Cursive, msg: &str) {
        Self::get_view(c).set_alert(msg);
    }

    /// Convenience function to set filter to CommandPalette.
    /// filter_info provides the column title (for the filtered field) and filter
    pub fn cp_filter(c: &mut Cursive, filter_info: Option<(String, String)>) {
        Self::get_view(c).get_cmd_palette().set_filter(filter_info);
    }
}
