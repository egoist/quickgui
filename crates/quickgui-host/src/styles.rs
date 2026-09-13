//! Declared CSS-shaped text, box, and layout styling mapped onto the Rust core's own style API.
//!
//! Every function here is a translator, never an implementation: a declaration is parsed once into
//! the core's own bounded value types ([`Gradient`], [`Filter`], [`Transform2D`], [`Corners`],
//! [`Outline`], [`TextShadow`], …) and handed to the matching [`Element`] builder. A malformed
//! declaration produces `None` and the property is simply not applied, so no input from JavaScript
//! can panic the renderer or reach the core unvalidated.

use super::*;
use serde::Deserialize;

use quickgui::{
    BackgroundPosition, BackgroundRepeat, BackgroundSize, BlendMode, BorderStyle, ColorStops,
    Corners, Direction, DropShadow, ElementStateStyle, Filter, Gradient, GradientCenter,
    GradientColorSpace, Hyphens, LinearColorStop, MAX_FILTERS_PER_ELEMENT, MAX_GRADIENT_STOPS,
    MAX_GROUP_STYLES_PER_ELEMENT, MAX_HOVER_GROUP_NAME_BYTES, Outline, OverflowWrap,
    RadialGradientExtent, RadialGradientShape, SnapAlign, SnapStrictness, TextDirection,
    TextShadow, TextTransform, Transform2D, WordBreak,
};

/// Longest declared style string parsed from one property.
///
/// Gradients, filter chains, and transform lists are all fixed-size core values, so a declaration
/// longer than this can only be malformed. It is rejected before any tokenizing runs.
pub(super) const MAX_STYLE_DECLARATION_BYTES: usize = 4_096;

/// Most comma-separated arguments read from one declared style function.
const MAX_STYLE_ARGUMENTS: usize = 32;

// ---------------------------------------------------------------------------------------------
// Colors
// ---------------------------------------------------------------------------------------------

/// Parse the same bounded CSS color grammar the JavaScript `parseColor` helper accepts.
///
/// Colors reach most properties already packed as a `u32` by the renderer. They stay textual only
/// inside a gradient, filter, or shadow declaration, which the core parses as one whole value.
pub(super) fn parse_css_color(value: &str) -> Option<Color> {
    let value = value.trim();
    if value.is_empty() || value.len() > 128 {
        return None;
    }
    if value.eq_ignore_ascii_case("transparent") {
        return Some(Color::TRANSPARENT);
    }
    if value.eq_ignore_ascii_case("black") {
        return Some(Color::rgb8(0, 0, 0));
    }
    if value.eq_ignore_ascii_case("white") {
        return Some(Color::rgb8(255, 255, 255));
    }
    if let Some(hex) = value.strip_prefix('#') {
        return parse_hex_color(hex);
    }
    let (name, arguments) = split_function(value)?;
    if !name.eq_ignore_ascii_case("rgb") && !name.eq_ignore_ascii_case("rgba") {
        return None;
    }
    let parts = split_arguments(arguments);
    if parts.len() != 3 && parts.len() != 4 {
        return None;
    }
    let channel = |index: usize| -> Option<u8> {
        let part = parts.get(index)?;
        let value = if let Some(percentage) = part.strip_suffix('%') {
            parse_finite_number(percentage)? * 2.55
        } else {
            parse_finite_number(part)?
        };
        Some(value.clamp(0.0, 255.0).round() as u8)
    };
    let alpha = match parts.get(3) {
        None => 255u8,
        Some(part) => {
            let value = if let Some(percentage) = part.strip_suffix('%') {
                parse_finite_number(percentage)? / 100.0
            } else {
                parse_finite_number(part)?
            };
            (value.clamp(0.0, 1.0) * 255.0).round() as u8
        }
    };
    Some(Color::rgba8(channel(0)?, channel(1)?, channel(2)?, alpha))
}

fn parse_hex_color(hex: &str) -> Option<Color> {
    if !hex.chars().all(|character| character.is_ascii_hexdigit()) {
        return None;
    }
    let pair = |text: &str| u8::from_str_radix(text, 16).ok();
    let double = |character: char| {
        let digit = character.to_digit(16)? as u8;
        Some(digit * 16 + digit)
    };
    match hex.len() {
        3 | 4 => {
            let mut characters = hex.chars();
            let red = double(characters.next()?)?;
            let green = double(characters.next()?)?;
            let blue = double(characters.next()?)?;
            let alpha = match characters.next() {
                Some(character) => double(character)?,
                None => 255,
            };
            Some(Color::rgba8(red, green, blue, alpha))
        }
        6 | 8 => Some(Color::rgba8(
            pair(&hex[0..2])?,
            pair(&hex[2..4])?,
            pair(&hex[4..6])?,
            if hex.len() == 8 {
                pair(&hex[6..8])?
            } else {
                255
            },
        )),
        _ => None,
    }
}

// ---------------------------------------------------------------------------------------------
// Shared tokenizing
// ---------------------------------------------------------------------------------------------

/// Split `name(arguments)` into its parts, keeping nested parentheses inside the arguments.
fn split_function(value: &str) -> Option<(&str, &str)> {
    let value = value.trim();
    let open = value.find('(')?;
    if !value.ends_with(')') {
        return None;
    }
    let name = value[..open].trim();
    if name.is_empty() {
        return None;
    }
    Some((name, &value[open + 1..value.len() - 1]))
}

/// Split a comma-separated argument list while keeping parenthesized functions intact.
fn split_arguments(value: &str) -> Vec<String> {
    split_on(value, |character| character == ',')
}

/// Split a whitespace-separated token list while keeping parenthesized functions intact.
fn split_tokens(value: &str) -> Vec<String> {
    split_on(value, char::is_whitespace)
}

fn split_on(value: &str, is_separator: impl Fn(char) -> bool) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut depth = 0usize;
    for character in value.chars() {
        match character {
            '(' => {
                depth += 1;
                current.push(character);
            }
            ')' => {
                depth = depth.saturating_sub(1);
                current.push(character);
            }
            character if depth == 0 && is_separator(character) => {
                let part = std::mem::take(&mut current);
                let part = part.trim().to_owned();
                if !part.is_empty() {
                    parts.push(part);
                }
                if parts.len() >= MAX_STYLE_ARGUMENTS {
                    return parts;
                }
            }
            character => current.push(character),
        }
    }
    let part = current.trim().to_owned();
    if !part.is_empty() {
        parts.push(part);
    }
    parts
}

fn parse_finite_number(value: &str) -> Option<f32> {
    value
        .trim()
        .parse::<f32>()
        .ok()
        .filter(|value| value.is_finite())
}

/// A logical-pixel length. A bare number and a `px` length mean the same thing.
fn parse_length(value: &str) -> Option<f32> {
    let value = value.trim();
    parse_finite_number(value.strip_suffix("px").unwrap_or(value))
}

