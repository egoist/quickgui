# {{README_TITLE}}

A native QuickGUI application written in Rust.

```console
bun install
bun run dev
```

`bun run dev` compiles the crate, packages and launches the app, and rebuilds on source changes. `bun run build` produces a distributable application. Rust apps link the `quickgui` crate directly; they do not load the Go/TypeScript host shared library.

Use a current stable Rust toolchain (1.90+), Bun, and Xcode Command Line Tools on macOS. The first development build compiles wgpu and takes longer; later edits reuse incremental Cargo artifacts.

From a QuickGUI source checkout, point the crate at the local tree:

```toml
quickgui = { path = "../quickgui" }
```

Configuration can use `quickgui.toml` or `quickgui.config.ts`. The CLI looks for TOML first; `--config path/to/file` selects a file explicitly. Set `language = "rust"`.
