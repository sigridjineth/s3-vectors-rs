#!/bin/bash
#
# Build binaries for multiple platforms locally
# Requires: cargo, cross (for cross-compilation)
#
# Install cross: cargo install cross

set -e

VERSION="${1:-v0.1.0}"
BINARY_NAME="s3-vectors"

echo "Building S3 Vectors CLI ${VERSION} for multiple platforms..."

# Create release directory
mkdir -p release

# Function to build for a target
build_target() {
    local target=$1
    local output_name=$2
    local use_cross=${3:-false}
    
    echo "Building for ${target}..."
    
    if [ "$use_cross" = "true" ]; then
        cross build --release --target "$target"
    else
        cargo build --release --target "$target"
    fi
    
    # Copy binary
    if [[ "$target" == *"windows"* ]]; then
        cp "target/${target}/release/${BINARY_NAME}.exe" "release/${output_name}.exe"
        # Create checksum
        shasum -a 256 "release/${output_name}.exe" > "release/${output_name}.exe.sha256"
    else
        cp "target/${target}/release/${BINARY_NAME}" "release/${output_name}"
        # Create checksum
        shasum -a 256 "release/${output_name}" > "release/${output_name}.sha256"
    fi
    
    echo "✓ Built ${output_name}"
}

# Build for current platform (macOS ARM64 in your case)
build_target "aarch64-apple-darwin" "s3-vectors-darwin-aarch64"

# Build for macOS Intel (if on ARM Mac)
build_target "x86_64-apple-darwin" "s3-vectors-darwin-x86_64"

# For Linux targets, you'll need cross
# Uncomment these if you have cross installed:
# build_target "x86_64-unknown-linux-gnu" "s3-vectors-linux-x86_64" true
# build_target "aarch64-unknown-linux-gnu" "s3-vectors-linux-aarch64" true

# For Windows, you'll need cross
# build_target "x86_64-pc-windows-msvc" "s3-vectors-windows-x86_64" true

echo ""
echo "Build complete! Binaries are in ./release/"
echo ""
echo "To upload to GitHub:"
echo "1. Create a release on GitHub"
echo "2. Upload all files from ./release/"
echo ""
echo "Or use GitHub CLI:"
echo "gh release create ${VERSION} ./release/*"