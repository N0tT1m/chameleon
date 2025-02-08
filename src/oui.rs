use std::error::Error;
use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use crate::error::MacError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VendorInfo {
    pub prefix: String,
    pub name: String,
    pub country: String,
}

pub struct OUIDatabase {
    db_path: PathBuf,
    vendors: HashMap<String, VendorInfo>,
}

// src/oui.rs (relevant section)
impl OUIDatabase {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let db_path = dirs::config_dir()
            .ok_or_else(|| MacError::DatabaseError("Could not find config directory".into()))?
            .join("mac_changer")
            .join("oui.json");

        // Create directory if it doesn't exist
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let vendors = if db_path.exists() {
            let content = std::fs::read_to_string(&db_path)?;
            serde_json::from_str(&content)?
        } else {
            // Initialize with default vendors
            let mut defaults = HashMap::new();

            // Add some common vendors
            defaults.insert("00:17:F2".to_string(), VendorInfo {
                prefix: "00:17:F2".to_string(),
                name: "Apple, Inc.".to_string(),
                country: "US".to_string(),
            });

            defaults.insert("00:1A:11".to_string(), VendorInfo {
                prefix: "00:1A:11".to_string(),
                name: "Google, Inc.".to_string(),
                country: "US".to_string(),
            });

            defaults
        };

        Ok(Self { db_path, vendors })
    }

    pub async fn update(&mut self) -> Result<(), Box<dyn Error>> {
        println!("Downloading OUI database from IEEE...");

        // Download the OUI database
        let response = reqwest::get("http://standards-oui.ieee.org/oui/oui.txt").await?;
        let content = response.text().await?;

        // Parse the text file
        let mut new_vendors = HashMap::new();

        for line in content.lines() {
            if line.contains("(hex)") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 4 {
                    continue;
                }

                let prefix = parts[0].replace("-", ":");

                // Find company name and address
                let mut company_name = String::new();
                let mut found_company = false;
                let mut country = String::new();

                for part in parts[3..].iter() {
                    if !found_company {
                        if !company_name.is_empty() {
                            company_name.push(' ');
                        }
                        company_name.push_str(part);
                        if company_name.ends_with('.') {
                            found_company = true;
                        }
                    }
                }

                // Try to find country from remaining lines
                let mut lines = content.lines().skip_while(|&l| l != line).skip(1);
                while let Some(address_line) = lines.next() {
                    if address_line.trim().is_empty() {
                        break;
                    }
                    // Usually the country is on the last line of the address
                    country = address_line.trim().to_string();
                }

                // Extract country code (assuming last word is country)
                let country_code = country.split_whitespace()
                    .last()
                    .unwrap_or("US")  // Default to US if we can't determine
                    .to_string();

                new_vendors.insert(prefix.clone(), VendorInfo {
                    prefix,
                    name: company_name,
                    country: country_code,
                });
            }
        }

        // Save to file
        if !new_vendors.is_empty() {
            let json = serde_json::to_string_pretty(&new_vendors)?;
            std::fs::write(&self.db_path, json)?;
            self.vendors = new_vendors;
        }

        println!("OUI database updated successfully. Found {} vendors.", self.vendors.len());
        Ok(())
    }

    pub fn get_vendor(&self, mac_prefix: &str) -> Option<&VendorInfo> {
        let prefix = mac_prefix
            .replace([':', '-', '.'], "")
            .to_uppercase();

        if prefix.len() >= 6 {
            self.vendors.get(&prefix[0..6])
        } else {
            None
        }
    }

    pub fn vendors_by_country(&self, country: &str) -> Vec<&VendorInfo> {
        self.vendors
            .values()
            .filter(|v| v.country.to_uppercase() == country.to_uppercase())
            .collect()
    }

    pub fn list_countries(&self) -> Vec<String> {
        let mut countries: Vec<String> = self.vendors
            .values()
            .map(|v| v.country.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        countries.sort();
        countries
    }
}