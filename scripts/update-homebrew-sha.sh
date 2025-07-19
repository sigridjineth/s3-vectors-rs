#!/bin/bash
#
# Update Homebrew formula with actual SHA256 checksums from a release
# Usage: ./scripts/update-homebrew-sha.sh v1.0.0

set -e

VERSION="${1:-latest}"
FORMULA_FILE="homebrew-formula.rb"

if [ ! -f "$FORMULA_FILE" ]; then
    echo "Error: $FORMULA_FILE not found"
    exit 1
fi

echo "Fetching SHA256 checksums for version: $VERSION"

# Function to get SHA256 for a specific asset
get_sha256() {
    local asset_name="$1"
    local url="https://github.com/USER/s3-vectors-rust/releases/download/${VERSION}/${asset_name}.sha256"
    
    echo "Fetching: $url"
    local sha=$(curl -sSL "$url" 2>/dev/null | awk '{print $1}')
    
    if [ -z "$sha" ]; then
        echo "Warning: Could not fetch SHA256 for $asset_name"
        return 1
    fi
    
    echo "  SHA256: $sha"
    echo "$sha"
}

# Update formula with actual SHA256 values
echo "Updating Homebrew formula..."

# Darwin x86_64
if sha=$(get_sha256 "s3-vectors-darwin-x86_64"); then
    sed -i.bak "s/PLACEHOLDER_SHA256_DARWIN_X86_64/$sha/g" "$FORMULA_FILE"
fi

# Darwin aarch64
if sha=$(get_sha256 "s3-vectors-darwin-aarch64"); then
    sed -i.bak "s/PLACEHOLDER_SHA256_DARWIN_AARCH64/$sha/g" "$FORMULA_FILE"
fi

# Linux x86_64
if sha=$(get_sha256 "s3-vectors-linux-x86_64"); then
    sed -i.bak "s/PLACEHOLDER_SHA256_LINUX_X86_64/$sha/g" "$FORMULA_FILE"
fi

# Linux aarch64
if sha=$(get_sha256 "s3-vectors-linux-aarch64"); then
    sed -i.bak "s/PLACEHOLDER_SHA256_LINUX_AARCH64/$sha/g" "$FORMULA_FILE"
fi

# Update version in formula
VERSION_NUM="${VERSION#v}"  # Remove 'v' prefix if present
sed -i.bak "s/version \".*\"/version \"$VERSION_NUM\"/g" "$FORMULA_FILE"

# Clean up backup
rm -f "${FORMULA_FILE}.bak"

echo "Done! Homebrew formula updated with actual checksums."
echo "Don't forget to test the formula locally:"
echo "  brew install --build-from-source ./homebrew-formula.rb"