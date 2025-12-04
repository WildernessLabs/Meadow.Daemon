# Step 3: Provision Device with Meadow.Cloud

Register your WSL2 device with Meadow.Cloud to enable OTA updates.

## What is Provisioning?

Provisioning links your device to your Meadow.Cloud account using SSH keys for secure authentication. This allows:
- Remote OTA update deployment
- Device management in the cloud
- Secure communication between device and Meadow.Cloud

## Prerequisites

- Meadow.Cloud account (sign up at [meadowcloud.co](https://www.meadowcloud.co))
- WSL2 device with mc-daemon installed
- Windows machine with browser access
- sudo privileges on WSL2

## Overview

Provisioning is a **two-machine process**:
1. **Device (WSL2)**: Generate SSH keys and provisioning command
2. **Machine with browser**: Execute provisioning command to register device

This is needed because WSL2 may not have direct browser access to complete the OAuth flow.

## Step-by-Step Instructions

### 1. Generate SSH Keys on Device

SSH keys are used to authenticate your device with Meadow.Cloud.

```bash
# Generate RSA key pair in PEM format
ssh-keygen -t rsa -b 2048 -m PEM -f ~/.ssh/id_rsa -N ""
```

**Expected output**:
```
Generating public/private rsa key pair.
Your identification has been saved in /home/username/.ssh/id_rsa
Your public key has been saved in /home/username/.ssh/id_rsa.pub
The key fingerprint is:
SHA256:xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx username@hostname
```

**Verify keys**:
```bash
ls -la ~/.ssh/id_rsa*
```

**Expected output**:
```
-rw------- 1 username username 1679 Dec  3 10:00 /home/username/.ssh/id_rsa
-rw-r--r-- 1 username username  394 Dec  3 10:00 /home/username/.ssh/id_rsa.pub
```

⚠️ **Important**: Keys MUST be in PEM format (`-m PEM` flag). The daemon cannot read OpenSSH format keys.

### 2. Copy Keys to Root User

The daemon runs as root and needs access to the keys.

```bash
# Create root SSH directory
sudo mkdir -p /root/.ssh

# Copy keys to root
sudo cp ~/.ssh/id_rsa* /root/.ssh/

# Set proper permissions
sudo chmod 600 /root/.ssh/id_rsa
sudo chmod 644 /root/.ssh/id_rsa.pub
```

**Verify**:
```bash
sudo ls -la /root/.ssh/id_rsa*
```

**Expected output**:
```
-rw------- 1 root root 1679 Dec  3 10:00 /root/.ssh/id_rsa
-rw-r--r-- 1 root root  394 Dec  3 10:00 /root/.ssh/id_rsa.pub
```

### 3. Get Device Machine ID

Meadow.Cloud uses the machine ID to uniquely identify your device.

```bash
cat /etc/machine-id
```

**Expected output** (32-character hex string):
```
1c8150f752614dec80f88752256e829f
```

Save this ID - you'll need it to verify the device appears in Meadow.Cloud.

### 4. Generate Provisioning Command

Use Meadow CLI to create the provisioning command.

```bash
meadow device provision --help
```

Generate the command with your organization:

```bash
meadow device provision --org "Your Organization Name"
```

**Expected output**:
```
Please run this command on a machine with browser access:

meadow login --sethost https://www.meadowcloud.co --setport 443
meadow device provision --org "Your Organization Name" --pubkey "ssh-rsa AAAAB3NzaC1yc2EAAAADAQA..." --machineid "1c8150f752614dec80f88752256e829f"
```

**Copy the entire output** - you'll run this on your Windows machine.

### 5. Provision on Machine with Browser

On your **Windows machine** (with browser access):

#### A. Install Meadow CLI (if not already installed)

Open PowerShell:
```powershell
dotnet tool install WildernessLabs.Meadow.CLI --global
```

#### B. Paste and Run the Provisioning Command

Paste the full command generated in Step 4.

Example:
```powershell
meadow login --sethost https://www.meadowcloud.co --setport 443
meadow device provision --org "Your Organization Name" --pubkey "ssh-rsa AAAAB3..." --machineid "1c8150f752614dec80f88752256e829f"
```

**Expected flow**:
1. Browser window opens for Meadow.Cloud login
2. Log in with your account credentials
3. Authorize the device
4. Return to terminal

**Expected output**:
```
Opening browser for authentication...
Authentication successful!
Device provisioned successfully.
Device ID: 1c8150f752614dec80f88752256e829f
Organization: Your Organization Name
```

### 6. Verify Device in Meadow.Cloud

Open browser and go to [meadowcloud.co](https://www.meadowcloud.co).

1. Log in to your account
2. Navigate to **Devices** section
3. Look for your device by Machine ID

**What you should see**:
- Device listed with Machine ID `1c8150f752614dec80f88752256e829f`
- Status: "Online" or "Offline" (depends on if daemon is running)
- Organization: Your organization name

### 7. Restart Daemon (Optional)

If the daemon was running during provisioning, restart it to pick up the new credentials:

```bash
sudo systemctl restart mc-daemon
```

Check logs to verify authentication:

```bash
journalctl -u mc-daemon -n 20
```

**Look for**:
- "Authentication successful"
- "Connected to Meadow.Cloud"

## Verification Checklist

Run these commands to verify provisioning is complete:

```bash
# SSH keys exist for root
sudo test -f /root/.ssh/id_rsa && echo "✓ Private key exists"
sudo test -f /root/.ssh/id_rsa.pub && echo "✓ Public key exists"

# Machine ID is valid
test -f /etc/machine-id && echo "✓ Machine ID exists"
cat /etc/machine-id

# Daemon is running
systemctl is-active mc-daemon && echo "✓ Daemon is running"
```

## Troubleshooting

### "meadow: command not found" on Windows

PATH issue. Restart PowerShell or add to PATH:
```powershell
$env:PATH += ";$env:USERPROFILE\.dotnet\tools"
```

### Keys Not in PEM Format

If you see errors about key format:
```bash
# Remove old keys
rm ~/.ssh/id_rsa*

# Regenerate with -m PEM flag
ssh-keygen -t rsa -b 2048 -m PEM -f ~/.ssh/id_rsa -N ""
```

### Browser Doesn't Open

Manually navigate to the URL shown in the terminal output.

### "Device already provisioned"

Device was provisioned before. To re-provision:
1. Delete device from Meadow.Cloud web interface
2. Generate new SSH keys
3. Repeat provisioning process

### Authentication Fails in Daemon Logs

Check daemon logs:
```bash
journalctl -u mc-daemon -n 50
```

Common issues:
- Keys not readable by root: `sudo chmod 600 /root/.ssh/id_rsa`
- Keys not in PEM format: Regenerate with `-m PEM`
- Device not provisioned: Complete Step 5 again

For more help, see [Troubleshooting Guide](06-troubleshooting.md#provisioning-issues).

## Understanding SSH Keys

### Why PEM Format?

The Rust crypto libraries used by mc-daemon require PEM (Privacy-Enhanced Mail) format. Newer SSH tools default to OpenSSH format, which is incompatible.

**PEM format** (correct):
```
-----BEGIN RSA PRIVATE KEY-----
MIIEpAIBAAKCAQEA...
```

**OpenSSH format** (incorrect):
```
-----BEGIN OPENSSH PRIVATE KEY-----
b3BlbnNzaC1rZXktdjEA...
```

### Key Security

- Private key (`id_rsa`): Must be mode 600 (read/write owner only)
- Public key (`id_rsa.pub`): Can be mode 644 (readable by all)
- Never share your private key
- Keys are stored in `/root/.ssh` because daemon runs as root

## What Happens During Provisioning?

1. **Key Generation**: RSA-2048 key pair created locally
2. **CLI Command**: Generates provisioning request with public key
3. **Authentication**: Browser OAuth flow links device to your account
4. **Registration**: Meadow.Cloud stores public key and machine ID
5. **Daemon Auth**: Daemon uses private key to authenticate future connections

## Next Steps

Device is provisioned and connected to Meadow.Cloud!

Next: [Setup Your Application](04-setup-app.md)
