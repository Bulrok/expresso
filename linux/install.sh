#!/bin/bash
# Install Expresso for the current user: shell extension, toggle command,
# launcher and icon. Safe to rerun; `--uninstall` removes it all again.
set -euo pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
UUID=expresso@bulrok
EXT=~/.local/share/gnome-shell/extensions/$UUID
BIN=~/.local/bin/expresso-toggle
DESKTOP=~/.local/share/applications/io.github.expresso.Toggle.desktop
ICON=~/.local/share/icons/hicolor/scalable/apps/expresso.svg

if [[ ${1:-} == --uninstall ]]; then
    gnome-extensions disable "$UUID" 2>/dev/null || true
    rm -rf "$EXT" "$BIN" "$DESKTOP" "$ICON"
    echo "Expresso removed. It disappears from the running shell at next login."
    exit 0
fi

mkdir -p "$EXT" "$(dirname "$BIN")" "$(dirname "$DESKTOP")" "$(dirname "$ICON")"
install -m644 "$HERE/extension/$UUID"/{extension.js,calc.js,metadata.json,stylesheet.css} "$EXT/"
install -m755 "$HERE/expresso-toggle" "$BIN"
install -m644 "$HERE/data/io.github.expresso.Toggle.desktop" "$DESKTOP"
install -m644 "$HERE/../assets/expresso.svg" "$ICON"
update-desktop-database -q "$(dirname "$DESKTOP")" 2>/dev/null || true
gtk-update-icon-cache -q -t ~/.local/share/icons/hicolor 2>/dev/null || true

# Enable through the shell if it already knows the extension; otherwise put it
# on the list so it loads at next login (new extensions are found at login).
if gnome-extensions enable "$UUID" 2>/dev/null; then
    echo "Expresso installed and enabled. Code changes take effect at next login."
else
    current=$(gsettings get org.gnome.shell enabled-extensions)
    if [[ $current != *"'$UUID'"* ]]; then
        if [[ $current == "@as []" || $current == "[]" ]]; then
            gsettings set org.gnome.shell enabled-extensions "['$UUID']"
        else
            gsettings set org.gnome.shell enabled-extensions "${current%]}, '$UUID']"
        fi
    fi
    echo "Expresso installed. Log out and back in once for GNOME Shell to load it."
fi
