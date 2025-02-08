// src/geolocation.rs
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::collections::HashMap;
use crate::error::MacError;
use crate::oui::OUIDatabase;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoLocation {
    pub country: String,
    pub region: String,
    pub city: String,
    pub vendor: String,
}

pub struct GeoLocationService {
    cache: HashMap<String, GeoLocation>,
}

impl GeoLocationService {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    pub fn get_location(&mut self, mac: &str, oui_db: &OUIDatabase) -> Result<GeoLocation, Box<dyn Error>> {
        // Check cache first
        if let Some(location) = self.cache.get(mac) {
            return Ok(location.clone());
        }

        // Get vendor prefix
        let prefix = &mac[0..8].to_uppercase();

        // Look up vendor info from OUI database
        let vendor_info = oui_db.get_vendor(prefix)
            .ok_or_else(|| MacError::ValidationFailed(
                format!("No vendor found for prefix {}", prefix)
            ))?;

        let location = GeoLocation {
            country: vendor_info.country.clone(),
            region: String::new(),
            city: String::new(),
            vendor: vendor_info.name.clone(),
        };

        self.cache.insert(mac.to_string(), location.clone());
        Ok(location)
    }

    pub fn suggest_mac_for_location(&self, country: &str, oui_db: &OUIDatabase) -> Option<String> {
        // Find vendors for the specified country
        let vendors = oui_db.vendors_by_country(country);

        if vendors.is_empty() {
            return None;
        }

        // Use the first vendor found
        let vendor = vendors[0];

        // Generate random suffix
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let random_suffix: u32 = rng.gen_range(0..0xFFFFFF);

        Some(format!("{}:{:06X}", vendor.prefix, random_suffix))
    }
}