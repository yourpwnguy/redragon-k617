#!/bin/sh
# Install the CLI and GUI for the current user, without root.
#
# The important bit is the generated desktop entry: application launchers do
# not reliably inherit the user's shell PATH, so an entry that says
# `Exec=redragon-k617-gui` can silently do nothing when the binary lives in
# ~/.local/bin. We substitute absolute paths and refresh the desktop/icon
# caches so the menu item works on the first click.
set -eu

cd "$(dirname "$0")"

bindir="${HOME}/.local/bin"
appdir="${HOME}/.local/share/applications"
icondir="${HOME}/.local/share/icons/hicolor/256x256/apps"
desktop_out="${appdir}/redragon-k617.desktop"

cargo build --release

install -Dm755 target/release/redragon-k617 "${bindir}/redragon-k617"
install -Dm755 target/release/redragon-k617-gui "${bindir}/redragon-k617-gui"
install -Dm644 assets/gui/appico.png "${icondir}/redragon-k617.png"
install -d "${appdir}"

# Quote the executable so a home directory with spaces still works.
sed \
  -e "s|^Exec=.*|Exec=\"${bindir}/redragon-k617-gui\"|" \
  -e "s|^TryExec=.*|TryExec=${bindir}/redragon-k617-gui|" \
  -e "s|^Icon=.*|Icon=${icondir}/redragon-k617.png|" \
  desktop/redragon-k617.desktop > "${desktop_out}"

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "${appdir}" || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -f -t "${HOME}/.local/share/icons/hicolor" >/dev/null 2>&1 || true
fi

printf 'installed:\n  %s\n  %s\n  %s\n' \
  "${bindir}/redragon-k617" \
  "${bindir}/redragon-k617-gui" \
  "${desktop_out}"