/// A CSS angle in degrees, accepting `deg`, `rad`, `grad`, and `turn`.
fn parse_angle(value: &str) -> Option<f32> {
    let value = value.trim();
    for (suffix, scale) in [
        ("deg", 1.0),
        ("grad", 0.9),
        ("turn", 360.0),
        ("rad", 180.0 / std::f32::consts::PI),
    ] {
        if let Some(number) = value.strip_suffix(suffix) {
            return Some(parse_finite_number(number)? * scale);
        }
    }
    parse_finite_number(value)
}

/// A CSS `<number> | <percentage>` amount, where `50%` is `0.5`.
fn parse_amount(value: &str) -> Option<f32> {
    let value = value.trim();
    match value.strip_suffix('%') {
        Some(percentage) => Some(parse_finite_number(percentage)? / 100.0),
        None => parse_finite_number(value),
    }
}

/// A fraction of a box: `30%`, `0.3`, or one of the CSS position keywords.
fn parse_fraction(value: &str, start: &str, end: &str) -> Option<f32> {
    let value = value.trim();
    if value.eq_ignore_ascii_case("center") {
        return Some(0.5);
    }
    if value.eq_ignore_ascii_case(start) {
        return Some(0.0);
    }
    if value.eq_ignore_ascii_case(end) {
        return Some(1.0);
    }
    parse_amount(value)
}

