#!/bin/sh
# Lumen installer for macOS and Linux.
#
#   curl -fsSL https://raw.githubusercontent.com/AbelHristodor/lumen/main/install.sh | sh
#
# macOS: installs Lumen.app from the latest dmg into ~/Applications.
# Linux: installs the portable `lumen` binary to $BIN_DIR, default
# ~/.local/bin, plus a menu entry. It can update itself from there.
#
#   curl -fsSL .../install.sh | BIN_DIR=/usr/local/bin sh
set -eu

REPO="AbelHristodor/lumen"
BIN_DIR="${BIN_DIR:-$HOME/.local/bin}"

os="$(uname -s)"
arch="$(uname -m)"

# The asset names carry the version, so ask GitHub which tag is latest.
latest="$(curl -fsSLI -o /dev/null -w '%{url_effective}' "https://github.com/$REPO/releases/latest")"
tag="${latest##*/}"
version="${tag#v}"
case "$version" in
  [0-9]*) ;;
  *) echo "Error: could not find the latest release of $REPO." >&2; exit 1 ;;
esac

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

fetch() {
  url="https://github.com/$REPO/releases/download/$tag/$1"
  echo "Downloading $1 ..."
  curl -fsSL "$url" -o "$tmp/$1" || { echo "Error: download failed from $url" >&2; exit 1; }
}

case "$os" in
  Darwin)
    asset="lumen-$version-macos-universal.dmg"
    fetch "$asset"
    hdiutil attach -quiet -nobrowse -readonly -mountpoint "$tmp/mnt" "$tmp/$asset"
    mkdir -p "$HOME/Applications"
    rm -rf "$HOME/Applications/Lumen.app"
    cp -R "$tmp/mnt/Lumen.app" "$HOME/Applications/"
    hdiutil detach -quiet "$tmp/mnt"
    echo "Installed Lumen $version to $HOME/Applications/Lumen.app"
    echo
    echo "The app is unsigned. The first time, right-click it and choose Open."
    ;;
  Linux)
    case "$arch" in
      x86_64|amd64) arch="x86_64" ;;
      arm64|aarch64) arch="aarch64" ;;
      *) echo "Error: unsupported Linux architecture '$arch'." >&2; exit 1 ;;
    esac
    asset="lumen-$version-linux-$arch.tar.gz"
    fetch "$asset"
    tar -xzf "$tmp/$asset" -C "$tmp"
    mkdir -p "$BIN_DIR"
    install -m 0755 "$tmp/lumen" "$BIN_DIR/lumen"

    share="${XDG_DATA_HOME:-$HOME/.local/share}"
    mkdir -p "$share/applications" "$share/icons/hicolor/512x512/apps"
    curl -fsSL "https://raw.githubusercontent.com/$REPO/$tag/assets/icon.png" \
      -o "$share/icons/hicolor/512x512/apps/lumen.png" || true
    curl -fsSL "https://raw.githubusercontent.com/$REPO/$tag/packaging/linux/lumen.desktop" |
      sed "s|^Exec=lumen|Exec=$BIN_DIR/lumen|" > "$share/applications/lumen.desktop" || true
    echo "Installed Lumen $version to $BIN_DIR/lumen"

    case ":$PATH:" in
      *":$BIN_DIR:"*) ;;
      *)
        echo
        echo "Note: $BIN_DIR is not on your PATH. Add this to your shell profile:"
        echo "  export PATH=\"$BIN_DIR:\$PATH\""
        ;;
    esac
    ;;
  *)
    echo "Error: unsupported OS '$os'. Use install.ps1 on Windows." >&2
    exit 1
    ;;
esac

# Earlier installs left these behind.
rm -f "$BIN_DIR/hymnal-gui"
if [ "$os" = "Darwin" ]; then rm -f "$BIN_DIR/lumen"; fi
