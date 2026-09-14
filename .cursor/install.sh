#!/usr/bin/env bash
# Idempotent Cloud Agent setup for the QuickGUI workspace.
#
# QuickGUI is a native desktop GUI framework with Go, TypeScript, and Rust
# applications that share a Rust core (built as a shared library and loaded in
# the same process via purego / bun:ffi). This script installs the Linux system
# libraries the native core needs, provisions the language toolchains with mise
# (pinned in ./mise.toml), installs JS dependencies, and builds the native
# library once.
#
# It is safe to run repeatedly: every step checks for the desired state first.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

log() { printf '\n[install] %s\n' "$*"; }

# ---------------------------------------------------------------------------
# System packages: build tooling plus the X11/Wayland/GL/Vulkan/udev/dbus
# development and runtime libraries winit + wgpu need on Linux. mesa-vulkan-
# drivers provides lavapipe, a software Vulkan implementation, so the GUI apps
# render even without a physical GPU. (mise manages the language toolchains, not
# these OS-level libraries.)
# ---------------------------------------------------------------------------
log "Installing system packages"
sudo DEBIAN_FRONTEND=noninteractive apt-get update -qq
sudo DEBIAN_FRONTEND=noninteractive apt-get install -y -qq --no-install-recommends \
  build-essential pkg-config curl git ca-certificates unzip xz-utils \
  libwayland-dev libxkbcommon-dev libxkbcommon-x11-dev libxcb-xkb-dev \
  libx11-dev libxext-dev libxrandr-dev libxi-dev libxcursor-dev libxfixes-dev \
  libudev-dev libgtk-3-dev libglib2.0-dev libdbus-1-dev libssl-dev \
  libvulkan-dev libvulkan1 mesa-vulkan-drivers vulkan-tools \
  libgl1-mesa-dri libegl1-mesa-dev libgles2-mesa-dev \
  xvfb x11-utils

# ---------------------------------------------------------------------------
# mise: manages the Rust (stable; the workspace is edition 2024 with a 1.90
# MSRV), Go, Bun, and Zig toolchains pinned in ./mise.toml.
# ---------------------------------------------------------------------------
export MISE_DATA_DIR="${MISE_DATA_DIR:-$HOME/.local/share/mise}"
if ! command -v mise >/dev/null 2>&1 && [ ! -x "$HOME/.local/bin/mise" ]; then
  log "Installing mise"
  curl -fsSL https://mise.run | sh
fi
export PATH="$HOME/.local/bin:$MISE_DATA_DIR/shims:$PATH"
mise --version

log "Installing pinned toolchains with mise"
mise trust "$REPO_ROOT/mise.toml"
mise install
mise reshim

# The rust core tool drives rustup; make sure a default toolchain is selected so
# bare `cargo`/`rustc` (outside a mise shim) still resolve during builds.
if command -v rustup >/dev/null 2>&1; then
  rustup default stable >/dev/null 2>&1 || true
fi

log "Toolchain versions"
mise exec -- rustc --version
mise exec -- go version
mise exec -- bun --version

# ---------------------------------------------------------------------------
# Persist PATH and GUI runtime environment for interactive agent shells.
# The mise shim directory exposes the pinned tools in non-interactive shells;
# `mise activate` wires them up for interactive ones. VK_ICD_FILENAMES pins
# lavapipe (software Vulkan) and XDG_RUNTIME_DIR gives the native host a valid
# runtime directory when running example apps.
# ---------------------------------------------------------------------------
log "Configuring shell environment"
BASHRC="$HOME/.bashrc"
add_line() { grep -qxF "$1" "$BASHRC" 2>/dev/null || echo "$1" >> "$BASHRC"; }
touch "$BASHRC"
add_line 'export PATH="$HOME/.local/bin:$HOME/.local/share/mise/shims:$PATH"'
add_line 'command -v mise >/dev/null 2>&1 && eval "$(mise activate bash)"'
add_line 'export VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json'
add_line 'export XDG_RUNTIME_DIR=/tmp/xdg-runtime'

# ---------------------------------------------------------------------------
# JS dependencies, using the mise-managed tools.
#
# The shared native library (`bun run build:native`, a full release cargo
# build) is intentionally NOT built here: it is only needed to actually run an
# application, and building it every install is slow. Build it on demand when
# running an app, e.g.:
#     bun run build:native           # release (staged for Go/TS apps)
#     bun run build:native --debug   # faster, for iterating locally
# Rust examples build with cargo directly (`cargo run --example <name>`).
# ---------------------------------------------------------------------------
log "Installing JS dependencies (bun install --frozen-lockfile)"
mise exec -- bun install --frozen-lockfile

log "Done. Toolchains (via mise) and JS dependencies are ready."
log "Run 'bun run build:native' when you need the native library to run an app."
