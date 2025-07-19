#!/bin/bash
#
# Upload binaries to an existing GitHub release
# Usage: ./scripts/upload-release.sh v0.1.0

set -e

VERSION="${1}"
if [ -z "$VERSION" ]; then
    echo "Usage: $0 <version>"
    echo "Example: $0 v0.1.0"
    exit 1
fi

# Check if gh is installed
if ! command -v gh &> /dev/null; then
    echo "Error: GitHub CLI (gh) is not installed"
    echo "Install it from: https://cli.github.com/"
    exit 1
fi

# Check if release directory exists
if [ ! -d "release" ]; then
    echo "Error: release/ directory not found"
    echo "Run ./scripts/build-all.sh first"
    exit 1
fi

echo "Uploading binaries to release ${VERSION}..."

# Upload all files in release directory
for file in release/*; do
    if [ -f "$file" ]; then
        echo "Uploading $(basename "$file")..."
        gh release upload "$VERSION" "$file" --clobber
    fi
done

echo "✓ Upload complete!"
echo ""
echo "View the release at:"
echo "https://github.com/$(gh repo view --json nameWithOwner -q .nameWithOwner)/releases/tag/${VERSION}"