// ---------------------------------------------------------------------------------------------
// Gradients
// ---------------------------------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(untagged)]
enum DeclaredStop {
    Color(String),
    Positioned {
        color: String,
        #[serde(default)]
        position: Option<f32>,
    },
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DeclaredCenter {
    x: f32,
    y: f32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DeclaredGradient {
    #[serde(default)]
    dither: bool,
    #[serde(default)]
    box_projection: bool,
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    angle: Option<f32>,
    #[serde(default)]
    from_angle: Option<f32>,
    #[serde(default)]
    shape: Option<String>,
    #[serde(default)]
    extent: Option<String>,
    #[serde(default)]
    center: Option<DeclaredCenter>,
    #[serde(default)]
    interpolation: Option<String>,
    stops: Vec<DeclaredStop>,
}

/// Build the core gradient a `background` declaration asked for.
///
/// Accepts the CSS `linear-gradient()`, `radial-gradient()`, and `conic-gradient()` functions and
/// the equivalent JSON object form. Stops past [`MAX_GRADIENT_STOPS`] are dropped by the core's own
/// bounded [`ColorStops`], and a declaration the grammar does not cover paints nothing at all.
pub(super) fn parse_gradient(value: &str) -> Option<Gradient> {
    let value = value.trim();
    if value.is_empty() || value.len() > MAX_STYLE_DECLARATION_BYTES {
        return None;
    }
    if value.starts_with('{') {
        return parse_gradient_object(value);
    }
    parse_gradient_css(value)
}

fn parse_gradient_object(value: &str) -> Option<Gradient> {
    let declared = serde_json::from_str::<DeclaredGradient>(value).ok()?;
    let stops = declared_stops(&declared.stops)?;
    let center = declared
        .center
        .map(|center| GradientCenter::new(center.x, center.y))
        .unwrap_or(GradientCenter::CENTER);
    let gradient = match declared.kind.as_str() {
        "linear" => Gradient::linear(declared.angle.unwrap_or(180.0), stops),
        "radial" => Gradient::radial(stops)
            .shape(match declared.shape.as_deref() {
                Some("circle") => RadialGradientShape::Circle,
                _ => RadialGradientShape::Ellipse,
            })
            .extent(radial_extent(declared.extent.as_deref()))
            .center(center),
        "conic" => Gradient::conic(declared.from_angle.or(declared.angle).unwrap_or(0.0), stops)
            .center(center),
        _ => return None,
    };
    Some(
        gradient
            .color_space(color_space(declared.interpolation.as_deref()))
            .dither(declared.dither)
            .box_projection(declared.box_projection),
    )
}

fn declared_stops(stops: &[DeclaredStop]) -> Option<ColorStops> {
    let mut parsed = Vec::with_capacity(stops.len().min(MAX_GRADIENT_STOPS));
    let mut positioned = false;
    for stop in stops.iter().take(MAX_GRADIENT_STOPS) {
        let (text, position) = match stop {
            DeclaredStop::Color(color) => (color.as_str(), None),
            DeclaredStop::Positioned { color, position } => (color.as_str(), *position),
        };
        positioned |= position.is_some();
        parsed.push((parse_css_color(text)?, position));
    }
    if parsed.is_empty() {
        return None;
    }
    Some(collect_stops(parsed, positioned))
}

/// Distribute unpositioned stops evenly, exactly as CSS does, then hand them to the core.
fn collect_stops(stops: Vec<(Color, Option<f32>)>, positioned: bool) -> ColorStops {
    if !positioned {
        return ColorStops::evenly_spaced(stops.into_iter().map(|(color, _)| color));
    }
    let last = stops.len().saturating_sub(1);
    ColorStops::new(stops.iter().enumerate().map(|(index, (color, position))| {
        let position = position.unwrap_or(if last == 0 {
            0.0
        } else {
            index as f32 / last as f32
        });
        LinearColorStop::new(*color, position)
    }))
}

fn radial_extent(value: Option<&str>) -> RadialGradientExtent {
    match value {
        Some("closest-side") => RadialGradientExtent::ClosestSide,
        Some("farthest-side") => RadialGradientExtent::FarthestSide,
        _ => RadialGradientExtent::FarthestCorner,
    }
}

fn color_space(value: Option<&str>) -> GradientColorSpace {
    match value {
        Some("srgb") => GradientColorSpace::Srgb,
        Some("oklab") => GradientColorSpace::Oklab,
        _ => GradientColorSpace::LinearSrgb,
    }
}

fn parse_gradient_css(value: &str) -> Option<Gradient> {
    let (name, arguments) = split_function(value)?;
    let mut parts = split_arguments(arguments);
    if parts.is_empty() {
        return None;
    }
    // `linear-gradient(in oklab, ...)` declares the interpolation space ahead of the geometry.
    let mut space = GradientColorSpace::LinearSrgb;
    if let Some(rest) = parts[0].strip_prefix("in ") {
        space = color_space(Some(rest.trim()));
        parts.remove(0);
        if parts.is_empty() {
            return None;
        }
    }

    let name = name.to_ascii_lowercase();
    let gradient = match name.as_str() {
        "linear-gradient" => {
            let mut angle = 180.0;
            if let Some(declared) = parse_linear_angle(&parts[0]) {
                angle = declared;
                parts.remove(0);
            }
            Gradient::linear(angle, css_stops(&parts)?)
        }
        "radial-gradient" => {
            let mut shape = RadialGradientShape::Ellipse;
            let mut extent = RadialGradientExtent::FarthestCorner;
            let mut center = GradientCenter::CENTER;
            if let Some(geometry) = parse_radial_geometry(&parts[0]) {
                (shape, extent, center) = geometry;
                parts.remove(0);
            }
            Gradient::radial(css_stops(&parts)?)
                .shape(shape)
                .extent(extent)
                .center(center)
        }
        "conic-gradient" => {
            let mut from_angle = 0.0;
            let mut center = GradientCenter::CENTER;
            if let Some(geometry) = parse_conic_geometry(&parts[0]) {
                (from_angle, center) = geometry;
                parts.remove(0);
            }
            Gradient::conic(from_angle, css_stops(&parts)?).center(center)
        }
        _ => return None,
    };
    Some(gradient.color_space(space))
}

/// `to bottom right`, `45deg`, or nothing at all.
fn parse_linear_angle(value: &str) -> Option<f32> {
    let value = value.trim();
    if let Some(sides) = value.strip_prefix("to ") {
        let mut top = false;
        let mut bottom = false;
        let mut left = false;
        let mut right = false;
        for token in split_tokens(sides) {
            match token.as_str() {
                "top" => top = true,
                "bottom" => bottom = true,
                "left" => left = true,
                "right" => right = true,
                _ => return None,
            }
        }
        return Some(match (top, right, bottom, left) {
            (true, true, _, _) => 45.0,
            (_, true, true, _) => 135.0,
            (_, _, true, true) => 225.0,
            (true, _, _, true) => 315.0,
            (true, ..) => 0.0,
            (_, true, ..) => 90.0,
            (_, _, true, _) => 180.0,
            (_, _, _, true) => 270.0,
            _ => return None,
        });
    }
    // A stop always starts with a color, so an angle is the only numeric first argument.
    if parse_css_color(value).is_some() {
        return None;
    }
    parse_angle(value)
}

type RadialGeometry = (RadialGradientShape, RadialGradientExtent, GradientCenter);

fn parse_radial_geometry(value: &str) -> Option<RadialGeometry> {
    let mut shape = RadialGradientShape::Ellipse;
    let mut extent = RadialGradientExtent::FarthestCorner;
    let mut center = GradientCenter::CENTER;
    let mut recognized = false;
    let tokens = split_tokens(value);
    let mut index = 0;
    while index < tokens.len() {
        match tokens[index].as_str() {
            "circle" => shape = RadialGradientShape::Circle,
            "ellipse" => shape = RadialGradientShape::Ellipse,
            "closest-side" => extent = RadialGradientExtent::ClosestSide,
            "farthest-side" => extent = RadialGradientExtent::FarthestSide,
            "farthest-corner" => extent = RadialGradientExtent::FarthestCorner,
            "at" => {
                center = parse_center(&tokens[index + 1..])?;
                recognized = true;
                break;
            }
            _ => return None,
        }
        recognized = true;
        index += 1;
    }
    recognized.then_some((shape, extent, center))
}

fn parse_conic_geometry(value: &str) -> Option<(f32, GradientCenter)> {
    let mut from_angle = 0.0;
    let mut center = GradientCenter::CENTER;
    let mut recognized = false;
    let tokens = split_tokens(value);
    let mut index = 0;
    while index < tokens.len() {
        match tokens[index].as_str() {
            "from" => {
                from_angle = parse_angle(tokens.get(index + 1)?)?;
                index += 2;
            }
            "at" => {
                center = parse_center(&tokens[index + 1..])?;
                index = tokens.len();
            }
            _ => return None,
        }
        recognized = true;
    }
    recognized.then_some((from_angle, center))
}

fn parse_center(tokens: &[String]) -> Option<GradientCenter> {
    let x = parse_fraction(tokens.first()?, "left", "right")?;
    let y = match tokens.get(1) {
        Some(token) => parse_fraction(token, "top", "bottom")?,
        None => 0.5,
    };
    Some(GradientCenter::new(x, y))
}

fn css_stops(parts: &[String]) -> Option<ColorStops> {
    let mut stops = Vec::with_capacity(parts.len().min(MAX_GRADIENT_STOPS));
    let mut positioned = false;
    for part in parts.iter().take(MAX_GRADIENT_STOPS) {
        let tokens = split_tokens(part);
        let color = parse_css_color(tokens.first()?)?;
        let position = match tokens.get(1) {
            Some(token) => {
                positioned = true;
                Some(parse_amount(token)?)
            }
            None => None,
        };
        stops.push((color, position));
    }
    if stops.is_empty() {
        return None;
    }
    Some(collect_stops(stops, positioned))
}

// ---------------------------------------------------------------------------------------------
// Filters and transforms
// ---------------------------------------------------------------------------------------------

/// Parse a CSS filter-function list into the core's bounded filter chain.
pub(super) fn parse_filters(value: &str) -> Option<Vec<Filter>> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > MAX_STYLE_DECLARATION_BYTES
        || value.eq_ignore_ascii_case("none")
    {
        return None;
    }
    let mut filters = Vec::new();
    for token in split_tokens(value) {
        if filters.len() == MAX_FILTERS_PER_ELEMENT {
            break;
        }
        filters.push(parse_filter(&token)?);
    }
    (!filters.is_empty()).then_some(filters)
}

fn parse_filter(value: &str) -> Option<Filter> {
    let (name, arguments) = split_function(value)?;
    let name = name.to_ascii_lowercase();
    if name == "drop-shadow" {
        return parse_drop_shadow(arguments).map(Filter::DropShadow);
    }
    // `hue-rotate` takes an angle and `blur` a length, so the amount grammar is only consulted for
    // the filters CSS actually defines it for.
    Some(match name.as_str() {
        "brightness" => Filter::Brightness(parse_amount(arguments)?),
        "contrast" => Filter::Contrast(parse_amount(arguments)?),
        "saturate" => Filter::Saturate(parse_amount(arguments)?),
        "grayscale" => Filter::Grayscale(parse_amount(arguments)?),
        "invert" => Filter::Invert(parse_amount(arguments)?),
        "sepia" => Filter::Sepia(parse_amount(arguments)?),
        "opacity" => Filter::Opacity(parse_amount(arguments)?),
        "hue-rotate" => Filter::HueRotate(parse_angle(arguments)?),
        "blur" => Filter::Blur(parse_length(arguments)?),
        _ => return None,
    })
}

fn parse_drop_shadow(arguments: &str) -> Option<DropShadow> {
    let tokens = split_tokens(arguments);
    let mut lengths = Vec::new();
    let mut color = Color::BLACK;
    for token in &tokens {
        match parse_length(token) {
            Some(length) if lengths.len() < 3 => lengths.push(length),
            Some(_) => return None,
            None => color = parse_css_color(token)?,
        }
    }
    if lengths.len() < 2 {
        return None;
    }
    Some(DropShadow::new(
        quickgui::Vector::new(lengths[0], lengths[1]),
        lengths.get(2).copied().unwrap_or(0.0),
        color,
    ))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DeclaredMatrix {
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    tx: f32,
    ty: f32,
}

/// Parse a CSS transform-function list, or the equivalent `matrix()` object, into one transform.
///
/// Functions compose exactly as CSS does: the leftmost function is the outermost, so the rightmost
/// is applied to the point first.
pub(super) fn parse_transform(value: &str) -> Option<Transform2D> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > MAX_STYLE_DECLARATION_BYTES
        || value.eq_ignore_ascii_case("none")
    {
        return None;
    }
    if value.starts_with('{') {
        let matrix = serde_json::from_str::<DeclaredMatrix>(value).ok()?;
        return Some(Transform2D::new(
            matrix.a, matrix.b, matrix.c, matrix.d, matrix.tx, matrix.ty,
        ));
    }
    let mut transform = Transform2D::IDENTITY;
    let mut declared = false;
    for token in split_tokens(value) {
        transform = transform.compose(parse_transform_function(&token)?);
        declared = true;
    }
    declared.then_some(transform)
}

fn parse_transform_function(value: &str) -> Option<Transform2D> {
    let (name, arguments) = split_function(value)?;
    let parts = split_arguments(arguments);
    let length = |index: usize| parts.get(index).and_then(|part| parse_length(part));
    let number = |index: usize| parts.get(index).and_then(|part| parse_amount(part));
    let angle = |index: usize| parts.get(index).and_then(|part| parse_angle(part));
    Some(match name.to_ascii_lowercase().as_str() {
        "translate" => Transform2D::translate(length(0)?, length(1).unwrap_or(0.0)),
        "translatex" => Transform2D::translate(length(0)?, 0.0),
        "translatey" => Transform2D::translate(0.0, length(0)?),
        "scale" => {
            let x = number(0)?;
            Transform2D::scale(x, number(1).unwrap_or(x))
        }
        "scalex" => Transform2D::scale(number(0)?, 1.0),
        "scaley" => Transform2D::scale(1.0, number(0)?),
        "rotate" => Transform2D::rotate_degrees(angle(0)?),
        "skew" => Transform2D::skew_degrees(angle(0)?, angle(1).unwrap_or(0.0)),
        "skewx" => Transform2D::skew_degrees(angle(0)?, 0.0),
        "skewy" => Transform2D::skew_degrees(0.0, angle(0)?),
        "matrix" => Transform2D::new(
            number(0)?,
            number(1)?,
            number(2)?,
            number(3)?,
            length(4)?,
            length(5)?,
        ),
        _ => return None,
    })
}

/// Parse a `transform-origin` declaration into the core's box fractions.
pub(super) fn parse_transform_origin(value: &str) -> Option<(f32, f32)> {
    let tokens = split_tokens(value.trim());
    if tokens.is_empty() || tokens.len() > 2 {
        return None;
    }
    let x = parse_fraction(&tokens[0], "left", "right")?;
    let y = match tokens.get(1) {
        Some(token) => parse_fraction(token, "top", "bottom")?,
        None => 0.5,
    };
    Some((x, y))
}

// ---------------------------------------------------------------------------------------------
// Text shadow, corners, outlines, and background images
// ---------------------------------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DeclaredTextShadow {
    offset_x: f32,
    offset_y: f32,
    #[serde(default)]
    blur: f32,
    #[serde(default)]
    color: Option<String>,
}

