#!/bin/bash
# The version lives in two places: windows/Cargo.toml and the extension's
# metadata.json ("version-name"). Fail if they disagree, or if a release tag
# (vX.Y.Z, passed as $1 or via GITHUB_REF_NAME) doesn't match them.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo=$(sed -n 's/^version = "\(.*\)"/\1/p' windows/Cargo.toml | head -1)
ext=$(python3 -c 'import json; print(json.load(open("linux/extension/expresso@bulrok/metadata.json"))["version-name"])')
tag=${1:-${GITHUB_REF_NAME:-}}
if [[ $cargo != "$ext" ]]; then
    echo "version mismatch: windows/Cargo.toml=$cargo, metadata.json version-name=$ext" >&2
    exit 1
fi
if [[ $tag == v* && ${tag#v} != "$cargo" ]]; then
    echo "tag $tag does not match version $cargo" >&2
    exit 1
fi
echo "version $cargo"
