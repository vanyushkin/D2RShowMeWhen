#!/usr/bin/env bash
set -euo pipefail

TAG="${1:?Usage: $0 <tag>  e.g. $0 v1.0.0}"

DMG=$(find app/src-tauri/target/release/bundle/dmg -name "*.dmg" 2>/dev/null | head -1)

if [ -z "$DMG" ]; then
  echo "Error: no .dmg found in app/src-tauri/target/release/bundle/dmg/"
  echo "Run this first:  cd app && npm run tauri:build"
  exit 1
fi

echo "Uploading $(basename "$DMG") to release $TAG ..."
gh release upload "$TAG" "$DMG" --repo vanyushkin/D2RShowMeWhen
echo "Done: https://github.com/vanyushkin/D2RShowMeWhen/releases/tag/$TAG"
