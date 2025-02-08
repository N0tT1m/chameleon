use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct MacChange {
    pub timestamp: DateTime<Utc>,
    pub interface: String,
    pub old_mac: String,
    pub new_mac: String,
    pub geo_location: Option<String>,
    pub permanent: bool,
}

pub struct MacLogger {
    log_dir: PathBuf,
    max_log_size: u64,
    max_log_files: usize,
}

impl MacLogger {
    pub fn new() -> Self {
        let log_dir = dirs::data_dir()
            .unwrap_or_default()
            .join("mac_changer")
            .join("logs");

        fs::create_dir_all(&log_dir).unwrap_or_default();

        Self {
            log_dir,
            max_log_size: 10 * 1024 * 1024, // 10MB
            max_log_files: 5,
        }
    }

    pub fn log_change(&self, change: MacChange) -> Result<(), Box<dyn std::error::Error>> {
        let log_file = self.log_dir.join("mac_changes.log");

        // Check if rotation needed
        if let Ok(metadata) = fs::metadata(&log_file) {
            if metadata.len() > self.max_log_size {
                self.rotate_logs()?;
            }
        }

        // Append to log file
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_file)?;

        let log_entry = serde_json::to_string(&change)?;
        writeln!(file, "{}", log_entry)?;

        Ok(())
    }

    fn rotate_logs(&self) -> Result<(), Box<dyn std::error::Error>> {
        for i in (1..self.max_log_files).rev() {
            let old_path = self.log_dir.join(format!("mac_changes.{}.log", i));
            let new_path = self.log_dir.join(format!("mac_changes.{}.log", i + 1));

            if old_path.exists() {
                fs::rename(old_path, new_path)?;
            }
        }

        let current = self.log_dir.join("mac_changes.log");
        let backup = self.log_dir.join("mac_changes.1.log");

        if current.exists() {
            fs::rename(current, backup)?;
        }

        Ok(())
    }

    pub fn get_history(&self) -> Result<Vec<MacChange>, Box<dyn std::error::Error>> {
        let mut history = Vec::new();
        let log_file = self.log_dir.join("mac_changes.log");

        if log_file.exists() {
            let content = fs::read_to_string(log_file)?;
            for line in content.lines() {
                if let Ok(change) = serde_json::from_str(line) {
                    history.push(change);
                }
            }
        }

        Ok(history)
    }
}