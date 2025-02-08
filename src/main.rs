// File: src/main.rs
mod error;
mod mac;
mod network;
mod platform;
mod config;
mod geolocation;
mod filter;
mod logger;
use crate::geolocation::GeoLocationService;
use crate::filter::MacFilter;
use crate::logger::{MacLogger, MacChange};

use clap::{Parser, ArgGroup};
use error::MacError;
use mac::{MacAddress, MacFormat};
use network::NetworkCard;
use platform::change_mac;
use config::{save_original_mac, get_original_mac};
use std::error::Error;
use chrono::Utc;

#[derive(Parser, Debug)]
#[command(
    name = "Chameleon",
    about = "A tool to change MAC addresses across different platforms",
    version = "1.0.0",
    author = "Nathan Moritz <nathan.moritz@duocore.dev>",
    long_about = None
)]
#[command(group(
    ArgGroup::new("mac_source")
        .args(["random", "mac", "restore"])
        .required(true)
))]
struct Cli {
    /// Network interface to modify
    #[arg(
        short = 'i',
        long = "interface",
        required = true,
        help = "Network interface (e.g., eth0, wlan0)"
    )]
    interface: String,

    /// Generate a random MAC address
    #[arg(
        short = 'r',
        long = "random",
        help = "Generate a random MAC address",
        conflicts_with_all = ["mac", "restore"]
    )]
    random: bool,

    /// Set a specific MAC address
    #[arg(
        short = 'm',
        long = "mac",
        value_name = "MAC",
        help = "Set a specific MAC address (format: XX:XX:XX:XX:XX:XX)",
        conflicts_with_all = ["random", "restore"]
    )]
    mac: Option<String>,

    /// Make MAC change permanent
    #[arg(
        short = 'p',
        long = "permanent",
        help = "Make the MAC address change permanent",
        conflicts_with = "restore"
    )]
    permanent: bool,

    /// Use a specific vendor prefix
    #[arg(
        short = 'v',
        long = "vendor",
        value_name = "VENDOR",
        help = "Use a specific vendor prefix (first 3 bytes, e.g., 00:11:22)",
        requires = "random",
        conflicts_with_all = ["mac", "restore"]
    )]
    vendor: Option<String>,

    /// Restore original MAC
    #[arg(
        short = 'o',
        long = "restore",
        help = "Restore the original MAC address",
        conflicts_with_all = ["random", "mac", "permanent", "vendor"]
    )]
    restore: bool,

    /// Spoof location to specific country
    #[arg(long, value_name = "COUNTRY")]
    spoof_location: Option<String>,

    /// Add MAC prefix to whitelist
    #[arg(long, value_name = "PREFIX")]
    whitelist: Option<String>,

    /// Add MAC prefix to blacklist
    #[arg(long, value_name = "PREFIX")]
    blacklist: Option<String>,

    /// Show MAC change history
    #[arg(long)]
    history: bool,
}

impl Cli {
    fn validate(&self) -> Result<(), MacError> {
        // Validate interface
        if self.interface.is_empty() {
            return Err(MacError::ValidationFailed("Interface name cannot be empty".into()));
        }

        // Validate MAC if provided
        if let Some(mac) = &self.mac {
            if !is_valid_mac_format(mac) {
                return Err(MacError::InvalidFormat(
                    "Invalid MAC address format. Use XX:XX:XX:XX:XX:XX".into()
                ));
            }
        }

        // Validate vendor if provided
        if let Some(vendor) = &self.vendor {
            if !is_valid_vendor_format(vendor) {
                return Err(MacError::InvalidFormat(
                    "Invalid vendor prefix format. Use XX:XX:XX".into()
                ));
            }
        }

        Ok(())
    }
}

fn is_valid_mac_format(mac: &str) -> bool {
    let re = regex::Regex::new(r"^([0-9A-Fa-f]{2}[:-]){5}([0-9A-Fa-f]{2})$").unwrap();
    re.is_match(mac)
}

fn is_valid_vendor_format(vendor: &str) -> bool {
    let re = regex::Regex::new(r"^([0-9A-Fa-f]{2}[:-]){2}([0-9A-Fa-f]{2})$").unwrap();
    re.is_match(vendor)
}

