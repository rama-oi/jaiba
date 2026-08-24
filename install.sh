#!/usr/bin/env sh

set -eu

PREFIX="${PREFIX:-$HOME/.local}"
BIN_DIR="$PREFIX/bin"
BIN_NAME="jaiba"
ICON_THEME_DIR="$PREFIX/share/icons/hicolor"
ICON_NAME="jaiba"
DESKTOP_DIR="$PREFIX/share/applications"
DESKTOP_NAME="jaiba.desktop"
ICON_SIZES="16 22 24 32 48 64 72 96 128 192 256 512"

if [ "${1:-}" = "--uninstall" ]; then
    if [ -f "$BIN_DIR/$BIN_NAME" ]; then
        rm -f "$BIN_DIR/$BIN_NAME"
        echo "Removed $BIN_DIR/$BIN_NAME"
    else
        echo "Nothing installed at $BIN_DIR/$BIN_NAME"
    fi

    if [ -f "$DESKTOP_DIR/$DESKTOP_NAME" ]; then
        rm -f "$DESKTOP_DIR/$DESKTOP_NAME"
        echo "Removed $DESKTOP_DIR/$DESKTOP_NAME"
    fi

    for s in $ICON_SIZES; do
        f="$ICON_THEME_DIR/${s}x${s}/apps/$ICON_NAME.png"
        [ -f "$f" ] && rm -f "$f" && echo "Removed $f"
    done

    if [ -f "$ICON_THEME_DIR/scalable/apps/$ICON_NAME.svg" ]; then
        rm -f "$ICON_THEME_DIR/scalable/apps/$ICON_NAME.svg"
        echo "Removed $ICON_THEME_DIR/scalable/apps/$ICON_NAME.svg"
    fi

    command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$DESKTOP_DIR" >/dev/null 2>&1 || true

    echo "Note: ~/.config/jaiba (your config, vault path, and themes) was left in place."
    exit 0
fi

command -v cargo >/dev/null 2>&1 || {
    echo "error: cargo not found. Install a Rust toolchain (rustup.rs), Rust 1.85+ required." >&2
    exit 1
}

echo "Building jaiba (release)..."
cargo build --release

mkdir -p "$BIN_DIR"
install -m 755 "target/release/$BIN_NAME" "$BIN_DIR/$BIN_NAME"
echo "Installed to $BIN_DIR/$BIN_NAME"

mkdir -p "$DESKTOP_DIR"
for s in $ICON_SIZES; do
    src="assets/icon_${s}x${s}.png"
    [ -f "$src" ] || { echo "warning: missing $src, skipping" >&2; continue; }
    dest_dir="$ICON_THEME_DIR/${s}x${s}/apps"
    mkdir -p "$dest_dir"
    install -m 644 "$src" "$dest_dir/$ICON_NAME.png"
done

mkdir -p "$ICON_THEME_DIR/scalable/apps"
install -m 644 "assets/jaiba.svg" "$ICON_THEME_DIR/scalable/apps/$ICON_NAME.svg"
install -m 644 "assets/jaiba.desktop" "$DESKTOP_DIR/$DESKTOP_NAME"
echo "Installed icons to $ICON_THEME_DIR/{size}/apps/$ICON_NAME.png and scalable/apps/$ICON_NAME.svg"
echo "Installed launcher entry to $DESKTOP_DIR/$DESKTOP_NAME"

command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$DESKTOP_DIR" >/dev/null 2>&1 || true
command -v gtk-update-icon-cache >/dev/null 2>&1 && gtk-update-icon-cache -q "$ICON_THEME_DIR" >/dev/null 2>&1 || true

case ":$PATH:" in
    *":$BIN_DIR:"*) ;;
    *)
        echo ""
        echo "warning: $BIN_DIR is not on your PATH."
        echo "Add this to your shell profile:"
        echo "  export PATH=\"$BIN_DIR:\$PATH\""
        ;;
esac

echo ""
echo "Run 'jaiba' to get started. On first launch it creates:"
echo "  ~/.config/rama/jaiba_config.toml"
echo "  ~/.config/rama/themes/ (seeded with the built-in themes)"