/// Parse a `"x y blur color"` text shadow, or the equivalent object form.
///
/// `current_color` stands in for an omitted color, matching the CSS `text-shadow` default.
pub(super) fn parse_text_shadow(value: &str, current_color: Color) -> Option<TextShadow> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > MAX_STYLE_DECLARATION_BYTES
        || value.eq_ignore_ascii_case("none")
    {
        return None;
    }
    if value.starts_with('{') {
        let declared = serde_json::from_str::<DeclaredTextShadow>(value).ok()?;
        let color = match declared.color.as_deref() {
            Some(color) => parse_css_color(color)?,
            None => current_color,
        };
        return Some(TextShadow::new(
            declared.offset_x,
            declared.offset_y,
            declared.blur,
            color,
        ));
    }
    let mut lengths = Vec::new();
    let mut color = current_color;
    for token in split_tokens(value) {
        match parse_length(&token) {
            Some(length) if lengths.len() < 3 => lengths.push(length),
            Some(_) => return None,
            None => color = parse_css_color(&token)?,
        }
    }
    if lengths.len() < 2 {
        return None;
    }
    Some(TextShadow::new(
        lengths[0],
        lengths[1],
        lengths.get(2).copied().unwrap_or(0.0),
        color,
    ))
}

/// Parse a one-to-four value CSS `border-radius` shorthand into per-corner radii.
pub(super) fn parse_corner_radii(value: &str) -> Option<Corners> {
    let tokens = split_tokens(value.trim());
    if tokens.is_empty() || tokens.len() > 4 {
        return None;
    }
    let radius = |index: usize| parse_length(&tokens[index]);
    Some(match tokens.len() {
        1 => Corners::all(radius(0)?),
        2 => Corners::new(radius(0)?, radius(1)?, radius(0)?, radius(1)?),
        3 => Corners::new(radius(0)?, radius(1)?, radius(2)?, radius(1)?),
        _ => Corners::new(radius(0)?, radius(1)?, radius(2)?, radius(3)?),
    })
}

pub(super) fn parse_border_style(value: &str) -> Option<BorderStyle> {
    Some(match value.trim() {
        "solid" => BorderStyle::Solid,
        "dashed" => BorderStyle::Dashed,
        "dotted" => BorderStyle::Dotted,
        _ => return None,
    })
}

/// Parse the CSS `outline` shorthand a state style declares, such as `2px solid #3b82f6`.
pub(super) fn parse_outline(value: &str, offset: f32) -> Option<Outline> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > MAX_STYLE_DECLARATION_BYTES
        || value.eq_ignore_ascii_case("none")
    {
        return None;
    }
    let mut width = None;
    let mut color = None;
    let mut style = BorderStyle::Solid;
    for token in split_tokens(value) {
        if let Some(declared) = parse_border_style(&token) {
            style = declared;
        } else if let Some(declared) = parse_length(&token) {
            width = Some(declared);
        } else {
            color = Some(parse_css_color(&token)?);
        }
    }
    Some(
        Outline::new(width?, color.unwrap_or(Color::BLACK))
            .offset(offset)
            .style(style),
    )
}

pub(super) fn parse_background_size(value: &str) -> Option<BackgroundSize> {
    let value = value.trim();
    match value {
        "auto" => return Some(BackgroundSize::Auto),
        "cover" => return Some(BackgroundSize::Cover),
        "contain" => return Some(BackgroundSize::Contain),
        _ => {}
    }
    let tokens = split_tokens(value);
    let width = parse_length(tokens.first()?)?;
    let height = match tokens.get(1) {
        Some(token) => parse_length(token)?,
        None => width,
    };
    (width > 0.0 && height > 0.0).then_some(BackgroundSize::Fixed(width, height))
}

