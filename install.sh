#!/bin/sh
# Build the release binary (offline, from ./vendor if present) and install it
# for the current user: ~/.local/bin + a menu entry. No root needed.
set -e
cd "$(dirname "$0")"
OFFLINE=""
[ -d vendor ] && OFFLINE="--offline"
cargo build --release $OFFLINE
mkdir -p "$HOME/.local/bin" "$HOME/.local/share/applications"
install -m 755 target/release/guitar-trainer "$HOME/.local/bin/guitar-trainer"
install -m 644 packaging/guitar-trainer.desktop "$HOME/.local/share/applications/guitar-trainer.desktop"
echo "Installed: $HOME/.local/bin/guitar-trainer (menu entry: Guitar Trainer)"
