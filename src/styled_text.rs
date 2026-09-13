use std::{ops::Range, sync::Arc};

use glyphon::{Style as GlyphStyle, Weight};

use crate::{
    Color, Font, FontFallbacks, FontFamily, FontFeatures,
    font::{assert_valid_font_family, normalize_fallbacks},
};

/// Hard limit for the number of styled byte ranges retained by one text element.
///
/// This keeps adversarial syntax/highlight input from creating an unbounded amount of shaping
/// metadata or paint geometry in one element. Long documents should be split into visible blocks.
pub const MAX_TEXT_HIGHLIGHTS: usize = 4_096;

/// Maximum UTF-8 size of one custom family name retained by a highlighted run.
pub const MAX_HIGHLIGHT_FONT_FAMILY_BYTES: usize = 1_024;

/// Underline shape for one highlighted text range.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum TextUnderline {
    #[default]
    None,
    Single,
    Double,
}

/// A partial text style applied to one UTF-8 byte range in [`StyledText`].
///
/// Unspecified font properties inherit from the surrounding element. Backgrounds and text
/// decorations do not alter flexbox metrics. Foreground, font, and decoration configuration
/// participate in the retained Cosmic Text buffer key; changing only a background color does not.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HighlightStyle {
    pub(crate) link: Option<Arc<str>>,
    pub(crate) color: Option<Color>,
    pub(crate) background: Option<Color>,
    pub(crate) background_radius: f32,
    pub(crate) background_padding_x: f32,
    pub(crate) background_inset_y: f32,
    pub(crate) family: Option<FontFamily>,
    pub(crate) features: Option<FontFeatures>,
    pub(crate) fallbacks: Option<Option<FontFallbacks>>,
    pub(crate) weight: Option<Weight>,
    pub(crate) glyph_style: Option<GlyphStyle>,
    pub(crate) underline: TextUnderline,
    pub(crate) underline_color: Option<Color>,
    pub(crate) underline_wavy: Option<bool>,
    pub(crate) underline_thickness: Option<f32>,
    pub(crate) underline_descent_fraction: Option<f32>,
    pub(crate) strikethrough: bool,
    pub(crate) strikethrough_color: Option<Color>,
}

impl HighlightStyle {
    /// Open this URL when the range is clicked without selecting text.
    pub fn link(mut self, url: impl Into<Arc<str>>) -> Self {
        self.link = Some(url.into());
        self
    }

