# Meadow.Daemon Packaging

This directory contains the infrastructure for building Debian packages and hosting them via APT on GitHub Pages.

## Quick Start

### Prerequisites

```bash
# Install cross-compilation tools (for ARM64 builds)
sudo apt-get install gcc-aarch64-linux-gnu g++-aarch64-linux-gnu dpkg-dev

# Install Rust targets
rustup target add x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu
```

### Build Packages

```bash
# Build both architectures
./build-deb.sh

# Build specific architecture
./build-deb.sh --arch amd64

# Clean build
./build-deb.sh --clean
```

Output: `build/mc-daemon_{version}_{arch}.deb`

### Test Locally

```bash
# Install package
sudo dpkg -i build/mc-daemon_0.9.0_amd64.deb

# Check status
systemctl status mc-daemon

# View logs
journalctl -u mc-daemon -f

# Test API
curl http://127.0.0.1:5000/api/info

# Uninstall
sudo apt purge mc-daemon
```

### Publish to GitHub Pages

```bash
# 1. Update repository metadata
./update-repo.sh

# 2. Publish to GitHub Pages
./publish-to-gh-pages.sh
```

## Directory Structure

```
packaging/
├── build-deb.sh                 # Cross-compile and build .deb packages
├── update-repo.sh               # Generate APT repository metadata
├── publish-to-gh-pages.sh       # Publish to GitHub Pages
├── README.md                    # This file
├── debian-template/             # Package template
│   ├── DEBIAN/
│   │   ├── control              # Package metadata
│   │   ├── postinst             # Post-install (setup directories, start service)
│   │   ├── prerm                # Pre-removal (stop service)
│   │   └── postrm               # Post-removal (cleanup)
│   ├── usr/bin/                 # Binary installation path
│   ├── etc/
│   │   ├── meadow.conf          # Default config
│   │   └── systemd/system/
│   │       └── mc-daemon.service # Systemd unit file
│   └── opt/meadow/              # Application root
├── build/                       # Build output (gitignored)
│   ├── mc-daemon_0.9.0_amd64.deb
│   ├── mc-daemon_0.9.0_arm64.deb
│   └── mc-daemon-{version}-{arch}/ (temp package dirs)
└── gh-pages-repo/               # Local clone of gh-pages (gitignored)
    ├── apt/
    │   ├── pool/main/           # .deb files
    │   └── dists/stable/main/   # Repository metadata
    ├── index.html
    └── .nojekyll
```

## Release Workflow

### 1. Prepare Release

```bash
cd ../Source/mc-daemon

# Update version in Cargo.toml
nano Cargo.toml
# Change: version = "0.10.0"

# Commit version bump
git add Cargo.toml
git commit -m "Bump version to 0.10.0"
git push origin develop
```

### 2. Build Packages

```bash
cd ../../packaging

# Clean build for release
./build-deb.sh --clean
```

### 3. Test Packages

```bash
# Test on AMD64 system
sudo dpkg -i build/mc-daemon_0.10.0_amd64.deb

# Verify
systemctl status mc-daemon
curl http://127.0.0.1:5000/api/info
journalctl -u mc-daemon -n 20

# Clean up
sudo apt purge mc-daemon
```

### 4. Update Repository

```bash
./update-repo.sh
```

This creates/updates `gh-pages-repo/` with:
- `.deb` files in `apt/pool/main/`
- Package indexes in `apt/dists/stable/main/binary-{arch}/`
- Checksums in Release files

### 5. Publish

```bash
./publish-to-gh-pages.sh
```

This pushes to the `gh-pages` branch on GitHub.

**Wait 2-5 minutes** for GitHub Pages to deploy.

### 6. Verify Publication

```bash
# Check GitHub Pages is live
curl -I https://wildernesslabs.github.io/Meadow.Daemon/

# Check package metadata
curl https://wildernesslabs.github.io/Meadow.Daemon/apt/dists/stable/main/binary-amd64/Packages

# Test installation on fresh system
echo "deb [trusted=yes] https://wildernesslabs.github.io/Meadow.Daemon/apt stable main" | sudo tee /etc/apt/sources.list.d/meadow-daemon.list
sudo apt update
sudo apt install mc-daemon
```