fn check_privileges() -> Result<(), MacError> {
    #[cfg(unix)]
    {
        if !nix::unistd::Uid::effective().is_root() {
            return Err(MacError::PermissionDenied("Must run with root privileges".into()));
        }
    }

    #[cfg(windows)]
    {
        if !is_elevated::is_elevated() {
            return Err(MacError::PermissionDenied("Must run with administrator privileges".into()));
        }
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    // Validate arguments
    cli.validate()?;

    // Check privileges
    check_privileges()?;

    // Initialize services
    let mut geo_service = GeoLocationService::new();
    let mut mac_filter = MacFilter::new();
    let mac_logger = MacLogger::new();

    // Verify interface
    let card = NetworkCard::verify_interface(&cli.interface)?;
    println!("Detected network card: {:?}", card);

    if cli.restore {
        if let Some(original_mac) = get_original_mac(&cli.interface)? {
            println!("Restoring original MAC address: {}", original_mac);
            change_mac(&cli.interface, &original_mac, false)?;
            println!("Successfully restored original MAC address");
        } else {
            return Err(Box::new(MacError::ValidationFailed("No original MAC address saved".into())));
        }
        return Ok(());
    }

    // Get new MAC address
    let new_mac = if cli.random {
        println!("Generating random MAC address{}...",
                 if cli.vendor.is_some() { " with vendor prefix" } else { "" });
        mac::generate_random_mac(cli.vendor.as_deref())?.to_string()
    } else if let Some(mac) = cli.mac {
        mac
    } else {
        return Err(Box::new(MacError::ValidationFailed("No MAC address specified".into())));
    };

    // Save original MAC if first time
    if get_original_mac(&cli.interface)?.is_none() {
        match network::get_current_mac(&cli.interface) {
            Ok(current_mac) => {
                println!("Saving original MAC address: {}", current_mac);
                save_original_mac(&cli.interface, &current_mac)?;
            },
            Err(e) => {
                println!("Warning: Could not save original MAC address: {}", e);
            }
        }
    }

    // Check for platform compatibility with permanent changes
    #[cfg(target_os = "macos")]
    if cli.permanent {
        println!("Warning: Permanent MAC address changes are not supported on macOS.");
        println!("Continuing with temporary change...");
        // Force permanent to false on macOS
        cli.permanent = false;
    }

    // Save original MAC if first time (only for non-restore operations)
    if !cli.restore && get_original_mac(&cli.interface)?.is_none() {
        match network::get_current_mac(&cli.interface) {
            Ok(current_mac) => {
                println!("Saving original MAC address: {}", current_mac);
                save_original_mac(&cli.interface, &current_mac)?;
            },
            Err(e) => {
                println!("Warning: Could not save original MAC address: {}", e);
            }
        }
    }

    // Change MAC
    println!("Changing MAC address to {} for interface {}", new_mac, cli.interface);

    #[cfg(not(target_os = "macos"))]
    let permanent = cli.permanent;

    #[cfg(target_os = "macos")]
    let permanent = false;

    change_mac(&cli.interface, &new_mac, permanent)?;

    // Print success message
    if cli.restore {
        println!("Successfully restored original MAC address");
    } else {
        println!("Successfully changed MAC address");
        if cli.permanent {
            println!("Changes have been made permanent");
        } else {
            println!("Note: This change is temporary. Use -p to make it permanent");
        }
    }

    // Handle filter commands
    if let Some(prefix) = cli.whitelist {
        mac_filter.add_to_whitelist(&prefix)?;
        println!("Added {} to whitelist", prefix);
        return Ok(());
    }

    if let Some(prefix) = cli.blacklist {
        mac_filter.add_to_blacklist(&prefix)?;
        println!("Added {} to blacklist", prefix);
        return Ok(());
    }

    if cli.history {
        let history = mac_logger.get_history()?;
        for change in history {
            println!("{}: {} -> {} ({})",
                     change.timestamp,
                     change.old_mac,
                     change.new_mac,
                     change.interface
            );
        }
        return Ok(());
    }

    // Get new MAC address
    let new_mac = if cli.random {
        if let Some(country) = &cli.spoof_location {
            if let Some(mac) = geo_service.suggest_mac_for_location(country) {
                mac
            } else {
                return Err(Box::new(MacError::ValidationFailed(
                    format!("No vendor found for country {}", country)
                )));
            }
        } else {
            mac::generate_random_mac(cli.vendor.as_deref())?.to_string()
        }
    } else if let Some(mac) = cli.mac {
        mac
    } else {
        return Err(Box::new(MacError::ValidationFailed("No MAC address specified".into())));
    };

    // Check against filters
    if !mac_filter.is_allowed(&new_mac) {
        return Err(Box::new(MacError::ValidationFailed("MAC address not allowed by filters".into())));
    }

    // Get current MAC for logging
    let old_mac = network::get_current_mac(&cli.interface)?;

    // Change MAC
    change_mac(&cli.interface, &new_mac, cli.permanent)?;

    // Log the change
    let change = MacChange {
        timestamp: Utc::now(),
        interface: cli.interface,
        old_mac,
        new_mac,
        geo_location: cli.spoof_location,
        permanent: cli.permanent,
    };
    mac_logger.log_change(change)?;

    Ok(())
}