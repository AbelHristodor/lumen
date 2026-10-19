#!/bin/sh
# Builds dist/lumen-<version>-macos-universal.dmg from the two release
# binaries. Usage: packaging/macos/package.sh 2.0.1
set -eu
version="$1"
root="$(cd "$(dirname "$0")/../.." && pwd)"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

app="$work/dmg/Lumen.app/Contents"
mkdir -p "$app/MacOS" "$app/Resources" "$root/dist"
lipo -create -output "$app/MacOS/lumen" \
  "$root/target/aarch64-apple-darwin/release/lumen" \
  "$root/target/x86_64-apple-darwin/release/lumen"
cp "$root/assets/icon.icns" "$app/Resources/"
cat > "$app/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleName</key><string>Lumen</string>
  <key>CFBundleDisplayName</key><string>Lumen</string>
  <key>CFBundleIdentifier</key><string>com.abelhristodor.lumen</string>
  <key>CFBundleExecutable</key><string>lumen</string>
  <key>CFBundleIconFile</key><string>icon</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$version</string>
  <key>CFBundleVersion</key><string>$version</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST
# Ad hoc signature so the bundle is consistent. Not a Developer ID, so
# Gatekeeper still asks on first open.
codesign --force --deep --sign - "$work/dmg/Lumen.app"
ln -s /Applications "$work/dmg/Applications"
hdiutil create -quiet -volname Lumen -srcfolder "$work/dmg" -ov -format ULMO \
  "$root/dist/lumen-$version-macos-universal.dmg"
