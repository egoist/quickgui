# @quickgui/gpuix

Experimental native-backend adapter for GPUIX React applications. See
[the fx-ui experiment](../../examples/fx-ui/README.md) for the exact-parity contract and current gaps.

This package translates GPUIX committed mutations into QuickGUI native nodes. It is not a
second GPU renderer and does not yet implement the complete GPUIX native interface.
Build with `gpuixQuickGuiPlugin()` so runtime imports of `@gpuix/native` resolve to this adapter.
Use the QuickGUI main-thread host/worker lifecycle; do not launch the GPUIX frame loop.

Run `bun run test:gpuix` from the repository root.