    /// Position the underline below the baseline as a fraction of the font descent.
    pub fn underline_descent_fraction(mut self, fraction: f32) -> Self {
        self.underline_descent_fraction = fraction.is_finite().then(|| fraction.clamp(0.0, 4.0));
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    /// Shape a paint-only range wash without changing text measurement or selection.
    pub fn background_shape(mut self, radius: f32, padding_x: f32, inset_y: f32) -> Self {
        let valid = |v: f32| {
            if v.is_finite() {
                v.clamp(0.0, 4096.0)
            } else {
                0.0
            }
        };
        self.background_radius = valid(radius);
        self.background_padding_x = valid(padding_x);
        self.background_inset_y = valid(inset_y);
        self
    }

    pub fn font_family(mut self, family: impl Into<FontFamily>) -> Self {
        let family = family.into();
        assert_valid_font_family(&family);
        self.family = Some(family);
        self
    }

    /// Override OpenType features for this range while inheriting all other font properties.
    pub fn font_features(mut self, features: FontFeatures) -> Self {
        self.features = Some(features);
        self
    }

    /// Override the ordered custom fallback stack for this range.
    ///
    /// Passing an empty stack intentionally clears the surrounding custom fallbacks.
    pub fn font_fallbacks(mut self, fallbacks: FontFallbacks) -> Self {
        self.fallbacks = Some((!fallbacks.is_empty()).then_some(fallbacks));
        self
    }

    /// Replace the complete font configuration for this range.
    pub fn font(mut self, font: Font) -> Self {
        assert_valid_font_family(&font.family);
        self.family = Some(font.family);
        self.features = Some(font.features);
        self.fallbacks = Some(normalize_fallbacks(font.fallbacks));
        self.weight = Some(font.weight);
        self.glyph_style = Some(font.style);
        self
    }

    pub fn font_weight(mut self, weight: Weight) -> Self {
        self.weight = Some(weight);
        self
    }

    pub fn font_normal(mut self) -> Self {
        self.weight = Some(Weight::NORMAL);
        self
    }

    pub fn font_medium(mut self) -> Self {
        self.weight = Some(Weight::MEDIUM);
        self
    }

    pub fn font_semibold(mut self) -> Self {
        self.weight = Some(Weight::SEMIBOLD);
        self
    }

    pub fn font_bold(mut self) -> Self {
        self.weight = Some(Weight::BOLD);
        self
    }

    pub fn italic(mut self) -> Self {
        self.glyph_style = Some(GlyphStyle::Italic);
        self
    }

    pub fn not_italic(mut self) -> Self {
        self.glyph_style = Some(GlyphStyle::Normal);
        self
    }

    pub fn underline(mut self) -> Self {
        self.underline = TextUnderline::Single;
        self.underline_wavy = Some(false);
        self.underline_thickness = Some(1.0);
        self
    }

    pub fn double_underline(mut self) -> Self {
        self.underline = TextUnderline::Double;
        self.underline_wavy = Some(false);
        self.underline_thickness = Some(1.0);
        self
    }

    pub fn underline_color(mut self, color: Color) -> Self {
        self.underline_color = Some(color);
        if self.underline == TextUnderline::None {
            self.underline = TextUnderline::Single;
            self.underline_wavy = Some(false);
            self.underline_thickness = Some(1.0);
        }
        self
    }

    pub fn text_decoration_solid(mut self) -> Self {
        if self.underline == TextUnderline::None {
            self.underline = TextUnderline::Single;
            self.underline_thickness = Some(1.0);
        }
        self.underline_wavy = Some(false);
        self
    }

    pub fn text_decoration_wavy(mut self) -> Self {
        if self.underline == TextUnderline::None {
            self.underline = TextUnderline::Single;
            self.underline_thickness = Some(1.0);
        }
        self.underline_wavy = Some(true);
        self
    }

    fn text_decoration_thickness(mut self, thickness: f32) -> Self {
        if self.underline == TextUnderline::None {
            self.underline = TextUnderline::Single;
        }
        self.underline_thickness = Some(thickness);
        self
    }

    pub fn text_decoration_0(self) -> Self {
        self.text_decoration_thickness(0.0)
    }

    pub fn text_decoration_1(self) -> Self {
        self.text_decoration_thickness(1.0)
    }

    pub fn text_decoration_2(self) -> Self {
        self.text_decoration_thickness(2.0)
    }

    pub fn text_decoration_4(self) -> Self {
        self.text_decoration_thickness(4.0)
    }

    pub fn text_decoration_8(self) -> Self {
        self.text_decoration_thickness(8.0)
    }

    pub fn strikethrough(mut self) -> Self {
        self.strikethrough = true;
        self
    }

    pub fn strikethrough_color(mut self, color: Color) -> Self {
        self.strikethrough = true;
        self.strikethrough_color = Some(color);
        self
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.link.is_none()
            && self.color.is_none()
            && self.background.is_none()
            && self.family.is_none()
            && self.features.is_none()
            && self.fallbacks.is_none()
            && self.weight.is_none()
            && self.glyph_style.is_none()
            && self.underline == TextUnderline::None
            && self.underline_color.is_none()
            && self.underline_wavy.is_none()
            && self.underline_thickness.is_none()
            && self.underline_descent_fraction.is_none()
            && !self.strikethrough
            && self.strikethrough_color.is_none()
    }
}

/// One validated styled UTF-8 byte range.
#[derive(Clone, Debug, PartialEq)]
pub struct TextHighlight {
    pub(crate) range: Range<usize>,
    pub(crate) style: HighlightStyle,
}

impl TextHighlight {
    pub fn range(&self) -> Range<usize> {
        self.range.clone()
    }

    pub fn style(&self) -> &HighlightStyle {
        &self.style
    }
}

/// A retained text element with sorted, non-overlapping byte-range styles.
///
/// The complete string is shaped in one Cosmic Text buffer. Cloning this value shares both its
/// UTF-8 content and immutable highlight table.
#[derive(Clone, Debug, PartialEq)]
pub struct StyledText {
    content: Arc<str>,
    highlights: Arc<[TextHighlight]>,
}

impl StyledText {
    pub fn new(content: impl Into<Arc<str>>) -> Self {
        Self {
            content: content.into(),
            highlights: Arc::from([]),
        }
    }

    /// Replace the complete highlight table.
    ///
    /// Ranges are UTF-8 byte offsets and must be sorted, non-overlapping, in bounds, and on
    /// character boundaries. Empty ranges and empty styles are omitted. The hard run limit is
    /// checked after those no-op entries are removed.
    pub fn with_highlights(
        mut self,
        highlights: impl IntoIterator<Item = (Range<usize>, HighlightStyle)>,
    ) -> Self {
        let mut validated = Vec::new();
        let mut previous_end = 0;
        for (range, style) in highlights {
            assert!(
                range.start <= range.end && range.end <= self.content.len(),
                "styled-text range {range:?} is outside content length {}",
                self.content.len()
            );
            assert!(
                self.content.is_char_boundary(range.start)
                    && self.content.is_char_boundary(range.end),
                "styled-text range {range:?} must use UTF-8 character boundaries"
            );
            if range.is_empty() || style.is_empty() {
                continue;
            }
            assert!(
                !matches!(
                    style.family.as_ref(),
                    Some(FontFamily::Named(name)) if name.len() > MAX_HIGHLIGHT_FONT_FAMILY_BYTES
                ),
                "styled-text font family names support at most {MAX_HIGHLIGHT_FONT_FAMILY_BYTES} UTF-8 bytes"
            );
            if let Some(family) = style.family.as_ref() {
                assert_valid_font_family(family);
            }
            assert!(
                range.start >= previous_end,
                "styled-text ranges must be sorted and non-overlapping"
            );
            previous_end = range.end;
            assert!(
                validated.len() < MAX_TEXT_HIGHLIGHTS,
                "styled text supports at most {MAX_TEXT_HIGHLIGHTS} non-empty highlights"
            );
            validated.push(TextHighlight { range, style });
        }
        self.highlights = validated.into();
        self
    }

