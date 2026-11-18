#!/usr/bin/env bash
# Writes the Homebrew cask for a published release to stdout.
# Usage: packaging/homebrew/update-cask.sh 1.0.0 > ../homebrew-tap/Casks/notecal.rb
set -euo pipefail
version=${1:?usage: update-cask.sh <version>}
url="https://github.com/troazz/notecal/releases/download/v$version/notecal_${version}_universal.dmg"
sha=$(curl -fsSL "$url" | sha256sum | cut -d' ' -f1)
sed -e "s/@VERSION@/$version/" -e "s/@SHA256@/$sha/" "$(dirname "$0")/notecal.rb.in"