pub(super) fn parse_background_repeat(value: &str) -> Option<BackgroundRepeat> {
    Some(match value.trim() {
        "no-repeat" => BackgroundRepeat::NoRepeat,
        "repeat" => BackgroundRepeat::Repeat,
        "repeat-x" => BackgroundRepeat::RepeatX,
        "repeat-y" => BackgroundRepeat::RepeatY,
        _ => return None,
    })
}

pub(super) fn parse_background_position(value: &str) -> Option<BackgroundPosition> {
    let tokens = split_tokens(value.trim());
    if tokens.is_empty() || tokens.len() > 2 {
        return None;
    }
    let x = parse_fraction(&tokens[0], "left", "right")?;
    let y = match tokens.get(1) {
        Some(token) => parse_fraction(token, "top", "bottom")?,
        None => 0.5,
    };
    Some(BackgroundPosition::new(x, y))
}

pub(super) fn parse_blend_mode(value: &str) -> Option<BlendMode> {
    Some(match value.trim() {
        "normal" => BlendMode::Normal,
        "multiply" => BlendMode::Multiply,
        "screen" => BlendMode::Screen,
        "darken" => BlendMode::Darken,
        "lighten" => BlendMode::Lighten,
        "overlay" => BlendMode::Overlay,
        "difference" => BlendMode::Difference,
        "exclusion" => BlendMode::Exclusion,
        "hard-light" => BlendMode::HardLight,
        "color-dodge" => BlendMode::ColorDodge,
        "color-burn" => BlendMode::ColorBurn,
        _ => return None,
    })
}

// ---------------------------------------------------------------------------------------------
// Declared property application
// ---------------------------------------------------------------------------------------------

/// Apply the declared extended text styling to a text-bearing element.
pub(super) fn apply_text_styles(mut element: Element, node: &NativeNode) -> Element {
    if let Some(value) = node.number(property::LETTER_SPACING) {
        element = element.letter_spacing(value);
    }
    if let Some(value) = node.number(property::WORD_SPACING) {
        element = element.word_spacing(value);
    }
    if let Some(value) = node.string(property::TEXT_TRANSFORM) {
        element = match value {
            "uppercase" => element.text_transform(TextTransform::Uppercase),
            "lowercase" => element.text_transform(TextTransform::Lowercase),
            "capitalize" => element.text_transform(TextTransform::Capitalize),
            "none" => element.text_transform_none(),
            _ => element,
        };
    }
    if let Some(value) = node.string(property::TEXT_SHADOW) {
        let current = node.color(property::COLOR).unwrap_or(Color::BLACK);
        element = match parse_text_shadow(value, current) {
            Some(shadow) => {
                element.text_shadow(shadow.offset_x, shadow.offset_y, shadow.blur, shadow.color)
            }
            None if value.trim().eq_ignore_ascii_case("none") => element.text_shadow_none(),
            None => element,
        };
    }
    element = apply_text_decoration(element, node);
    if let Some(value) = node.string(property::WORD_BREAK) {
        element = match value {
            "break-all" => element.word_break(WordBreak::BreakAll),
            "keep-all" => element.word_break(WordBreak::KeepAll),
            "normal" => element.word_break(WordBreak::Normal),
            _ => element,
        };
    }
    if let Some(value) = node.string(property::OVERFLOW_WRAP) {
        element = match value {
            "anywhere" => element.overflow_wrap(OverflowWrap::Anywhere),
            "break-word" => element.overflow_wrap(OverflowWrap::BreakWord),
            "normal" => element.overflow_wrap(OverflowWrap::Normal),
            _ => element,
        };
    }
    if let Some(value) = node.string(property::HYPHENS) {
        element = match value {
            // The core renders author-placed soft hyphens; it never hyphenates automatically, so
            // `auto` adopts the same manual behavior rather than promising dictionary hyphenation.
            "manual" | "auto" => element.hyphens(Hyphens::Manual),
            "none" => element.hyphens(Hyphens::None),
            _ => element,
        };
    }
    if let Some(value) = node.string(property::TEXT_DIRECTION) {
        element = match value {
            "ltr" => element.text_direction(TextDirection::Ltr),
            "rtl" => element.text_direction(TextDirection::Rtl),
            "auto" => element.text_direction(TextDirection::Auto),
            _ => element,
        };
    }
    element
}

fn apply_text_decoration(mut element: Element, node: &NativeNode) -> Element {
    let lines = node.string(property::TEXT_DECORATION_LINE);
    let has =
        |name: &str| lines.is_some_and(|lines| lines.split_whitespace().any(|token| token == name));
    let underline = has("underline");
    let overline = has("overline");
    if lines.is_some_and(|lines| lines.trim() == "none") {
        element = element.text_decoration_none();
    }
    if underline {
        element = element.underline();
    }
    if has("line-through") {
        element = element.line_through();
    }
    if overline {
        element = element.overline();
    }
    match node.string(property::TEXT_DECORATION_STYLE) {
        Some("double") => element = element.double_underline(),
        Some("wavy") => element = element.text_decoration_wavy(),
        Some("solid") => element = element.text_decoration_solid(),
        _ => {}
    }
    if let Some(color) = node.color(property::TEXT_DECORATION_COLOR) {
        // An overline carries its own color in the core, so a declared color reaches whichever
        // lines were declared instead of silently applying to only one of them.
        if overline {
            element = element.overline_color(color);
        }
        if underline || !overline {
            element = element.text_decoration_color(color);
        }
    }
    if let Some(thickness) = node.number(property::TEXT_DECORATION_THICKNESS) {
        // The core exposes a fixed set of native underline thicknesses; a declaration adopts the
        // closest one rather than inventing an unsupported stroke width.
        element = match thickness {
            value if !value.is_finite() => element,
            value if value <= 0.5 => element.text_decoration_0(),
            value if value <= 1.5 => element.text_decoration_1(),
            value if value <= 3.0 => element.text_decoration_2(),
            value if value <= 6.0 => element.text_decoration_4(),
            _ => element.text_decoration_8(),
        };
    }
    element
}

