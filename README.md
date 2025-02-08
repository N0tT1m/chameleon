# Chameleon

A powerful cross-platform MAC address changer with advanced features for network interface management.

## Features

- **Cross-Platform Support**: Works on Linux, Windows, and macOS
- **Multiple MAC Address Sources**:
    - Generate random MAC addresses
    - Set specific MAC addresses
    - Use vendor-specific prefixes
    - Restore original MAC addresses
- **Advanced Features**:
    - Geolocation-based MAC spoofing
    - MAC address whitelisting and blacklisting
    - Application-specific MAC rules
    - Scheduled MAC address changes
    - MAC change history logging
    - Permanent MAC address changes (where supported)

## Prerequisites

- Rust toolchain (rustc, cargo)
- Administrative/root privileges
- Platform-specific requirements:
    - Linux: iproute2
    - Windows: Administrator access
    - macOS: Root access (note: permanent changes not supported)

## Installation

```bash
# Clone the repository
git clone https://github.com/username/chameleon.git
cd chameleon

# Build the project
cargo build --release

# Move the binary to a suitable location
sudo mv target/release/chameleon /usr/local/bin/
```

## Usage

Basic command structure:
```bash
chameleon [OPTIONS] -i <interface> (-r | -m <MAC> | --restore)
```

### Common Operations

1. Generate a random MAC address:
```bash
sudo chameleon -i eth0 -r
```

2. Set a specific MAC address:
```bash
sudo chameleon -i wlan0 -m 00:11:22:33:44:55
```

3. Restore original MAC address:
```bash
sudo chameleon -i eth0 --restore
```

4. Make changes permanent (not available on macOS):
```bash
sudo chameleon -i eth0 -r -p
```

### Advanced Features

1. Use vendor-specific prefix:
```bash
sudo chameleon -i eth0 -r -v 00:11:22
```

2. Spoof location to specific country:
```bash
sudo chameleon -i wlan0 -r --spoof-location US
```

3. Add MAC prefix to whitelist:
```bash
sudo chameleon -i eth0 --whitelist 00:11:22
```

4. View MAC change history:
```bash
chameleon --history
```

### Application Rules

1. Add an application-specific MAC rule:
```bash
sudo chameleon -i eth0 --add-rule --app-name "MyApp" --mac 00:11:22:33:44:55 --schedule "mon,tue,wed:09:00-17:00"
```

2. List all rules:
```bash
chameleon --list-rules
```

3. Remove a rule:
```bash
chameleon --remove-rule --app-name "MyApp"
```

## Configuration

Chameleon stores its configuration in the following locations:

- Config directory: `~/.config/mac_changer/` (Unix) or `%APPDATA%\mac_changer\` (Windows)
- Log directory: `~/.local/share/mac_changer/logs/` (Unix) or `%LOCALAPPDATA%\mac_changer\logs\` (Windows)

Configuration files:
- `filters.json`: MAC address whitelist/blacklist
- `app_rules.json`: Application-specific MAC rules
- `{interface}.json`: Original MAC address backup

## Security Considerations

- Always run with appropriate privileges (root/administrator)
- Be cautious when using MAC addresses from specific vendors
- Consider network policies and restrictions
- Keep logs secure as they contain network configuration history

## Error Handling

Chameleon provides detailed error messages for common issues:
- Invalid MAC address format
- Interface not found
- Insufficient privileges
- Unsupported platform features
- Network card compatibility issues

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Author

Nathan Moritz <nathan.moritz@duocore.dev>

## Acknowledgments

- Thanks to the Rust community for excellent networking crates
- Contributors and testers across different platforms