use std::collections::HashSet;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Serialize, Deserialize)]
pub struct MacFilter {
    whitelist: HashSet<String>,
    blacklist: HashSet<String>,
    config_path: PathBuf,
}

impl MacFilter {
    pub fn new() -> Self {
        let config_path = dirs::config_dir()
            .unwrap_or_default()
            .join("mac_changer")
            .join("filters.json");

        let mut filter = Self {
            whitelist: HashSet::new(),
            blacklist: HashSet::new(),
            config_path,
        };

        filter.load_filters();
        filter
    }

    fn load_filters(&mut self) {
        if let Ok(content) = fs::read_to_string(&self.config_path) {
            if let Ok(filters) = serde_json::from_str::<MacFilter>(&content) {
                self.whitelist = filters.whitelist;
                self.blacklist = filters.blacklist;
            }
        }
    }

    pub fn save_filters(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        fs::write(&self.config_path, content)?;
        Ok(())
    }

    pub fn add_to_whitelist(&mut self, mac_prefix: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.whitelist.insert(mac_prefix.to_uppercase());
        self.save_filters()?;
        Ok(())
    }

    pub fn add_to_blacklist(&mut self, mac_prefix: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.blacklist.insert(mac_prefix.to_uppercase());
        self.save_filters()?;
        Ok(())
    }

    pub fn is_allowed(&self, mac: &str) -> bool {
        let prefix = &mac[0..8].to_uppercase();

        if !self.whitelist.is_empty() {
            return self.whitelist.contains(prefix);
        }

        if !self.blacklist.is_empty() {
            return !self.blacklist.contains(prefix);
        }

        true
    }
}