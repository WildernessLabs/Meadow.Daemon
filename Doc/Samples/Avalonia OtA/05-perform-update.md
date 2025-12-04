# Step 5: Perform OTA Update

Create an updated version of your application and deploy it via Meadow.Cloud.

## What We're Doing

This demonstrates the complete OTA update flow:
1. Make a visible change to your app (version 1.0 → 2.0)
2. Build and package the updated app as an MPAK file
3. Publish the MPAK to Meadow.Cloud
4. Watch as your app automatically downloads, stages, and applies the update
5. Verify the new version is running

## Prerequisites

- App running as systemd service ([Step 4](04-setup-app.md))
- Device provisioned and connected to Meadow.Cloud ([Step 3](03-provision-device.md))
- Meadow CLI installed ([Step 2](02-install-dotnet.md))

## Create Updated Application

### 1. Make a Visible Change

Update your app's version number and UI so you can confirm the update worked.

#### Update app.config.yaml

```bash
cd /mnt/f/temp/MyAvaloniaApp
nano app.config.yaml
```

Change version:
```yaml
ApplicationSettings:
  Version: "2.0.0"  # Was: 1.0.0
  Name: "MyAvaloniaApp"
```

#### Update UI (Optional but Recommended)

Make a visual change so you can see when the update applies.

**Example - Add version to window title**:

`MainWindow.axaml`:
```xml
<Window ...
        Title="MyAvaloniaApp - Version 2.0">
```

**Example - Change a label**:
```xml
<TextBlock Text="Version 2.0 - Updated!" FontSize="24" />
```

**Example - Change background color**:
```xml
<Window ...
        Background="LightBlue">  <!-- Was: White -->
```

Save all changes.

### 2. Build Updated Application

Build the app in Release mode:

```bash
cd /mnt/f/temp/MyAvaloniaApp
dotnet clean
dotnet publish -c Release -o ./publish
```

**Expected output**:
```
Microsoft (R) Build Engine version...
  Determining projects to restore...
  All projects are up-to-date for restore.
  MyAvaloniaApp -> /mnt/f/temp/MyAvaloniaApp/bin/Release/net8.0/MyAvaloniaApp.dll
  MyAvaloniaApp -> /mnt/f/temp/MyAvaloniaApp/publish/
```

**Verify**:
```bash
ls ./publish/MyAvaloniaApp.dll
cat ./publish/app.config.yaml | grep Version
```

Should show `Version: "2.0.0"`.

## Create MPAK Package

### What is an MPAK?

An MPAK (Meadow Package) is simply a ZIP file containing your application binaries. Structure:
```
MyAvaloniaApp.mpak (ZIP file)
├── MyAvaloniaApp.dll
├── MyAvaloniaApp.deps.json
├── MyAvaloniaApp.runtimeconfig.json
├── app.config.yaml
└── [all dependency DLLs]
```

### 1. Package Application

Use Meadow CLI to create the MPAK:

```bash
cd /mnt/f/temp/MyAvaloniaApp
meadow app package -p ./publish
```

**Expected output**:
```
Packaging application...
Created package: MyAvaloniaApp.mpak (12.5 MB)
```

**Verify package**:
```bash
ls -lh MyAvaloniaApp.mpak
```

Should show a file of several MB.

**Optional - Inspect package contents**:
```bash
unzip -l MyAvaloniaApp.mpak | head -20
```

### 2. Alternative: Manual ZIP Creation

If `meadow app package` doesn't work, create ZIP manually:

**On Linux**:
```bash
cd publish
zip -r ../MyAvaloniaApp.mpak *
cd ..
```

**On Windows (PowerShell)**:
```powershell
Compress-Archive -Path .\publish\* -DestinationPath .\MyAvaloniaApp.mpak -Force
```

## Publish to Meadow.Cloud

### 1. Authenticate with Meadow CLI

```bash
meadow login
```

