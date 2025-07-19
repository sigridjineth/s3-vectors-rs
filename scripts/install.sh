#!/bin/bash
#
# S3 Vectors CLI Installation Script
# 
# This script downloads and installs the S3 Vectors CLI tool
# Usage: curl -sSL https://raw.githubusercontent.com/USER/s3-vectors-rust/main/install.sh | bash
#

set -e

# Configuration
REPO_OWNER="${REPO_OWNER:-USER}"
REPO_NAME="${REPO_NAME:-s3-vectors-rust}"
BINARY_NAME="s3-vectors"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Helper functions
print_error() {
    echo -e "${RED}Error: $1${NC}" >&2
}

print_success() {
    echo -e "${GREEN}$1${NC}"
}

print_info() {
    echo -e "${BLUE}$1${NC}"
}

print_warning() {
    echo -e "${YELLOW}$1${NC}"
}

# Detect OS and architecture
detect_platform() {
    local os arch

    # Detect OS
    case "$(uname -s)" in
        Linux*)     os="linux";;
        Darwin*)    os="darwin";;
        CYGWIN*|MINGW*|MSYS*) os="windows";;
        *)          print_error "Unsupported operating system: $(uname -s)"; exit 1;;
    esac

    # Detect architecture
    case "$(uname -m)" in
        x86_64|amd64)       arch="x86_64";;
        aarch64|arm64)      arch="aarch64";;
        armv7l|armv7)       arch="armv7";;
        *)                  print_error "Unsupported architecture: $(uname -m)"; exit 1;;
    esac

    echo "${os}-${arch}"
}

# Get the latest release version
get_latest_version() {
    local api_url="https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/releases/latest"
    
    if command -v curl >/dev/null 2>&1; then
        curl -sSL "$api_url" | grep '"tag_name"' | sed -E 's/.*"([^"]+)".*/\1/'
    elif command -v wget >/dev/null 2>&1; then
        wget -qO- "$api_url" | grep '"tag_name"' | sed -E 's/.*"([^"]+)".*/\1/'
    else
        print_error "Neither curl nor wget found. Please install one of them."
        exit 1
    fi
}

# Download file
download_file() {
    local url="$1"
    local output="$2"
    
    if command -v curl >/dev/null 2>&1; then
        curl -sSL "$url" -o "$output"
    elif command -v wget >/dev/null 2>&1; then
        wget -q "$url" -O "$output"
    else
        print_error "Neither curl nor wget found. Please install one of them."
        exit 1
    fi
}

# Verify checksum
verify_checksum() {
    local file="$1"
    local checksum_url="$2"
    local expected_checksum
    
    # Download checksum file
    print_info "Verifying checksum..."
    download_file "$checksum_url" "${file}.sha256"
    
    # Extract expected checksum
    expected_checksum=$(cat "${file}.sha256" | awk '{print $1}')
    
    # Calculate actual checksum
    if command -v sha256sum >/dev/null 2>&1; then
        actual_checksum=$(sha256sum "$file" | awk '{print $1}')
    elif command -v shasum >/dev/null 2>&1; then
        actual_checksum=$(shasum -a 256 "$file" | awk '{print $1}')
    else
        print_warning "No checksum utility found. Skipping verification."
        rm -f "${file}.sha256"
        return 0
    fi
    
    # Compare checksums
    if [ "$expected_checksum" = "$actual_checksum" ]; then
        print_success "✓ Checksum verified"
        rm -f "${file}.sha256"
        return 0
    else
        print_error "Checksum verification failed!"
        print_error "Expected: $expected_checksum"
        print_error "Actual:   $actual_checksum"
        rm -f "${file}.sha256"
        return 1
    fi
}

# Main installation function
install_s3_vectors() {
    print_info "S3 Vectors CLI Installer"
    print_info "========================"
    echo
    
    # Detect platform
    local platform=$(detect_platform)
    print_info "Detected platform: $platform"
    
    # Get version
    local version="${VERSION:-$(get_latest_version)}"
    if [ -z "$version" ]; then
        print_error "Failed to determine latest version"
        exit 1
    fi
    print_info "Installing version: $version"
    
    # Construct download URL
    local binary_file="${BINARY_NAME}-${platform}"
    local download_url="https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/download/${version}/${binary_file}"
    local checksum_url="${download_url}.sha256"
    
    # Create temporary directory
    local temp_dir=$(mktemp -d)
    trap "rm -rf $temp_dir" EXIT
    
    # Download binary
    print_info "Downloading S3 Vectors CLI..."
    local temp_binary="${temp_dir}/${BINARY_NAME}"
    if ! download_file "$download_url" "$temp_binary"; then
        print_error "Failed to download binary from: $download_url"
        print_error "This might mean binaries for $platform are not available yet."
        exit 1
    fi
    
    # Verify checksum
    if ! verify_checksum "$temp_binary" "$checksum_url"; then
        print_error "Checksum verification failed. Aborting installation."
        exit 1
    fi
    
    # Make binary executable
    chmod +x "$temp_binary"
    
    # Create install directory if it doesn't exist
    if [ ! -d "$INSTALL_DIR" ]; then
        print_info "Creating install directory: $INSTALL_DIR"
        mkdir -p "$INSTALL_DIR"
    fi
    
    # Check if we can write to install directory
    if [ ! -w "$INSTALL_DIR" ]; then
        print_error "Cannot write to $INSTALL_DIR"
        print_info "Try running with sudo or set INSTALL_DIR to a writable location"
        exit 1
    fi
    
    # Install binary
    print_info "Installing to: $INSTALL_DIR/$BINARY_NAME"
    mv "$temp_binary" "$INSTALL_DIR/$BINARY_NAME"
    
    # Check if install directory is in PATH
    if ! echo "$PATH" | grep -q "$INSTALL_DIR"; then
        print_warning "⚠️  $INSTALL_DIR is not in your PATH"
        echo
        print_info "Add it to your PATH by adding this line to your shell profile:"
        echo
        echo "    export PATH=\"\$PATH:$INSTALL_DIR\""
        echo
        print_info "For bash: ~/.bashrc or ~/.bash_profile"
        print_info "For zsh:  ~/.zshrc"
        echo
    fi
    
    # Test installation
    if [ -x "$INSTALL_DIR/$BINARY_NAME" ]; then
        print_success "✓ S3 Vectors CLI installed successfully!"
        echo
        
        # Show version if binary is in PATH
        if command -v "$BINARY_NAME" >/dev/null 2>&1; then
            print_info "Installed version:"
            "$BINARY_NAME" --version || true
        else
            print_info "Run the following command to get started:"
            echo "    $INSTALL_DIR/$BINARY_NAME --help"
        fi
    else
        print_error "Installation failed"
        exit 1
    fi
}

# Run installation
install_s3_vectors