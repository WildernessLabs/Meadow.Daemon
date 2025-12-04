# Step 4: Setup Your Avalonia Application

Configure and deploy your Avalonia .NET application to run as a systemd service with OTA update support.

## What We're Building

An Avalonia GUI application that:
- Runs as a systemd service (auto-starts on boot, can be managed by mc-daemon)
- Uses Meadow.Core to connect to Meadow.Cloud via MQTT
- Downloads and stages OTA updates via built-in UpdateService
- Displays GUI in WSL2 (requires X11 display access)
- Exits gracefully when update is ready, allowing daemon to apply it

## Prerequisites

- .NET 8 SDK installed ([Step 2](02-install-dotnet.md))
- Device provisioned with Meadow.Cloud ([Step 3](03-provision-device.md))
- mc-daemon running ([Step 1](01-install-daemon.md))
- Your Avalonia app using Meadow.Linux and Meadow.Core packages

## Application Configuration

### 1. Create app.config.yaml

Your app needs a configuration file to connect to Meadow.Cloud. Create `app.config.yaml` in your project root (same directory as `.csproj`):

```yaml
# Meadow.Cloud Connection
MeadowCloudSettings:
  # Enable cloud connectivity
  Enabled: true

  # MQTT Broker (Meadow.Cloud)
  MqttHostname: mqtt.meadowcloud.co
  MqttPort: 1883

  # HTTP endpoints for authentication
  DataHostname: www.meadowcloud.co
  DataPort: 443

  # Organization and device identifiers
  # {ID} will be replaced with machine ID from /etc/machine-id
  # {OID} will be replaced with organization ID from Meadow.Cloud
  MqttTopicPrefix: "{OID}/ota/{ID}"

# Application Settings
ApplicationSettings:
  # Your app version (increment for OTA updates)
  Version: "1.0.0"

  # Application name
  Name: "MyAvaloniaApp"
```

**Include in project**:

Edit your `.csproj` to copy this file on build:

```xml
<ItemGroup>
  <None Update="app.config.yaml">
    <CopyToOutputDirectory>PreserveNewest</CopyToOutputDirectory>
  </None>
</ItemGroup>
```

### 2. Configure Meadow.Core in Your App

In your `Program.cs` or startup code, initialize Meadow:

```csharp
using Meadow;
using Meadow.Linux;

var builder = MeadowApp.CreateBuilder();
builder.UsePlatform<LinuxPlatform>();
builder.UseAvalonia();  // If using Meadow.Avalonia

var app = builder.Build();

// UpdateService will automatically start monitoring for updates
await app.RunAsync();
```

**Key points**:
- `MeadowApp` automatically reads `app.config.yaml`
- `UpdateService` handles MQTT connection and update downloads
- App is responsible for downloading and extracting updates
- App signals daemon when ready to apply update (by exiting)

### 3. Add Version Display (Optional)

Add a version label to your UI to verify OTA updates:

```xml
<!-- In your MainWindow.axaml or view -->
<TextBlock Text="{Binding Version}" />
```

```csharp
// In your ViewModel
public string Version => MeadowApp.Current.AppVersion ?? "1.0.0";
```

## Build and Deploy Application

### 1. Build Release Version

```bash
cd /mnt/f/temp/MyAvaloniaApp
dotnet publish -c Release -o ./publish
```

**Expected output**:
```
Microsoft (R) Build Engine version...
  MyAvaloniaApp -> /mnt/f/temp/MyAvaloniaApp/bin/Release/net8.0/MyAvaloniaApp.dll
  MyAvaloniaApp -> /mnt/f/temp/MyAvaloniaApp/publish/
```

**Verify build**:
```bash
ls ./publish/MyAvaloniaApp.dll
ls ./publish/app.config.yaml
```

### 2. Deploy to /opt/meadow

This is the directory where the app runs and where updates are applied:

```bash
# Create deployment directory (if not already created)
sudo mkdir -p /opt/meadow
sudo chown $USER:$USER /opt/meadow

# Copy application files
cp -r ./publish/* /opt/meadow/

# Verify deployment
ls -la /opt/meadow/
```

**Expected files**:
```
MyAvaloniaApp.dll
MyAvaloniaApp.deps.json
MyAvaloniaApp.runtimeconfig.json
app.config.yaml
Avalonia.*.dll
Meadow.*.dll
[other dependencies]
```

