#!/bin/bash
set -e

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILD_DIR="$SCRIPT_DIR/build"
REPO_DIR="$SCRIPT_DIR/gh-pages-repo"
APT_DIR="$REPO_DIR/apt"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${GREEN}Updating APT repository metadata...${NC}"

# Check if build directory exists
if [ ! -d "$BUILD_DIR" ]; then
    echo "Error: Build directory not found. Run build-deb.sh first."
    exit 1
fi

# Check if .deb files exist
DEB_FILES=("$BUILD_DIR"/*.deb)
if [ ! -f "${DEB_FILES[0]}" ]; then
    echo "Error: No .deb files found in $BUILD_DIR"
    exit 1
fi

# Clone or update gh-pages branch
if [ ! -d "$REPO_DIR" ]; then
    echo "Cloning gh-pages branch..."
    cd "$SCRIPT_DIR/.."
    git clone -b gh-pages "$(git remote get-url origin)" "$REPO_DIR" || {
        echo "gh-pages branch doesn't exist yet. Creating..."
        mkdir -p "$REPO_DIR"
        cd "$REPO_DIR"
        git init
        git checkout -b gh-pages
        git remote add origin "$(cd "$SCRIPT_DIR/.." && git remote get-url origin)"
    }
else
    echo "Updating gh-pages repository..."
    cd "$REPO_DIR"
    git pull origin gh-pages || echo "No remote gh-pages branch yet"
fi

# Create APT repository structure
echo "Creating repository structure..."
mkdir -p "$APT_DIR/pool/main"
mkdir -p "$APT_DIR/dists/stable/main/binary-amd64"
mkdir -p "$APT_DIR/dists/stable/main/binary-arm64"

# Copy .deb files to pool
echo "Copying packages to pool..."
cp -v "$BUILD_DIR"/*.deb "$APT_DIR/pool/main/"

# Generate Packages files
generate_packages_file() {
    local ARCH=$1
    local BINARY_DIR="$APT_DIR/dists/stable/main/binary-$ARCH"

    echo "Generating Packages file for $ARCH..."

    cd "$APT_DIR"

    # Create Packages file
    dpkg-scanpackages --arch "$ARCH" pool/main /dev/null > "$BINARY_DIR/Packages"

    # Compress Packages file
    gzip -9 -c "$BINARY_DIR/Packages" > "$BINARY_DIR/Packages.gz"

    # Create Release file for binary directory
    cat > "$BINARY_DIR/Release" <<EOF
Archive: stable
Component: main
Architecture: $ARCH
EOF

    echo "✓ Generated Packages for $ARCH"
}

# Generate for both architectures
generate_packages_file "amd64"
generate_packages_file "arm64"

# Generate main Release file
echo "Generating main Release file..."
RELEASE_FILE="$APT_DIR/dists/stable/main/Release"

cat > "$RELEASE_FILE" <<EOF
Origin: Wilderness Labs
Label: Meadow Daemon
Suite: stable
Codename: stable
Architectures: amd64 arm64
Components: main
Description: Meadow Daemon APT Repository
Date: $(date -R)
EOF

# Generate checksums for Release file
cd "$APT_DIR/dists/stable/main"
{
    echo "MD5Sum:"
    find binary-* -type f | while read file; do
        md5sum "$file" | awk '{printf " %s %16d %s\n", $1, 0, $2}'
    done
    echo "SHA1:"
    find binary-* -type f | while read file; do
        sha1sum "$file" | awk '{printf " %s %16d %s\n", $1, 0, $2}'
    done
    echo "SHA256:"
    find binary-* -type f | while read file; do
        sha256sum "$file" | awk '{printf " %s %16d %s\n", $1, 0, $2}'
    done
} >> "$RELEASE_FILE"

# Update file sizes in Release file
cd "$APT_DIR/dists/stable/main"
find binary-* -type f | while read file; do
    size=$(stat -c%s "$file")
    sed -i "s|0 $file|$size $file|g" "$RELEASE_FILE"
done

echo ""
echo -e "${GREEN}Repository metadata updated successfully!${NC}"
echo ""
echo "Repository structure:"
tree "$APT_DIR" 2>/dev/null || find "$APT_DIR" -type f
echo ""
echo "Next step: ./publish-to-gh-pages.sh"
