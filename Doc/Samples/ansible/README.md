# Ansible Deployment for Meadow.Daemon

Automated deployment of Meadow.Daemon and .NET 8 runtime to Debian-based systems using Ansible.

## Overview

This playbook automates the entire setup process documented in the [Avalonia OtA guide](../Avalonia%20OtA/README.md), including:

- ✅ Architecture detection (ARM64 for Raspberry Pi, AMD64 for x86-64)
- ✅ .NET 8 runtime installation
- ✅ mc-daemon binary download and installation
- ✅ systemd service configuration
- ✅ Directory structure setup
- ✅ Configuration file deployment

## Supported Platforms

### Tested Platforms
- **Raspberry Pi** (Debian Trixie Lite, 64-bit) - ARM64
- **Standard PC/Server** (Debian 12+) - AMD64

### Requirements
- Debian 12 (Bookworm) or newer
- 64-bit architecture (ARM64 or AMD64)
- systemd-based system
- SSH access with sudo privileges

## Prerequisites

### On Your Control Machine (where you run Ansible)

1. **Install Ansible**
   ```bash
   # On Ubuntu/Debian
   sudo apt update
   sudo apt install ansible

   # On macOS
   brew install ansible

   # On Windows (WSL)
   sudo apt update 
   sudo apt install ansible
   ```

2. **Verify Ansible installation**
   ```bash
   ansible --version
   # Should show version 2.9+ (2.15+ recommended)
   ```

3. **SSH access to target devices**
   ```bash
   # Generate SSH key if you don't have one
   ssh-keygen -t ed25519 -C "your_email@example.com"

   # Copy your public key to the target device
   ssh-copy-id pi@192.168.1.100  # Replace with your device's IP

   # Test SSH connection
   ssh pi@192.168.1.100
   ```

### On Target Devices

- **SSH server running**: Usually pre-configured on Raspberry Pi OS
- **User with sudo privileges**: Default `pi` user on Raspberry Pi has this
- **Network connectivity**: Device can reach internet to download packages

## Quick Start

### 1. Clone or Navigate to This Directory

```bash
cd Meadow.Daemon/Doc/Samples/ansible/
```

### 2. Create Your Inventory File

```bash
# Copy the example inventory
cp inventory.example.yaml inventory.yaml

# Edit with your device details
nano inventory.yaml
```

**Example inventory.yaml**:
```yaml
all:
  children:
    raspberry_pi:
      hosts:
        pi1:
          ansible_host: 192.168.1.100  # Your Pi's IP
          ansible_user: pi              # Your SSH user
```

### 3. Test Connectivity

```bash
ansible -i inventory.yaml raspberry_pi -m ping
```

**Expected output**:
```
pi1 | SUCCESS => {
    "changed": false,
    "ping": "pong"
}
```

### 4. Run the Playbook

```bash
ansible-playbook -i inventory.yaml mc-daemon-playbook.yaml
```

The playbook will:
1. Detect the system architecture
2. Install .NET 8 runtime
3. Download the appropriate mc-daemon binary
4. Create directory structure
5. Deploy configuration files
6. Set up and start the systemd service
7. Verify the installation

**Expected runtime**: 5-10 minutes (depending on network speed)

### 5. Verify Installation

After the playbook completes, SSH into your device:

```bash
ssh pi@192.168.1.100
```

Check the daemon status:
```bash
systemctl status mc-daemon
```

Test the API:
```bash
curl http://127.0.0.1:5000/api/info
```

## Configuration

### Playbook Variables

You can customize these variables in the playbook or via inventory:

| Variable | Default | Description |
|----------|---------|-------------|
| `meadow_user` | `{{ ansible_user }}` | User that will run the app |
| `meadow_group` | `{{ ansible_user }}` | Group for app files |
| `meadow_root` | `/opt/meadow` | App deployment directory |
| `meadow_temp_base` | `/home/{{ meadow_user }}/.meadow` | Temp directory base path |
| `app_service_name` | `meadowapp.service` | Your app's systemd service name |

### Override Variables in Inventory

```yaml
all:
  children:
    raspberry_pi:
      hosts:
        pi1:
          ansible_host: 192.168.1.100
          ansible_user: pi
      vars:
        # Override defaults for all Raspberry Pis
        meadow_user: meadowapp
        app_service_name: myapp.service
```

### Override Variables at Runtime

```bash
ansible-playbook -i inventory.yaml mc-daemon-playbook.yaml \
  -e "meadow_user=myuser" \
  -e "app_service_name=customapp.service"
```

## Architecture Detection

The playbook automatically detects the system architecture:

- **ARM64** (`aarch64`) → Downloads `mc-daemon-arm64`
- **AMD64** (`x86_64`) → Downloads `mc-daemon-amd64`

You can verify detection during playbook execution:
```
TASK [Display detected architecture]
ok: [pi1] => {
    "msg": "Detected architecture: aarch64 -> mc-daemon-arm64"
}
```

## Directory Structure Created

```
/etc/
├── meadow.conf                    # Daemon configuration
└── systemd/system/
    └── mc-daemon.service          # Daemon systemd service

/usr/bin/
└── mc-daemon                      # Daemon binary

/opt/meadow/                       # App deployment (empty after daemon install)

/home/pi/.meadow/tmp/              # Daemon working directory
├── updates/                       # Downloaded MPAKs
├── update/                        # Extraction staging
├── staging/                       # Pre-deployment staging
└── rollback/                      # Backup of previous version
```

## Troubleshooting

### WSL: World Writable Directory Warning

**Warning**: `Ansible is being run in a world writable directory`

