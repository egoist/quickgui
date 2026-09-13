// SPDX-License-Identifier: MIT OR Apache-2.0

bitflags::bitflags! {
    /// Flags that change rendering
    #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[repr(transparent)]
    pub struct CacheKeyFlags: u32 {
        /// Skew by 14 degrees to synthesize italic
        const FAKE_ITALIC = 1;
        /// Disable hinting
        const DISABLE_HINTING = 2;
        /// Render as a pixel font
        const PIXEL_FONT = 4;
        /// Apply subtle optical stem thickening without selecting a heavier font face
        const FONT_THICKEN = 8;
        /// Use the platform rasterizer for native UI glyphs where available.
        const NATIVE_RASTERIZATION = 16;
    }
}

/// Key for building a glyph cache
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CacheKey {
    /// Font ID
    pub font_id: fontdb::ID,
    /// Glyph ID
    pub glyph_id: u16,
    /// `f32` bits of font size
    pub font_size_bits: u32,
    /// Logical optical size, independent of rasterization scale.
    pub optical_size_bits: u32,
    /// Native grayscale smoothing level (0–4), selected from the foreground color.
    pub dilation: u8,
    /// Binning of fractional X offset
    pub x_bin: SubpixelBin,
    /// Binning of fractional Y offset
    pub y_bin: SubpixelBin,
    /// Font weight
    pub font_weight: fontdb::Weight,
    /// [`CacheKeyFlags`]
    pub flags: CacheKeyFlags,
}

impl CacheKey {
    pub fn new(
        font_id: fontdb::ID,
        glyph_id: u16,
        font_size: f32,
        pos: (f32, f32),
        weight: fontdb::Weight,
        flags: CacheKeyFlags,
    ) -> (Self, i32, i32) {
        #[cfg(target_os = "macos")]
        let pos = if flags.contains(CacheKeyFlags::NATIVE_RASTERIZATION) {
            // Native UI text resolves exact half-bin ties toward zero.
            let x = ((pos.0.abs() * 4.0 - 0.5).ceil() / 4.0).copysign(pos.0);
            (x, pos.1)
        } else {
            pos
        };
        let (x, x_bin) = SubpixelBin::new(pos.0);
        let (y, y_bin) = SubpixelBin::new(pos.1);
        (
            Self {
                font_id,
                glyph_id,
                font_size_bits: font_size.to_bits(),
                optical_size_bits: 0,
                dilation: 0,
                x_bin,
                y_bin,
                flags,
                font_weight: weight,
            },
            x,
            y,
        )
    }

    /// Include the platform's color-dependent font smoothing in the atlas identity.
    #[allow(unused_mut)] // Only macOS varies its native masks by foreground color.
    pub fn with_color(mut self, color: crate::Color) -> Self {
        #[cfg(all(target_os = "macos", feature = "swash", feature = "std"))]
        if self.flags.contains(CacheKeyFlags::NATIVE_RASTERIZATION) {
            self.dilation = crate::macos::dilation_for_color(color);
        }
        let _ = color;
        self
    }
}

/// Binning of subpixel position for cache optimization
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SubpixelBin {
    Zero,
    One,
    Two,
    Three,
}

impl SubpixelBin {
    pub fn new(pos: f32) -> (i32, Self) {
        let trunc = pos as i32;
        let fract = pos - trunc as f32;

        if pos.is_sign_negative() {
            if fract > -0.125 {
                (trunc, Self::Zero)
            } else if fract > -0.375 {
                (trunc - 1, Self::Three)
            } else if fract > -0.625 {
                (trunc - 1, Self::Two)
            } else if fract > -0.875 {
                (trunc - 1, Self::One)
            } else {
                (trunc - 1, Self::Zero)
            }
        } else {
            #[allow(clippy::collapsible_else_if)]
            if fract < 0.125 {
                (trunc, Self::Zero)
            } else if fract < 0.375 {
                (trunc, Self::One)
            } else if fract < 0.625 {
                (trunc, Self::Two)
            } else if fract < 0.875 {
                (trunc, Self::Three)
            } else {
                (trunc + 1, Self::Zero)
            }
        }
    }

    pub const fn as_float(&self) -> f32 {
        match self {
            Self::Zero => 0.0,
            Self::One => 0.25,
            Self::Two => 0.5,
            Self::Three => 0.75,
        }
    }
}

#[test]
fn test_subpixel_bins() {
    // POSITIVE TESTS

    // Maps to 0.0
    assert_eq!(SubpixelBin::new(0.0), (0, SubpixelBin::Zero));
    assert_eq!(SubpixelBin::new(0.124), (0, SubpixelBin::Zero));

    // Maps to 0.25
    assert_eq!(SubpixelBin::new(0.125), (0, SubpixelBin::One));
    assert_eq!(SubpixelBin::new(0.25), (0, SubpixelBin::One));
    assert_eq!(SubpixelBin::new(0.374), (0, SubpixelBin::One));

    // Maps to 0.5
    assert_eq!(SubpixelBin::new(0.375), (0, SubpixelBin::Two));
    assert_eq!(SubpixelBin::new(0.5), (0, SubpixelBin::Two));
    assert_eq!(SubpixelBin::new(0.624), (0, SubpixelBin::Two));

    // Maps to 0.75
    assert_eq!(SubpixelBin::new(0.625), (0, SubpixelBin::Three));
    assert_eq!(SubpixelBin::new(0.75), (0, SubpixelBin::Three));
    assert_eq!(SubpixelBin::new(0.874), (0, SubpixelBin::Three));

    // Maps to 1.0
    assert_eq!(SubpixelBin::new(0.875), (1, SubpixelBin::Zero));
    assert_eq!(SubpixelBin::new(0.999), (1, SubpixelBin::Zero));
    assert_eq!(SubpixelBin::new(1.0), (1, SubpixelBin::Zero));
    assert_eq!(SubpixelBin::new(1.124), (1, SubpixelBin::Zero));

    // NEGATIVE TESTS

    // Maps to 0.0
    assert_eq!(SubpixelBin::new(-0.0), (0, SubpixelBin::Zero));
    assert_eq!(SubpixelBin::new(-0.124), (0, SubpixelBin::Zero));

    // Maps to 0.25
    assert_eq!(SubpixelBin::new(-0.125), (-1, SubpixelBin::Three));
    assert_eq!(SubpixelBin::new(-0.25), (-1, SubpixelBin::Three));
    assert_eq!(SubpixelBin::new(-0.374), (-1, SubpixelBin::Three));

    // Maps to 0.5
    assert_eq!(SubpixelBin::new(-0.375), (-1, SubpixelBin::Two));
    assert_eq!(SubpixelBin::new(-0.5), (-1, SubpixelBin::Two));
    assert_eq!(SubpixelBin::new(-0.624), (-1, SubpixelBin::Two));

    // Maps to 0.75
    assert_eq!(SubpixelBin::new(-0.625), (-1, SubpixelBin::One));
    assert_eq!(SubpixelBin::new(-0.75), (-1, SubpixelBin::One));
    assert_eq!(SubpixelBin::new(-0.874), (-1, SubpixelBin::One));

    // Maps to 1.0
    assert_eq!(SubpixelBin::new(-0.875), (-1, SubpixelBin::Zero));
    assert_eq!(SubpixelBin::new(-0.999), (-1, SubpixelBin::Zero));
    assert_eq!(SubpixelBin::new(-1.0), (-1, SubpixelBin::Zero));
    assert_eq!(SubpixelBin::new(-1.124), (-1, SubpixelBin::Zero));
}
