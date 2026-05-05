//! Log format parsers for different PostgreSQL log formats

pub mod text;

pub use text::{TextLogFormat, TextLogParser};
