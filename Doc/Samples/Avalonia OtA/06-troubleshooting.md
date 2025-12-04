# Troubleshooting Guide

Common issues and solutions for Meadow.Cloud OTA updates with Avalonia apps in WSL2.

## Table of Contents

1. [Daemon Issues](#daemon-issues)
2. [Application Issues](#application-issues)
3. [Update Issues](#update-issues)
4. [Display Issues](#display-issues)
5. [Permission Issues](#permission-issues)
6. [Provisioning Issues](#provisioning-issues)
7. [Useful Commands](#useful-commands)

---

## Daemon Issues

### Daemon Won't Start

**Symptoms**:
```bash
systemctl status mc-daemon
# Shows: Failed to start mc-daemon.service
```

**Diagnosis**:
```bash
journalctl -u mc-daemon -n 50
```

**Common causes**:

1. **Missing /etc/meadow.conf**
   ```bash
   # Verify file exists
   ls -la /etc/meadow.conf

   # If missing, create from template
   sudo cp /path/to/meadow.conf.example /etc/meadow.conf
   ```

2. **Binary not executable**
   ```bash
   sudo chmod 755 /usr/bin/mc-daemon
   ```

3. **Port 5000 already in use**
   ```bash
   # Find what's using port 5000
   sudo lsof -i :5000

   # Kill the conflicting process
   kill <PID>
   ```

4. **Syntax error in meadow.conf**
   ```bash
   # Check for common errors
   cat /etc/meadow.conf

   # Each setting should be: key value (no = sign)
   # ✓ Correct: enabled yes
   # ✗ Wrong:   enabled=yes
   ```

### Can't Connect to Meadow.Cloud

**Symptoms**:
```bash
journalctl -u mc-daemon
# Shows: Failed to connect to mqtt.meadowcloud.co
```

**Note**: If device isn't provisioned yet, this is expected! Complete [Step 3](03-provision-device.md) first.

**If provisioned, check**:

1. **Network connectivity**
   ```bash
   ping mqtt.meadowcloud.co
   ```

2. **SSH keys exist**
   ```bash
   sudo ls -la /root/.ssh/id_rsa*
   ```

3. **Keys in PEM format**
   ```bash
   sudo head -1 /root/.ssh/id_rsa
   # Should show: -----BEGIN RSA PRIVATE KEY-----
   # NOT: -----BEGIN OPENSSH PRIVATE KEY-----
   ```

4. **Device is provisioned**
   - Check [meadowcloud.co](https://www.meadowcloud.co) → Devices
   - Verify your machine ID appears in the list

### MQTT Connection Drops

**Symptoms**:
```bash
journalctl -u mc-daemon -f
# Shows: MQTT connection lost, reconnecting...
```

**This is normal** if network is unstable. Daemon will auto-reconnect.

**If reconnection fails**:
```bash
# Restart daemon
sudo systemctl restart mc-daemon

# Check logs
journalctl -u mc-daemon -n 30
```

### Configuration Override Issues

**Symptoms**:
Daemon uses `/tmp/meadow` despite config file specifying `~/.meadow/tmp`.

**Cause**:
Environment variables in systemd service file override config file.

**Fix**:
```bash
# Check service file
systemctl cat mc-daemon | grep Environment

# Should be EMPTY or NOT have MEADOW_* variables
# If you see: Environment="MEADOW_TEMP=/tmp/meadow"
# Remove it:

sudo nano /etc/systemd/system/mc-daemon.service
# Delete all Environment= lines

sudo systemctl daemon-reload
sudo systemctl restart mc-daemon
```

---

## Application Issues

### GUI Doesn't Appear

**Symptoms**:
Service shows `active (running)` but no window appears.

**Check display environment**:
```bash
systemctl show meadowapp -p Environment
```

**Should show**:
```
Environment=DISPLAY=:0 XAUTHORITY=/home/username/.Xauthority XDG_RUNTIME_DIR=/run/user/1000
```

**Fix if missing**:
```bash
sudo nano /etc/systemd/system/meadowapp.service

# Add these lines under [Service]:
Environment="DISPLAY=:0"
Environment="XAUTHORITY=/home/YOUR_USERNAME/.Xauthority"
Environment="XDG_RUNTIME_DIR=/run/user/YOUR_UID"

# Save, reload, restart
sudo systemctl daemon-reload
sudo systemctl restart meadowapp
```

**Verify X11 is working**:
```bash
echo $DISPLAY  # Should show :0 or similar
xclock         # Should show a clock window
```

**WSL2-specific X11 setup**:

**Windows 11**: WSLg is built-in, should work automatically

**Windows 10**:
1. Install VcXsrv or Xming on Windows
2. Start X server
3. Set DISPLAY variable:
   ```bash
   export DISPLAY=$(cat /etc/resolv.conf | grep nameserver | awk '{print $2}'):0
   echo 'export DISPLAY=$(cat /etc/resolv.conf | grep nameserver | awk '{print $2}'):0' >> ~/.bashrc
   ```

### UnauthorizedAccessException

**Symptoms**:
```bash
journalctl -u meadowapp -n 50
# Shows: UnauthorizedAccessException: Access to the path '/tmp/meadow/...' is denied.
```

**Cause**:
App (running as user) trying to access directory owned by root.

**Fix - Use user-owned directory**:

1. **Update daemon config**:
   ```bash
   sudo nano /etc/meadow.conf

   # Change to user-owned path:
   meadow_temp /home/YOUR_USERNAME/.meadow/tmp
   ```

2. **Create directories**:
   ```bash
   mkdir -p ~/.meadow/tmp/{updates,update,staging,rollback}
   ```

3. **Restart daemon**:
   ```bash
   sudo systemctl restart mc-daemon
   sudo systemctl restart meadowapp
   ```

### Service Fails to Start

**Check logs**:
```bash
journalctl -u meadowapp -n 100
```

**Common issues**:

1. **DLL not found**
   ```bash
   # Verify app exists
   ls -la /opt/meadow/MyAvaloniaApp.dll

   # If missing, redeploy
   cp -r ~/MyAvaloniaApp/publish/* /opt/meadow/
   ```

2. **Permission denied on app directory**
   ```bash
   # Ensure user owns /opt/meadow
   sudo chown -R $USER:$USER /opt/meadow
   ```

3. **Wrong user in service file**
   ```bash
   systemctl cat meadowapp | grep User

   # Should match your username
   # If wrong, edit service file
   sudo nano /etc/systemd/system/meadowapp.service
   ```

4. **Working directory doesn't exist**
   ```bash
   # Verify directory
   ls -ld /opt/meadow

   # If missing
   sudo mkdir -p /opt/meadow
   sudo chown $USER:$USER /opt/meadow
   ```

### App Can't Connect to Meadow.Cloud

**Symptoms**:
```bash
journalctl -u meadowapp -f
# Shows: Failed to connect to MQTT broker
```

**Check app configuration**:
```bash
cat /opt/meadow/app.config.yaml
```

**Verify settings**:
```yaml
MeadowCloudSettings:
  Enabled: true
  MqttHostname: mqtt.meadowcloud.co
  MqttPort: 1883
```

**Check network**:
```bash
ping mqtt.meadowcloud.co
telnet mqtt.meadowcloud.co 1883
```

**Verify provisioning**:
- Device must be provisioned (Step 3)
- Check [meadowcloud.co](https://www.meadowcloud.co) → Devices

### App Crashes on Startup

**Check logs for exceptions**:
```bash
journalctl -u meadowapp -n 100 | less
```

**Common issues**:

1. **Missing dependencies**
   ```bash
   # Verify all DLLs present
   ls /opt/meadow/*.dll

   # Redeploy if needed
   dotnet publish -c Release -o ./publish
   cp -r ./publish/* /opt/meadow/
   ```

2. **Config file not found**
   ```bash
   # Verify config exists in working directory
   ls -la /opt/meadow/app.config.yaml

   # Copy if missing
   cp ~/MyAvaloniaApp/app.config.yaml /opt/meadow/
   ```

---

## Update Issues

### Update Not Detected by App

**Check MQTT connection**:
```bash
journalctl -u meadowapp -n 50 | grep -i mqtt
```

Should show:
```
Connected to Meadow.Cloud MQTT broker
Subscribed to topic: YOUR_ORG_ID/ota/YOUR_MACHINE_ID
```

**Verify deployment in cloud**:
1. Go to [meadowcloud.co](https://www.meadowcloud.co)
2. Navigate to **Deployments**
3. Verify status shows "Sent" or "Delivered"

**Manually trigger update check**:
```bash
# Restart app to force re-check
sudo systemctl restart meadowapp

# Watch logs
journalctl -u meadowapp -f
```

**Verify MQTT topic configuration**:
```bash
cat /opt/meadow/app.config.yaml | grep MqttTopicPrefix
```

Should have:
```yaml
MqttTopicPrefix: "{OID}/ota/{ID}"
```

### Update Download Fails

**Check app logs**:
```bash
journalctl -u meadowapp -n 100 | grep -i download
```

**Common issues**:

1. **Network connectivity**
   ```bash
   ping meadowcloud.co
   curl -I https://www.meadowcloud.co
   ```

2. **Permission on staging directory**
   ```bash
   ls -ld ~/.meadow/tmp/staging

   # Should be owned by your user
   # If not:
   mkdir -p ~/.meadow/tmp/staging
   ```

3. **Disk space**
   ```bash
   df -h /opt/meadow
   df -h ~/.meadow

   # Ensure sufficient space for app + update
   ```

### Update Extraction Fails

**Check staging directory**:
```bash
ls -la ~/.meadow/tmp/staging/
```

**Should contain extracted files**. If empty:
```bash
# Check app logs for extraction errors
journalctl -u meadowapp -n 200 | grep -i extract
```

**Common issues**:

1. **Corrupted MPAK**
   ```bash
   # Re-download or re-create package
   meadow package upload MyAvaloniaApp.mpak --org "Your Org"
   ```

2. **Permission errors**
   ```bash
   # Ensure staging directory is writable
   chmod 755 ~/.meadow/tmp/staging
   ```

### Daemon Doesn't Move Files After Update

**Check daemon logs**:
```bash
journalctl -u mc-daemon -n 100 | grep -i update
```

**Verify daemon configuration**:
```bash
cat /etc/meadow.conf | grep -E "meadow_root|meadow_temp|app_is_systemd"
```

**Should show**:
```
meadow_root /opt/meadow
meadow_temp /home/YOUR_USERNAME/.meadow/tmp
app_is_systemd_service yes
app_service_name meadowapp.service
```

**Test file move manually**:
```bash
# Check staging has files
ls ~/.meadow/tmp/staging/

# Check daemon can write to app directory
sudo -u root ls /opt/meadow/
sudo -u root touch /opt/meadow/test.txt
sudo -u root rm /opt/meadow/test.txt
```

### App Doesn't Restart After Update

**Check daemon logs**:
```bash
journalctl -u mc-daemon -n 50 | grep -i restart
```

**Manually restart to test**:
```bash
sudo systemctl start meadowapp
systemctl status meadowapp
```

**Verify systemd service configuration**:
```bash
cat /etc/meadow.conf | grep app_service_name
# Should show: app_service_name meadowapp.service

# Verify service file exists
systemctl cat meadowapp
```

---

## Display Issues

### .Xauthority File Not Found

**Symptoms**:
```
No such file or directory: /home/username/.Xauthority
```

**Create the file**:
```bash
touch ~/.Xauthority
chmod 600 ~/.Xauthority
```

**Restart X11**:
```bash
# On WSL2, restart WSL from Windows PowerShell:
wsl --shutdown
# Then reopen WSL
```

### XDG_RUNTIME_DIR Issues

**Symptoms**:
```
XDG_RUNTIME_DIR not set
```

**Find your UID**:
```bash
id -u
# Usually 1000 for first user
```

**Add to service file**:
```bash
sudo nano /etc/systemd/system/meadowapp.service

# Add:
Environment="XDG_RUNTIME_DIR=/run/user/1000"

sudo systemctl daemon-reload
sudo systemctl restart meadowapp
```

### Wrong DISPLAY Value

**Symptoms**:
GUI doesn't appear or shows "cannot open display".

**Try different DISPLAY values**:
```bash
# Test each:
DISPLAY=:0 xclock
DISPLAY=:1 xclock
DISPLAY=$(cat /etc/resolv.conf | grep nameserver | awk '{print $2}'):0 xclock
```

**Use the one that works**:
```bash
sudo nano /etc/systemd/system/meadowapp.service

# Update:
Environment="DISPLAY=:0"  # or :1, or hostname:0
```

---

## Permission Issues

### Root vs User Ownership Conflicts

**Problem**: Daemon (root) creates files, app (user) can't access them.

**Solution**: Use user-owned directories for shared paths.

**Set correct ownership**:
```bash
# App deployment directory (owned by user)
sudo chown -R $USER:$USER /opt/meadow

# Temp/staging directories (owned by user)
chown -R $USER:$USER ~/.meadow

# SSH keys (owned by root, daemon needs these)
sudo chown root:root /root/.ssh/id_rsa*
sudo chmod 600 /root/.ssh/id_rsa
```

### Can't Write to /tmp/meadow

**DON'T use `/tmp/meadow`** if app runs as user and daemon runs as root.

**Use user-owned path instead**:
```bash
sudo nano /etc/meadow.conf

# Change:
meadow_temp /home/YOUR_USERNAME/.meadow/tmp

# Create directories:
mkdir -p ~/.meadow/tmp/{updates,update,staging,rollback}
```

---

## Provisioning Issues

### Device Already Provisioned

**Symptoms**:
```
Error: Device with this machine ID is already provisioned
```

**Options**:

1. **Use existing provisioning** - No action needed

2. **Re-provision**:
   - Delete device from [meadowcloud.co](https://www.meadowcloud.co) → Devices
   - Generate new SSH keys
   - Re-run provisioning (Step 3)

### Keys Not in PEM Format

**Symptoms**:
Daemon logs show RSA key parsing errors.

**Check key format**:
```bash
sudo head -1 /root/.ssh/id_rsa
```

**Should be**: `-----BEGIN RSA PRIVATE KEY-----`
**NOT**: `-----BEGIN OPENSSH PRIVATE KEY-----`

**Fix - Regenerate keys**:
```bash
# Remove old keys
rm ~/.ssh/id_rsa*

# Generate new keys in PEM format
ssh-keygen -t rsa -b 2048 -m PEM -f ~/.ssh/id_rsa -N ""

# Copy to root
sudo mkdir -p /root/.ssh
sudo cp ~/.ssh/id_rsa* /root/.ssh/
sudo chmod 600 /root/.ssh/id_rsa

# Re-provision (Step 3)
```

### Machine ID Not Found

**Symptoms**:
```
Error: Could not read /etc/machine-id
```

**Generate machine ID**:
```bash
# Check if file exists
ls -la /etc/machine-id

# If missing, create
sudo systemd-machine-id-setup
```

---

## Useful Commands

### Check Service Status

```bash
# Daemon status
systemctl status mc-daemon

# App status
systemctl status meadowapp

# Both services
systemctl status mc-daemon meadowapp
```

### View Logs

```bash
# Last 50 lines
journalctl -u mc-daemon -n 50
journalctl -u meadowapp -n 50

# Follow (live tail)
journalctl -u mc-daemon -f
journalctl -u meadowapp -f

# Since specific time
journalctl -u meadowapp --since "10 minutes ago"

# Both services, last hour
journalctl -u mc-daemon -u meadowapp --since "1 hour ago"

# Search for errors
journalctl -u meadowapp | grep -i error
```

### Restart Services

```bash
# Restart daemon
sudo systemctl restart mc-daemon

# Restart app
sudo systemctl restart meadowapp

# Restart both
sudo systemctl restart mc-daemon meadowapp
```

### Stop Services

```bash
# Stop app
sudo systemctl stop meadowapp

# Stop daemon
sudo systemctl stop mc-daemon
```

### Disable Auto-Start

```bash
# Disable app
sudo systemctl disable meadowapp

# Disable daemon
sudo systemctl disable mc-daemon

# Re-enable
sudo systemctl enable meadowapp
sudo systemctl enable mc-daemon
```

### Check Configuration

```bash
# Daemon config
cat /etc/meadow.conf

# App config
cat /opt/meadow/app.config.yaml

# Service files
systemctl cat mc-daemon
systemctl cat meadowapp

# Environment variables
systemctl show meadowapp -p Environment
```

### Test REST API

```bash
# Daemon info
curl http://127.0.0.1:5000/api/info

# Pretty print JSON
curl http://127.0.0.1:5000/api/info | jq

# Available updates
curl http://127.0.0.1:5000/api/updates
```

### Manual Update Testing

```bash
# Check staging directory
ls -la ~/.meadow/tmp/staging/

# Check deployed app
ls -la /opt/meadow/

# Compare versions
cat ~/.meadow/tmp/staging/app.config.yaml | grep Version
cat /opt/meadow/app.config.yaml | grep Version

# Manually deploy staged update
sudo systemctl stop meadowapp
cp -r ~/.meadow/tmp/staging/* /opt/meadow/
sudo systemctl start meadowapp
```

### Reset State

```bash
# Clear all updates
rm -rf ~/.meadow/tmp/updates/*
rm -rf ~/.meadow/tmp/staging/*

# Reset app to clean state
sudo systemctl stop meadowapp
rm -rf /opt/meadow/*
cp -r ~/MyAvaloniaApp/publish/* /opt/meadow/
sudo systemctl start meadowapp

# Restart everything
sudo systemctl restart mc-daemon meadowapp
```

### Check Network Connectivity

```bash
# Ping Meadow.Cloud
ping mqtt.meadowcloud.co
ping www.meadowcloud.co

# Test MQTT port
telnet mqtt.meadowcloud.co 1883

# Test HTTP/HTTPS
curl -I https://www.meadowcloud.co

# Check DNS
nslookup meadowcloud.co
```

---

## Getting More Help

If none of these solutions work:

1. **Collect diagnostic info**:
   ```bash
   # Save logs
   journalctl -u mc-daemon -n 200 > daemon.log
   journalctl -u meadowapp -n 200 > app.log

   # Save config
   cat /etc/meadow.conf > meadow-conf.txt
   cat /opt/meadow/app.config.yaml > app-config.txt
   ```

2. **Check documentation**:
   - [Meadow.Daemon Repository](https://github.com/WildernessLabs/Meadow.Daemon)
   - [Meadow.Cloud Docs](https://developer.wildernesslabs.co)

3. **Community support**:
   - Wilderness Labs Slack
   - GitHub Issues

4. **Include in bug reports**:
   - WSL2 version: `wsl --version`
   - Debian version: `cat /etc/debian_version`
   - .NET version: `dotnet --version`
   - Daemon version: `/usr/bin/mc-daemon --version`
   - Logs from both services
   - Configuration files
