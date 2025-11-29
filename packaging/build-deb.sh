#!/bin/bash
set -e

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DAEMON_DIR="$REPO_ROOT/Source/mc-daemon"
TEMPLATE_DIR="$SCRIPT_DIR/debian-template"
BUILD_DIR="$SCRIPT_DIR/build"

# Linux-native staging root (guaranteed ext4 under WSL)
STAGING_ROOT="/tmp/mc-daemon-deb-staging"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Parse command line arguments
ARCHITECTURES=("amd64" "arm64")
CLEAN_BUILD=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --arch)
            ARCHITECTURES=("$2")
            shift 2
            ;;
        --clean)
            CLEAN_BUILD=true
            shift
            ;;
        --help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --arch ARCH    Build for specific architecture (amd64 or arm64)"
            echo "  --clean        Clean build directories before building"
            echo "  --help         Show this help message"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Extract version from Cargo.toml
VERSION=$(grep '^version' "$DAEMON_DIR/Cargo.toml" | head -1 | sed 's/version = "\(.*\)"/\1/')
echo -e "${GREEN}Building mc-daemon version $VERSION${NC}"

# Clean previous builds if requested
if [ "$CLEAN_BUILD" = true ]; then
    echo -e "${YELLOW}Cleaning previous builds...${NC}"
    rm -rf "$BUILD_DIR"
    rm -rf "$STAGING_ROOT"
    cd "$DAEMON_DIR"
    cargo clean
fi

# Create build directories
mkdir -p "$BUILD_DIR"
mkdir -p "$STAGING_ROOT"

# Build function
build_for_arch() {
    local ARCH=$1
    local RUST_TARGET=""

    echo ""
    echo -e "${GREEN}========================================${NC}"
    echo -e "${GREEN}Building for architecture: $ARCH${NC}"
    echo -e "${GREEN}========================================${NC}"

    # Map Debian architecture to Rust target triple
    case $ARCH in
        amd64)
            RUST_TARGET="x86_64-unknown-linux-gnu"
            ;;
        arm64)
            RUST_TARGET="aarch64-unknown-linux-gnu"
            ;;
        *)
            echo -e "${RED}Unsupported architecture: $ARCH${NC}"
            exit 1
            ;;
    esac

    # Install Rust target if not already installed
    echo "Ensuring Rust target $RUST_TARGET is installed..."
    rustup target add $RUST_TARGET

    # Build the binary
    echo "Building mc-daemon for $RUST_TARGET..."
    cd "$DAEMON_DIR"

    # Set cross-compilation environment if needed
    if [ "$ARCH" = "arm64" ]; then
        # Install cross-compilation toolchain if not present
        if ! command -v aarch64-linux-gnu-gcc &> /dev/null; then
            echo -e "${YELLOW}Warning: aarch64-linux-gnu-gcc not found. Install with:${NC}"
            echo "  sudo apt-get install gcc-aarch64-linux-gnu"
            echo -e "${YELLOW}Attempting build anyway...${NC}"
        fi

        export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc
        export CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc
        export AR_aarch64_unknown_linux_gnu=aarch64-linux-gnu-ar
    fi

    cargo build --release --target $RUST_TARGET

    # Staging/package directories
    # PACKAGE_DIR on (possibly Windows) script dir, for your reference
    PACKAGE_DIR="$BUILD_DIR/mc-daemon-${VERSION}-${ARCH}"
    echo "Preparing package directory (source) in $PACKAGE_DIR..."
    rm -rf "$PACKAGE_DIR"
    mkdir -p "$PACKAGE_DIR"

    # Copy template structure to PACKAGE_DIR
    cp -r "$TEMPLATE_DIR"/* "$PACKAGE_DIR/"

    # Copy binary
    mkdir -p "$PACKAGE_DIR/usr/bin"
    cp "$DAEMON_DIR/target/$RUST_TARGET/release/mc-daemon" "$PACKAGE_DIR/usr/bin/"
    chmod 755 "$PACKAGE_DIR/usr/bin/mc-daemon"

    # Strip binary to reduce size
    echo "Stripping binary..."
    if [ "$ARCH" = "arm64" ]; then
        aarch64-linux-gnu-strip "$PACKAGE_DIR/usr/bin/mc-daemon" || echo "Warning: strip failed"
    else
        strip "$PACKAGE_DIR/usr/bin/mc-daemon" || echo "Warning: strip failed"
    fi

    # Update control file with correct architecture and version
    sed -i "s/ARCHITECTURE_PLACEHOLDER/$ARCH/g" "$PACKAGE_DIR/DEBIAN/control"
    sed -i "s/Version: .*/Version: $VERSION/g" "$PACKAGE_DIR/DEBIAN/control"

    # Now create a Linux-native staging copy under /tmp
    STAGING_DIR="$STAGING_ROOT/mc-daemon-${VERSION}-${ARCH}"
    echo "Creating Linux-native staging directory in $STAGING_DIR..."
    rm -rf "$STAGING_DIR"
    mkdir -p "$STAGING_DIR"

    # rsync or cp -a to preserve modes; either is fine on ext4
    # Using cp -a to keep it simple:
    cp -a "$PACKAGE_DIR/." "$STAGING_DIR/"

    # Ensure directory and DEBIAN scripts have correct permissions in staging
    chmod 755 "$STAGING_DIR"
    chmod 755 "$STAGING_DIR/DEBIAN"
    find "$STAGING_DIR/DEBIAN" -type f -exec chmod 755 {} \;

    # Create .deb package from the staging dir (Linux-native)
    DEB_FILE="$BUILD_DIR/mc-daemon_${VERSION}_${ARCH}.deb"
    echo "Building .deb package from staging dir..."
    echo "dpkg-deb --build \"$STAGING_DIR\" \"$DEB_FILE\""
    dpkg-deb --build "$STAGING_DIR" "$DEB_FILE"

    # Verify package
    echo ""
    echo -e "${GREEN}Package built successfully:${NC}"
    echo "  File: $DEB_FILE"
    echo "  Size: $(du -h "$DEB_FILE" | cut -f1)"
    echo ""
    echo "Package contents:"
    dpkg-deb --contents "$DEB_FILE"
    echo ""
    echo "Package info:"
    dpkg-deb --info "$DEB_FILE"

    echo -e "${GREEN}✓ Build complete for $ARCH${NC}"
}

# Build for each architecture
for ARCH in "${ARCHITECTURES[@]}"; do
    build_for_arch "$ARCH"
done

echo ""
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}All builds complete!${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""
echo "Packages created in: $BUILD_DIR"
ls -lh "$BUILD_DIR"/*.deb
echo ""
echo "Next steps:"
echo "1. Test packages: sudo dpkg -i $BUILD_DIR/mc-daemon_${VERSION}_<arch>.deb"
echo "2. Update repository: ./update-repo.sh"
echo "3. Publish to GitHub Pages: ./publish-to-gh-pages.sh"
