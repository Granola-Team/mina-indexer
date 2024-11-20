#!/bin/bash

# Create temporary directory
readonly TMP_DIR=$(mktemp -d)
cd "$TMP_DIR"

readonly VERSION=$(curl -s https://formulae.brew.sh/api/formula/postgrest.json | jq -r '.versions.stable')

# Download the binary with proper headers
# Find proper sha256 at https://formulae.brew.sh/api/formula/postgrest.json
echo "Downloading PostgREST binary..."
curl -L \
    -H "Accept: application/vnd.oci.image.content.v1+json" \
    -H "Authorization: Bearer QQ==" \
    "https://ghcr.io/v2/homebrew/core/postgrest/blobs/sha256:68a1b201a0396ca4a9332f68f4a1a039c4239e984dbdd23558778809686237cf" \
    -o postgrest.tar.gz

# Create installation directory
sudo mkdir -p /usr/local/bin

# Extract and install
echo "Installing PostgREST..."
tar xzf postgrest.tar.gz "postgrest/$VERSION/bin/postgrest"
sudo cp "postgrest/$VERSION/bin/postgrest" /usr/local/bin/
sudo chmod +x /usr/local/bin/postgrest

# Clean up
cd - > /dev/null
rm -rf "$TMP_DIR"

echo "PostgREST installation complete."
