#!/bin/sh
# Builds Turbine on a development PC (Linux or WSL) the same way the online build does, installs it for this user
# and starts it. Needs: flatpak, flatpak-builder, cargo, curl, git (Ubuntu: sudo apt install flatpak flatpak-builder
# cargo curl git ccache). The first run downloads the KDE and Qt parts (a few GB); later runs take a minute or two.
#   sh tools/dev-build.sh          build, install, start
#   sh tools/dev-build.sh --pull   get the newest code from GitHub first
set -eu
cd "$(dirname "$0")/.."
[ "${1:-}" = "--pull" ] && git pull --ff-only
flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
# The Flatpak build is offline: the locked crates are vendored first, as the online build does.
mkdir -p .cargo
cargo vendor --locked vendor > .cargo/config.toml
sh tools/get-fonts.sh
git rev-parse --short=9 HEAD > app/build-commit.txt
flatpak-builder --user --install --force-clean --ccache --install-deps-from=flathub --disable-rofiles-fuse \
  build-dir packaging/io.github.ravenor907.TurbineModManager.json
flatpak run io.github.ravenor907.TurbineModManager
