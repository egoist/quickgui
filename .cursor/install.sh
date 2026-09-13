#!/usr/bin/env bash
# Idempotent Cloud Agent setup for the QuickGUI workspace.
#
# QuickGUI is a native desktop GUI framework with Go, TypeScript, and Rust
# applications that share a Rust core (built as a shared library and loaded in
# the same process via purego / bun:ffi). This script provisions the toolchains
# and Linux system libraries needed to build the native core and run the
# example applications, then installs JS dependencies and builds the native
# library once.
#
# It is safe to run repeatedly: every step checks for the desired state first.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

GO_VERSION="1.23.12"
BUN_VERSION="1.4.0"

log() { printf '\n[install] %s\n' "$*"; }

# ---------------------------------------------------------------------------
# System packages: build tooling plus the X11/Wayland/GL/Vulkan/udev/dbus
# development and runtime libraries winit + wgpu need on Linux. mesa-vulkan-
# drivers provides lavapipe, a software Vulkan implementation, so the GUI apps
# render even without a physical GPU.
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
# Rust: the workspace is edition 2024 with a 1.90 MSRV, so a current stable
# toolchain is required. rustup ships in the default base image.
# ---------------------------------------------------------------------------
log "Ensuring Rust stable toolchain (>= 1.90)"
if command -v rustup >/dev/null 2>&1; then
  rustup toolchain install stable --profile minimal --component clippy,rustfmt --no-self-update
  rustup default stable
else
  echo "[install] rustup not found; installing via rustup.rs" >&2
  curl -fsSL https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable --component clippy rustfmt
  # shellcheck disable=SC1090
  source "$HOME/.cargo/env"
fi
rustc --version

# ---------------------------------------------------------------------------
# Go 1.23+: install into /usr/local/go when a matching version is not present.
# ---------------------------------------------------------------------------
log "Ensuring Go ${GO_VERSION}"
if ! /usr/local/go/bin/go version 2>/dev/null | grep -q "go${GO_VERSION}"; then
  tmp_go="$(mktemp -d)"
  curl -fsSL "https://go.dev/dl/go${GO_VERSION}.linux-amd64.tar.gz" -o "$tmp_go/go.tgz"
  sudo rm -rf /usr/local/go
  sudo tar -C /usr/local -xzf "$tmp_go/go.tgz"
  rm -rf "$tmp_go"
fi
/usr/local/go/bin/go version

# ---------------------------------------------------------------------------
# Bun 1.4: JS runtime, package manager, and FFI host for the TypeScript apps.
# ---------------------------------------------------------------------------
log "Ensuring Bun ${BUN_VERSION}"
if ! "$HOME/.bun/bin/bun" --version 2>/dev/null | grep -q "^${BUN_VERSION}$"; then
  curl -fsSL https://bun.sh/install | bash -s "bun-v${BUN_VERSION}"
fi
"$HOME/.bun/bin/bun" --version

# ---------------------------------------------------------------------------
# Persist PATH and GUI runtime environment for interactive agent shells.
# VK_ICD_FILENAMES pins lavapipe (software Vulkan); XDG_RUNTIME_DIR gives the
# native host a valid runtime directory when running example apps.
# ---------------------------------------------------------------------------
log "Configuring shell environment"
BASHRC="$HOME/.bashrc"
add_line() { grep -qxF "$1" "$BASHRC" 2>/dev/null || echo "$1" >> "$BASHRC"; }
touch "$BASHRC"
add_line 'export PATH="/usr/local/go/bin:$HOME/.bun/bin:$HOME/.cargo/bin:$PATH"'
add_line 'export VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json'
add_line 'export XDG_RUNTIME_DIR=/tmp/xdg-runtime'

export PATH="/usr/local/go/bin:$HOME/.bun/bin:$HOME/.cargo/bin:$PATH"

# ---------------------------------------------------------------------------
# JS dependencies and the shared native library.
# ---------------------------------------------------------------------------
log "Installing JS dependencies (bun install --frozen-lockfile)"
bun install --frozen-lockfile

log "Building the native shared library (bun run build:native)"
bun run build:native

log "Done. Toolchains, dependencies, and the native library are ready."
