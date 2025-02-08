use serde::{Serialize, Deserialize};
use std::error::Error;
use std::fs;
use crate::error::MacError;
use std;

#[derive(Debug, Serialize, Deserialize)]
pub struct MacConfig {
    pub original_mac: String,
    pub interface: String,
    pub vendor: Option<String>,
    pub last_modified: chrono::DateTime<chrono::Utc>,
}

pub fn save_original_mac(interface: &str, mac: &str) -> Result<(), Box<dyn Error>> {
    let config = MacConfig {
        original_mac: mac.to_string(),
        interface: interface.to_string(),
        vendor: None,
        last_modified: chrono::Utc::now(),
    };

    let config_dir = dirs::config_dir()
        .ok_or_else(|| MacError::SystemError("Could not find config directory".into()))?
        .join("mac_changer");

    fs::create_dir_all(&config_dir)?;

    let config_file = config_dir.join(format!("{}.json", interface));
    let config_json = serde_json::to_string_pretty(&config)?;
    fs::write(config_file, config_json)?;

    Ok(())
}

pub fn get_original_mac(interface: &str) -> Result<Option<String>, Box<dyn Error>> {
    let config_file = dirs::config_dir()
        .ok_or_else(|| MacError::SystemError("Could not find config directory".into()))?
        .join("mac_changer")
        .join(format!("{}.json", interface));

    if config_file.exists() {
        let content = fs::read_to_string(config_file)?;
        let config: MacConfig = serde_json::from_str(&content)?;
        Ok(Some(config.original_mac))
    } else {
        Ok(None)
    }
}
