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

## create-dmg

`packages/cli/vendor/create-dmg` is create-dmg 1.3.0 from
https://github.com/create-dmg/create-dmg, used to package production macOS disk images.
create-dmg is Copyright 2008-2014 Andrey Tarantsov and Copyright 2020 Andrew Janke, and
is licensed under the MIT License; see `packages/cli/vendor/create-dmg/LICENSE`.

## Text-shaping and documentation fonts

`tests/fixtures/fonts/Inter-Regular.ttf` is Copyright 2020 The Inter Project Authors, and
`tests/fixtures/fonts/NotoSansHebrew.ttf` is Copyright 2012 Google Inc. Both test fixtures are
licensed under the SIL Open Font License 1.1; see `tests/fixtures/fonts/Inter-LICENSE` and
`tests/fixtures/fonts/NotoSans-LICENSE`. Noto Sans is compiled only into framework tests.
Inter is also embedded in the browser documentation demos; their build includes
`Inter-LICENSE.txt` beside the generated WebAssembly bundle.
