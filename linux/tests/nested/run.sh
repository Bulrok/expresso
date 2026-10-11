#!/bin/bash
# Headless nested gnome-shell with Expresso + the test harness extension.
# Everything (dconf, runtime dir, ibus) is isolated in a temp dir, so this never
# touches the live session.
#
#   EXPRESSO_APP    command for the app under test (default: GTK4 test window)
#   EXPRESSO_SECS   how long to keep the shell running (default 25)
#   EXPRESSO_OUT    where log.txt and screenshots go (default ./out)
#   EXPRESSO_TAG    prefix for screenshot names
#   EXPRESSO_SHOTS  screenshot-only mode: take N shots instead of typing
#                   (EXPRESSO_SHOTS_DELAY / EXPRESSO_SHOTS_EVERY in ms)
# The app gets WAYLAND_DISPLAY, plus DISPLAY/XAUTHORITY for X11 apps (Wine).
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
T=$(mktemp -d)
# The document portal leaves a FUSE mount in the runtime dir; unmount first.
trap 'fusermount -u "$T/run/doc" 2>/dev/null; rm -rf "$T"' EXIT
export EXPRESSO_OUT=${EXPRESSO_OUT:-$HERE/out}
export EXPRESSO_APP=${EXPRESSO_APP:-gjs -m $HERE/app.js}
export EXPRESSO_SECS=${EXPRESSO_SECS:-25}
mkdir -p "$EXPRESSO_OUT" "$T/data/gnome-shell/extensions"
cp -r "$HERE/../../extension/expresso@bulrok" "$HERE/harness@local" "$T/data/gnome-shell/extensions/"
export XDG_DATA_HOME=$T/data XDG_CONFIG_HOME=$T/config XDG_CACHE_HOME=$T/cache XDG_STATE_HOME=$T/state
export XDG_RUNTIME_DIR=$T/run
mkdir -m700 -p "$XDG_RUNTIME_DIR"
unset DISPLAY WAYLAND_DISPLAY
LOG=$EXPRESSO_OUT/log.txt

inner() {
    gsettings set org.gnome.shell enabled-extensions "['harness@local']"
    gsettings set org.gnome.shell disable-user-extensions false
    gsettings set org.gnome.shell welcome-dialog-last-shown-version '999'
    gnome-shell --headless --virtual-monitor 1600x900 --wayland-display wayland-expresso &
    local shell=$!
    for _ in $(seq 100); do
        [ -S "$XDG_RUNTIME_DIR/wayland-expresso" ] && grep -q "public X11 display" "$LOG" && break
        sleep 0.1
    done
    local display xauth
    display=$(grep -o "public X11 display :[0-9]*" "$LOG" | grep -o ":[0-9]*")
    xauth=$(ls "$XDG_RUNTIME_DIR"/.mutter-Xwaylandauth.* 2>/dev/null | head -1)
    WAYLAND_DISPLAY=wayland-expresso DISPLAY=$display XAUTHORITY=$xauth \
        GDK_BACKEND=wayland MOZ_ENABLE_WAYLAND=1 $EXPRESSO_APP &
    sleep "$EXPRESSO_SECS"
    kill $shell 2>/dev/null
    sleep 1
}
export -f inner
export LOG

: > "$LOG"
dbus-run-session -- bash -c inner >> "$LOG" 2>&1
grep -E "HARNESS|JS ERROR|expresso@" "$LOG"
