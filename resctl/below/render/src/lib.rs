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

mod default_configs;

use common::util::{convert_bytes, fold_string};
use model::{Field, Queriable};

/// Specifies how to format a Field into String
#[derive(Clone)]
pub enum RenderFormat {
    /// Truncates String to a certain width.
    Precision(usize),
    /// Only works on numeric Fields. Format as human-readable size with
    /// suffixes (KB, MB, GB etc).
    ReadableSize,
    /// Only works on numeric Fields. Format number of 4K pages as
    /// human-readable size with suffixes (KB, MB, GB etc).
    PageReadableSize,
    /// Only works on int Fields. Same as ReadableSize except when Field is -1,
    /// in which case "max" is returned.
    MaxOrReadableSize,
}

/// Specifies how a long string is folded to fit into a shorter width.
#[derive(Clone)]
pub enum FoldOption {
    /// Starts elision from first non-alphanumeric character.
    Name,
    /// Starts elision from first subdirectory (second '/' as we skip root).
    Path,
}

/// Config object for specifying how to render a Field. Options are ordered
/// roughly by their order of processing.
#[derive(Default, Clone)]
pub struct RenderConfig {
    pub title: Option<&'static str>,
    /// Converting Field to String.
    pub format: Option<RenderFormat>,
    /// Prefix when rendered with indent. Each extra level adds same number of
    /// spaces equal to the length of this prefix. This allows us to render:
    /// <root>
    /// -+ branch
    ///    -* leaf
    /// -* another_leaf
    /// The example above use two prefixes, "-+ " and "-* ". Root has no prefix.
    pub indented_prefix: Option<&'static str>,
    pub suffix: Option<&'static str>,
    /// Fit a long rendered Field into smaller width by omitting some characters
    /// in the middle instead of truncating. Only applies when rendering Field
    /// with fixed width. Taken indent, prefix and suffix len into account.
    pub fold: Option<FoldOption>,
    /// For fixed width rendering. Truncate or pad whitespace to output.
    pub width: Option<usize>,
}

/// Create RenderConfig by specifying only some of the fields. Other fields are
/// set to None. Syntax choosen to work with rustfmt.
///
/// ## Example
///
/// ```
/// let name = render_config!(title("Name"), width(2));
/// ```
#[macro_export]
macro_rules! render_config {
    ($($opt:ident ( $val:expr )),+) => { render_config!($($opt ( $val ),)+) };
    ($($opt:ident ( $val:expr ),)*) => {
        $crate::RenderConfig {
            $($opt: Some($val),)*
            ..Default::default()
        }
    }
}

pub fn get_fixed_width(val: &str, width: usize) -> String {
    format!("{val:width$.width$}", val = val, width = width)
}

impl RenderConfig {
    pub fn update(mut self, overrides: Self) -> Self {
        self.title = overrides.title.or(self.title);
        self.format = overrides.format.or(self.format);
        self.indented_prefix = overrides.indented_prefix.or(self.indented_prefix);
        self.suffix = overrides.suffix.or(self.suffix);
        self.fold = overrides.fold.or(self.fold);
        self.width = overrides.width.or(self.width);
        self
    }

    pub fn get_title(&self) -> &str {
        self.title.unwrap_or("unknown")
    }

    /// Value for fixed-width rendering, with default as title width + 2 and
    /// minimum width 11.
    fn get_width(&self) -> usize {
        const MIN_WIDTH: usize = 11;
        std::cmp::max(MIN_WIDTH, self.width.unwrap_or(self.get_title().len() + 2))
    }

    pub fn render_title(&self, fixed_width: bool) -> String {
        if fixed_width {
            get_fixed_width(self.get_title(), self.get_width())
        } else {
            self.get_title().to_owned()
        }
    }

    /// Applies format to render a Field into a String.
    fn format(&self, field: Field) -> String {
        use RenderFormat::*;
        match &self.format {
            Some(format) => match format {
                Precision(precision) => format!("{:.precision$}", field, precision = precision),
                ReadableSize => convert_bytes(f64::from(field)),
                PageReadableSize => convert_bytes(4096.0 * f64::from(field)),
                MaxOrReadableSize => {
                    let field = i64::from(field);
                    if field == -1 {
                        "max".to_owned()
                    } else {
                        convert_bytes(field as f64)
                    }
                }
            },
            None => field.to_string(),
        }
    }

    fn fold_str(&self, val: &str, width: usize) -> String {
        match self.fold {
            Some(FoldOption::Name) => fold_string(val, width, 0, |c: char| !c.is_alphanumeric()),
            Some(FoldOption::Path) => fold_string(val, width, 1, |c: char| c == '/'),
            None => val.to_owned(),
        }
    }

    /// Renders Field with all options applied. `depth` specifies the depth of
    /// the model of this Field, where the model is Recursive, i.e. it works as
    /// a node in a tree. Currently this only affects indented_prefix.
    pub fn render_indented(&self, field: Option<Field>, fixed_width: bool, depth: usize) -> String {
        let res = match field {
            Some(field) => self.format(field),
            None => {
                return if fixed_width {
                    get_fixed_width("?", self.get_width())
                } else {
                    "?".to_owned()
                };
            }
        };
        let indented_prefix = self.indented_prefix.unwrap_or("");
        let suffix = self.suffix.unwrap_or("");
        // May contain UTF8 chars
        let indented_prefix_len = indented_prefix.chars().count();
        let suffix_len = suffix.chars().count();
        // When depth == 0, neither indent nor prefix is rendered.
        let indented_prefix_width = indented_prefix_len * depth;
        if fixed_width {
            // The folded string has target len be fixed width subtracts indent,
            // indented_prefix, and suffix.
            let remain_len = self
                .get_width()
                .saturating_sub(indented_prefix_width + suffix_len);
            let res = self.fold_str(&res, remain_len);
            let res = format!(
                "{:>prefix_width$.prefix_width$}{}{}",
                indented_prefix,
                res,
                suffix,
                prefix_width = indented_prefix_width
            );
            get_fixed_width(&res, self.get_width())
        } else {
            format!(
                "{:>prefix_width$.prefix_width$}{}{}",
                indented_prefix,
                res,
                suffix,
                prefix_width = indented_prefix_width
            )
        }
    }

    /// Renders Field with all options without indent.
    pub fn render(&self, field: Option<Field>, fixed_width: bool) -> String {
        self.render_indented(field, fixed_width, 0)
    }
}

/// Provide default RenderConfig for each Field in a Model
pub trait HasRenderConfig: Queriable {
    fn get_render_config(field_id: &Self::FieldId) -> RenderConfig;
}