/// Apply the declared direction, logical spacing, sticky insets, and scroll snapping.
pub(super) fn apply_layout_styles(mut element: Element, node: &NativeNode) -> Element {
    if let Some(value) = node.string(property::DIRECTION) {
        element = match value {
            "rtl" => element.direction(Direction::Rtl),
            "ltr" => element.direction(Direction::Ltr),
            _ => element,
        };
    }
    if let Some(value) = node.number(property::PADDING_START) {
        element = element.ps(value);
    }
    if let Some(value) = node.number(property::PADDING_END) {
        element = element.pe(value);
    }
    if let Some(value) = node.number(property::MARGIN_START) {
        element = element.ms(value);
    }
    if let Some(value) = node.number(property::MARGIN_END) {
        element = element.me(value);
    }
    if let Some(value) = node.number(property::BORDER_START_WIDTH) {
        element = element.border_s(value);
    }
    if let Some(value) = node.number(property::BORDER_END_WIDTH) {
        element = element.border_e(value);
    }
    if let Some(value) = node.string(property::SCROLL_SNAP_TYPE) {
        element = apply_scroll_snap_type(element, value);
    }
    for (code, horizontal) in [
        (property::SCROLL_SNAP_X, true),
        (property::SCROLL_SNAP_Y, false),
    ] {
        let strictness = match node.string(code) {
            Some("mandatory") => SnapStrictness::Mandatory,
            Some("proximity") => SnapStrictness::Proximity,
            _ => continue,
        };
        element = if horizontal {
            element.scroll_snap_x(strictness)
        } else {
            element.scroll_snap_y(strictness)
        };
    }
    if let Some(value) = node.string(property::SCROLL_SNAP_ALIGN) {
        element = match value {
            "start" => element.snap_align(SnapAlign::Start),
            "center" => element.snap_align(SnapAlign::Center),
            "end" => element.snap_align(SnapAlign::End),
            _ => element,
        };
    }
    if node.string(property::SCROLL_SNAP_STOP) == Some("always") {
        element = element.snap_stop_always();
    }
    element
}

fn apply_scroll_snap_type(mut element: Element, value: &str) -> Element {
    let tokens: Vec<&str> = value.split_whitespace().collect();
    let strictness = match tokens.get(1) {
        Some(&"proximity") => SnapStrictness::Proximity,
        // CSS defaults an axis-only `scroll-snap-type` to proximity; the core has no default, so
        // the binding declares the same one.
        Some(&"mandatory") => SnapStrictness::Mandatory,
        Some(_) => return element,
        None => SnapStrictness::Proximity,
    };
    match tokens.first() {
        Some(&"x") | Some(&"inline") => element = element.scroll_snap_x(strictness),
        Some(&"y") | Some(&"block") => element = element.scroll_snap_y(strictness),
        Some(&"both") => {
            element = element.scroll_snap_x(strictness).scroll_snap_y(strictness);
        }
        _ => {}
    }
    element
}

/// Apply the declared gradient, per-corner radii, border and outline rings, filters, transform,
/// and blend mode.
pub(super) fn apply_box_styles(mut element: Element, node: &NativeNode) -> Element {
    if let Some(gradient) = node
        .string(property::BACKGROUND_GRADIENT)
        .and_then(parse_gradient)
    {
        element = element.bg_gradient(gradient);
    }
    if let Some(radii) = corner_radii(node) {
        element = element.corner_radii(radii);
    }
    if let Some(style) = node
        .string(property::BORDER_STYLE)
        .and_then(parse_border_style)
    {
        element = element.border_style(style);
    }
    element = apply_outline(element, node);
    element = apply_filters(element, node);
    if let Some(transform) = node.string(property::TRANSFORM).and_then(parse_transform) {
        element = element.transform(transform);
    }
    if let Some((x, y)) = node
        .string(property::TRANSFORM_ORIGIN)
        .and_then(parse_transform_origin)
    {
        element = element.transform_origin(x, y);
    }
    if let Some(blend) = node
        .string(property::MIX_BLEND_MODE)
        .and_then(parse_blend_mode)
    {
        element = element.blend_mode(blend);
    }
    element
}

/// Resolve the declared per-corner radii, falling back to the uniform `borderRadius`.
///
/// Returns `None` when nothing but the uniform numeric radius was declared, which the caller
/// already applies through the core's own `rounded`.
fn corner_radii(node: &NativeNode) -> Option<Corners> {
    let shorthand = node
        .string(property::BORDER_RADIUS)
        .and_then(parse_corner_radii);
    let uniform = node.number(property::BORDER_RADIUS).unwrap_or(0.0);
    let base = shorthand.unwrap_or(Corners::all(uniform));
    let corner = |code: u16, fallback: f32| node.number(code).unwrap_or(fallback);
    let radii = Corners::new(
        corner(property::BORDER_TOP_LEFT_RADIUS, base.top_left),
        corner(property::BORDER_TOP_RIGHT_RADIUS, base.top_right),
        corner(property::BORDER_BOTTOM_RIGHT_RADIUS, base.bottom_right),
        corner(property::BORDER_BOTTOM_LEFT_RADIUS, base.bottom_left),
    );
    (radii != Corners::all(uniform)).then_some(radii)
}

fn apply_outline(mut element: Element, node: &NativeNode) -> Element {
    let offset = node.number(property::OUTLINE_OFFSET);
    let width = node.number(property::OUTLINE_WIDTH);
    let color = node.color(property::OUTLINE_COLOR);
    let style = node
        .string(property::OUTLINE_STYLE)
        .and_then(parse_border_style);
    if node.string(property::OUTLINE_STYLE) == Some("none") {
        return element.outline_none();
    }
    if width.is_none() && color.is_none() && offset.is_none() && style.is_none() {
        return element;
    }
    element = element.outline(
        width.unwrap_or(0.0),
        color.unwrap_or(quickgui::Color::TRANSPARENT),
    );
    if let Some(offset) = offset {
        element = element.outline_offset(offset);
    }
    if let Some(style) = style {
        element = element.outline_style(style);
    }
    element
}

fn apply_filters(mut element: Element, node: &NativeNode) -> Element {
    if let Some(filters) = node.string(property::FILTER).and_then(parse_filters) {
        element = element.filters(filters);
    }
    if let Some(filters) = node
        .string(property::BACKDROP_FILTER)
        .and_then(parse_filters)
    {
        // `backdrop_blur` owns the blur slot of the backdrop chain, so the color part is declared
        // first and the blur radius, if any, is pushed onto it afterwards.
        let mut blur = 0.0_f32;
        let colors: Vec<Filter> = filters
            .into_iter()
            .filter(|filter| match filter {
                Filter::Blur(radius) => {
                    blur = blur.max(*radius);
                    false
                }
                // A subtree drop shadow is not a backdrop effect; the core's backdrop chain is
                // colour filters plus one blur.
                Filter::DropShadow(_) => false,
                _ => true,
            })
            .collect();
        element = element.backdrop_filter(colors);
        if blur > 0.0 {
            element = element.backdrop_blur(blur);
        }
    }
    element
}

// ---------------------------------------------------------------------------------------------
// Interaction-state styles
// ---------------------------------------------------------------------------------------------

/// Longest nested state-style declaration parsed from one `hover`-shaped property.
pub(super) const MAX_STATE_STYLE_JSON_BYTES: usize = 16 * 1024;

/// The property codes one interaction state is declared through.
pub(super) struct StateStyleCodes {
    /// The nested JSON declaration carrying everything the core's `ElementStateStyle` can swap.
    pub(super) style: u16,
    /// The flat declarations (`hoverBackgroundColor`, …) hover, active, and focus keep honouring.
    /// They overlay the nested declaration, so an application migrating one property at a time
    /// sees no change.
    pub(super) legacy: Option<LegacyStateStyleCodes>,
}