    /// Add one highlight after every currently retained range.
    pub fn highlight(self, range: Range<usize>, style: HighlightStyle) -> Self {
        let mut highlights = self.highlights.to_vec();
        highlights.push(TextHighlight { range, style });
        self.with_highlights(
            highlights
                .into_iter()
                .map(|highlight| (highlight.range, highlight.style)),
        )
    }

    pub fn content(&self) -> &Arc<str> {
        &self.content
    }

    pub fn highlights(&self) -> &[TextHighlight] {
        &self.highlights
    }

    pub(crate) fn shared_highlights(&self) -> &Arc<[TextHighlight]> {
        &self.highlights
    }

    pub(crate) fn into_parts(self) -> (Arc<str>, Arc<[TextHighlight]>) {
        (self.content, self.highlights)
    }
}

/// Construct a retained styled-text element for use with `.child(...)`.
pub fn styled_text(content: impl Into<Arc<str>>) -> StyledText {
    StyledText::new(content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highlights_are_shared_and_keep_unicode_safe_byte_ranges() {
        let text = StyledText::new("a🙂z").with_highlights([
            (0..1, HighlightStyle::default().font_bold()),
            (
                1..5,
                HighlightStyle::default().color(Color::rgb8(56, 189, 248)),
            ),
        ]);
        let clone = text.clone();

        assert_eq!(text.highlights().len(), 2);
        assert_eq!(text.highlights()[1].range(), 1..5);
        assert!(Arc::ptr_eq(
            text.shared_highlights(),
            clone.shared_highlights()
        ));
    }

    #[test]
    #[should_panic(expected = "UTF-8 character boundaries")]
    fn highlights_reject_ranges_inside_a_unicode_scalar() {
        let _ =
            StyledText::new("🙂").with_highlights([(1..4, HighlightStyle::default().font_bold())]);
    }

    #[test]
    #[should_panic(expected = "sorted and non-overlapping")]
    fn highlights_reject_overlapping_ranges() {
        let _ = StyledText::new("abcdef").with_highlights([
            (1..4, HighlightStyle::default().font_bold()),
            (3..5, HighlightStyle::default().underline()),
        ]);
    }

    #[test]
    fn no_op_highlights_do_not_consume_retained_runs() {
        let text = StyledText::new("abc").with_highlights([
            (0..0, HighlightStyle::default().font_bold()),
            (0..3, HighlightStyle::default()),
        ]);
        assert!(text.highlights().is_empty());
    }

    #[test]
    fn highlighted_ranges_support_wavy_and_fixed_thickness_underlines() {
        let wavy = HighlightStyle::default()
            .text_decoration_wavy()
            .text_decoration_4();
        assert_eq!(wavy.underline, TextUnderline::Single);
        assert_eq!(wavy.underline_wavy, Some(true));
        assert_eq!(wavy.underline_thickness, Some(4.0));

        let solid = wavy.text_decoration_solid().text_decoration_0();
        assert_eq!(solid.underline_wavy, Some(false));
        assert_eq!(solid.underline_thickness, Some(0.0));
        assert!(!solid.is_empty());
    }

    #[test]
    fn highlighted_ranges_support_partial_and_complete_font_configuration() {
        let features = FontFeatures::new().disable(crate::FontFeatureTag::STANDARD_LIGATURES);
        let fallbacks = FontFallbacks::from_fonts(["Noto Sans Hebrew"]);
        let partial = HighlightStyle::default()
            .font_features(features.clone())
            .font_fallbacks(fallbacks.clone());
        assert_eq!(partial.features, Some(features));
        assert_eq!(partial.fallbacks, Some(Some(fallbacks)));
        assert!(partial.family.is_none());

        let complete = HighlightStyle::default().font(
            Font::new("Inter")
                .features(FontFeatures::new().enable(crate::FontFeatureTag::TABULAR_NUMBERS))
                .bold(),
        );
        assert_eq!(complete.family, Some(FontFamily::named("Inter")));
        assert_eq!(complete.fallbacks, Some(None));
        assert_eq!(complete.weight, Some(Weight::BOLD));
        assert_eq!(complete.glyph_style, Some(GlyphStyle::Normal));
    }

    #[test]
    #[should_panic(expected = "font family names support at most")]
    fn highlighted_family_names_are_byte_bounded() {
        let family = FontFamily::Named(Arc::from("x".repeat(MAX_HIGHLIGHT_FONT_FAMILY_BYTES + 1)));
        let _ = StyledText::new("x")
            .with_highlights([(0..1, HighlightStyle::default().font_family(family))]);
    }
}
