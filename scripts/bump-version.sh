#!/usr/bin/env bash
# Usage: ./scripts/bump-version.sh [patch|minor|major]
# Bumps the version in Cargo.toml and Cargo.lock, then stages the changes.
# Run this in develop before opening a release PR to main.
set -euo pipefail

BUMP=${1:-patch}

current=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
major=$(echo "$current" | cut -d. -f1)
minor=$(echo "$current" | cut -d. -f2)
patch=$(echo "$current" | cut -d. -f3)

case "$BUMP" in
  major) new_version="$((major + 1)).0.0" ;;
  minor) new_version="$major.$((minor + 1)).0" ;;
  patch) new_version="$major.$minor.$((patch + 1))" ;;
  *)
    echo "Usage: $0 [patch|minor|major]" >&2
    exit 1
    ;;
esac

echo "Bumping $current → $new_version"
sed -i "0,/^version = \"$current\"/s//version = \"$new_version\"/" Cargo.toml
cargo generate-lockfile

echo "Done. Commit with:"
echo "  git add Cargo.toml Cargo.lock && git commit -m \"chore: bump version to $new_version\""