pub(super) struct LegacyStateStyleCodes {
    pub(super) background: u16,
    pub(super) color: u16,
    pub(super) gradient: u16,
    pub(super) outline: u16,
    pub(super) transform: u16,
}

pub(super) const HOVER_STYLE_CODES: StateStyleCodes = StateStyleCodes {
    style: property::HOVER_STYLE,
    legacy: Some(LegacyStateStyleCodes {
        background: property::HOVER_BACKGROUND_COLOR,
        color: property::HOVER_COLOR,
        gradient: property::HOVER_BACKGROUND_GRADIENT,
        outline: property::HOVER_OUTLINE,
        transform: property::HOVER_TRANSFORM,
    }),
};
pub(super) const ACTIVE_STYLE_CODES: StateStyleCodes = StateStyleCodes {
    style: property::ACTIVE_STYLE,
    legacy: Some(LegacyStateStyleCodes {
        background: property::ACTIVE_BACKGROUND_COLOR,
        color: property::ACTIVE_COLOR,
        gradient: property::ACTIVE_BACKGROUND_GRADIENT,
        outline: property::ACTIVE_OUTLINE,
        transform: property::ACTIVE_TRANSFORM,
    }),
};
pub(super) const FOCUS_STYLE_CODES: StateStyleCodes = StateStyleCodes {
    style: property::FOCUS_STYLE,
    legacy: Some(LegacyStateStyleCodes {
        background: property::FOCUS_BACKGROUND_COLOR,
        color: property::FOCUS_COLOR,
        gradient: property::FOCUS_BACKGROUND_GRADIENT,
        outline: property::FOCUS_OUTLINE,
        transform: property::FOCUS_TRANSFORM,
    }),
};
pub(super) const DISABLED_STYLE_CODES: StateStyleCodes = StateStyleCodes {
    style: property::DISABLED_STYLE,
    legacy: None,
};
pub(super) const INVALID_STYLE_CODES: StateStyleCodes = StateStyleCodes {
    style: property::INVALID_STYLE,
    legacy: None,
};
pub(super) const DRAGGING_STYLE_CODES: StateStyleCodes = StateStyleCodes {
    style: property::DRAGGING_STYLE,
    legacy: None,
};
pub(super) const DRAG_OVER_STYLE_CODES: StateStyleCodes = StateStyleCodes {
    style: property::DRAG_OVER_STYLE,
    legacy: None,
};
pub(super) const FOCUS_WITHIN_STYLE_CODES: StateStyleCodes = StateStyleCodes {
    style: property::FOCUS_WITHIN_STYLE,
    legacy: None,
};
pub(super) const SELECTED_STYLE_CODES: StateStyleCodes = StateStyleCodes {
    style: property::SELECTED_STYLE,
    legacy: None,
};

/// One nested state-style declaration exactly as the renderer encodes it: colors packed the way
/// every color property travels, lengths in logical pixels, and every string in the grammar the
/// matching base property already uses. Unknown keys are ignored so an older binary tolerates a
/// newer renderer.
#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct StateStyleDeclaration {
    background_color: Option<u32>,
    background: Option<String>,
    color: Option<u32>,
    border_color: Option<u32>,
    border_width: Option<f32>,
    border_radius: Option<f32>,
    outline: Option<String>,
    box_shadow: Option<Vec<NativeBoxShadow>>,
    opacity: Option<f32>,
    cursor: Option<String>,
    transform: Option<String>,
    transform_origin: Option<String>,
    /// The named group a `groupHover` or `groupActive` entry follows.
    group: Option<String>,
}

/// A group state's declaration: one entry, or a list of entries each following its own group.
#[derive(Deserialize)]
#[serde(untagged)]
enum GroupStyleDeclarations {
    One(StateStyleDeclaration),
    Many(Vec<StateStyleDeclaration>),
}

/// The group a node declares: `Some(None)` for an unnamed group, `Some(Some(name))` for a named
/// one, and `None` when it declares no group or a name the core would refuse.
pub(super) fn native_group(node: &NativeNode) -> Option<Option<Arc<str>>> {
    match node.property(property::HOVER_GROUP)? {
        PropertyValue::Bool(true) => Some(None),
        PropertyValue::String(name)
            if !name.is_empty() && name.len() <= MAX_HOVER_GROUP_NAME_BYTES =>
        {
            Some(Some(name.clone()))
        }
        _ => None,
    }
}

/// The entries of one group state — `groupHover` or `groupActive` — in declaration order, each
/// with the named group it follows, or `None` for the nearest one.
///
/// Entries past the core's `MAX_GROUP_STYLES_PER_ELEMENT` are dropped in source order, an entry
/// naming a group the core would refuse is dropped, and an entry declaring nothing is not an entry.
pub(super) fn native_group_styles(
    node: &NativeNode,
    code: u16,
) -> Vec<(Option<Arc<str>>, ElementStateStyle)> {
    let outline_offset = node.number(property::OUTLINE_OFFSET).unwrap_or(0.0);
    let Some(declarations) = node
        .string(code)
        .filter(|value| value.len() <= MAX_STATE_STYLE_JSON_BYTES)
        .and_then(|value| serde_json::from_str::<GroupStyleDeclarations>(value).ok())
    else {
        return Vec::new();
    };
    let declarations = match declarations {
        GroupStyleDeclarations::One(declaration) => vec![declaration],
        GroupStyleDeclarations::Many(declarations) => declarations,
    };
    declarations
        .into_iter()
        .take(MAX_GROUP_STYLES_PER_ELEMENT)
        .filter_map(|declaration| {
            let target = match declaration.group.as_deref() {
                None => None,
                Some(name) if !name.is_empty() && name.len() <= MAX_HOVER_GROUP_NAME_BYTES => {
                    Some(Arc::<str>::from(name))
                }
                Some(_) => return None,
            };
            let style = apply_state_style_declaration(
                ElementStateStyle::default(),
                declaration,
                node,
                outline_offset,
            );
            (style != ElementStateStyle::default()).then_some((target, style))
        })
        .collect()
}

/// Collect one state's declared style, or `None` when the state declares nothing at all, so an
/// empty state never registers a core state style.
pub(super) fn native_state_style(
    node: &NativeNode,
    codes: &StateStyleCodes,
) -> Option<ElementStateStyle> {
    let outline_offset = node.number(property::OUTLINE_OFFSET).unwrap_or(0.0);
    let mut style = ElementStateStyle::default();
    if let Some(declaration) = node
        .string(codes.style)
        .filter(|value| value.len() <= MAX_STATE_STYLE_JSON_BYTES)
        .and_then(|value| serde_json::from_str::<StateStyleDeclaration>(value).ok())
    {
        style = apply_state_style_declaration(style, declaration, node, outline_offset);
    }
    if let Some(legacy) = &codes.legacy {
        if let Some(color) = node.color(legacy.background) {
            style = style.bg(color);
        }
        if let Some(gradient) = node.string(legacy.gradient).and_then(parse_gradient) {
            style = style.bg_gradient(gradient);
        }
        if let Some(color) = node.color(legacy.color) {
            style = style.text_color(color);
        }
        if let Some(outline) = node
            .string(legacy.outline)
            .and_then(|value| parse_outline(value, outline_offset))
        {
            style = state_outline(style, outline);
        }
        if let Some(transform) = node.string(legacy.transform).and_then(parse_transform) {
            style = style.transform(transform);
        }
    }
    (style != ElementStateStyle::default()).then_some(style)
}

