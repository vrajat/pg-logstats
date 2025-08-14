//! Output formatters for pg-loggrep analysis results

pub mod json;
pub mod text;

pub use json::JsonFormatter;
pub use text::TextFormatter;
