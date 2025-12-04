# Step 1: Install Meadow Daemon

Install and configure the Meadow Daemon (`mc-daemon`) as a systemd service.

## What is mc-daemon?

The Meadow Daemon is a Rust-based service that manages application updates for .NET apps using Meadow.Core:
- Waits for your application process to exit after it downloads an update
- Moves updated files from staging to the deployment directory
- Restarts your application as a systemd service
- Provides a REST API for status and control (port 5000)

**Note**: When using Meadow.Core in your .NET app, the app itself handles the MQTT connection and update downloads. The daemon's role is limited to file management and app restart.

## Prerequisites

- WSL2 with Debian installed
- sudo privileges
- Internet connection

## Installation Methods

Choose one:
- **Method A**: Build from source (recommended if you have Rust installed)
- **Method B**: Use pre-built binary from GitHub Releases

## Method A: Build from Source

### 1. Install Rust

If you don't have Rust installed:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

**Verify installation**:
```bash
rustc --version
```

### 2. Clone and Build

```bash
cd ~
git clone https://github.com/WildernessLabs/Meadow.Daemon.git
cd Meadow.Daemon/Source/mc-daemon
cargo build --release
```

**Expected output**: Build completes successfully (may take 5-10 minutes first time)

### 3. Install Binary

```bash
sudo cp target/release/mc-daemon /usr/bin/
sudo chmod 755 /usr/bin/mc-daemon
```

**Verify**:
```bash
/usr/bin/mc-daemon --version  # Should show version info
```

## Method B: Use Pre-built Binary

### 1. Download from GitHub Releases

```bash
# Download latest release (replace VERSION with actual version like v0.9.0)
wget https://github.com/WildernessLabs/Meadow.Daemon/releases/download/VERSION/mc-daemon-amd64

# Make executable and move to /usr/bin
chmod +x mc-daemon-amd64
sudo mv mc-daemon-amd64 /usr/bin/mc-daemon
```

**Verify**:
```bash
/usr/bin/mc-daemon --version
```

## Configure systemd Service

### 1. Create Service File

Create `/etc/systemd/system/mc-daemon.service`:

```bash
sudo nano /etc/systemd/system/mc-daemon.service
```

Paste this content (see also [mc-daemon.service.example](mc-daemon.service.example)):

```ini
[Unit]
Description=Meadow Daemon - OTA Update and Cloud Connectivity Service
Documentation=https://github.com/WildernessLabs/Meadow.Daemon
After=network-online.target
Wants=network-online.target
ConditionPathExists=/etc/meadow.conf

[Service]
Type=simple
User=root
Group=root
ExecStart=/usr/bin/mc-daemon
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal
SyslogIdentifier=mc-daemon

# Security hardening
NoNewPrivileges=true
PrivateTmp=false
ProtectSystem=full
ProtectHome=false

# Resource limits
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target
```

**Important**: Do NOT set `Environment=` variables in this file! Let `/etc/meadow.conf` control all paths.

Save and exit (Ctrl+O, Enter, Ctrl+X in nano).

### 2. Create Configuration File

Create `/etc/meadow.conf`:

```bash
sudo nano /etc/meadow.conf
```

Paste this minimal configuration (full template at [meadow.conf.example](meadow.conf.example)):

```
# Meadow.Daemon Configuration

# Enable cloud connection
enabled yes

# Disable MQTT listener - app (Meadow.Core) handles MQTT connection
enable_mqtt_listener no

# Application directory (where updates are applied)
meadow_root /opt/meadow

# Temporary directory for downloads and extraction
meadow_temp /home/YOUR_USERNAME/.meadow/tmp

# REST API (localhost only for security)
rest_api_bind_address 127.0.0.1

# Meadow.Cloud MQTT broker
update_server_address tcp://mqtt.meadowcloud.co
update_server_port 1883

# Authentication
use_authentication yes
auth_server_address https://www.meadowcloud.co
auth_server_port 443

# MQTT topic for updates ({OID} = org ID, {ID} = machine ID)
mqtt_topics {OID}/ota/{ID}

# Application is managed by systemd
app_is_systemd_service yes
app_service_name meadowapp.service
```

⚠️ **Replace `YOUR_USERNAME`** with your actual Linux username (run `whoami` to find it).

**Why use `~/.meadow/tmp` instead of `/tmp/meadow`?**
- Avoids permission conflicts
- Your app (running as user) can write to it
- Daemon (running as root) can also access it

Save and exit.

### 3. Create Required Directories

```bash
# Create app deployment directory
sudo mkdir -p /opt/meadow
sudo chown $USER:$USER /opt/meadow

# Create daemon working directory
mkdir -p ~/.meadow/tmp/{updates,update,staging,rollback}
```

**Verify**:
```bash
ls -la /opt/meadow
ls -la ~/.meadow/tmp
```

## Start the Daemon

### 1. Reload systemd

```bash
sudo systemctl daemon-reload
```

### 2. Enable Service (start on boot)

```bash
sudo systemctl enable mc-daemon
```

**Expected output**:
```
Created symlink /etc/systemd/system/multi-user.target.wants/mc-daemon.service → /etc/systemd/system/mc-daemon.service.
```

### 3. Start Service

```bash
sudo systemctl start mc-daemon
```

## Verify Installation

### Check Service Status

```bash
systemctl status mc-daemon
```

**Expected output** (should show "active (running)"):
```
● mc-daemon.service - Meadow Daemon - OTA Update and Cloud Connectivity Service
     Loaded: loaded (/etc/systemd/system/mc-daemon.service; enabled)
     Active: active (running) since Tue 2025-12-03 10:00:00 PST
   Main PID: 1234 (mc-daemon)
      Tasks: 5
     Memory: 12.0M
        CPU: 100ms
     CGroup: /system.slice/mc-daemon.service
             └─1234 /usr/bin/mc-daemon
```

### Check Logs

```bash
journalctl -u mc-daemon -n 20
```

**Look for**:
- "Starting Meadow Daemon"
- Configuration loaded from `/etc/meadow.conf`
- "Using MEADOW_TEMP from config: /home/username/.meadow/tmp"
- REST server starting on port 5000

### Test REST API

```bash
curl http://127.0.0.1:5000/api/info
```

**Expected output** (JSON with daemon info):
```json
{
  "machine_id": "1c8150f752614dec80f88752256e829f",
  "status": "Idle",
  "version": "0.9.0"
}
```

## Troubleshooting

### Service Won't Start

```bash
# Check logs for errors
journalctl -u mc-daemon -n 50

# Common issues:
# - /etc/meadow.conf missing or has syntax errors
# - Binary not executable (chmod 755 /usr/bin/mc-daemon)
# - Port 5000 already in use
```

### Can't Connect to Meadow.Cloud

You'll see this in logs if device isn't provisioned yet - that's normal!
Provisioning is covered in [Step 3](03-provision-device.md).

### Permission Denied on Directories

```bash
# Ensure your user owns the directories
sudo chown -R $USER:$USER ~/.meadow
sudo chown -R $USER:$USER /opt/meadow
```

### Port 5000 Already in Use

```bash
# Find what's using port 5000
sudo lsof -i :5000

# Change port in source code (currently hardcoded)
# Or kill the conflicting process
```

For more issues, see [Troubleshooting Guide](06-troubleshooting.md).

## Next Steps

Daemon is installed and running!

Next: [Install .NET Runtime](02-install-dotnet.md)
