# Step 2: Install .NET Runtime

Install .NET 8 SDK and Meadow CLI tools.

## Prerequisites

- WSL2 with Debian installed
- Internet connection
- sudo privileges

## Install .NET 8 SDK

### 1. Add Microsoft Package Repository

```bash
# Download Microsoft package signing key
wget https://packages.microsoft.com/config/debian/12/packages-microsoft-prod.deb

# Install the repository configuration
sudo dpkg -i packages-microsoft-prod.deb

# Clean up
rm packages-microsoft-prod.deb
```

**Expected output**:
```
Selecting previously unselected package packages-microsoft-prod.
(Reading database ... 123456 files and directories currently installed.)
Preparing to unpack packages-microsoft-prod.deb ...
Unpacking packages-microsoft-prod (1.0-debian12.1) ...
Setting up packages-microsoft-prod (1.0-debian12.1) ...
```

### 2. Update Package Lists

```bash
sudo apt update
```

**Expected output**:
```
Hit:1 http://deb.debian.org/debian bookworm InRelease
Get:2 https://packages.microsoft.com/debian/12/prod bookworm InRelease [...]
Reading package lists... Done
```

### 3. Install .NET 8 SDK

```bash
sudo apt install -y dotnet-sdk-8.0
```

**Expected output** (installation will take 2-3 minutes):
```
Reading package lists... Done
Building dependency tree... Done
The following NEW packages will be installed:
  dotnet-sdk-8.0
...
Setting up dotnet-sdk-8.0 (8.0.xxx-1) ...
```

### 4. Verify Installation

```bash
dotnet --version
```

**Expected output**:
```
8.0.xxx
```

Test with simple command:
```bash
dotnet --list-sdks
```

**Expected output**:
```
8.0.xxx [/usr/share/dotnet/sdk]
```

## Install Meadow CLI

### 1. Install as Global Tool

```bash
dotnet tool install WildernessLabs.Meadow.CLI --global
```

**Expected output**:
```
You can invoke the tool using the following command: meadow
Tool 'wildernesslabs.meadow.cli' (version 'x.x.x') was successfully installed.
```

### 2. Add to PATH (if needed)

The installer should add `~/.dotnet/tools` to your PATH. Verify:

```bash
echo $PATH | grep dotnet
```

If it's not there, add it:

```bash
echo 'export PATH="$PATH:$HOME/.dotnet/tools"' >> ~/.bashrc
source ~/.bashrc
```

### 3. Verify Meadow CLI

```bash
meadow --version
```

**Expected output**:
```
Meadow CLI vx.x.x
```

Test help command:
```bash
meadow --help
```

## Install X11 Support (for GUI Apps)

Since we're running an Avalonia GUI app, install X11 support:

```bash
sudo apt install -y x11-apps
```

**Test X11** (optional):
```bash
xclock
```

If a clock window appears, X11 is working! (Ctrl+C to close)

If it doesn't appear, you may need to:
1. Install an X server on Windows (VcXsrv or Windows 11 built-in)
2. Set `DISPLAY` environment variable

```bash
# For WSL2, usually:
export DISPLAY=$(cat /etc/resolv.conf | grep nameserver | awk '{print $2}'):0
```

Add to `~/.bashrc` to make permanent:
```bash
echo 'export DISPLAY=$(cat /etc/resolv.conf | grep nameserver | awk '{print $2}'):0' >> ~/.bashrc
```

## Verification Checklist

Run these commands to verify everything is installed:

```bash
# .NET SDK
dotnet --version          # Should show 8.0.xxx

# Meadow CLI
meadow --version          # Should show version

# X11 (optional test)
echo $DISPLAY             # Should show :0 or hostname:0
```

## Troubleshooting

### "dotnet: command not found"

PATH issue. Try:
```bash
export PATH="$PATH:/usr/share/dotnet"
echo 'export PATH="$PATH:/usr/share/dotnet"' >> ~/.bashrc
source ~/.bashrc
```

### "meadow: command not found"

Global tools PATH issue:
```bash
export PATH="$PATH:$HOME/.dotnet/tools"
echo 'export PATH="$PATH:$HOME/.dotnet/tools"' >> ~/.bashrc
source ~/.bashrc
```

### Package repository errors

```bash
# Remove old repository
sudo rm /etc/apt/sources.list.d/microsoft-prod.list

# Re-add
wget https://packages.microsoft.com/config/debian/12/packages-microsoft-prod.deb
sudo dpkg -i packages-microsoft-prod.deb
sudo apt update
```

### X11 doesn't work

1. **Windows 11**: WSLg is built-in, should work automatically
2. **Windows 10**: Install VcXsrv or Xming
3. **Set DISPLAY**: See X11 section above
4. **Firewall**: Ensure Windows Firewall allows X server

For more help, see [Troubleshooting Guide](06-troubleshooting.md#gui-doesnt-appear).

## Next Steps

.NET and tools are installed!

Next: [Provision Device with Meadow.Cloud](03-provision-device.md)
