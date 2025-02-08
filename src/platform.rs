use std::error::Error;
use std::{fs, string};
use std::process::Command;
use crate::error::MacError;
use is_elevated;
use winreg::{RegKey, RegValue};
use winreg::enums::*;

#[cfg(target_os = "linux")]
fn find_command(cmd: &str) -> Option<String> {
    let paths = vec![
        "/sbin",
        "/usr/sbin",
        "/bin",
        "/usr/bin",
        "/usr/local/sbin",
        "/usr/local/bin"
    ];

    for path in paths {
        let full_path = format!("{}/{}", path, cmd);
        if std::path::Path::new(&full_path).exists() {
            return Some(full_path);
        }
    }
    None
}

#[cfg(target_os = "windows")]
fn find_command(cmd: &str) -> Option<String> {
    let paths = vec![
        "C:\\tools",
        "C:\\ProgramData",
        "C:\\Users\\Nathan\\AppData\\Roaming",
        "C:\\ProgramData\\chocolatey",
        "C:\\Program Files\\Common Files",
        "C:\\Program Files (x86)\\Common Files",
        "C:\\Program Files\\Common Files",
        "C:\\Windows",
    ];

    for path in paths {
        let full_path = format!("{}/{}", path, cmd);
        if std::path::Path::new(&full_path).exists() {
            return Some(full_path);
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn verify_interface_exists(interface: &str) -> Result<(), Box<dyn Error>> {
    let sys_path = std::path::Path::new("/sys/class/net").join(interface);
    if !sys_path.exists() {
        return Err(Box::new(MacError::ValidationFailed(
            format!("Interface {} does not exist", interface)
        )));
    }

    // Check if interface is operational
    if let Ok(operstate) = std::fs::read_to_string(sys_path.join("operstate")) {
        println!("Interface {} current state: {}", interface, operstate.trim());
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn verify_interface_exists(interface: &str) -> Result<(), Box<dyn Error>> {
    // On Windows, we'll use WMI to verify the interface
    let output = Command::new("wmic")
        .args(&["nic", "get", "name,netconnectionid", "/format:csv"])
        .output()?;

    let output_str = String::from_utf8_lossy(&output.stdout);
    if !output_str.to_lowercase().contains(&interface.to_lowercase()) {
        return Err(Box::new(MacError::ValidationFailed(
            format!("Interface {} does not exist", interface)
        )));
    }

    // Check if interface is enabled using netsh
    let status = Command::new("netsh")
        .args(&["interface", "show", "interface", interface])
        .output()?;

    let status_str = String::from_utf8_lossy(&status.stdout);
    println!("Interface {} current state: {}", interface, status_str.trim());

    Ok(())
}

#[cfg(target_os = "linux")]
fn check_permissions() -> Result<(), Box<dyn Error>> {
    if !nix::unistd::Uid::effective().is_root() {
        return Err(Box::new(MacError::PermissionDenied(
            "This program must be run with root privileges. Please use sudo.".into()
        )));
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn check_permissions() -> Result<(), Box<dyn Error>> {
    if !is_elevated::is_elevated() {
        return Err(Box::new(MacError::PermissionDenied(
            "This program must be run with Administrator privileges. Please use as admin.".into()
        )));
    }

    Ok(())
}

#[cfg(target_os = "linux")]
fn execute_command(cmd: &str, args: &[&str]) -> Result<(), Box<dyn Error>> {
    let output = Command::new("sudo")
        .arg(cmd)
        .args(args)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let error_msg = if !stderr.is_empty() {
            stderr.to_string()
        } else if !stdout.is_empty() {
            stdout.to_string()
        } else {
            "Unknown error".to_string()
        };

        return Err(Box::new(MacError::SystemError(error_msg)));
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn execute_command(cmd: &str, args: &[&str]) -> Result<(), Box<dyn Error>> {
    let output = Command::new(cmd)
        .args(args)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let error_msg = if !stderr.is_empty() {
            stderr.to_string()
        } else if !stdout.is_empty() {
            stdout.to_string()
        } else {
            "Unknown error".to_string()
        };

        return Err(Box::new(MacError::SystemError(error_msg)));
    }

    Ok(())
}

#[cfg(target_os = "linux")]
pub fn change_mac(interface: &str, mac: &str, permanent: bool) -> Result<(), Box<dyn Error>> {
    // Verify root privileges
    check_permissions()?;

    // Verify interface exists
    verify_interface_exists(interface)?;

    // Find ip command path
    let ip_cmd = find_command("ip").ok_or_else(||
        MacError::SystemError("'ip' command not found. Please install iproute2".into()))?;

    println!("Using command: {}", ip_cmd);
    println!("Bringing interface {} down...", interface);

    // Stop NetworkManager if it's running
    let _ = Command::new("sudo")
        .args(&["systemctl", "stop", "NetworkManager"])
        .output();

    // Try to bring interface down with retries
    let max_retries = 3;
    let mut success = false;
    let mut last_error = None;

    for attempt in 1..=max_retries {
        match execute_command(&ip_cmd, &["link", "set", "dev", interface, "down"]) {
            Ok(_) => {
                success = true;
                break;
            }
            Err(e) => {
                println!("Attempt {} failed, retrying...", attempt);
                std::thread::sleep(std::time::Duration::from_secs(1));
                last_error = Some(e);
            }
        }
    }

    if !success {
        return Err(last_error.unwrap());
    }

    println!("Changing MAC address to {}...", mac);

    // Change MAC address
    execute_command(&ip_cmd, &["link", "set", "dev", interface, "address", mac])?;

    println!("Bringing interface back up...");

    // Bring interface back up
    execute_command(&ip_cmd, &["link", "set", "dev", interface, "up"])?;

    // Restart NetworkManager if it was running
    let _ = Command::new("sudo")
        .args(&["systemctl", "start", "NetworkManager"])
        .output();

    if permanent {
        println!("Making change permanent...");
        make_permanent(interface, mac)?;
    }

    // Verify the change
    println!("Verifying MAC address change...");
    verify_mac_change(interface, mac)?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn find_network_adapter(interface: &str) -> Result<(RegKey, String), Box<dyn Error>> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let net_reg_path = "SYSTEM\\CurrentControlSet\\Control\\Class\\{4D36E972-E325-11CE-BFC1-08002BE10318}";
    let net_reg_key = hklm.open_subkey_with_flags(net_reg_path, KEY_READ | KEY_WRITE)?;

    // First get the exact adapter name from Windows
    let output = Command::new("wmic")
        .args(&["nic", "where", &format!("NetConnectionID='{}'", interface), "get", "Name,NetConnectionID", "/format:csv"])
        .output()?;

    let output_str = String::from_utf8_lossy(&output.stdout);
    let mut adapter_name = String::new();
    let mut found_adapter = false;

    for line in output_str.lines().skip(1) { // Skip header
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() >= 3 && parts[2].trim() == interface {
            adapter_name = parts[1].trim().to_string();
            found_adapter = true;
            break;
        }
    }

    if !found_adapter {
        return Err(Box::new(MacError::ValidationFailed(
            format!("Could not find adapter with name {}", interface)
        )));
    }

    // Now search through registry for this adapter
    for subkey_name in net_reg_key.enum_keys() {
        let subkey_name = subkey_name?;
        if let Ok(subkey) = net_reg_key.open_subkey_with_flags(&subkey_name, KEY_READ | KEY_WRITE) {
            if let Ok(driver_desc) = subkey.get_value::<String, &str>("DriverDesc") {
                if driver_desc.trim() == adapter_name {
                    return Ok((subkey, interface.to_string()));
                }
            }
        }
    }

    Err(Box::new(MacError::SystemError(
        format!("Could not find registry key for interface {}", interface)
    )))
}

#[cfg(target_os = "windows")]
pub fn change_mac(interface: &str, mac: &str, permanent: bool) -> Result<(), Box<dyn Error>> {
    // Verify admin privileges first
    check_permissions()?;

    // Get the network adapter's registry information
    let (adapter_key, adapter_name) = find_network_adapter(interface)?;

    println!("Found network adapter: {}", adapter_name);
    println!("Changing MAC address to {}...", mac);

    // Disable the network adapter
    println!("Disabling network adapter...");
    execute_command(
        "netsh",
        &["interface", "set", "interface", &adapter_name, "admin=disable"]
    )?;

    // Set the MAC address in registry
    let cleaned_mac = mac.replace(":", "").replace("-", "").replace(".", "");
    adapter_key.set_value("NetworkAddress", &cleaned_mac)?;

    // Enable the network adapter
    println!("Enabling network adapter...");
    execute_command(
        "netsh",
        &["interface", "set", "interface", &adapter_name, "admin=enable"]
    )?;

    // Wait for interface to come back up
    std::thread::sleep(std::time::Duration::from_secs(2));

    // Verify the change
    println!("Verifying MAC address change...");
    verify_mac_change(&adapter_name, mac)?;

    Ok(())
}

#[cfg(target_os = "linux")]
fn verify_mac_change(interface: &str, expected_mac: &str) -> Result<(), Box<dyn Error>> {
    // Wait a bit for the change to take effect
    std::thread::sleep(std::time::Duration::from_secs(1));

    let current_mac = crate::network::get_current_mac(interface)?;
    if current_mac.to_lowercase() != expected_mac.to_lowercase() {
        return Err(Box::new(MacError::ValidationFailed(
            format!("MAC address change verification failed. Expected {}, got {}",
                    expected_mac, current_mac)
        )));
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn verify_mac_change(interface: &str, expected_mac: &str) -> Result<(), Box<dyn Error>> {
    // Wait a bit for the change to take effect
    std::thread::sleep(std::time::Duration::from_secs(1));

    let current_mac = crate::network::get_current_mac(interface)?;

    // Convert both MACs to the same format (hyphen-separated) for comparison
    let expected_mac = expected_mac
        .replace(":", "-")
        .replace(".", "-")
        .to_lowercase();

    let current_mac = current_mac
        .replace(":", "-")
        .replace(".", "-")
        .to_lowercase();

    if current_mac != expected_mac {
        return Err(Box::new(MacError::ValidationFailed(
            format!("MAC address change verification failed. Expected {}, got {}",
                    expected_mac, current_mac)
        )));
    }

    Ok(())
}

#[cfg(target_os = "linux")]
fn make_permanent(interface: &str, mac: &str) -> Result<(), Box<dyn Error>> {
    use std::fs;
    use std::path::Path;

    // Create udev rule
    let rule = format!(
        r#"ACTION=="add", SUBSYSTEM=="net", ATTR{{address}}=="*", ATTR{{dev_id}}=="0x0", ATTR{{type}}=="1", KERNEL=="{}", ATTR{{address}}="{}"
"#,
        interface, mac
    );

    let rule_path = Path::new("/etc/udev/rules.d/70-persistent-net.rules");

    // Check if we can write to the directory
    if !Path::new("/etc/udev/rules.d").exists() {
        return Err(Box::new(MacError::SystemError(
            "Directory /etc/udev/rules.d does not exist".into()
        )));
    }

    fs::write(rule_path, rule)
        .map_err(|e| MacError::SystemError(format!("Failed to write udev rule: {}", e)))?;

    // Reload udev rules
    Command::new("udevadm")
        .args(&["control", "--reload-rules"])
        .output()
        .map_err(|e| MacError::SystemError(format!("Failed to reload udev rules: {}", e)))?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn make_permanent(_interface: &str, _mac: &str) -> Result<(), Box<dyn Error>> {
    // On Windows, the registry change made in change_mac() is already permanent
    Ok(())
}

pub fn get_running_applications() -> Result<Vec<String>, Box<dyn Error>> {
    let mut apps = Vec::new();

    #[cfg(target_os = "linux")]
    {
        // Get running processes from /proc
        for entry in fs::read_dir("/proc")? {
            let entry = entry?;
            if let Ok(name) = fs::read_to_string(entry.path().join("comm")) {
                apps.push(name.trim().to_string());
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("ps")
            .args(&["-e", "-o", "comm="])
            .output()?;

        let output_str = String::from_utf8_lossy(&output.stdout);
        apps.extend(output_str.lines().map(|s| s.trim().to_string()));
    }

    #[cfg(target_os = "windows")]
    {
        let output = std::process::Command::new("tasklist")
            .args(&["/FO", "CSV", "/NH"])
            .output()?;

        let output_str = String::from_utf8_lossy(&output.stdout);
        for line in output_str.lines() {
            if let Some(name) = line.split(',').next() {
                apps.push(name.trim_matches('"').to_string());
            }
        }
    }

    Ok(apps)
}
