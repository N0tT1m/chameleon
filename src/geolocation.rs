use serde::{Deserialize, Serialize};
use std::error::Error;
use std::collections::HashMap;
use reqwest;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoLocation {
    pub country: String,
    pub region: String,
    pub city: String,
    pub vendor: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VendorInfo {
    pub prefix: String,
    pub name: String,
    pub country: String,
}

pub struct GeoLocationService {
    vendor_db: HashMap<String, VendorInfo>,
    cache: HashMap<String, GeoLocation>,
}

impl GeoLocationService {
    pub fn new() -> Self {
        Self {
            vendor_db: Self::load_vendor_db(),
            cache: HashMap::new(),
        }
    }

    fn load_vendor_db() -> HashMap<String, VendorInfo> {
        // Load from local OUI database
        let db_path = dirs::config_dir()
            .unwrap_or_default()
            .join("mac_changer")
            .join("oui.json");

        if let Ok(content) = std::fs::read_to_string(db_path) {
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            HashMap::new()
        }
    }

    pub async fn get_location(&mut self, mac: &str) -> Result<GeoLocation, Box<dyn Error>> {
        // Check cache first
        if let Some(location) = self.cache.get(mac) {
            return Ok(location.clone());
        }

        // Get vendor prefix
        let prefix = &mac[0..8].to_uppercase();

        // Look up vendor info
        if let Some(vendor_info) = self.vendor_db.get(prefix) {
            let location = GeoLocation {
                country: vendor_info.country.clone(),
                region: String::new(),
                city: String::new(),
                vendor: vendor_info.name.clone(),
            };

            self.cache.insert(mac.to_string(), location.clone());
            return Ok(location);
        }

        Err("Location not found".into())
    }

    pub fn suggest_mac_for_location(&self, country: &str) -> Option<String> {
        // Find vendor from desired country
        for vendor in self.vendor_db.values() {
            if vendor.country == country {
                return Some(format!("{}:00:00:00", vendor.prefix));
            }
        }
        None
    }
}