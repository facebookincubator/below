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

use base_render::{HasRenderConfig, RenderConfig};
use model::{Field, FieldId, Queriable, Recursive};

use cursive::utils::markup::StyledString;

/// Details for generating StyledString from a Field.
#[derive(Clone)]
pub enum ViewStyle {
    /// Highlight the Field if its value is above some threshold.
    HighlightAbove(Field),
}

pub const PRESSURE_HIGHLIGHT: ViewStyle = ViewStyle::HighlightAbove(Field::F64(40.0));
pub const CPU_HIGHLIGHT: ViewStyle = ViewStyle::HighlightAbove(Field::F64(100.0));

#[derive(Clone, Default)]
pub struct ViewConfig {
    pub render_config: RenderConfig,
    pub view_style: Option<ViewStyle>,
}

impl ViewConfig {
    pub fn update(mut self, render_config: RenderConfig) -> Self {
        self.render_config = self.render_config.update(render_config);
        self
    }

    pub fn set_style(mut self, style: ViewStyle) -> Self {
        self.view_style = Some(style);
        self
    }

    fn apply_style(&self, rendered: String, field: Option<Field>) -> StyledString {
        match &self.view_style {
            Some(view_style) => match view_style {
                ViewStyle::HighlightAbove(threshold) => {
                    if field.as_ref().map_or(false, |field| field > threshold) {
                        StyledString::styled(
                            rendered,
                            cursive::theme::Color::Light(cursive::theme::BaseColor::Red),
                        )
                    } else {
                        StyledString::plain(rendered)
                    }
                }
            },
            None => StyledString::plain(rendered),
        }
    }

    pub fn render_title(&self) -> String {
        self.render_config.render_title(true)
    }

    pub fn render(&self, field: Option<Field>) -> StyledString {
        let rendered = self.render_config.render(field.clone(), true);
        self.apply_style(rendered, field)
    }

    pub fn render_indented(&self, field: Option<Field>, depth: usize) -> StyledString {
        let rendered = self
            .render_config
            .render_indented(field.clone(), true, depth);
        self.apply_style(rendered, field)
    }
}

/// Necessary metadata for rendering a Field in View given an appropriate model.
#[derive(Clone, Default)]
pub struct ViewItem<F: FieldId> {
    /// For retrieving Field from a model.
    pub field_id: F,
    /// For rendering a Field into a StyledString.
    pub config: ViewConfig,
}

pub trait HasViewStyle: Queriable {
    fn get_view_style(_field_id: &Self::FieldId) -> Option<ViewStyle> {
        None
    }
}

impl<T, F> ViewItem<F>
where
    T: Queriable<FieldId = F> + HasRenderConfig + HasViewStyle,
    F: FieldId<Queriable = T>,
{
    pub fn from_default(field_id: F) -> Self {
        let config = ViewConfig {
            render_config: T::get_render_config(&field_id),
            view_style: T::get_view_style(&field_id),
        };
        Self { field_id, config }
    }
}

impl<F: FieldId> ViewItem<F> {
    pub fn update(mut self, render_config: RenderConfig) -> Self {
        self.config = self.config.update(render_config);
        self
    }

    pub fn set_style(mut self, style: ViewStyle) -> Self {
        self.config = self.config.set_style(style);
        self
    }

    pub fn render(&self, model: &F::Queriable) -> StyledString {
        self.config.render(model.query(&self.field_id))
    }
}

impl<F, T> ViewItem<F>
where
    T: Queriable<FieldId = F> + Recursive,
    F: FieldId<Queriable = T>,
{
    pub fn render_indented(&self, model: &T) -> StyledString {
        self.config
            .render_indented(model.query(&self.field_id), model.get_depth())
    }
}
