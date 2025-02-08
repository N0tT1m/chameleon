use std::error::Error;
use std::process::Command;
use std::fs;
use std::path::Path;
use crate::error::MacError;

#[derive(Debug)]
pub struct NetworkCard {
    pub interface: String,
    pub vendor: Option<String>,
    pub supports_mac_change: bool,
    pub permanent_change_supported: bool,
    pub driver: String,
}

impl NetworkCard {
    pub fn verify_interface(interface: &str) -> Result<Self, Box<dyn Error>> {
        let interfaces = pnet::datalink::interfaces();

        if !interfaces.iter().any(|iface| iface.name == interface) {
            return Err(Box::new(MacError::ValidationFailed(
                format!("Interface {} not found", interface)
            )));
        }

        Self::new(interface)
    }

    #[cfg(target_os = "linux")]
    fn new(interface: &str) -> Result<Self, Box<dyn Error>> {
        let sys_net_path = Path::new("/sys/class/net").join(interface);

        // Check if interface exists in sysfs
        if !sys_net_path.exists() {
            return Err(Box::new(MacError::ValidationFailed(
                format!("Interface {} not found in sysfs", interface)
            )));
        }

        // Check interface type
        let interface_type = fs::read_to_string(sys_net_path.join("type"))
            .map_err(|_| MacError::ValidationFailed(format!("Failed to read interface type for {}", interface)))?
            .trim()
            .to_string();

        // Check if this is a loopback interface
        if interface_type == "772" {
            return Err(Box::new(MacError::ValidationFailed(
                format!("Cannot change MAC address of loopback interface {}", interface)
            )));
        }

        // Get device information if available
        let (vendor, driver) = if sys_net_path.join("device").exists() {
            let uevent_path = sys_net_path.join("device/uevent");
            if uevent_path.exists() {
                let content = fs::read_to_string(&uevent_path)
                    .unwrap_or_default();

                let vendor = content.lines()
                    .find(|line| line.starts_with("DRIVER="))
                    .map(|line| line.split('=').nth(1).unwrap_or("").to_string());

                let driver = content.lines()
                    .find(|line| line.starts_with("DRIVER="))
                    .map(|line| line.split('=').nth(1).unwrap_or("").to_string())
                    .unwrap_or_default();

                (vendor, driver)
            } else {
                (None, String::new())
            }
        } else {
            (None, String::new())
        };

        // Check if address file exists (indicates MAC change support)
        let supports_mac_change = sys_net_path.join("address").exists() &&
            interface_type != "772" && // Not loopback
            interface_type != "768";   // Not point to point

        Ok(NetworkCard {
            interface: interface.to_string(),
            vendor,
            supports_mac_change,
            permanent_change_supported: supports_mac_change,
            driver,
        })
    }

    #[cfg(target_os = "macos")]
    fn new(interface: &str) -> Result<Self, Box<dyn Error>> {
        let output = Command::new("networksetup")
            .args(&["-listallhardwareports"])
            .output()?;

        if !output.status.success() {
            return Err(Box::new(MacError::SystemError(
                String::from_utf8_lossy(&output.stderr).to_string()
            )));
        }

        Ok(NetworkCard {
            interface: interface.to_string(),
            vendor: None,
            supports_mac_change: true,
            permanent_change_supported: false,
            driver: String::new(),
        })
    }

    #[cfg(target_os = "windows")]
    fn new(interface: &str) -> Result<Self, Box<dyn Error>> {
        let output = Command::new("wmic")
            .args(&["nic", "get", "name,manufacturer,servicename"])
            .output()?;

        if !output.status.success() {
            return Err(Box::new(MacError::SystemError(
                String::from_utf8_lossy(&output.stderr).to_string()
            )));
        }

        Ok(NetworkCard {
            interface: interface.to_string(),
            vendor: None,
            supports_mac_change: true,
            permanent_change_supported: true,
            driver: String::new(),
        })
    }
}

pub fn get_current_mac(interface: &str) -> Result<String, Box<dyn Error>> {
    #[cfg(target_os = "linux")]
    {
        // First try reading from sysfs
        let addr_path = Path::new("/sys/class/net").join(interface).join("address");
        if addr_path.exists() {
            if let Ok(mac) = fs::read_to_string(addr_path) {
                let mac = mac.trim();
                if !mac.is_empty() {
                    return Ok(mac.to_string());
                }
            }
        }

        // Fallback to ip command
        let output = Command::new("ip")
            .args(&["link", "show", interface])
            .output()?;

        if !output.status.success() {
            return Err(Box::new(MacError::SystemError(
                String::from_utf8_lossy(&output.stderr).to_string()
            )));
        }

        let output_str = String::from_utf8_lossy(&output.stdout);
        if let Some(mac) = output_str
            .lines()
            .find(|line| line.contains("link/ether"))
            .and_then(|line| line.split_whitespace().nth(1))
        {
            return Ok(mac.to_string());
        }

        return Err(Box::new(MacError::ValidationFailed(
            format!("Could not get current MAC address for interface {}", interface)
        )));
    }

    #[cfg(target_os = "macos")]
    {
        let output = Command::new("ifconfig")
            .arg(interface)
            .output()?;

        if !output.status.success() {
            return Err(Box::new(MacError::SystemError(
                String::from_utf8_lossy(&output.stderr).to_string()
            )));
        }

        let output_str = String::from_utf8_lossy(&output.stdout);
        if let Some(mac) = output_str
            .lines()
            .find(|line| line.contains("ether"))
            .and_then(|line| line.split_whitespace().nth(1))
        {
            return Ok(mac.to_string());
        }

        return Err(Box::new(MacError::ValidationFailed(
            format!("Could not get current MAC address for interface {}", interface)
        )));
    }

    #[cfg(target_os = "windows")]
    {
        let output = Command::new("getmac")
            .args(&["/v", "/fo", "csv"])
            .output()?;

        if !output.status.success() {
            return Err(Box::new(MacError::SystemError(
                String::from_utf8_lossy(&output.stderr).to_string()
            )));
        }

        let output_str = String::from_utf8_lossy(&output.stdout);
        if let Some(mac) = output_str
            .lines()
            .find(|line| line.contains(interface))
            .and_then(|line| line.split(',').nth(2))
        {
            return Ok(mac.trim_matches('"').to_string());
        }

        return Err(Box::new(MacError::ValidationFailed(
            format!("Could not get current MAC address for interface {}", interface)
        )));
    }

    #[allow(unreachable_code)]
    Err(Box::new(MacError::UnsupportedPlatform(
        "Unsupported operating system".into()
    )))
}