### 3. Test Manual Run (Optional)

Before setting up systemd, test the app runs:

```bash
cd /opt/meadow
dotnet MyAvaloniaApp.dll
```

**Expected**: GUI window appears with your app

⚠️ **If GUI doesn't appear**, see [Troubleshooting: GUI Issues](#troubleshooting-gui-issues) below.

Press Ctrl+C to stop.

## Create systemd Service

### 1. Create Service File

Create `/etc/systemd/system/meadowapp.service`:

```bash
sudo nano /etc/systemd/system/meadowapp.service
```

Paste this content (see also [meadowapp.service.example](meadowapp.service.example)):

```ini
[Unit]
Description=My Avalonia Meadow Application
Documentation=https://github.com/yourusername/MyAvaloniaApp
After=network-online.target mc-daemon.service
Wants=network-online.target
Requires=mc-daemon.service

[Service]
Type=simple

# Run as your user (NOT root) for display access
User=YOUR_USERNAME
Group=YOUR_USERNAME

# App deployment directory
WorkingDirectory=/opt/meadow

# Start command
ExecStart=/usr/bin/dotnet /opt/meadow/MyAvaloniaApp.dll

# Restart policy - daemon will restart after updates
Restart=no
RestartSec=10

# Display access for GUI apps (CRITICAL for WSL2 GUI)
Environment="DISPLAY=:0"
Environment="XAUTHORITY=/home/YOUR_USERNAME/.Xauthority"
Environment="XDG_RUNTIME_DIR=/run/user/YOUR_UID"

# Logging
StandardOutput=journal
StandardError=journal
SyslogIdentifier=meadowapp

# Security hardening (minimal for GUI apps)
NoNewPrivileges=true

[Install]
WantedBy=graphical.target
```

**⚠️ IMPORTANT**: Replace these placeholders:
- `YOUR_USERNAME`: Your Linux username (run `whoami` to find it)
- `YOUR_UID`: Your user ID (run `id -u` to find it)

**Example for user `ctacke`** (UID 1000):
```ini
User=ctacke
Group=ctacke
Environment="XAUTHORITY=/home/ctacke/.Xauthority"
Environment="XDG_RUNTIME_DIR=/run/user/1000"
```

Save and exit (Ctrl+O, Enter, Ctrl+X in nano).

### 2. Critical: Display Environment Variables

For GUI apps to work in WSL2, you MUST set these environment variables:

| Variable | Purpose | Example Value |
|----------|---------|---------------|
| `DISPLAY` | X11 display server | `:0` |
| `XAUTHORITY` | X11 auth cookie file | `/home/ctacke/.Xauthority` |
| `XDG_RUNTIME_DIR` | Runtime directory | `/run/user/1000` |

**Why user, not root?**
- Root doesn't have access to user's X11 session
- `.Xauthority` file is owned by user
- Running as user simplifies permissions

### 3. Enable and Start Service

```bash
# Reload systemd to recognize new service
sudo systemctl daemon-reload

# Enable service (start on boot)
sudo systemctl enable meadowapp

# Start service
sudo systemctl start meadowapp
```

**Expected output**:
```
Created symlink /etc/systemd/system/graphical.target.wants/meadowapp.service → /etc/systemd/system/meadowapp.service.
```

## Verify Application is Running

### Check Service Status

```bash
systemctl status meadowapp
```

**Expected output** (should show "active (running)"):
```
● meadowapp.service - My Avalonia Meadow Application
     Loaded: loaded (/etc/systemd/system/meadowapp.service; enabled)
     Active: active (running) since Tue 2025-12-03 10:00:00 PST
   Main PID: 5678 (dotnet)
      Tasks: 25
     Memory: 45.0M
        CPU: 500ms
     CGroup: /system.slice/meadowapp.service
             └─5678 /usr/bin/dotnet /opt/meadow/MyAvaloniaApp.dll
```

### Check Logs

```bash
journalctl -u meadowapp -n 30
```

**Look for**:
- Application startup messages
- "Connected to Meadow.Cloud" (from Meadow.Core MQTT client)
- No UnauthorizedAccessException or display errors

### Verify GUI Appears

Look for your application window on your display.

If using Windows 11 with WSLg, the window should appear automatically.

## Update Daemon Configuration

