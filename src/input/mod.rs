//! Log input sources.

pub mod cloudwatch;
pub mod file;

pub use cloudwatch::{process_cloudwatch_input, CloudWatchInput, CloudWatchSince, CloudWatchUntil};
pub use file::{
    discover_log_files, discover_log_files_for_path, process_log_file, process_log_paths,
    validate_file_input_args, LocalLogInput,
};
