# Third-party notices

## Zed GPUI macOS key-equivalent data

`src/macos_key_equivalents.rs` adapts the finite keyboard-layout mapping data from Zed GPUI,
Copyright 2022-2025 Zed Industries, Inc. The source data is licensed under the Apache License 2.0;
see `LICENSE-APACHE`.

The surrounding QuickGUI implementation, storage model, layout-change integration, indexing, and
public API are QuickGUI code.

## Winit support package

`vendor/winit` is derived from Winit 0.30.13 and carries the native macOS panel and touch changes
needed by QuickGUI while the stable Winit and AccessKit release lines converge. Winit is Copyright
the Winit contributors and licensed under the Apache License 2.0; see `vendor/winit/LICENSE`.

## AccessKit Winit support package

`vendor/accesskit_winit` is derived from `accesskit_winit` 0.33.2 and changes its Winit dependency
to the versioned QuickGUI support package. AccessKit is Copyright the AccessKit contributors and
licensed under the Apache License 2.0; see `vendor/accesskit_winit/LICENSE-APACHE`.

## Glyphon support package

`vendor/glyphon` is derived from Glyphon 0.12.0 and adds one paint-only per-text-area opacity value
to its glyph instance upload and shader. Glyphon is Copyright the Glyphon contributors and is
available under MIT, Apache-2.0, or Zlib; see the three license files in `vendor/glyphon`.

## Cosmic Text support package

`vendor/cosmic_text` is derived from Cosmic Text 0.19.0 and adds ordered per-style family
fallbacks to owned text attributes, fallback selection, and shaping-cache identity. Cosmic Text is
Copyright the Cosmic Text contributors and is available under MIT or Apache-2.0; see
`vendor/cosmic_text/LICENSE-MIT` and `vendor/cosmic_text/LICENSE-APACHE`.

## Text-shaping and documentation fonts

`tests/fixtures/fonts/Inter-Regular.ttf` is Copyright 2020 The Inter Project Authors, and
`tests/fixtures/fonts/NotoSansHebrew.ttf` is Copyright 2012 Google Inc. Both test fixtures are
licensed under the SIL Open Font License 1.1; see `tests/fixtures/fonts/Inter-LICENSE` and
`tests/fixtures/fonts/NotoSans-LICENSE`. Noto Sans is compiled only into framework tests.
Inter is also embedded in the browser documentation demos; their build includes
`Inter-LICENSE.txt` beside the generated WebAssembly bundle.

## GPUIX motion semantics

`crates/quickgui-host/src/motion.rs` adapts the motion interpolation, validation and easing
from `remorses/gpuix`, commit `0fac5c941e8431261605eaf0f48d8528322b414d`,
`packages/native/src/motion.rs`. Copyright the GPUIX contributors, Apache-2.0; see
`LICENSE-APACHE`. Scheduling and retained QuickGUI element integration are adapted here.

## GPUIX and Comet document components

`src/document/` adapts GPUIX's syntax highlighter, unified-patch parser, theme,
and code/diff layout from `remorses/gpuix` commit
`18e695ed0ee8121a7793413ca795e08eda2a13df` (Apache-2.0).
The themed rendering in `src/markdown/mod.rs` follows the same port.
These GPUIX components were ported from Comet, Copyright (c) 2026 Wing, MIT:

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

## GPUI shadow and gradient rendering

The rounded rectangle Gaussian shadow integration and triangular gradient dither in
`src/quad.wgsl` adapt the GPUI Metal shaders from Zed commit
`8b94defe56992b3ca4ffd4853ace741d8168111a`. Copyright Zed Industries and contributors,
Apache-2.0; see `LICENSE-APACHE`.

## GPUI native text placement

The first-font-run isolation and baseline-relative underline placement in
`vendor/cosmic_text/src/macos.rs` and `src/renderer/text_layout.rs` follow Zed GPUI
commit `8b94defe56992b3ca4ffd4853ace741d8168111a`, `gpui_macos/src/text_system.rs`
and `gpui/src/text_system/line.rs`. Copyright Zed Industries and contributors,
Apache-2.0; see `LICENSE-APACHE`.
