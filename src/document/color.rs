use crate::Color;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hsla {
    pub h: f32,
    pub s: f32,
    pub l: f32,
    pub a: f32,
}

pub const fn hsla(h: f32, s: f32, l: f32, a: f32) -> Hsla {
    Hsla { h, s, l, a }
}

impl From<Hsla> for Color {
    fn from(value: Hsla) -> Self {
        let chroma = (1.0 - (2.0 * value.l - 1.0).abs()) * value.s;
        let h = value.h.rem_euclid(1.0) * 6.0;
        let x = chroma * (1.0 - (h.rem_euclid(2.0) - 1.0).abs());
        let (r, g, b) = match h as u8 {
            0 => (chroma, x, 0.0),
            1 => (x, chroma, 0.0),
            2 => (0.0, chroma, x),
            3 => (0.0, x, chroma),
            4 => (x, 0.0, chroma),
            _ => (chroma, 0.0, x),
        };
        let m = value.l - chroma * 0.5;
        fn linear(x: f32) -> f32 {
            if x <= 0.04045 {
                x / 12.92
            } else {
                ((x + 0.055) / 1.055).powf(2.4)
            }
        }
        Color::linear(linear(r + m), linear(g + m), linear(b + m), value.a)
    }
}

pub fn parse_color(value: &str) -> Option<Hsla> {
    let color = value.parse::<csscolorparser::Color>().ok()?;
    let [r, g, b, a] = color.to_array().map(|v| v as f32);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) * 0.5;
    let delta = max - min;
    if delta < f32::EPSILON {
        return Some(hsla(0.0, 0.0, l, a));
    }
    let s = delta / (1.0 - (2.0 * l - 1.0).abs());
    let h = if max == r {
        ((g - b) / delta).rem_euclid(6.0)
    } else if max == g {
        (b - r) / delta + 2.0
    } else {
        (r - g) / delta + 4.0
    };
    Some(hsla(h / 6.0, s, l, a))
}
