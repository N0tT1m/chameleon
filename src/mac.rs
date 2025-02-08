// src/mac.rs
use std::fmt;
use rand::Rng;
use std::num::ParseIntError;
use crate::error::MacError;

#[derive(Debug, Clone)]
pub enum MacFormat {
    Colon,      // XX:XX:XX:XX:XX:XX
    Hyphen,     // XX-XX-XX-XX-XX-XX
    Dot,        // XX.XX.XX.XX.XX.XX
    Raw,        // XXXXXXXXXXXX (no separators)
}

#[derive(Debug, Clone)]
pub struct MacAddress {
    bytes: [u8; 6],
    format: MacFormat,
}

impl MacAddress {
    pub fn new(bytes: [u8; 6], format: MacFormat) -> Self {
        Self { bytes, format }
    }

    pub fn with_format(&self, format: MacFormat) -> Self {
        Self {
            bytes: self.bytes,
            format: format,
        }
    }

    pub fn parse(mac_str: &str) -> Result<Self, MacError> {
        let clean_mac = mac_str.replace([':', '-', '.'], "");
        if clean_mac.len() != 12 {
            return Err(MacError::InvalidFormat("MAC address must be 12 hexadecimal characters".into()));
        }

        let bytes: Result<Vec<u8>, ParseIntError> = (0..6)
            .map(|i| u8::from_str_radix(&clean_mac[i * 2..(i + 1) * 2], 16))
            .collect();

        match bytes {
            Ok(b) => {
                let mut array = [0u8; 6];
                array.copy_from_slice(&b);

                // Determine format from original string
                let format = if mac_str.contains(':') {
                    MacFormat::Colon
                } else if mac_str.contains('-') {
                    MacFormat::Hyphen
                } else if mac_str.contains('.') {
                    MacFormat::Dot
                } else {
                    MacFormat::Raw
                };

                Ok(Self { bytes: array, format })
            }
            Err(e) => Err(MacError::from(e))
        }
    }

    pub fn as_string(&self) -> String {
        match self.format {
            MacFormat::Colon => format!(
                "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                self.bytes[0], self.bytes[1], self.bytes[2],
                self.bytes[3], self.bytes[4], self.bytes[5]
            ),
            MacFormat::Hyphen => format!(
                "{:02x}-{:02x}-{:02x}-{:02x}-{:02x}-{:02x}",
                self.bytes[0], self.bytes[1], self.bytes[2],
                self.bytes[3], self.bytes[4], self.bytes[5]
            ),
            MacFormat::Dot => format!(
                "{:02x}.{:02x}.{:02x}.{:02x}.{:02x}.{:02x}",
                self.bytes[0], self.bytes[1], self.bytes[2],
                self.bytes[3], self.bytes[4], self.bytes[5]
            ),
            MacFormat::Raw => format!(
                "{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
                self.bytes[0], self.bytes[1], self.bytes[2],
                self.bytes[3], self.bytes[4], self.bytes[5]
            ),
        }
    }

    pub fn get_bytes(&self) -> &[u8; 6] {
        &self.bytes
    }
}

// Remove the ToString implementation since it's automatically
// derived from Display
impl fmt::Display for MacAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_string())
    }
}

pub fn generate_random_mac(vendor_prefix: Option<&str>) -> Result<MacAddress, MacError> {
    let mut rng = rand::thread_rng();
    let mut bytes = [0u8; 6];

    if let Some(prefix) = vendor_prefix {
        let prefix_bytes = prefix.split(|c| c == ':' || c == '-')
            .take(3)
            .map(|b| u8::from_str_radix(b, 16))
            .collect::<Result<Vec<_>, _>>()?;

        if prefix_bytes.len() != 3 {
            return Err(MacError::VendorNotFound("Vendor prefix must be 3 bytes".into()));
        }

        bytes[0..3].copy_from_slice(&prefix_bytes);
    } else {
        // Generate random locally administered unicast address
        bytes[0] = rng.r#gen::<u8>() & 0xFE | 0x02;
        bytes[1] = rng.r#gen();
        bytes[2] = rng.r#gen();
    }

    bytes[3] = rng.r#gen();
    bytes[4] = rng.r#gen();
    bytes[5] = rng.r#gen();

    Ok(MacAddress::new(bytes, MacFormat::Colon))
}