fn apply_state_style_declaration(
    mut style: ElementStateStyle,
    declaration: StateStyleDeclaration,
    node: &NativeNode,
    outline_offset: f32,
) -> ElementStateStyle {
    if let Some(color) = declaration.background_color {
        style = style.bg(unpack_color(color));
    }
    if let Some(gradient) = declaration.background.as_deref().and_then(parse_gradient) {
        style = style.bg_gradient(gradient);
    }
    if let Some(color) = declaration.color {
        style = style.text_color(unpack_color(color));
    }
    if let Some(color) = declaration.border_color {
        style = style.border_color(unpack_color(color));
    }
    if let Some(width) = declaration.border_width.filter(|width| width.is_finite()) {
        style = style.border_width(width);
    }
    if let Some(radius) = declaration
        .border_radius
        .filter(|radius| radius.is_finite())
    {
        style = style.rounded(radius);
    }
    if let Some(outline) = declaration.outline.as_deref() {
        if outline.trim().eq_ignore_ascii_case("none") {
            style = style.outline_none();
        } else if let Some(outline) = parse_outline(outline, outline_offset) {
            style = state_outline(style, outline);
        }
    }
    if let Some(shadows) = declaration.box_shadow {
        if shadows.is_empty() {
            style = style.shadow_none();
        } else if let Some(shadows) = box_shadows_from_declarations(
            shadows,
            node.color(property::COLOR).unwrap_or(Color::BLACK),
        ) {
            style = style.shadows(shadows);
        }
    }
    if let Some(opacity) = declaration.opacity.filter(|opacity| opacity.is_finite()) {
        style = style.opacity(opacity);
    }
    if let Some(name) = declaration.cursor.as_deref() {
        style = style.cursor(cursor(name));
    }
    if let Some(transform) = declaration.transform.as_deref().and_then(parse_transform) {
        style = style.transform(transform);
    }
    if let Some((x, y)) = declaration
        .transform_origin
        .as_deref()
        .and_then(parse_transform_origin)
    {
        style = style.transform_origin(x, y);
    }
    style
}

/// Install one parsed outline ring, keeping the dash style the shorthand declared.
fn state_outline(style: ElementStateStyle, outline: Outline) -> ElementStateStyle {
    let style = style.outline_offset(outline.width, outline.color, outline.offset);
    match outline.style {
        BorderStyle::Dashed => style.outline_dashed(),
        BorderStyle::Dotted => style.outline_dotted(),
        _ => style,
    }
}

// ---------------------------------------------------------------------------------------------
// Box shadows
// ---------------------------------------------------------------------------------------------

/// One CSS `box-shadow` entry exactly as the renderer encodes it.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct NativeBoxShadow {
    offset_x: f32,
    offset_y: f32,
    blur_radius: f32,
    spread_radius: f32,
    color: Option<u32>,
    #[serde(default)]
    inset: bool,
    #[serde(default)]
    order_by_subject: bool,
}

/// The element's declared shadow list, or `None` when it declares none or too many.
pub(super) fn native_box_shadows(node: &NativeNode) -> Option<Vec<BoxShadow>> {
    let encoded = node.string(property::BOX_SHADOW)?;
    let shadows = serde_json::from_str::<Vec<NativeBoxShadow>>(encoded).ok()?;
    box_shadows_from_declarations(shadows, node.color(property::COLOR).unwrap_or(Color::BLACK))
}

/// Core shadows for declared entries; an omitted color adopts the element's current text color.
fn box_shadows_from_declarations(
    shadows: Vec<NativeBoxShadow>,
    current_color: Color,
) -> Option<Vec<BoxShadow>> {
    if shadows.len() > MAX_BOX_SHADOWS_PER_ELEMENT {
        return None;
    }
    Some(
        shadows
            .into_iter()
            .map(|shadow| {
                BoxShadow::new(
                    shadow.offset_x,
                    shadow.offset_y,
                    shadow.color.map(unpack_color).unwrap_or(current_color),
                )
                .blur_radius(shadow.blur_radius)
                .spread_radius(shadow.spread_radius)
                .inset(shadow.inset)
                .order_by_subject(shadow.order_by_subject)
            })
            .collect(),
    )
}

// ---------------------------------------------------------------------------------------------
// Background images
// ---------------------------------------------------------------------------------------------

/// One decoded raster background retained until its declaration changes.
///
/// A background image must be a decoded [`quickgui::Image`] rather than the lazy `ImageResource`
/// an `<Image>` node uses, so the binding decodes it exactly once per declaration and keeps it for
/// as long as the node declares the same source.
pub(super) struct NativeBackgroundImageState {
    source: Arc<str>,
    image: Option<quickgui::Image>,
}

impl NativeBackgroundImageState {
    pub(super) fn new(source: Arc<str>) -> Self {
        let image = decode_background_image(source.as_ref());
        Self { source, image }
    }

    pub(super) fn sync(&mut self, source: &str) {
        if self.source.as_ref() == source {
            return;
        }
        *self = Self::new(Arc::from(source));
    }
}

fn decode_background_image(source: &str) -> Option<quickgui::Image> {
    if source.is_empty() {
        return None;
    }
    if source.starts_with("data:") || source.starts_with("DATA:") {
        return quickgui::Image::from_data_url(source).ok();
    }
    let path = source.strip_prefix("file://").unwrap_or(source);
    quickgui::Image::open(path).ok()
}

/// Apply the declared raster background, decoding its source at most once per declaration.
pub(super) fn apply_background_image(
    element: Element,
    id: u32,
    node: &NativeNode,
    images: &mut HashMap<u32, NativeBackgroundImageState>,
) -> Element {
    let Some(source) = node.string(property::BACKGROUND_IMAGE) else {
        images.remove(&id);
        return element;
    };
    let state = images
        .entry(id)
        .or_insert_with(|| NativeBackgroundImageState::new(Arc::from(source)));
    state.sync(source);
    let Some(image) = state.image.clone() else {
        return element;
    };
    element.bg_image(
        image,
        node.string(property::BACKGROUND_SIZE)
            .and_then(parse_background_size)
            .unwrap_or_default(),
        node.string(property::BACKGROUND_REPEAT)
            .and_then(parse_background_repeat)
            .unwrap_or_default(),
        node.string(property::BACKGROUND_POSITION)
            .and_then(parse_background_position)
            .unwrap_or_default(),
    )
}
