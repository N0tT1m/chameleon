use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppRule {
    pub app_name: String,
    pub service_name: Option<String>,
    pub mac_address: String,
    pub interface: String,
    pub schedule: Option<Schedule>,
    pub last_applied: Option<DateTime<Utc>>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schedule {
    pub days: Vec<String>,  // "monday", "tuesday", etc.
    pub start_time: String, // "HH:MM"
    pub end_time: String,   // "HH:MM"
}

#[derive(Debug)]
pub struct RuleManager {
    rules: HashMap<String, AppRule>,
    config_path: PathBuf,
}

impl RuleManager {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let config_path = dirs::config_dir()
            .ok_or("Could not find config directory")?
            .join("mac_changer")
            .join("app_rules.json");

        let mut manager = Self {
            rules: HashMap::new(),
            config_path,
        };

        manager.load_rules()?;
        Ok(manager)
    }

    fn load_rules(&mut self) -> Result<(), Box<dyn Error>> {
        if self.config_path.exists() {
            let content = fs::read_to_string(&self.config_path)?;
            self.rules = serde_json::from_str(&content)?;
        }
        Ok(())
    }

    fn save_rules(&self) -> Result<(), Box<dyn Error>> {
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(&self.rules)?;
        fs::write(&self.config_path, content)?;
        Ok(())
    }

    pub fn add_rule(&mut self, rule: AppRule) -> Result<(), Box<dyn Error>> {
        let key = format!("{}:{}", rule.app_name, rule.interface);
        self.rules.insert(key, rule);
        self.save_rules()?;
        Ok(())
    }

    pub fn remove_rule(&mut self, app_name: &str, interface: &str) -> Result<(), Box<dyn Error>> {
        let key = format!("{}:{}", app_name, interface);
        self.rules.remove(&key);
        self.save_rules()?;
        Ok(())
    }

    pub fn get_rule(&self, app_name: &str, interface: &str) -> Option<&AppRule> {
        let key = format!("{}:{}", app_name, interface);
        self.rules.get(&key)
    }

    pub fn list_rules(&self) -> Vec<&AppRule> {
        self.rules.values().collect()
    }

    pub fn is_rule_active(&self, rule: &AppRule) -> bool {
        if !rule.enabled {
            return false;
        }

        if let Some(schedule) = &rule.schedule {
            let now = chrono::Local::now();
            let current_day = now.format("%A").to_string().to_lowercase();

            // Check if current day is in schedule
            if !schedule.days.iter().any(|day| day.to_lowercase() == current_day) {
                return false;
            }

            // Parse schedule times
            let start_time = chrono::NaiveTime::parse_from_str(&schedule.start_time, "%H:%M").unwrap();
            let end_time = chrono::NaiveTime::parse_from_str(&schedule.end_time, "%H:%M").unwrap();
            let current_time = now.time();

            // Check if current time is within schedule
            if current_time < start_time || current_time > end_time {
                return false;
            }
        }

        true
    }
}
