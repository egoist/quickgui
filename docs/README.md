# Contributor documentation

User-facing guides and the component reference live in [`website/src/content/docs`](../website/src/content/docs). Start there (or `cd website && bun run dev`) if you are writing an application.

This folder is for people working on QuickGUI itself: architecture, release process, and implementation notes.

## Architecture

The [architecture index](architecture/README.md) routes to focused notes:

- [Runtime and ownership](architecture/runtime.md)
- [Scheduling and performance](architecture/scheduling.md)
- [Input and interaction](architecture/input.md)
- [Layout and rendering](architecture/rendering.md)
- [macOS composition](architecture/macos.md)
- [Deterministic testing](architecture/testing.md)
- [Current boundaries](architecture/boundaries.md)
- [Performance](architecture/performance.md)

## Tooling

- [Editor, CodeBlock, and diff view](editor-and-diffs.md)
- [Portable language packs](language-packs.md)
- [TypeScript toolchain](typescript.md)
- [Go toolchain](go.md)
- [CLI and packaging](cli.md)
- [Releasing](releasing.md)
- [Changelog](../CHANGELOG.md)
- [Status](status.md)

Return to the [project README](../README.md).
