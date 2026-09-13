# @quickgui/gpuix

Experimental native-backend adapter for GPUIX 0.7.0 React applications.
QuickGUI owns native layout, input, text, drawing, and application services.
Application components and reactivity remain in React.

This package translates GPUIX committed mutations into QuickGUI native nodes. It is not a
second GPU renderer and does not yet implement the complete GPUIX native interface.
Build with `gpuixQuickGuiPlugin()` so runtime imports of `@gpuix/native` resolve to this adapter.
Use the QuickGUI main-thread host/worker lifecycle; do not launch the GPUIX frame loop.

Run `bun run test:gpuix` from the repository root.
