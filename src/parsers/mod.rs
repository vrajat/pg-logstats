//! Log format parsers for different PostgreSQL log formats

pub mod stderr;

pub use stderr::{StderrParser, LogEntry};
