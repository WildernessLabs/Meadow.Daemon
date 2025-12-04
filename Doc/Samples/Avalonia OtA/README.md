# Meadow.Cloud OTA Update Demonstration

Complete end-to-end guide for demonstrating Over-The-Air (OTA) updates of an Avalonia GUI application running in WSL2 (Debian) using Meadow.Cloud.

## Overview

This demonstration shows how to:
1. Install and configure the Meadow Daemon on a Linux device
2. Provision the device with Meadow.Cloud
3. Deploy an Avalonia .NET application as a systemd service
4. Perform OTA updates via Meadow.Cloud

### Architecture

```
┌───────────────────────────────────────────────────────────┐
│                      WSL2 (Debian)                        │
│                                                           │
│  ┌──────────────────┐         ┌────────────────────────┐  │
│  │  mc-daemon       │         │  Your Avalonia App     │  │
│  │  (systemd)       │◄────────┤  (systemd service)     │  │
│  │                  │ manages │                        │  │
│  │  - File Moves    │         │  - Meadow.Core         │  │
│  │  - App Restart   │         │  - MQTT Client         │  │
│  │                  │         │  - Update Download     │  │
│  └──────────────────┘         │  - GUI Display         │  │
│                               └─────────┬──────────────┘  │
│                                         │                 │
└─────────────────────────────────────────┼─────────────────┘
                                          │
                                          │ MQTT over Internet
                                          ▼
                                  ┌───────────────────┐
                                  │  Meadow.Cloud     │
                                  │  - Authentication │
                                  │  - OTA Packages   │
                                  │  - Device Mgmt    │
                                  └───────────────────┘
```

## Prerequisites

### Required

- **WSL2** with Debian (tested on Debian Trixie)
- **Basic .NET knowledge** - Creating and building .NET applications
- **Meadow.Cloud account** - Sign up at [meadowcloud.co](https://www.meadowcloud.co)
- **Windows machine** with browser (for Meadow.Cloud provisioning)

### Recommended Knowledge

- Linux command line basics
- systemd service management
- X11 display basics (for GUI apps)

## Quick Start

Follow these steps in order:

### 1. [Install Meadow Daemon](01-install-daemon.md)

Build or obtain the `mc-daemon` binary, create systemd service, and configure the daemon.

**Time**: ~15 minutes

### 2. [Install .NET Runtime](02-install-dotnet.md)

Install .NET 8 SDK and Meadow CLI tools.

**Time**: ~10 minutes

### 3. [Provision Device](03-provision-device.md)

Generate SSH keys and register the device with Meadow.Cloud.

**Time**: ~10 minutes
**Note**: Requires access to Windows machine with browser

### 4. [Setup Your Application](04-setup-app.md)

Configure your Avalonia app, deploy it, and set up systemd service for GUI display.

**Time**: ~20 minutes

### 5. [Perform OTA Update](05-perform-update.md)

Create an updated version, package it, publish via Meadow.Cloud, and watch it update!

**Time**: ~15 minutes

### 6. [Troubleshooting](06-troubleshooting.md)

Reference guide for common issues and solutions.

## Expected Outcome

After completing all steps, you will have:

✅ Meadow Daemon running as a systemd service
✅ Device provisioned and connected to Meadow.Cloud
✅ Avalonia app running with GUI in WSL2
✅ App configured as a systemd service
✅ Successful OTA update from version 1.0 → 2.0
✅ Understanding of the complete OTA workflow

## What You'll Learn

- How to run .NET GUI apps in WSL2 with systemd
- Meadow.Cloud device provisioning process
- OTA update workflow and MPAK packaging
- systemd service configuration for GUI applications
- Troubleshooting common WSL2 + .NET + systemd issues

## Key Concepts

### MPAK (Meadow Package)

A simple ZIP file containing your application binaries inside a root `app` folder. Example structure:

```
MyApp.mpak (ZIP file)
└── app
    ├── MyApp.dll
    ├── MyApp.deps.json
    ├── MyApp.runtimeconfig.json
    ├── app.config.yaml
    └── [other DLLs and dependencies]
```

### OTA Update Flow

1. **Publish** - Upload MPAK to Meadow.Cloud
2. **Notify** - Cloud sends MQTT message to app (Meadow.Core MQTT client)
3. **Download** - App's UpdateService downloads MPAK
4. **Extract** - App's UpdateService extracts MPAK to staging directory
5. **Exit** - App signals update ready and exits
6. **File Moves** - Daemon detects exit, moves files from staging to `/opt/meadow`
7. **Restart** - Daemon restarts app via `systemctl start`
8. **Verify** - New version runs with updated code

### systemd Service Management

Both the daemon and your app run as systemd services:
- **mc-daemon**: Runs as root, waits for app exit, moves files, restarts app
- **your-app**: Runs as user, handles MQTT/downloads, displays GUI
- Daemon configured with `enable_mqtt_listener no` (app handles MQTT)

## Directory Structure

After setup, your system will look like this:

```
/etc/
├── meadow.conf                    # Daemon configuration
└── systemd/system/
    ├── mc-daemon.service          # Daemon service
    └── meadowapp.service          # Your app service

/opt/meadow/                       # App deployment directory
├── MyAvaloniaApp.dll
├── app.config.yaml
└── [dependencies]

/home/USER/.meadow.tmp             # Daemon working directory
├── updates/                       # Downloaded MPAKs
├── update/                        # Extraction staging
├── staging/                       # Pre-deployment staging
└── rollback/                      # Backup of previous version

/root/.ssh/
├── id_rsa                         # Device private key
└── id_rsa.pub                     # Device public key (registered with Cloud)

/home/USER/.ssh/
├── id_rsa                         # Device private key
└── id_rsa.pub                     # Device public key (registered with Cloud)
```

## Important Notes

### Display Access for GUI Apps

GUI apps in WSL2 require special configuration:
- `DISPLAY=:0` environment variable
- Access to `.Xauthority` file
- Proper `XDG_RUNTIME_DIR` setting

All covered in [04-setup-app.md](04-setup-app.md)

### Permission Considerations

- **mc-daemon** runs as `root` (needs systemctl access)
- **Your app** can run as user (better security, easier display access)
- **Temp directories** must be writable by both

See [Troubleshooting: Permission Issues](06-troubleshooting.md#permission-issues)

### Environment Variable Precedence

Configuration values are loaded in this order (highest to lowest priority):

1. **Environment variables** in systemd service
2. **Config file** (`/etc/meadow.conf` or `app.config.yaml`)
3. **Hardcoded defaults** in code

⚠️ **Common mistake**: Setting `MEADOW_TEMP` in systemd service overrides config file!

## Support & Resources

- **Meadow.Daemon Repository**: [github.com/WildernessLabs/Meadow.Daemon](https://github.com/WildernessLabs/Meadow.Daemon)
- **Meadow.Cloud Documentation**: [developer.wildernesslabs.co](https://developer.wildernesslabs.co)
- **Troubleshooting Guide**: [06-troubleshooting.md](06-troubleshooting.md)

## Quick Reference

### Check Service Status
```bash
systemctl status mc-daemon
systemctl status meadowapp
```

### View Logs
```bash
journalctl -u mc-daemon -f
journalctl -u meadowapp -f
```

### Test Daemon API
```bash
curl http://127.0.0.1:5000/api/info
curl http://127.0.0.1:5000/api/updates
```

### Restart Services
```bash
sudo systemctl restart mc-daemon
sudo systemctl restart meadowapp
```

## Next Steps

Ready to begin? Start with [01-install-daemon.md](01-install-daemon.md)!
