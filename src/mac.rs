use crate::error::MacError;
use rand::Rng;
use regex::Regex;
use std::str::FromStr;
use rand::prelude::ThreadRng;

#[derive(Debug)]
pub struct MacAddress {
    pub bytes: [u8; 6],
    pub format: MacFormat,
}

#[derive(Debug)]
pub enum MacFormat {
    Colon,      // 00:11:22:33:44:55
    Hyphen,     // 00-11-22-33-44-55
    Dot,        // 0011.2233.4455
    Plain,      // 001122334455
}

impl FromStr for MacAddress {
    type Err = MacError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let re_colon = Regex::new(r"^([0-9A-Fa-f]{2}:){5}[0-9A-Fa-f]{2}$").unwrap();
        let re_hyphen = Regex::new(r"^([0-9A-Fa-f]{2}-){5}[0-9A-Fa-f]{2}$").unwrap();
        let re_dot = Regex::new(r"^[0-9A-Fa-f]{4}\.[0-9A-Fa-f]{4}\.[0-9A-Fa-f]{4}$").unwrap();
        let re_plain = Regex::new(r"^[0-9A-Fa-f]{12}$").unwrap();

        let (bytes, format) = if re_colon.is_match(s) {
            (s.split(':')
                 .map(|x| u8::from_str_radix(x, 16))
                 .collect::<Result<Vec<_>, _>>()?, MacFormat::Colon)
        } else if re_hyphen.is_match(s) {
            (s.split('-')
                 .map(|x| u8::from_str_radix(x, 16))
                 .collect::<Result<Vec<_>, _>>()?, MacFormat::Hyphen)
        } else if re_dot.is_match(s) {
            let s = s.replace(".", "");
            let bytes: Vec<u8> = (0..6)
                .map(|i| u8::from_str_radix(&s[i*2..i*2+2], 16))
                .collect::<Result<Vec<_>, _>>()?;
            (bytes, MacFormat::Dot)
        } else if re_plain.is_match(s) {
            let bytes: Vec<u8> = (0..6)
                .map(|i| u8::from_str_radix(&s[i*2..i*2+2], 16))
                .collect::<Result<Vec<_>, _>>()?;
            (bytes, MacFormat::Plain)
        } else {
            return Err(MacError::InvalidFormat("Invalid MAC address format".into()));
        };

        if bytes.len() != 6 {
            return Err(MacError::InvalidFormat("MAC address must be 6 bytes".into()));
        }

        let mut arr = [0u8; 6];
        arr.copy_from_slice(&bytes);

        Ok(MacAddress { bytes: arr, format })
    }
}

impl ToString for MacAddress {
    fn to_string(&self) -> String {
        match self.format {
            MacFormat::Colon => self.bytes
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<Vec<_>>()
                .join(":"),
            MacFormat::Hyphen => self.bytes
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<Vec<_>>()
                .join("-"),
            MacFormat::Dot => format!(
                "{:02x}{:02x}.{:02x}{:02x}.{:02x}{:02x}",
                self.bytes[0], self.bytes[1], self.bytes[2],
                self.bytes[3], self.bytes[4], self.bytes[5]
            ),
            MacFormat::Plain => self.bytes
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<String>(),
        }
    }
}

pub fn generate_random_mac(vendor: Option<&str>) -> Result<MacAddress, MacError> {
    let mut rng = rand::thread_rng();
    let mut bytes = [0u8; 6];

    if let Some(vendor_str) = vendor {
        // Parse vendor prefix (first 3 bytes)
        let vendor_bytes = hex::decode(vendor_str.replace(":", ""))
            .map_err(|_| MacError::VendorNotFound("Invalid vendor prefix format".into()))?;

        if vendor_bytes.len() != 3 {
            return Err(MacError::VendorNotFound("Vendor prefix must be 3 bytes".into()));
        }
        bytes[..3].copy_from_slice(&vendor_bytes);

        // Generate random bytes for the rest
        for i in 3..6 {
            bytes[i] = rng.r#gen::<u8>();
        }
    } else {
        // Generate completely random MAC, ensuring locally administered bit is set
        bytes[0] = (rng.r#gen::<u8>() & 0xFE) | 0x02; // Set locally administered bit
        for i in 1..6 {
            bytes[i] = rng.r#gen::<u8>();
        }
    }

    Ok(MacAddress { bytes, format: MacFormat::Colon })
}