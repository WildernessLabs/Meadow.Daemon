#!/bin/bash
set -e

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$SCRIPT_DIR/gh-pages-repo"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${GREEN}Publishing APT repository to GitHub Pages...${NC}"

# Check if repository directory exists
if [ ! -d "$REPO_DIR" ]; then
    echo -e "${RED}Error: Repository directory not found. Run update-repo.sh first.${NC}"
    exit 1
fi

cd "$REPO_DIR"

# Check if there are changes
if git diff --quiet && git diff --cached --quiet; then
    echo -e "${YELLOW}No changes to publish.${NC}"
    exit 0
fi

# Create .nojekyll to prevent GitHub Pages from processing
touch .nojekyll

# Create index.html for documentation
cat > index.html <<'EOF'
<!DOCTYPE html>
<html>
<head>
    <title>Meadow Daemon APT Repository</title>
    <style>
        body {
            font-family: Arial, sans-serif;
            max-width: 800px;
            margin: 50px auto;
            padding: 20px;
            line-height: 1.6;
        }
        code {
            background: #f4f4f4;
            padding: 2px 6px;
            border-radius: 3px;
        }
        pre {
            background: #f4f4f4;
            padding: 15px;
            border-radius: 5px;
            overflow-x: auto;
        }
        h1 { color: #333; }
        h2 { color: #666; margin-top: 30px; }
    </style>
</head>
<body>
    <h1>Meadow Daemon APT Repository</h1>

    <p>This is the APT repository for <strong>mc-daemon</strong>, a Linux service providing OTA updates and Meadow.Cloud connectivity.</p>

    <h2>Installation</h2>

    <p>Add the repository to your system:</p>
    <pre><code>echo "deb [trusted=yes] https://wildernesslabs.github.io/Meadow.Daemon/apt stable main" | sudo tee /etc/apt/sources.list.d/meadow-daemon.list
sudo apt update</code></pre>

    <p>Install mc-daemon:</p>
    <pre><code>sudo apt install mc-daemon</code></pre>

    <h2>Supported Architectures</h2>
    <ul>
        <li>AMD64 (x86_64)</li>
        <li>ARM64 (aarch64)</li>
    </ul>

    <h2>Post-Installation</h2>

    <p>After installation, the service will start automatically. Configuration file is located at:</p>
    <pre><code>/etc/meadow.conf</code></pre>

    <p>View service status:</p>
    <pre><code>systemctl status mc-daemon</code></pre>

    <p>View logs:</p>
    <pre><code>journalctl -u mc-daemon -f</code></pre>

    <h2>Links</h2>
    <ul>
        <li><a href="https://github.com/WildernessLabs/Meadow.Daemon">GitHub Repository</a></li>
        <li><a href="https://github.com/WildernessLabs/Meadow.Daemon/blob/main/README.md">Documentation</a></li>
    </ul>
</body>
</html>
EOF

# Stage all changes
git add -A

# Create commit
COMMIT_MSG="Update APT repository - $(date '+%Y-%m-%d %H:%M:%S')"
echo "Creating commit: $COMMIT_MSG"
git commit -m "$COMMIT_MSG" || {
    echo -e "${YELLOW}No changes to commit.${NC}"
    exit 0
}

# Push to GitHub
echo "Pushing to GitHub..."
git push origin gh-pages

echo ""
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}Published successfully!${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""
echo "GitHub Pages URL: https://wildernesslabs.github.io/Meadow.Daemon/"
echo "APT Repository: https://wildernesslabs.github.io/Meadow.Daemon/apt"
echo ""
echo "Note: GitHub Pages may take a few minutes to update."
echo ""
echo "Users can now install with:"
echo '  echo "deb [trusted=yes] https://wildernesslabs.github.io/Meadow.Daemon/apt stable main" | sudo tee /etc/apt/sources.list.d/meadow-daemon.list'
echo "  sudo apt update"
echo "  sudo apt install mc-daemon"
