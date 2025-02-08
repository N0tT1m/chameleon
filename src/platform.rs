use std::error::Error;
use std::process::Command;
use crate::error::MacError;

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

#[cfg(target_os = "linux")]
fn check_permissions() -> Result<(), Box<dyn Error>> {
    if !nix::unistd::Uid::effective().is_root() {
        return Err(Box::new(MacError::PermissionDenied(
            "This program must be run with root privileges. Please use sudo.".into()
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