### 7. Create Git Tag

```bash
cd ..
git tag -a v0.10.0 -m "Release version 0.10.0"
git push origin v0.10.0
```

### 8. Create GitHub Release (Optional)

1. Go to: https://github.com/WildernessLabs/Meadow.Daemon/releases/new
2. Tag: `v0.10.0`
3. Title: `mc-daemon 0.10.0`
4. Description: Release notes
5. Attach: `build/mc-daemon_0.10.0_amd64.deb` and `build/mc-daemon_0.10.0_arm64.deb`
6. Publish

## GitHub Pages Setup (One-Time)

### 1. Create gh-pages Branch

```bash
cd ..
git checkout --orphan gh-pages
git rm -rf .
mkdir -p apt/pool/main apt/dists/stable/main
touch .nojekyll
echo "# Meadow Daemon APT Repository" > README.md
git add .
git commit -m "Initial gh-pages setup"
git push origin gh-pages
git checkout develop
```

### 2. Enable GitHub Pages

1. Go to repository Settings on GitHub
2. Navigate to "Pages"
3. Source: `gh-pages` branch, `/ (root)` folder
4. Save

URL will be: https://wildernesslabs.github.io/Meadow.Daemon/

## Troubleshooting

### Cross-Compilation Fails for ARM64

```bash
# Install cross-compilation toolchain
sudo apt-get install gcc-aarch64-linux-gnu g++-aarch64-linux-gnu

# If OpenSSL fails, it should auto-build with vendored feature
# Check Cargo.toml has: openssl = { version = "0.10", features = ["vendored"] }
```

### dpkg-scanpackages Not Found

```bash
sudo apt-get install dpkg-dev
```

### gh-pages Push Fails

```bash
# Check remote URL
cd gh-pages-repo
git remote -v

# Re-add remote if needed
git remote set-url origin https://github.com/WildernessLabs/Meadow.Daemon.git

# Force push if needed (be careful!)
git push -f origin gh-pages
```

### GitHub Pages Not Updating

- GitHub Pages can take 2-5 minutes to deploy
- Check GitHub Actions tab for build status
- Ensure gh-pages branch is set as source in Settings → Pages

## File Ownership & Permissions

All package files are owned by root:

- Binary: `/usr/bin/mc-daemon` (755)
- Config: `/etc/meadow.conf` (644)
- Service: `/etc/systemd/system/mc-daemon.service` (644)
- Scripts: `/DEBIAN/{postinst,prerm,postrm}` (755)

The service runs as root and creates:
- `/opt/meadow` (755, root:root)
- `/tmp/meadow/*` (755, root:root)
- `/root/.ssh/id_rsa` (600, root:root)

## Version Management

Version is extracted from `Source/mc-daemon/Cargo.toml`:
```toml
[package]
version = "0.9.0"
```

This version is automatically used in:
- Package filename: `mc-daemon_{version}_{arch}.deb`
- Control file: `Version: {version}`

## Supported Architectures

- **amd64** (x86_64) - Rust target: `x86_64-unknown-linux-gnu`
- **arm64** (aarch64) - Rust target: `aarch64-unknown-linux-gnu`

## Security Notes

- Packages are **unsigned** (users must use `[trusted=yes]`)
- GitHub Pages serves over HTTPS (safe for package integrity)
- To add GPG signing in the future, see: https://wiki.debian.org/DebianRepository/Setup

## Repository Size Management

GitHub Pages has a 1GB soft limit. To keep repository size small:

```bash
# Remove old packages from pool
cd gh-pages-repo/apt/pool/main
rm mc-daemon_0.8.0_*.deb

# Regenerate metadata
cd ../../../../
./update-repo.sh
./publish-to-gh-pages.sh
```

Keep only the latest 2-3 versions in the repository.

## Support

For issues with packaging:
- Check build logs: `./build-deb.sh 2>&1 | tee build.log`
- Test package: `dpkg-deb --contents build/mc-daemon_*.deb`
- Validate control: `dpkg-deb --info build/mc-daemon_*.deb`

For daemon issues:
- See: `Doc/apt-installation.md`
- Logs: `journalctl -u mc-daemon -f`
