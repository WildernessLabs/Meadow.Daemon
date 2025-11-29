# Installing mc-daemon via APT

## Prerequisites

- Debian-based Linux distribution (Debian, Ubuntu, Raspberry Pi OS, etc.)
- sudo privileges
- Internet connection

## Installation Steps

### 1. Add the APT Repository

```bash
echo "deb [trusted=yes] https://wildernesslabs.github.io/Meadow.Daemon/apt stable main" | sudo tee /etc/apt/sources.list.d/meadow-daemon.list
```

**Note**: We use `[trusted=yes]` because packages are not GPG-signed. This is safe when downloading from the official GitHub Pages repository over HTTPS.

### 2. Update Package Lists

```bash
sudo apt update
```

### 3. Install mc-daemon

```bash
sudo apt install mc-daemon
```

The installation will:
- Install the binary to `/usr/bin/mc-daemon`
- Create configuration file at `/etc/meadow.conf`
- Generate SSH keys at `/root/.ssh/id_rsa` (if not exists)
- Create required directories (`/opt/meadow`, `/tmp/meadow/*`)
- Enable and start the `mc-daemon.service` systemd service

### 4. Verify Installation

Check service status:
```bash
systemctl status mc-daemon
```

View logs:
```bash
journalctl -u mc-daemon -f
```

Test REST API:
```bash
curl http://127.0.0.1:5000/api/info
```

## Configuration

### Default Configuration

The default configuration file is created at `/etc/meadow.conf` with these settings:

- **enabled**: yes
- **enable_mqtt_listener**: yes
- **meadow_root**: /opt/meadow
- **meadow_temp**: /tmp/meadow
- **rest_api_bind_address**: 127.0.0.1 (localhost only)
- **update_server_address**: tcp://mqtt.meadowcloud.co
- **update_server_port**: 1883
- **use_authentication**: yes

### Customizing Configuration

1. Edit the configuration file:
   ```bash
   sudo nano /etc/meadow.conf
   ```

2. Make desired changes (see Source/mc-daemon/meadow.conf.example for all options)

3. Restart the service:
   ```bash
   sudo systemctl restart mc-daemon
   ```

### Meadow.Cloud Integration

To use with Meadow.Cloud:

1. Ensure SSH keys exist: `/root/.ssh/id_rsa` and `/root/.ssh/id_rsa.pub`
2. Register the public key with Meadow.Cloud
3. The daemon will automatically authenticate and connect

## Updating mc-daemon

When a new version is released:

```bash
sudo apt update
sudo apt upgrade mc-daemon
```

The service will automatically restart with the new version.

## Uninstallation

### Remove Package (Keep Configuration)

```bash
sudo apt remove mc-daemon
```

### Complete Removal (Purge)

```bash
sudo apt purge mc-daemon
```

This will remove:
- The binary (`/usr/bin/mc-daemon`)
- Configuration file (`/etc/meadow.conf`)
- Temporary directories (`/tmp/meadow`)
- Systemd service file

**Note**: Application data in `/opt/meadow` and SSH keys in `/root/.ssh/` are preserved.

## Troubleshooting

### Service Won't Start

1. Check logs:
   ```bash
   journalctl -u mc-daemon -n 50
   ```

2. Verify configuration:
   ```bash
   sudo cat /etc/meadow.conf
   ```

3. Check if required directories exist:
   ```bash
   ls -la /opt/meadow /tmp/meadow
   ```

### Port 5000 Already in Use

Another service may be using port 5000. Check what's using it:
```bash
sudo lsof -i :5000
```

The REST API port is currently hardcoded to 5000 in the daemon.

### SSH Key Issues

Generate new keys manually:
```bash
sudo ssh-keygen -t rsa -b 4096 -N "" -f /root/.ssh/id_rsa
```

Verify keys are in PEM format:
```bash
head -1 /root/.ssh/id_rsa
```

Should show: `-----BEGIN RSA PRIVATE KEY-----`

### Permission Issues

Ensure the daemon has access to required directories:
```bash
sudo chown -R root:root /opt/meadow /tmp/meadow
sudo chmod 755 /opt/meadow /tmp/meadow
```

### Network Connectivity

Test MQTT connectivity:
```bash
telnet mqtt.meadowcloud.co 1883
```

Check firewall rules (if applicable):
```bash
sudo ufw status
```

### Configuration Syntax Errors

The config file uses simple `key value` format (space-separated). Common errors:

- ❌ `enabled=yes` (no equals sign)
- ✅ `enabled yes`

- ❌ `port: 1883` (no colon)
- ✅ `update_server_port 1883`

## Advanced Configuration

### Remote Access (Development Only)

**Warning**: Only do this on trusted networks.

1. Edit `/etc/meadow.conf`:
   ```
   rest_api_bind_address 0.0.0.0
   ```

2. Restart service:
   ```bash
   sudo systemctl restart mc-daemon
   ```

3. Allow through firewall (if enabled):
   ```bash
   sudo ufw allow 5000/tcp
   ```

### Managing Applications as Systemd Services

If your application is a systemd service that the daemon should manage:

1. Edit `/etc/meadow.conf`:
   ```
   app_is_systemd_service yes
   app_service_name your-app.service
   ```

2. Restart daemon:
   ```bash
   sudo systemctl restart mc-daemon
   ```

**Note**: The daemon must have permission to control the service (may require polkit configuration).

## REST API Endpoints

The daemon exposes these endpoints on `http://127.0.0.1:5000`:

- `GET /api/info` - Daemon and device information
- `GET /api/updates` - List available updates
- `PUT /api/updates/{id}` - Download or apply update
- `DELETE /api/updates` - Clear all updates
- `GET /api/files` - List files in meadow_root

## Supported Architectures

- **AMD64** (x86_64) - Desktop/server systems
- **ARM64** (aarch64) - Raspberry Pi 4/5, ARM servers

## Support

- GitHub Issues: https://github.com/WildernessLabs/Meadow.Daemon/issues
- Documentation: https://github.com/WildernessLabs/Meadow.Daemon
- Example Config: Source/mc-daemon/meadow.conf.example
