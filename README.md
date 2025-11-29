This branch hosts the APT repository for mc-daemon.

Visit: https://wildernesslabs.github.io/Meadow.Daemon/

## Users

Add repository:

Currently this package is not GPG signed (we're working that direction) so you must set the `trusted` param.

To install on your Debian-based machine: 

```bash
echo "deb [trusted=yes] https://wildernesslabs.github.io/Meadow.Daemon/apt stable main" | sudo tee /etc/apt/sources.list.d/meadow-daemon.list
sudo apt update
sudo apt install mc-daemon
```

## Maintainers

Do not edit this branch directly. Use the packaging scripts from the develop branch:
1. ./packaging/build-deb.sh - Build packages
2. ./packaging/update-repo.sh - Update repository metadata
3. ./packaging/publish-to-gh-pages.sh - Publish to GitHub Pages