The daemon needs to know it's managing a systemd service. This should already be configured in [Step 1](01-install-daemon.md), but verify:

```bash
cat /etc/meadow.conf | grep -A 1 "systemd"
```

**Expected output**:
```
app_is_systemd_service yes
app_service_name meadowapp.service
```

If not present, add these lines to `/etc/meadow.conf`:

```bash
sudo nano /etc/meadow.conf
```

Add:
```
# Application is managed by systemd
app_is_systemd_service yes
app_service_name meadowapp.service
```

Restart daemon:
```bash
sudo systemctl restart mc-daemon
```

## Verification Checklist

Run these commands to verify everything is configured:

```bash
# App service is running
systemctl is-active meadowapp && echo "✓ App is running"

# Daemon is running
systemctl is-active mc-daemon && echo "✓ Daemon is running"

# App is using correct working directory
systemctl show meadowapp -p WorkingDirectory

# App is running as correct user
systemctl show meadowapp -p User

# Display environment is set
systemctl show meadowapp -p Environment
```

## Troubleshooting GUI Issues

### GUI Doesn't Appear

**Check display environment**:
```bash
systemctl show meadowapp -p Environment
```

Should show:
```
Environment=DISPLAY=:0 XAUTHORITY=/home/ctacke/.Xauthority XDG_RUNTIME_DIR=/run/user/1000
```

**Check X11 is working**:
```bash
echo $DISPLAY
xclock  # Should show a clock window
```

If `xclock` doesn't work, see [02-install-dotnet.md](02-install-dotnet.md#install-x11-support-for-gui-apps) for X11 setup.

**Check .Xauthority file**:
```bash
ls -la ~/.Xauthority
```

Should exist and be readable.

### "UnauthorizedAccessException" Errors

**Check logs**:
```bash
journalctl -u meadowapp -n 50 | grep -i unauthorized
```

If you see permission errors on `/tmp/meadow`:
1. Verify daemon config uses user-owned path:
   ```bash
   cat /etc/meadow.conf | grep meadow_temp
   ```

   Should be:
   ```
   meadow_temp /home/YOUR_USERNAME/.meadow/tmp
   ```

2. Verify directories exist and are owned by user:
   ```bash
   ls -la ~/.meadow/tmp
   mkdir -p ~/.meadow/tmp/{updates,update,staging,rollback}
   ```

3. Restart daemon to pick up config change:
   ```bash
   sudo systemctl restart mc-daemon
   sudo systemctl restart meadowapp
   ```

### Service Fails to Start

**Check logs for errors**:
```bash
journalctl -u meadowapp -n 50
```

**Common issues**:
- DLL not found: Verify `/opt/meadow/MyAvaloniaApp.dll` exists
- Display error: Verify environment variables
- Permission error: Verify User and Group are correct
- Port conflict: Check if app is already running (`ps aux | grep MyAvaloniaApp`)

### App Starts But No Cloud Connection

**Check app logs**:
```bash
journalctl -u meadowapp -f
```

**Look for**:
- MQTT connection errors
- Authentication failures
- "app.config.yaml not found"

**Verify config file**:
```bash
cat /opt/meadow/app.config.yaml
```

Should have correct `MqttHostname`, `MqttPort`, and `MqttTopicPrefix`.

**Verify SSH keys** (daemon uses these for auth):
```bash
sudo ls -la /root/.ssh/id_rsa*
```

For more help, see [Troubleshooting Guide](06-troubleshooting.md#app-issues).

## Important Notes

### Update Workflow Responsibilities

**Your app (via Meadow.Core)**:
- Connects to Meadow.Cloud MQTT
- Receives update notifications
- Downloads MPAK package
- Extracts package to staging directory
- Exits when ready to apply update

**mc-daemon**:
- Waits for app process to exit
- Moves files from staging to `/opt/meadow`
- Restarts app via `systemctl start meadowapp.service`

### Why `Restart=no`?

The systemd service has `Restart=no` because:
- App exits intentionally when update is ready
- Daemon is responsible for the restart (after moving files)
- Prevents systemd from restarting app before update is applied

### Working Directory is Critical

`WorkingDirectory=/opt/meadow` ensures:
- App finds `app.config.yaml`
- Relative paths work correctly
- Update staging paths are predictable

## Next Steps

Application is deployed and running!

Next: [Perform OTA Update](05-perform-update.md)