**Expected flow**:
- Browser opens for authentication
- Log in with your Meadow.Cloud credentials
- Return to terminal

**Expected output**:
```
Authentication successful!
```

### 2. Upload MPAK Package

```bash
meadow package upload MyAvaloniaApp.mpak --org "Your Organization Name"
```

**Expected output**:
```
Uploading package MyAvaloniaApp.mpak...
Upload complete: MyAvaloniaApp.mpak
Package ID: abc123...
```

### 3. Deploy Update to Device

Deploy the update to your provisioned device:

```bash
meadow package deploy MyAvaloniaApp.mpak --device YOUR_MACHINE_ID
```

Replace `YOUR_MACHINE_ID` with your device's machine ID from [Step 3](03-provision-device.md#3-get-device-machine-id).

**Alternative - Deploy via Web UI**:
1. Go to [meadowcloud.co](https://www.meadowcloud.co)
2. Navigate to **Packages**
3. Find `MyAvaloniaApp.mpak`
4. Click **Deploy**
5. Select your device
6. Click **Deploy Now**

**Expected output** (CLI):
```
Deploying package to device YOUR_MACHINE_ID...
Deployment initiated. Device will receive update notification.
```

## Monitor Update Process

### 1. Watch Application Logs

In one terminal, watch your application logs:

```bash
journalctl -u meadowapp -f
```

**What you'll see**:
1. **Update notification received** (from Meadow.Cloud MQTT)
   ```
   [INFO] Update available: MyAvaloniaApp v2.0.0
   ```

2. **Download starts** (app's UpdateService downloads MPAK)
   ```
   [INFO] Downloading update package...
   [INFO] Download progress: 25%... 50%... 100%
   ```

3. **Extraction** (app extracts to staging directory)
   ```
   [INFO] Extracting package to staging...
   [INFO] Extraction complete
   ```

4. **App exits** (signals update ready)
   ```
   [INFO] Update staged. Exiting for daemon to apply...
   Application stopping...
   ```

### 2. Watch Daemon Logs

In another terminal, watch daemon logs:

```bash
journalctl -u mc-daemon -f
```

**What you'll see**:
1. **Detects app exit**
   ```
   [INFO] Application process exited (update pending)
   ```

2. **Stops systemd service** (if not already stopped)
   ```
   [INFO] Stopping service: meadowapp.service
   ```

3. **Moves files**
   ```
   [INFO] Moving files from staging to /opt/meadow...
   [INFO] File move complete
   ```

4. **Restarts app**
   ```
   [INFO] Starting service: meadowapp.service
   Application started successfully
   ```

### 3. Watch Service Status

In a third terminal, monitor service status:

```bash
watch -n 1 'systemctl status meadowapp | head -15'
```

You'll see the service:
- **active (running)** → **inactive (dead)** → **active (running)**

Press Ctrl+C to exit watch when done.

## Verify Update Applied

### 1. Check Application Version

Look at your GUI window. You should see your changes:
- Updated window title
- New UI elements
- Changed colors

### 2. Check Version in Logs

```bash
journalctl -u meadowapp -n 20 | grep -i version
```

Should show version 2.0.0.

### 3. Check Configuration File

```bash
cat /opt/meadow/app.config.yaml | grep Version
```

**Expected output**:
```yaml
  Version: "2.0.0"
```

### 4. Check Service Status

```bash
systemctl status meadowapp
```

Should show:
- **Active: active (running)**
- Recent start time (within last few minutes)

## Troubleshooting

### Update Not Detected

**Check app MQTT connection**:
```bash
journalctl -u meadowapp -n 50 | grep -i mqtt
```

Should show:
```
Connected to Meadow.Cloud MQTT broker
Subscribed to topic: YOUR_ORG_ID/ota/YOUR_MACHINE_ID
```

**Check Meadow.Cloud deployment**:
1. Go to [meadowcloud.co](https://www.meadowcloud.co)
2. Navigate to **Deployments**
3. Verify deployment status is "Sent" or "Delivered"

**Manually trigger check**:

Restart app to force re-check:
```bash
sudo systemctl restart meadowapp
```

### Download Fails

**Check app logs**:
```bash
journalctl -u meadowapp -n 100 | grep -i download
```

**Common issues**:
- Network connectivity: `ping meadowcloud.co`
- Permission error on staging directory: Check `~/.meadow/tmp` ownership
- Disk space: `df -h /opt/meadow`

### Extraction Fails

**Check staging directory**:
```bash
ls -la ~/.meadow/tmp/staging/
```

Should contain extracted files.

**Check permissions**:
```bash
ls -ld ~/.meadow/tmp/
ls -ld ~/.meadow/tmp/staging/
```

Both should be owned by your user.

### Daemon Doesn't Move Files

**Check daemon logs**:
```bash
journalctl -u mc-daemon -n 100 | grep -i update
```

**Verify daemon config**:
```bash
cat /etc/meadow.conf | grep -E "meadow_root|meadow_temp|app_is_systemd"
```

Should show:
```
meadow_root /opt/meadow
meadow_temp /home/YOUR_USERNAME/.meadow/tmp
app_is_systemd_service yes
app_service_name meadowapp.service
```

**Manual file move test**:
```bash
# Check staging directory has files
ls ~/.meadow/tmp/staging/

# Check daemon has permission to write to app directory
sudo -u root ls /opt/meadow/
```

### App Doesn't Restart

**Check daemon logs**:
```bash
journalctl -u mc-daemon -n 50 | grep -i restart
```

**Manually restart**:
```bash
sudo systemctl start meadowapp
systemctl status meadowapp
```

**Check service file**:
```bash
systemctl cat meadowapp | grep -E "ExecStart|WorkingDirectory|User"
```

For more help, see [Troubleshooting Guide](06-troubleshooting.md#update-issues).

## Update Flow Summary

Here's what just happened:

1. **You**: Created MPAK package with version 2.0.0
2. **You**: Published to Meadow.Cloud
3. **Meadow.Cloud**: Sent MQTT notification to your device
4. **Your app** (Meadow.Core UpdateService):
   - Received notification
   - Downloaded MPAK from Meadow.Cloud
   - Extracted to `~/.meadow/tmp/staging/`
   - Exited cleanly
5. **mc-daemon**:
   - Detected app exit
   - Stopped systemd service (if needed)
   - Moved files from staging to `/opt/meadow/`
   - Restarted systemd service
6. **Your app**: Started with new version 2.0.0

## Next Steps

### Perform Multiple Updates

Try creating version 3.0:
1. Update `app.config.yaml` to `Version: "3.0.0"`
2. Make another UI change
3. Build, package, publish
4. Watch it update again!

### Rollback Testing

If an update fails, the daemon keeps a backup in `/home/YOUR_USERNAME/.meadow/tmp/rollback/`.

To manually rollback:
```bash
# Stop app
sudo systemctl stop meadowapp

# Restore from rollback
sudo cp -r ~/.meadow/tmp/rollback/* /opt/meadow/

# Start app
sudo systemctl start meadowapp
```

### Production Considerations

For production deployments:
- Implement health checks in your app
- Add automatic rollback on failure
- Use staged rollouts (deploy to subset of devices first)
- Monitor update success rates in Meadow.Cloud dashboard
- Test updates in staging environment first

## Troubleshooting Reference

Common issues and solutions: [06-troubleshooting.md](06-troubleshooting.md)

## Success!

Congratulations! You've successfully demonstrated OTA updates with Meadow.Cloud. Your application can now receive automatic updates without manual intervention.

Key takeaways:
- ✅ App (Meadow.Core) handles MQTT connection and downloads
- ✅ Daemon handles file moves and app restart
- ✅ Updates applied automatically with zero manual intervention
- ✅ GUI app continues to work after update
- ✅ Complete update workflow from build → deploy → apply
