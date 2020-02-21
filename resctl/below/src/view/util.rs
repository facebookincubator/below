/// This file contains various helpers for the UI bits

/// A row in the UI is composed of one or more `Field`s
pub struct Field {
    /// Formatted column name
    pub name: String,
    /// Formatted value
    pub value: String,
}

/// Macro to help unwrap nested optionals or return a default value
/// if unwrap fails.
///
/// Example of optional: Option<Option<String>>
#[macro_export]
macro_rules! get_inner_or_default {
    ($obj:expr, $default:ident) => {
        $obj.as_ref()
            .map_or_else(|| $default.clone(), |o| o.to_string())
    };

    ($obj:expr, $default:ident, $inner:ident, $inner_f:expr) => {
        $obj.as_ref().map_or_else(
            || $default.clone(),
            |o| o.$inner.map_or_else(|| $default.clone(), $inner_f),
        )
    };

    ($obj:expr, $default:ident, $inner:ident) => {
        get_inner_or_default!($obj, $default, $inner, |n| n.to_string())
    };
}

/// Helper to calculate the row header.
///
/// Note this assumes each row has the exact same field names.
pub fn get_header(rows: &Vec<Vec<Field>>) -> String {
    let mut header = String::new();

    if !rows.is_empty() {
        for field in &rows[0] {
            header.push_str(&field.name);
            header.push(' ');
        }
    }

    header
}

/// Convert `val` bytes into a human friendly string
pub fn convert_bytes(val: f64) -> String {
    let units = ["B", "KB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"];
    if val < 1_f64 {
        return format!("{:.1} B", val);
    }
    let delimiter = 1000_f64;
    let exponent = std::cmp::min(
        (val.ln() / delimiter.ln()).floor() as i32,
        (units.len() - 1) as i32,
    );
    let pretty_bytes = format!("{:.1}", val / delimiter.powi(exponent))
        .parse::<f64>()
        .unwrap()
        * 1_f64;
    let unit = units[exponent as usize];
    format!("{} {}", pretty_bytes, unit)
}
