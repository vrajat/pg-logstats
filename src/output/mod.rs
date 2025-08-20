//! Output formatters for pg-logstats analysis results

pub mod json;
pub mod text;

pub use json::JsonFormatter;
pub use text::TextFormatter;
