//! Optional gutter and indentation primitives for decorated multiline text inputs.
use crate::Color;

/// Editing behavior carried by a code input's retained state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TextInputIndentation {
    pub tab_size: u8,
    pub insert_spaces: bool,
    pub auto_indent: bool,
    pub read_only: bool,
}

impl Default for TextInputIndentation {
    fn default() -> Self {
        Self {
            tab_size: 4,
            insert_spaces: true,
            auto_indent: true,
            read_only: false,
        }
    }
}

impl TextInputIndentation {
    pub fn tab_size(mut self, size: usize) -> Self {
        self.tab_size = size.clamp(1, 16) as u8;
        self
    }

    pub const fn insert_spaces(mut self, insert_spaces: bool) -> Self {
        self.insert_spaces = insert_spaces;
        self
    }

    pub const fn auto_indent(mut self, auto_indent: bool) -> Self {
        self.auto_indent = auto_indent;
        self
    }

    pub const fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    pub(crate) fn sanitized(mut self) -> Self {
        self.tab_size = self.tab_size.clamp(1, 16);
        self
    }
}

/// Paint-only editor chrome consumed by the retained text-input renderer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextInputGutter {
    pub line_numbers: bool,
    pub minimum_line_number_digits: u8,
    pub gutter_background: Color,
    pub gutter_foreground: Option<Color>,
    pub gutter_active_foreground: Option<Color>,
    pub gutter_border: Color,
    pub active_line_background: Color,
    pub content_padding_left: f32,
    pub content_padding_right: f32,
    pub content_padding_y: f32,
    pub gutter_padding_left: f32,
    pub gutter_padding_right: f32,
}

impl Default for TextInputGutter {
    fn default() -> Self {
        Self {
            line_numbers: true,
            minimum_line_number_digits: 2,
            gutter_background: Color::TRANSPARENT,
            gutter_foreground: None,
            gutter_active_foreground: None,
            gutter_border: Color::TRANSPARENT,
            active_line_background: Color::TRANSPARENT,
            content_padding_left: 0.0,
            content_padding_right: 0.0,
            content_padding_y: 0.0,
            gutter_padding_left: 0.0,
            gutter_padding_right: 0.0,
        }
    }
}

impl TextInputGutter {
    pub(crate) fn sanitized(mut self) -> Self {
        self.minimum_line_number_digits = self.minimum_line_number_digits.clamp(1, 12);
        self.content_padding_left = finite_inset(self.content_padding_left, 0.0);
        self.content_padding_right = finite_inset(self.content_padding_right, 0.0);
        self.content_padding_y = finite_inset(self.content_padding_y, 0.0);
        self.gutter_padding_left = finite_inset(self.gutter_padding_left, 0.0);
        self.gutter_padding_right = finite_inset(self.gutter_padding_right, 0.0);
        self
    }
}

fn finite_inset(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 256.0)
    } else {
        fallback
    }
}
