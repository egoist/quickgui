#!/usr/bin/env bash
# Per-boot runtime preparation for QuickGUI example applications.
#
# The native host (winit + wgpu) expects a valid XDG_RUNTIME_DIR. This is
# cheap, idempotent, and safe to run on every boot. Toolchains, dependencies,
# and the native library are provisioned by install.sh, not here.
set -euo pipefail

RUNTIME_DIR="${XDG_RUNTIME_DIR:-/tmp/xdg-runtime}"
mkdir -p "$RUNTIME_DIR"
chmod 700 "$RUNTIME_DIR"

echo "[start] XDG_RUNTIME_DIR ready at $RUNTIME_DIR"
