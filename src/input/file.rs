use crate::{LogEntry, PgLogstatsError, Result, TextLogParser};
use log::{info, warn};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct LocalLogInput {
    pub log_dir: Option<PathBuf>,
    pub sample_size: Option<usize>,
    pub logfile_list: Option<String>,
    pub log_files: Vec<String>,
}

pub fn validate_file_input_args(input: &LocalLogInput) -> Result<()> {
    if let Some(log_dir) = &input.log_dir {
        if !log_dir.exists() {
            return Err(PgLogstatsError::Configuration {
                message: format!("Log directory does not exist: {}", log_dir.display()),
                field: Some("log_dir".to_string()),
            });
        }

        if !log_dir.is_dir() {
            return Err(PgLogstatsError::Configuration {
                message: format!(
                    "Log directory path is not a directory: {}",
                    log_dir.display()
                ),
                field: Some("log_dir".to_string()),
            });
        }

        fs::read_dir(log_dir).map_err(|err| PgLogstatsError::Configuration {
            message: format!("Cannot read log directory {}: {}", log_dir.display(), err),
            field: Some("log_dir".to_string()),
        })?;
    }

    Ok(())
}

pub fn discover_log_files(input: &LocalLogInput) -> Result<Vec<PathBuf>> {
    let mut log_files = Vec::new();

    if let Some(log_dir) = &input.log_dir {
        discover_files_in_directory(log_dir, &mut log_files)?;
    }

    for file_pattern in &input.log_files {
        if let Ok(path) = PathBuf::from(file_pattern).canonicalize() {
            if path.is_file() {
                log_files.push(path);
            }
        } else {
            let path = Path::new(file_pattern);
            if path.exists() && path.is_file() {
                log_files.push(path.to_path_buf());
            }
        }
    }

    if let Some(logfile_list) = &input.logfile_list {
        let list_content = fs::read_to_string(logfile_list).map_err(PgLogstatsError::Io)?;

        for line in list_content.lines() {
            let line = line.trim();
            if !line.is_empty() && !line.starts_with('#') {
                let path = Path::new(line);
                if path.exists() && path.is_file() {
                    log_files.push(path.to_path_buf());
                }
            }
        }
    }

    log_files.sort();
    log_files.dedup();

    log_files.retain(|path| match fs::metadata(path) {
        Ok(metadata) => {
            if metadata.len() == 0 {
                warn!("Skipping empty log file: {}", path.display());
                false
            } else {
                true
            }
        }
        Err(err) => {
            warn!("Cannot read metadata for {}: {}", path.display(), err);
            false
        }
    });

    Ok(log_files)
}

pub fn discover_log_files_for_path(path: &Path) -> Result<Vec<PathBuf>> {
    if !path.exists() {
        return Err(PgLogstatsError::Configuration {
            message: format!("Log path does not exist: {}", path.display()),
            field: Some("path".to_string()),
        });
    }

    let mut log_files = Vec::new();
    if path.is_file() {
        log_files.push(path.to_path_buf());
    } else if path.is_dir() {
        discover_files_in_directory(path, &mut log_files)?;
    } else {
        return Err(PgLogstatsError::Configuration {
            message: format!("Log path is neither file nor directory: {}", path.display()),
            field: Some("path".to_string()),
        });
    }

    log_files.sort();
    log_files.dedup();
    Ok(log_files)
}

fn discover_files_in_directory(dir: &Path, log_files: &mut Vec<PathBuf>) -> Result<()> {
    let entries = fs::read_dir(dir)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(extension) = path.extension() {
                let ext_str = extension.to_string_lossy().to_lowercase();
                if ext_str == "log" || ext_str == "txt" {
                    log_files.push(path);
                }
            } else if let Some(filename) = path.file_name() {
                let filename_str = filename.to_string_lossy().to_lowercase();
                if filename_str.contains("postgres") || filename_str.contains("pg") {
                    log_files.push(path);
                }
            }
        }
    }

    Ok(())
}

pub fn process_log_file(
    log_file: &Path,
    parser: &TextLogParser,
    sample_size: Option<usize>,
) -> Result<Vec<LogEntry>> {
    let content = fs::read_to_string(log_file)?;
    let lines: Vec<String> = content.lines().map(str::to_string).collect();

    let lines_to_process = if let Some(sample_size) = sample_size {
        if lines.len() > sample_size {
            info!(
                "Limiting analysis to first {} lines of {}",
                sample_size,
                log_file.display()
            );
            &lines[..sample_size]
        } else {
            &lines
        }
    } else {
        &lines
    };

    parser.parse_lines(lines_to_process)
}

pub fn process_log_paths(
    path: &Path,
    parser: &TextLogParser,
    sample_size: Option<usize>,
) -> Result<Vec<LogEntry>> {
    let log_files = discover_log_files_for_path(path)?;
    if log_files.is_empty() {
        return Err(PgLogstatsError::Configuration {
            message: format!("No log files found under {}", path.display()),
            field: Some("path".to_string()),
        });
    }

    let mut all_entries = Vec::new();
    for log_file in log_files {
        let mut entries = process_log_file(&log_file, parser, sample_size)?;
        all_entries.append(&mut entries);
    }

    Ok(all_entries)
}