This happens when running Ansible from a Windows filesystem mounted in WSL (like `/mnt/f/`). The warning is harmless but Ansible will ignore `ansible.cfg` from that directory.

**Solution 1: Copy to Linux filesystem** (Recommended)
```bash
# Copy the ansible directory to your WSL home directory
cp -r "/mnt/f/repos/wilderness/Meadow.Daemon/Doc/Samples/ansible" ~/meadow-ansible
cd ~/meadow-ansible

# Now run Ansible from here
ansible-playbook -i inventory.yaml mc-daemon-playbook.yaml
```

**Solution 2: Set environment variable**
```bash
# Stay in the Windows filesystem but explicitly set config path
export ANSIBLE_CONFIG="$(pwd)/ansible.cfg"
ansible-playbook -i inventory.yaml mc-daemon-playbook.yaml
```

**Solution 3: Use command-line options**
```bash
# Override settings directly on command line
ANSIBLE_HOST_KEY_CHECKING=False ansible-playbook -i inventory.yaml mc-daemon-playbook.yaml
```

### Connection Issues

**Error**: `Failed to connect to the host via ssh`

```bash
# Verify SSH access manually
ssh pi@192.168.1.100

# Check SSH key permissions
chmod 600 ~/.ssh/id_rsa
chmod 644 ~/.ssh/id_rsa.pub

# Add SSH key to agent
ssh-add ~/.ssh/id_rsa
```

### Permission Denied (sudo)

**Error**: `Missing sudo password`

```bash
# Run with password prompt
ansible-playbook -i inventory.yaml mc-daemon-playbook.yaml --ask-become-pass

# Or add to inventory:
# ansible_become_password: your_sudo_password  # Not recommended for production!
```

### Architecture Not Detected

**Error**: `daemon_asset.browser_download_url is undefined`

The playbook expects `aarch64` (ARM64) or `x86_64` (AMD64). Check:
```bash
ansible -i inventory.yaml raspberry_pi -m setup -a "filter=ansible_architecture"
```

### .NET Installation Fails

**Error**: `No package matching 'dotnet-runtime-8.0' is available`

This error occurred in older versions that tried to use the Microsoft APT repository. The current playbook uses Microsoft's official `dotnet-install.sh` script which works reliably across all Debian versions including Trixie.

The playbook now:
- Downloads the official dotnet-install script
- Installs ASP.NET Core Runtime 8.0.22
- Installs to `~/.dotnet` (user-local installation)
- Configures environment variables in `.bashrc`

**Issue**: wget fails to download

```bash
# SSH to the device and test manually
wget --inet4-only https://dot.net/v1/dotnet-install.sh

# If this fails, check internet connectivity:
ping -c 3 8.8.8.8
ping -c 3 dot.net
```

### Service Won't Start

```bash
# SSH to device and check logs
ssh pi@192.168.1.100
sudo journalctl -u mc-daemon -n 50

# Common issues:
# - /etc/meadow.conf syntax error
# - Permission issues with directories
# - Port 5000 already in use
```

### Verify Directory Permissions

```bash
# On the target device
ls -la /opt/meadow
ls -la ~/.meadow/tmp

# Fix permissions if needed
sudo chown -R pi:pi /opt/meadow
chown -R pi:pi ~/.meadow
```

## Next Steps After Installation

1. **Provision Device with Meadow.Cloud**
   - Follow [Step 3: Provision Device](../Avalonia%20OtA/03-provision-device.md)
   - Generate SSH keys and register with Meadow.Cloud

2. **Deploy Your Application**
   - Build your .NET application
   - Create the `meadowapp.service` systemd file
   - Deploy your app to `/opt/meadow/`

3. **Test OTA Updates**
   - Follow [Step 5: Perform OTA Update](../Avalonia%20OtA/05-perform-update.md)

## Advanced Usage

### Deploy to Multiple Devices

```yaml
# inventory.yaml
all:
  children:
    raspberry_pi:
      hosts:
        pi1:
          ansible_host: 192.168.1.100
        pi2:
          ansible_host: 192.168.1.101
        pi3:
          ansible_host: 192.168.1.102
      vars:
        ansible_user: pi
```

Run playbook against all devices:
```bash
ansible-playbook -i inventory.yaml mc-daemon-playbook.yaml
```

### Limit Execution to Specific Hosts

```bash
# Only deploy to pi1
ansible-playbook -i inventory.yaml mc-daemon-playbook.yaml --limit pi1

# Deploy to multiple specific hosts
ansible-playbook -i inventory.yaml mc-daemon-playbook.yaml --limit pi1,pi2
```

### Dry Run (Check Mode)

```bash
# See what would change without making changes
ansible-playbook -i inventory.yaml mc-daemon-playbook.yaml --check
```

### Verbose Output

```bash
# Show detailed execution info
ansible-playbook -i inventory.yaml mc-daemon-playbook.yaml -v

# Even more verbose (debug level)
ansible-playbook -i inventory.yaml mc-daemon-playbook.yaml -vvv
```

## Files in This Directory

- **mc-daemon-playbook.yaml**: Main Ansible playbook
- **inventory.example.yaml**: Example inventory file
- **templates/mc-daemon.service.j2**: systemd service template
- **templates/meadow.conf.j2**: Daemon configuration template
- **README.md**: This file

## Support

- **Meadow.Daemon Issues**: [GitHub Issues](https://github.com/WildernessLabs/Meadow.Daemon/issues)
- **Manual Installation**: See [Avalonia OtA Guide](../Avalonia%20OtA/README.md)
- **Ansible Documentation**: [docs.ansible.com](https://docs.ansible.com/)

## Contributing

Improvements and tested configurations for other platforms are welcome!

## License

Same as Meadow.Daemon project.
