use std::{
    collections::HashMap,
    fmt, mem,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use lyon::{
    algorithms::measure::{PathMeasurements, SampleType},
    geom::Angle,
    math::{Transform, point, vector},
    path::{ArcFlags, Path as LyonPath, Polygon, traits::SvgPathBuilder},
    tessellation::{
        FillGeometryBuilder, FillOptions as LyonFillOptions, FillTessellator, FillVertex,
        GeometryBuilder, GeometryBuilderError, StrokeGeometryBuilder,
        StrokeOptions as LyonStrokeOptions, StrokeTessellator, StrokeVertex, TessellationError,
        VertexId,
    },
};
use thiserror::Error;

use crate::{Color, Point, Rect, Size};

/// Maximum path-building commands accepted before tessellation.
pub const MAX_PATH_COMMANDS: usize = 65_536;
/// Maximum de-indexed vertices retained by one path.
pub const MAX_PATH_VERTICES: usize = 196_605;
/// Maximum retained triangle-position and boundary metadata owned by one path.
pub const MAX_PATH_BYTES: usize = 4 * 1024 * 1024;
/// Maximum dash segments generated from one stroked path.
pub const MAX_PATH_DASH_SEGMENTS: usize = 262_144;
/// Largest absolute source coordinate accepted by the builder.
pub const MAX_PATH_COORDINATE: f32 = 1_000_000.0;

const MAX_STROKE_WIDTH: f32 = 100_000.0;
const MAX_MITER_LIMIT: f32 = 1_000.0;
const MIN_TOLERANCE: f32 = 0.01;
const MAX_TOLERANCE: f32 = 10.0;
const MAX_DASH_VALUES: usize = 256;
const MAX_TRANSFORM_SCALE: f32 = 1_024.0;

static NEXT_PATH_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FillRule {
    NonZero,
    EvenOdd,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LineCap {
    Butt,
    Square,
    Round,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LineJoin {
    Miter,
    MiterClip,
    Round,
    Bevel,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FillOptions {
    pub fill_rule: FillRule,
    pub tolerance: f32,
}

impl FillOptions {
    pub const fn new() -> Self {
        Self {
            fill_rule: FillRule::NonZero,
            tolerance: 0.1,
        }
    }

    pub fn with_fill_rule(mut self, fill_rule: FillRule) -> Self {
        self.fill_rule = fill_rule;
        self
    }

    pub fn with_tolerance(mut self, tolerance: f32) -> Self {
        self.tolerance = tolerance;
        self
    }
}

impl Default for FillOptions {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrokeOptions {
    pub line_width: f32,
    pub line_cap: LineCap,
    pub line_join: LineJoin,
    pub miter_limit: f32,
    pub tolerance: f32,
}

impl StrokeOptions {
    pub const fn new() -> Self {
        Self {
            line_width: 1.0,
            line_cap: LineCap::Butt,
            line_join: LineJoin::Miter,
            miter_limit: 4.0,
            tolerance: 0.1,
        }
    }

    pub fn with_line_width(mut self, width: f32) -> Self {
        self.line_width = width;
        self
    }

    pub fn with_line_cap(mut self, cap: LineCap) -> Self {
        self.line_cap = cap;
        self
    }

    pub fn with_line_join(mut self, join: LineJoin) -> Self {
        self.line_join = join;
        self
    }

    pub fn with_miter_limit(mut self, limit: f32) -> Self {
        self.miter_limit = limit;
        self
    }

    pub fn with_tolerance(mut self, tolerance: f32) -> Self {
        self.tolerance = tolerance;
        self
    }
}

impl Default for StrokeOptions {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PathStyle {
    Fill(FillOptions),
    Stroke(StrokeOptions),
}

/// Color interpolation used by a two-stop linear gradient.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum GradientColorSpace {
    /// Interpolate the framework's linear-light RGB values directly.
    #[default]
    LinearSrgb,
    /// Interpolate encoded sRGB values, matching traditional CSS gradients.
    Srgb,
    /// Interpolate perceptually in Oklab.
    Oklab,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LinearColorStop {
    pub color: Color,
    pub position: f32,
}

impl LinearColorStop {
    pub fn new(color: Color, position: f32) -> Self {
        Self {
            color: sanitize_color(color),
            position: finite_or(position, 0.0).clamp(0.0, 1.0),
        }
    }
}

pub fn linear_color_stop(color: Color, position: f32) -> LinearColorStop {
    LinearColorStop::new(color, position)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LinearGradient {
    angle_degrees: f32,
    stops: [LinearColorStop; 2],
    color_space: GradientColorSpace,
}

impl LinearGradient {
    pub fn new(angle_degrees: f32, mut start: LinearColorStop, mut end: LinearColorStop) -> Self {
        if start.position > end.position {
            mem::swap(&mut start, &mut end);
        }
        Self {
            angle_degrees: finite_or(angle_degrees, 180.0).rem_euclid(360.0),
            stops: [start, end],
            color_space: GradientColorSpace::LinearSrgb,
        }
    }

    pub fn color_space(mut self, color_space: GradientColorSpace) -> Self {
        self.color_space = color_space;
        self
    }

    pub const fn angle_degrees(self) -> f32 {
        self.angle_degrees
    }

    pub const fn stops(self) -> [LinearColorStop; 2] {
        self.stops
    }

    pub const fn interpolation(self) -> GradientColorSpace {
        self.color_space
    }
}

pub fn linear_gradient(
    angle_degrees: f32,
    start: LinearColorStop,
    end: LinearColorStop,
) -> Background {
    Background::LinearGradient(LinearGradient::new(angle_degrees, start, end))
}

/// Largest number of color stops retained by one [`Gradient`].
///
/// Additional declared stops are dropped in source order rather than allocating: one gradient is
/// a fixed-size `Copy` value so it never introduces a per-frame heap allocation.
pub const MAX_GRADIENT_STOPS: usize = 8;

/// A bounded, ordered list of gradient color stops.
///
/// Stops are sanitized, clamped to `0.0..=1.0`, sorted by position, and truncated to
/// [`MAX_GRADIENT_STOPS`]. Stops without an explicit position are distributed evenly, matching
/// CSS. An empty list is treated as fully transparent.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorStops {
    length: u8,
    stops: [LinearColorStop; MAX_GRADIENT_STOPS],
}

impl Default for ColorStops {
    fn default() -> Self {
        Self::empty()
    }
}

impl ColorStops {
    const EMPTY_STOP: LinearColorStop = LinearColorStop {
        color: Color::TRANSPARENT,
        position: 0.0,
    };

    /// A gradient with no stops. It paints nothing.
    pub const fn empty() -> Self {
        Self {
            length: 0,
            stops: [Self::EMPTY_STOP; MAX_GRADIENT_STOPS],
        }
    }

    /// Collect at most [`MAX_GRADIENT_STOPS`] positioned stops in source order.
    pub fn new(stops: impl IntoIterator<Item = LinearColorStop>) -> Self {
        let mut collected = Self::empty();
        for stop in stops {
            if usize::from(collected.length) == MAX_GRADIENT_STOPS {
                break;
            }
            collected.stops[usize::from(collected.length)] =
                LinearColorStop::new(stop.color, stop.position);
            collected.length += 1;
        }
        collected.sort();
        collected
    }

    /// Collect at most [`MAX_GRADIENT_STOPS`] colors and distribute their positions evenly.
    pub fn evenly_spaced(colors: impl IntoIterator<Item = Color>) -> Self {
        let mut collected = Self::empty();
        for color in colors {
            if usize::from(collected.length) == MAX_GRADIENT_STOPS {
                break;
            }
            collected.stops[usize::from(collected.length)] = LinearColorStop::new(color, 0.0);
            collected.length += 1;
        }
        let last = collected.length.saturating_sub(1);
        for index in 0..usize::from(collected.length) {
            collected.stops[index].position = if last == 0 {
                0.0
            } else {
                index as f32 / f32::from(last)
            };
        }
        collected
    }

    /// The retained stops in ascending position order.
    pub fn as_slice(&self) -> &[LinearColorStop] {
        &self.stops[..usize::from(self.length)]
    }

    pub fn len(&self) -> usize {
        usize::from(self.length)
    }

    pub fn is_empty(&self) -> bool {
        self.length == 0
    }

    fn sort(&mut self) {
        // Insertion sort over at most eight elements keeps stop ordering stable and allocation
        // free; equal positions preserve declaration order like CSS.
        let length = usize::from(self.length);
        for index in 1..length {
            let mut position = index;
            while position > 0 && self.stops[position - 1].position > self.stops[position].position
            {
                self.stops.swap(position - 1, position);
                position -= 1;
            }
        }
    }

    fn multiply_alpha(mut self, opacity: f32) -> Self {
        for index in 0..usize::from(self.length) {
            self.stops[index].color = self.stops[index].color.multiply_alpha(opacity);
        }
        self
    }

    fn is_visible(&self) -> bool {
        self.as_slice().iter().any(|stop| stop.color.a > 0.0)
    }
}

impl FromIterator<LinearColorStop> for ColorStops {
    fn from_iter<T: IntoIterator<Item = LinearColorStop>>(stops: T) -> Self {
        Self::new(stops)
    }
}

impl FromIterator<Color> for ColorStops {
    fn from_iter<T: IntoIterator<Item = Color>>(colors: T) -> Self {
        Self::evenly_spaced(colors)
    }
}

/// A CSS-like named direction for a linear gradient.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GradientDirection {
    ToTop,
    ToTopRight,
    ToRight,
    ToBottomRight,
    ToBottom,
    ToBottomLeft,
    ToLeft,
    ToTopLeft,
}

/// The angle of a linear or conic gradient, in CSS degrees.
///
/// `0` points to the top of the element and angles increase clockwise. Both `f32` degrees and a
/// [`GradientDirection`] convert into this type.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GradientAngle(f32);

impl GradientAngle {
    pub fn new(degrees: f32) -> Self {
        Self(finite_or(degrees, 0.0).rem_euclid(360.0))
    }

    pub const fn degrees(self) -> f32 {
        self.0
    }
}

impl From<f32> for GradientAngle {
    fn from(degrees: f32) -> Self {
        Self::new(degrees)
    }
}

impl From<GradientDirection> for GradientAngle {
    fn from(direction: GradientDirection) -> Self {
        Self(match direction {
            GradientDirection::ToTop => 0.0,
            GradientDirection::ToTopRight => 45.0,
            GradientDirection::ToRight => 90.0,
            GradientDirection::ToBottomRight => 135.0,
            GradientDirection::ToBottom => 180.0,
            GradientDirection::ToBottomLeft => 225.0,
            GradientDirection::ToLeft => 270.0,
            GradientDirection::ToTopLeft => 315.0,
        })
    }
}

/// The outline shape of a radial gradient.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum RadialGradientShape {
    /// One radius in both axes.
    Circle,
    /// Independent horizontal and vertical radii, matching the element's box.
    #[default]
    Ellipse,
}

/// Where a radial gradient's ending shape is sized to.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum RadialGradientExtent {
    /// The ending shape passes through the box corner furthest from the center.
    #[default]
    FarthestCorner,
    /// The ending shape touches the box side furthest from the center.
    FarthestSide,
    /// The ending shape touches the box side closest to the center.
    ClosestSide,
}

/// A gradient center expressed as a fraction of the painted box.
///
/// `(0.0, 0.0)` is the top-left corner and `(1.0, 1.0)` the bottom-right corner. Values outside
/// that range are accepted and clamped to `-4.0..=5.0` so an off-box center cannot produce a
/// degenerate ending shape.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GradientCenter {
    pub x: f32,
    pub y: f32,
}

impl Default for GradientCenter {
    fn default() -> Self {
        Self::CENTER
    }
}

impl GradientCenter {
    /// The center of the painted box.
    pub const CENTER: Self = Self { x: 0.5, y: 0.5 };

    pub fn new(x: f32, y: f32) -> Self {
        Self {
            x: finite_or(x, 0.5).clamp(-4.0, 5.0),
            y: finite_or(y, 0.5).clamp(-4.0, 5.0),
        }
    }
}

/// The geometry of a [`Gradient`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GradientKind {
    /// Interpolates along a line through the box center at `angle`.
    Linear { angle: GradientAngle },
    /// Interpolates outward from `center` to an ending `shape` sized by `extent`.
    Radial {
        shape: RadialGradientShape,
        extent: RadialGradientExtent,
        center: GradientCenter,
    },
    /// Interpolates by angle around `center`, starting at `from_angle`.
    Conic {
        from_angle: GradientAngle,
        center: GradientCenter,
    },
}

/// A bounded multi-stop linear, radial, or conic gradient.
///
/// A gradient is a fixed-size `Copy` value with at most [`MAX_GRADIENT_STOPS`] stops. It is
/// evaluated analytically on the GPU, so it allocates no texture, ramp cache, or extra draw call
/// and participates in the same ordered instanced draw as solid quads and paths.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gradient {
    kind: GradientKind,
    stops: ColorStops,
    color_space: GradientColorSpace,
    dither: bool,
    box_projection: bool,
}

impl Gradient {
    /// Project the angle through the box aspect ratio, matching native GPUI gradients.
    pub fn box_projection(mut self, enabled: bool) -> Self {
        self.box_projection = enabled;
        self
    }

    /// Add deterministic physical-pixel noise to reduce visible gradient banding.
    pub fn dither(mut self, enabled: bool) -> Self {
        self.dither = enabled;
        self
    }
    /// A linear gradient at `angle`, where `0` degrees points to the top and increases clockwise.
    pub fn linear(angle: impl Into<GradientAngle>, stops: impl Into<ColorStops>) -> Self {
        Self {
            kind: GradientKind::Linear {
                angle: angle.into(),
            },
            stops: stops.into(),
            color_space: GradientColorSpace::LinearSrgb,
            dither: false,
            box_projection: false,
        }
    }

    /// A centered elliptical radial gradient sized to the farthest corner.
    pub fn radial(stops: impl Into<ColorStops>) -> Self {
        Self {
            kind: GradientKind::Radial {
                shape: RadialGradientShape::Ellipse,
                extent: RadialGradientExtent::FarthestCorner,
                center: GradientCenter::CENTER,
            },
            stops: stops.into(),
            color_space: GradientColorSpace::LinearSrgb,
            dither: false,
            box_projection: false,
        }
    }

    /// A conic gradient starting at `from_angle` and sweeping clockwise around the center.
    pub fn conic(from_angle: impl Into<GradientAngle>, stops: impl Into<ColorStops>) -> Self {
        Self {
            kind: GradientKind::Conic {
                from_angle: from_angle.into(),
                center: GradientCenter::CENTER,
            },
            stops: stops.into(),
            color_space: GradientColorSpace::LinearSrgb,
            dither: false,
            box_projection: false,
        }
    }

    /// Override the ending shape of a radial gradient. Other kinds are unchanged.
    pub fn shape(mut self, shape: RadialGradientShape) -> Self {
        if let GradientKind::Radial {
            shape: current_shape,
            ..
        } = &mut self.kind
        {
            *current_shape = shape;
        }
        self
    }

    /// Override how a radial gradient's ending shape is sized. Other kinds are unchanged.
    pub fn extent(mut self, extent: RadialGradientExtent) -> Self {
        if let GradientKind::Radial {
            extent: current_extent,
            ..
        } = &mut self.kind
        {
            *current_extent = extent;
        }
        self
    }

    /// Override the center of a radial or conic gradient. Linear gradients are unchanged.
    pub fn center(mut self, center: GradientCenter) -> Self {
        match &mut self.kind {
            GradientKind::Radial {
                center: current, ..
            }
            | GradientKind::Conic {
                center: current, ..
            } => *current = center,
            GradientKind::Linear { .. } => {}
        }
        self
    }

    /// Select the space colors are interpolated in.
    pub fn color_space(mut self, color_space: GradientColorSpace) -> Self {
        self.color_space = color_space;
        self
    }

    pub const fn kind(self) -> GradientKind {
        self.kind
    }

    pub const fn stops(&self) -> &ColorStops {
        &self.stops
    }

    pub const fn interpolation(self) -> GradientColorSpace {
        self.color_space
    }

    pub(crate) fn multiply_alpha(mut self, opacity: f32) -> Self {
        self.stops = self.stops.multiply_alpha(opacity);
        self
    }

    pub(crate) fn is_visible(&self) -> bool {
        self.stops.is_visible()
    }
}

impl From<LinearGradient> for Gradient {
    fn from(gradient: LinearGradient) -> Self {
        let stops = gradient.stops();
        Self {
            kind: GradientKind::Linear {
                angle: GradientAngle::new(gradient.angle_degrees()),
            },
            stops: ColorStops::new(stops),
            color_space: gradient.interpolation(),
            dither: false,
            box_projection: false,
        }
    }
}

impl From<Color> for ColorStops {
    fn from(color: Color) -> Self {
        Self::evenly_spaced([color])
    }
}

impl<const N: usize> From<[LinearColorStop; N]> for ColorStops {
    fn from(stops: [LinearColorStop; N]) -> Self {
        Self::new(stops)
    }
}

impl<const N: usize> From<[Color; N]> for ColorStops {
    fn from(colors: [Color; N]) -> Self {
        Self::evenly_spaced(colors)
    }
}

impl From<&[LinearColorStop]> for ColorStops {
    fn from(stops: &[LinearColorStop]) -> Self {
        Self::new(stops.iter().copied())
    }
}

impl From<&[Color]> for ColorStops {
    fn from(colors: &[Color]) -> Self {
        Self::evenly_spaced(colors.iter().copied())
    }
}

impl From<Vec<LinearColorStop>> for ColorStops {
    fn from(stops: Vec<LinearColorStop>) -> Self {
        Self::new(stops)
    }
}

impl From<Vec<Color>> for ColorStops {
    fn from(colors: Vec<Color>) -> Self {
        Self::evenly_spaced(colors)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Background {
    Solid(Color),
    LinearGradient(LinearGradient),
    /// A bounded multi-stop linear, radial, or conic gradient.
    Gradient(Gradient),
}

impl Background {
    pub(crate) fn is_visible(self) -> bool {
        match self {
            Self::Solid(color) => color.a > 0.0,
            Self::LinearGradient(gradient) => gradient.stops.iter().any(|stop| stop.color.a > 0.0),
            Self::Gradient(gradient) => gradient.stops.is_visible(),
        }
    }

    pub(crate) fn multiply_alpha(self, opacity: f32) -> Self {
        match self {
            Self::Solid(color) => Self::Solid(color.multiply_alpha(opacity)),
            Self::LinearGradient(mut gradient) => {
                for stop in &mut gradient.stops {
                    stop.color = stop.color.multiply_alpha(opacity);
                }
                Self::LinearGradient(gradient)
            }
            Self::Gradient(gradient) => Self::Gradient(gradient.multiply_alpha(opacity)),
        }
    }

    /// The multi-stop representation used by every gradient-capable renderer.
    pub(crate) fn as_gradient(self) -> Option<Gradient> {
        match self {
            Self::Solid(_) => None,
            Self::LinearGradient(gradient) => Some(Gradient::from(gradient)),
            Self::Gradient(gradient) => Some(gradient),
        }
    }
}

impl From<Color> for Background {
    fn from(color: Color) -> Self {
        Self::Solid(sanitize_color(color))
    }
}

impl From<LinearGradient> for Background {
    fn from(gradient: LinearGradient) -> Self {
        Self::LinearGradient(gradient)
    }
}

impl From<Gradient> for Background {
    fn from(gradient: Gradient) -> Self {
        Self::Gradient(gradient)
    }
}

/// GPU-ready packing of one gradient, resolved against the logical box it paints.
///
/// Every renderer that evaluates gradients uploads this identical fixed-size record, so linear,
/// radial, and conic interpolation is defined once on the CPU and once per shader family.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct GradientData {
    /// Kind, interpolation space, stop count, dithering flag.
    pub(crate) header: [f32; 4],
    /// Linear: start XY and end XY. Radial: center XY and radii XY. Conic: center XY, start angle.
    pub(crate) geometry: [f32; 4],
    pub(crate) projection: [f32; 4],
    /// Stop positions 0..4 followed by 4..8.
    pub(crate) positions: [[f32; 4]; 2],
    pub(crate) colors: [[f32; 4]; MAX_GRADIENT_STOPS],
}

pub(crate) const GRADIENT_KIND_LINEAR: f32 = 0.0;
pub(crate) const GRADIENT_KIND_RADIAL: f32 = 1.0;
pub(crate) const GRADIENT_KIND_CONIC: f32 = 2.0;

/// The logical endpoints of a CSS linear-gradient line spanning `bounds` at `angle_degrees`.
pub(crate) fn gradient_line(bounds: Rect, angle_degrees: f32) -> [f32; 4] {
    let radians = angle_degrees.to_radians();
    let direction = [radians.sin(), -radians.cos()];
    let center = [
        bounds.x + bounds.width * 0.5,
        bounds.y + bounds.height * 0.5,
    ];
    let half_length =
        (direction[0].abs() * bounds.width + direction[1].abs() * bounds.height) * 0.5;
    [
        center[0] - direction[0] * half_length,
        center[1] - direction[1] * half_length,
        center[0] + direction[0] * half_length,
        center[1] + direction[1] * half_length,
    ]
}

fn box_gradient_line(bounds: Rect, angle: f32) -> [f32; 4] {
    if bounds.width <= 0. || bounds.height <= 0. {
        return gradient_line(bounds, angle);
    }
    let radians = (angle.rem_euclid(360.) - 90.) * (std::f32::consts::PI / 180.);
    let mut direction = [radians.cos(), radians.sin()];
    if bounds.width > bounds.height {
        direction[1] *= bounds.height / bounds.width;
    } else {
        direction[0] *= bounds.width / bounds.height;
    }
    let length = direction[0].hypot(direction[1]);
    let extent = if direction[0].abs() > direction[1].abs() {
        bounds.width
    } else {
        bounds.height
    };
    let delta = [
        direction[0] / length * extent * 0.5,
        direction[1] / length * extent * 0.5,
    ];
    let center = [
        bounds.x + bounds.width * 0.5,
        bounds.y + bounds.height * 0.5,
    ];
    [
        center[0] - delta[0],
        center[1] - delta[1],
        center[0] + delta[0],
        center[1] + delta[1],
    ]
}

pub(crate) fn interpolation_code(color_space: GradientColorSpace) -> f32 {
    match color_space {
        GradientColorSpace::LinearSrgb => 0.0,
        GradientColorSpace::Srgb => 1.0,
        GradientColorSpace::Oklab => 2.0,
    }
}

impl GradientData {
    /// Resolve `gradient` against the logical `bounds` it paints.
    pub(crate) fn new(gradient: &Gradient, bounds: Rect) -> Self {
        let (kind, geometry) = match gradient.kind() {
            GradientKind::Linear { angle } => (
                GRADIENT_KIND_LINEAR,
                if gradient.box_projection {
                    box_gradient_line(bounds, angle.degrees())
                } else {
                    gradient_line(bounds, angle.degrees())
                },
            ),
            GradientKind::Radial {
                shape,
                extent,
                center,
            } => {
                let origin = [
                    bounds.x + bounds.width * center.x,
                    bounds.y + bounds.height * center.y,
                ];
                let left = (origin[0] - bounds.x).abs();
                let right = (bounds.right() - origin[0]).abs();
                let top = (origin[1] - bounds.y).abs();
                let bottom = (bounds.bottom() - origin[1]).abs();
                let (mut radius_x, mut radius_y) = match extent {
                    RadialGradientExtent::ClosestSide => (left.min(right), top.min(bottom)),
                    RadialGradientExtent::FarthestSide | RadialGradientExtent::FarthestCorner => {
                        (left.max(right), top.max(bottom))
                    }
                };
                radius_x = radius_x.max(f32::EPSILON);
                radius_y = radius_y.max(f32::EPSILON);
                if extent == RadialGradientExtent::FarthestCorner {
                    // The farthest-corner ellipse keeps the farthest-side aspect ratio and is
                    // scaled until it passes through the corner furthest from the center.
                    let corner_x = left.max(right);
                    let corner_y = top.max(bottom);
                    let scale = ((corner_x / radius_x).powi(2) + (corner_y / radius_y).powi(2))
                        .max(0.0)
                        .sqrt();
                    if scale.is_finite() && scale > 0.0 {
                        radius_x *= scale;
                        radius_y *= scale;
                    }
                }
                if shape == RadialGradientShape::Circle {
                    let radius = match extent {
                        RadialGradientExtent::ClosestSide => radius_x.min(radius_y),
                        RadialGradientExtent::FarthestSide => radius_x.max(radius_y),
                        RadialGradientExtent::FarthestCorner => {
                            (left.max(right).powi(2) + top.max(bottom).powi(2)).sqrt()
                        }
                    }
                    .max(f32::EPSILON);
                    radius_x = radius;
                    radius_y = radius;
                }
                (
                    GRADIENT_KIND_RADIAL,
                    [origin[0], origin[1], radius_x, radius_y],
                )
            }
            GradientKind::Conic { from_angle, center } => (
                GRADIENT_KIND_CONIC,
                [
                    bounds.x + bounds.width * center.x,
                    bounds.y + bounds.height * center.y,
                    from_angle.degrees().to_radians(),
                    0.0,
                ],
            ),
        };
        let stops = gradient.stops().as_slice();
        let mut positions = [[0.0_f32; 4]; 2];
        let mut colors = [[0.0_f32; 4]; MAX_GRADIENT_STOPS];
        for (index, stop) in stops.iter().enumerate() {
            positions[index / 4][index % 4] = stop.position;
            colors[index] = stop.color.as_array();
        }
        Self {
            header: [
                kind,
                interpolation_code(gradient.interpolation()),
                stops.len() as f32,
                if gradient.dither { 1.0 } else { 0.0 },
            ],
            geometry,
            projection: match gradient.kind() {
                GradientKind::Linear { angle } if gradient.box_projection => {
                    [angle.degrees(), 1., 0., 0.]
                }
                _ => [0.; 4],
            },
            positions,
            colors,
        }
    }
}

#[derive(Clone)]
pub struct Path(Arc<PathData>);

struct PathData {
    id: u64,
    /// Three positions per triangle, in source logical coordinates.
    positions: Arc<[[f32; 2]]>,
    /// One three-bit mask per triangle. Bit N marks the edge opposite vertex N as an outline.
    boundary_masks: Arc<[u8]>,
    bounds: Rect,
}

impl Path {
    fn from_geometry(geometry: LimitedGeometry) -> Result<Self, PathError> {
        if geometry.overflowed {
            return Err(PathError::TooComplex {
                vertices: geometry.indices.len(),
                maximum: MAX_PATH_VERTICES,
            });
        }
        if geometry.invalid_triangle {
            return Err(PathError::InvalidGeometry);
        }
        if geometry.indices.len() > MAX_PATH_VERTICES {
            return Err(PathError::TooComplex {
                vertices: geometry.indices.len(),
                maximum: MAX_PATH_VERTICES,
            });
        }

        let mut edge_counts = HashMap::<(u32, u32), u8>::with_capacity(geometry.indices.len());
        for triangle in geometry.indices.as_chunks::<3>().0 {
            for (first, second) in [
                (triangle[1], triangle[2]),
                (triangle[2], triangle[0]),
                (triangle[0], triangle[1]),
            ] {
                let edge = if first <= second {
                    (first, second)
                } else {
                    (second, first)
                };
                let count = edge_counts.entry(edge).or_default();
                *count = count.saturating_add(1);
            }
        }

        let mut positions = Vec::with_capacity(geometry.indices.len());
        let mut boundary_masks = Vec::with_capacity(geometry.indices.len() / 3);
        let mut minimum = [f32::INFINITY; 2];
        let mut maximum = [f32::NEG_INFINITY; 2];
        for triangle in geometry.indices.as_chunks::<3>().0 {
            let edges = [
                (triangle[1], triangle[2]),
                (triangle[2], triangle[0]),
                (triangle[0], triangle[1]),
            ];
            let mut mask = 0_u8;
            for (edge_index, (first, second)) in edges.into_iter().enumerate() {
                let edge = if first <= second {
                    (first, second)
                } else {
                    (second, first)
                };
                if edge_counts.get(&edge) == Some(&1) {
                    mask |= 1 << edge_index;
                }
            }
            boundary_masks.push(mask);
            for index in triangle {
                let position = geometry.vertices[*index as usize];
                if !valid_coordinate(position[0]) || !valid_coordinate(position[1]) {
                    return Err(PathError::InvalidGeometry);
                }
                minimum[0] = minimum[0].min(position[0]);
                minimum[1] = minimum[1].min(position[1]);
                maximum[0] = maximum[0].max(position[0]);
                maximum[1] = maximum[1].max(position[1]);
                positions.push(position);
            }
        }

        let bytes = positions
            .len()
            .saturating_mul(mem::size_of::<[f32; 2]>())
            .saturating_add(boundary_masks.len());
        if bytes > MAX_PATH_BYTES {
            return Err(PathError::TooLarge {
                bytes,
                maximum: MAX_PATH_BYTES,
            });
        }
        let bounds = if positions.is_empty() {
            Rect::ZERO
        } else {
            Rect::new(
                minimum[0],
                minimum[1],
                maximum[0] - minimum[0],
                maximum[1] - minimum[1],
            )
        };
        Ok(Self(Arc::new(PathData {
            id: NEXT_PATH_ID.fetch_add(1, Ordering::Relaxed),
            positions: positions.into(),
            boundary_masks: boundary_masks.into(),
            bounds,
        })))
    }

    pub fn bounds(&self) -> Rect {
        self.0.bounds
    }

    pub fn size(&self) -> Size {
        Size::new(self.0.bounds.width, self.0.bounds.height)
    }

    pub fn is_empty(&self) -> bool {
        self.0.positions.is_empty()
    }

    pub fn triangle_count(&self) -> usize {
        self.0.boundary_masks.len()
    }

    pub fn vertex_count(&self) -> usize {
        self.0.positions.len()
    }

    pub fn byte_len(&self) -> usize {
        self.0.positions.len() * mem::size_of::<[f32; 2]>() + self.0.boundary_masks.len()
    }

    pub(crate) fn positions(&self) -> &[[f32; 2]] {
        &self.0.positions
    }

    pub(crate) fn boundary_masks(&self) -> &[u8] {
        &self.0.boundary_masks
    }
}

impl From<&Path> for Path {
    fn from(path: &Path) -> Self {
        path.clone()
    }
}

impl fmt::Debug for Path {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Path")
            .field("bounds", &self.bounds())
            .field("triangles", &self.triangle_count())
            .field("bytes", &self.byte_len())
            .finish_non_exhaustive()
    }
}

impl PartialEq for Path {
    fn eq(&self, other: &Self) -> bool {
        self.0.id == other.0.id
    }
}

impl Eq for Path {}

pub struct PathBuilder {
    raw: lyon::path::builder::WithSvg<lyon::path::BuilderImpl>,
    style: PathStyle,
    dash_array: Option<Vec<f32>>,
    transform: Option<Transform>,
    commands: usize,
    error: Option<PathError>,
}

impl Default for PathBuilder {
    fn default() -> Self {
        Self::fill()
    }
}

impl PathBuilder {
    pub fn fill() -> Self {
        Self {
            raw: LyonPath::svg_builder(),
            style: PathStyle::Fill(FillOptions::default()),
            dash_array: None,
            transform: None,
            commands: 0,
            error: None,
        }
    }

    pub fn stroke(width: f32) -> Self {
        Self {
            style: PathStyle::Stroke(StrokeOptions::default().with_line_width(width)),
            ..Self::fill()
        }
    }

    pub fn with_style(mut self, style: PathStyle) -> Self {
        self.style = style;
        self
    }

    pub fn dash_array(mut self, values: &[f32]) -> Self {
        if values.is_empty() {
            self.dash_array = None;
            return self;
        }
        let output_len = if values.len().is_multiple_of(2) {
            values.len()
        } else {
            values.len().saturating_mul(2)
        };
        if output_len > MAX_DASH_VALUES
            || values
                .iter()
                .any(|value| !value.is_finite() || *value <= 0.0 || *value > MAX_PATH_COORDINATE)
        {
            self.record_error(PathError::InvalidDashPattern);
            return self;
        }
        let mut dash_array = Vec::with_capacity(output_len);
        dash_array.extend_from_slice(values);
        if !values.len().is_multiple_of(2) {
            dash_array.extend_from_slice(values);
        }
        self.dash_array = Some(dash_array);
        self
    }

    pub fn move_to(&mut self, to: Point) {
        if self.accept_points(&[to]) {
            self.raw.move_to(point(to.x, to.y));
        }
    }

    pub fn line_to(&mut self, to: Point) {
        if self.accept_points(&[to]) {
            self.raw.line_to(point(to.x, to.y));
        }
    }

    pub fn quadratic_to(&mut self, control: Point, to: Point) {
        if self.accept_points(&[control, to]) {
            self.raw
                .quadratic_bezier_to(point(control.x, control.y), point(to.x, to.y));
        }
    }

    /// GPUI-compatible argument order: destination followed by the control point.
    pub fn curve_to(&mut self, to: Point, control: Point) {
        self.quadratic_to(control, to);
    }

    pub fn cubic_to(&mut self, control_a: Point, control_b: Point, to: Point) {
        if self.accept_points(&[control_a, control_b, to]) {
            self.raw.cubic_bezier_to(
                point(control_a.x, control_a.y),
                point(control_b.x, control_b.y),
                point(to.x, to.y),
            );
        }
    }

    /// GPUI-compatible argument order: destination followed by both control points.
    pub fn cubic_bezier_to(&mut self, to: Point, control_a: Point, control_b: Point) {
        self.cubic_to(control_a, control_b, to);
    }

    pub fn arc_to(
        &mut self,
        radii: Size,
        x_rotation_degrees: f32,
        large_arc: bool,
        sweep: bool,
        to: Point,
    ) {
        if !valid_nonnegative_coordinate(radii.width)
            || !valid_nonnegative_coordinate(radii.height)
            || !x_rotation_degrees.is_finite()
        {
            self.record_error(PathError::InvalidCoordinate);
            return;
        }
        if self.accept_points(&[to]) {
            self.raw.arc_to(
                vector(radii.width, radii.height),
                Angle::degrees(x_rotation_degrees),
                ArcFlags { large_arc, sweep },
                point(to.x, to.y),
            );
        }
    }

    pub fn relative_arc_to(
        &mut self,
        radii: Size,
        x_rotation_degrees: f32,
        large_arc: bool,
        sweep: bool,
        to: Point,
    ) {
        if !valid_nonnegative_coordinate(radii.width)
            || !valid_nonnegative_coordinate(radii.height)
            || !x_rotation_degrees.is_finite()
        {
            self.record_error(PathError::InvalidCoordinate);
            return;
        }
        if self.accept_points(&[to]) {
            self.raw.relative_arc_to(
                vector(radii.width, radii.height),
                Angle::degrees(x_rotation_degrees),
                ArcFlags { large_arc, sweep },
                vector(to.x, to.y),
            );
        }
    }

    pub fn add_polygon(&mut self, points: &[Point], closed: bool) {
        if points.is_empty() {
            return;
        }
        if self.commands.saturating_add(points.len()) > MAX_PATH_COMMANDS
            || points.iter().any(|point| !valid_point(*point))
        {
            self.record_error(
                if self.commands.saturating_add(points.len()) > MAX_PATH_COMMANDS {
                    PathError::TooManyCommands {
                        commands: self.commands.saturating_add(points.len()),
                        maximum: MAX_PATH_COMMANDS,
                    }
                } else {
                    PathError::InvalidCoordinate
                },
            );
            return;
        }
        self.commands += points.len();
        let lyon_points = points
            .iter()
            .map(|source_point| point(source_point.x, source_point.y))
            .collect::<Vec<_>>();
        self.raw.add_polygon(Polygon {
            points: &lyon_points,
            closed,
        });
    }

    pub fn close(&mut self) {
        if self.accept_command() {
            self.raw.close();
        }
    }

    pub fn translate(&mut self, x: f32, y: f32) {
        if !valid_coordinate(x) || !valid_coordinate(y) {
            self.record_error(PathError::InvalidTransform);
            return;
        }
        self.transform = Some(
            self.transform
                .unwrap_or_else(Transform::identity)
                .then_translate(vector(x, y)),
        );
    }

    pub fn scale(&mut self, scale: f32) {
        self.scale_xy(scale, scale);
    }

    pub fn scale_xy(&mut self, x: f32, y: f32) {
        if !x.is_finite()
            || !y.is_finite()
            || x.abs() > MAX_TRANSFORM_SCALE
            || y.abs() > MAX_TRANSFORM_SCALE
        {
            self.record_error(PathError::InvalidTransform);
            return;
        }
        self.transform = Some(
            self.transform
                .unwrap_or_else(Transform::identity)
                .then_scale(x, y),
        );
    }

    pub fn rotate(&mut self, radians: f32) {
        if !radians.is_finite() {
            self.record_error(PathError::InvalidTransform);
            return;
        }
        self.transform = Some(
            self.transform
                .unwrap_or_else(Transform::identity)
                .then_rotate(Angle::radians(radians)),
        );
    }

    pub fn build(self) -> Result<Path, PathError> {
        if let Some(error) = self.error {
            return Err(error);
        }
        validate_style(self.style)?;
        let path = match self.transform {
            Some(transform) => self.raw.build().transformed(&transform),
            None => self.raw.build(),
        };
        let dashed;
        let path = if let (PathStyle::Stroke(_), Some(pattern)) = (self.style, self.dash_array) {
            dashed = dashed_path(&path, &pattern)?;
            &dashed
        } else {
            &path
        };
        let mut geometry = LimitedGeometry::default();
        let result = match self.style {
            PathStyle::Fill(options) => {
                let options = LyonFillOptions::default()
                    .with_fill_rule(match options.fill_rule {
                        FillRule::NonZero => lyon::path::FillRule::NonZero,
                        FillRule::EvenOdd => lyon::path::FillRule::EvenOdd,
                    })
                    .with_tolerance(options.tolerance);
                FillTessellator::new().tessellate_path(path, &options, &mut geometry)
            }
            PathStyle::Stroke(options) => {
                let options = LyonStrokeOptions::default()
                    .with_line_width(options.line_width)
                    .with_line_cap(match options.line_cap {
                        LineCap::Butt => lyon::path::LineCap::Butt,
                        LineCap::Square => lyon::path::LineCap::Square,
                        LineCap::Round => lyon::path::LineCap::Round,
                    })
                    .with_line_join(match options.line_join {
                        LineJoin::Miter => lyon::path::LineJoin::Miter,
                        LineJoin::MiterClip => lyon::path::LineJoin::MiterClip,
                        LineJoin::Round => lyon::path::LineJoin::Round,
                        LineJoin::Bevel => lyon::path::LineJoin::Bevel,
                    })
                    .with_miter_limit(options.miter_limit)
                    .with_tolerance(options.tolerance);
                StrokeTessellator::new().tessellate_path(path, &options, &mut geometry)
            }
        };
        if geometry.overflowed {
            return Err(PathError::TooComplex {
                vertices: geometry.indices.len().saturating_add(3),
                maximum: MAX_PATH_VERTICES,
            });
        }
        result.map_err(PathError::Tessellation)?;
        Path::from_geometry(geometry)
    }

    fn accept_points(&mut self, points: &[Point]) -> bool {
        if points.iter().any(|point| !valid_point(*point)) {
            self.record_error(PathError::InvalidCoordinate);
            return false;
        }
        self.accept_command()
    }

    fn accept_command(&mut self) -> bool {
        if self.error.is_some() {
            return false;
        }
        if self.commands == MAX_PATH_COMMANDS {
            self.record_error(PathError::TooManyCommands {
                commands: self.commands.saturating_add(1),
                maximum: MAX_PATH_COMMANDS,
            });
            return false;
        }
        self.commands += 1;
        true
    }

    fn record_error(&mut self, error: PathError) {
        if self.error.is_none() {
            self.error = Some(error);
        }
    }
}

fn validate_style(style: PathStyle) -> Result<(), PathError> {
    let tolerance = match style {
        PathStyle::Fill(options) => options.tolerance,
        PathStyle::Stroke(options) => {
            if !options.line_width.is_finite()
                || options.line_width <= 0.0
                || options.line_width > MAX_STROKE_WIDTH
            {
                return Err(PathError::InvalidStrokeWidth(options.line_width));
            }
            if !options.miter_limit.is_finite()
                || options.miter_limit < LyonStrokeOptions::MINIMUM_MITER_LIMIT
                || options.miter_limit > MAX_MITER_LIMIT
            {
                return Err(PathError::InvalidMiterLimit(options.miter_limit));
            }
            options.tolerance
        }
    };
    if !tolerance.is_finite() || !(MIN_TOLERANCE..=MAX_TOLERANCE).contains(&tolerance) {
        return Err(PathError::InvalidTolerance(tolerance));
    }
    Ok(())
}

fn dashed_path(path: &LyonPath, pattern: &[f32]) -> Result<LyonPath, PathError> {
    debug_assert!(!pattern.is_empty());
    let measurements = PathMeasurements::from_path(path, MIN_TOLERANCE);
    let total_length = measurements.length();
    if !total_length.is_finite() {
        return Err(PathError::InvalidGeometry);
    }
    if total_length <= 0.0 {
        return Ok(path.clone());
    }
    let minimum_dash = pattern.iter().copied().fold(f32::INFINITY, f32::min);
    let estimated_segments = (total_length / minimum_dash).ceil() as usize;
    if estimated_segments > MAX_PATH_DASH_SEGMENTS {
        return Err(PathError::TooManyDashSegments {
            segments: estimated_segments,
            maximum: MAX_PATH_DASH_SEGMENTS,
        });
    }

    let mut sampler = measurements.create_sampler(path, SampleType::Normalized);
    let mut builder = LyonPath::builder();
    let mut position = 0.0;
    let mut dash_index = 0_usize;
    while position < total_length {
        if dash_index == MAX_PATH_DASH_SEGMENTS {
            return Err(PathError::TooManyDashSegments {
                segments: dash_index.saturating_add(1),
                maximum: MAX_PATH_DASH_SEGMENTS,
            });
        }
        let end = (position + pattern[dash_index % pattern.len()]).min(total_length);
        if dash_index.is_multiple_of(2) {
            sampler.split_range(position / total_length..end / total_length, &mut builder);
        }
        position = end;
        dash_index += 1;
    }
    Ok(builder.build())
}

#[derive(Default)]
struct LimitedGeometry {
    vertices: Vec<[f32; 2]>,
    indices: Vec<u32>,
    overflowed: bool,
    invalid_triangle: bool,
}

impl GeometryBuilder for LimitedGeometry {
    fn add_triangle(&mut self, a: VertexId, b: VertexId, c: VertexId) {
        if self.indices.len().saturating_add(3) > MAX_PATH_VERTICES {
            self.overflowed = true;
            return;
        }
        let indices = [a.offset(), b.offset(), c.offset()];
        if indices
            .iter()
            .any(|index| *index as usize >= self.vertices.len())
        {
            self.invalid_triangle = true;
            return;
        }
        self.indices.extend_from_slice(&indices);
    }
}

impl FillGeometryBuilder for LimitedGeometry {
    fn add_fill_vertex(&mut self, vertex: FillVertex) -> Result<VertexId, GeometryBuilderError> {
        self.add_vertex(vertex.position().to_array())
    }
}

impl StrokeGeometryBuilder for LimitedGeometry {
    fn add_stroke_vertex(
        &mut self,
        vertex: StrokeVertex,
    ) -> Result<VertexId, GeometryBuilderError> {
        self.add_vertex(vertex.position().to_array())
    }
}

impl LimitedGeometry {
    fn add_vertex(&mut self, position: [f32; 2]) -> Result<VertexId, GeometryBuilderError> {
        if self.vertices.len() >= MAX_PATH_VERTICES {
            self.overflowed = true;
            return Err(GeometryBuilderError::TooManyVertices);
        }
        if !valid_coordinate(position[0]) || !valid_coordinate(position[1]) {
            return Err(GeometryBuilderError::InvalidVertex);
        }
        let id = VertexId::from_usize(self.vertices.len());
        self.vertices.push(position);
        Ok(id)
    }
}

fn valid_point(point: Point) -> bool {
    valid_coordinate(point.x) && valid_coordinate(point.y)
}

fn valid_coordinate(value: f32) -> bool {
    value.is_finite() && value.abs() <= MAX_PATH_COORDINATE
}

fn valid_nonnegative_coordinate(value: f32) -> bool {
    value.is_finite() && (0.0..=MAX_PATH_COORDINATE).contains(&value)
}

fn sanitize_color(color: Color) -> Color {
    Color::linear(
        finite_or(color.r, 0.0).clamp(0.0, 1.0),
        finite_or(color.g, 0.0).clamp(0.0, 1.0),
        finite_or(color.b, 0.0).clamp(0.0, 1.0),
        finite_or(color.a, 0.0).clamp(0.0, 1.0),
    )
}

fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}

#[derive(Debug, Error)]
pub enum PathError {
    #[error("path coordinates must be finite and within +/-{MAX_PATH_COORDINATE}")]
    InvalidCoordinate,
    #[error("path transforms must be finite and within the supported range")]
    InvalidTransform,
    #[error("stroke width {0} must be finite and in (0, {MAX_STROKE_WIDTH}]")]
    InvalidStrokeWidth(f32),
    #[error(
        "miter limit {0} must be finite and between {minimum} and {MAX_MITER_LIMIT}",
        minimum = LyonStrokeOptions::MINIMUM_MITER_LIMIT
    )]
    InvalidMiterLimit(f32),
    #[error("tolerance {0} must be finite and between {MIN_TOLERANCE} and {MAX_TOLERANCE}")]
    InvalidTolerance(f32),
    #[error(
        "dash values must be finite, positive, bounded, and contain at most {MAX_DASH_VALUES} values"
    )]
    InvalidDashPattern,
    #[error("path contains {commands} commands; the maximum is {maximum}")]
    TooManyCommands { commands: usize, maximum: usize },
    #[error("dashing would generate {segments} segments; the maximum is {maximum}")]
    TooManyDashSegments { segments: usize, maximum: usize },
    #[error("path tessellation generated {vertices} vertices; the maximum is {maximum}")]
    TooComplex { vertices: usize, maximum: usize },
    #[error("retained path data is {bytes} bytes; the maximum is {maximum} bytes")]
    TooLarge { bytes: usize, maximum: usize },
    #[error("path tessellation produced invalid geometry")]
    InvalidGeometry,
    #[error("path tessellation failed: {0}")]
    Tessellation(#[source] TessellationError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_stops_are_bounded_sorted_and_sanitized() {
        let stops = ColorStops::new(
            (0..16).map(|index| LinearColorStop::new(Color::WHITE, 1.0 - index as f32 / 15.0)),
        );
        assert_eq!(stops.len(), MAX_GRADIENT_STOPS);
        let positions: Vec<_> = stops.as_slice().iter().map(|stop| stop.position).collect();
        assert!(positions.windows(2).all(|pair| pair[0] <= pair[1]));

        let sanitized = ColorStops::new([
            LinearColorStop::new(Color::BLACK, f32::NAN),
            LinearColorStop::new(Color::WHITE, 4.0),
        ]);
        assert_eq!(sanitized.as_slice()[0].position, 0.0);
        assert_eq!(sanitized.as_slice()[1].position, 1.0);

        let even = ColorStops::evenly_spaced([Color::BLACK, Color::WHITE, Color::TRANSPARENT]);
        let positions: Vec<_> = even.as_slice().iter().map(|stop| stop.position).collect();
        assert_eq!(positions, vec![0.0, 0.5, 1.0]);
        assert!(ColorStops::empty().is_empty());
        assert!(!ColorStops::empty().is_visible());
    }

    #[test]
    fn gradient_directions_match_css_angles() {
        assert_eq!(GradientAngle::from(GradientDirection::ToTop).degrees(), 0.0);
        assert_eq!(
            GradientAngle::from(GradientDirection::ToRight).degrees(),
            90.0
        );
        assert_eq!(
            GradientAngle::from(GradientDirection::ToBottom).degrees(),
            180.0
        );
        assert_eq!(GradientAngle::new(-90.0).degrees(), 270.0);
        assert_eq!(GradientAngle::new(f32::INFINITY).degrees(), 0.0);
    }

    #[test]
    fn css_gradient_angles_span_transformed_bounds() {
        let bounds = Rect::new(10.0, 20.0, 40.0, 60.0);
        assert_eq!(gradient_line(bounds, 0.0), [30.0, 80.0, 30.0, 20.0]);
        let horizontal = gradient_line(bounds, 90.0);
        assert!((horizontal[0] - 10.0).abs() < 0.001);
        assert!((horizontal[1] - 50.0).abs() < 0.001);
        assert!((horizontal[2] - 50.0).abs() < 0.001);
        assert!((horizontal[3] - 50.0).abs() < 0.001);
    }

    #[test]
    fn gradient_data_resolves_every_kind_against_its_box() {
        let bounds = Rect::new(0.0, 0.0, 80.0, 40.0);
        let linear = GradientData::new(
            &Gradient::linear(GradientDirection::ToRight, [Color::BLACK, Color::WHITE]),
            bounds,
        );
        assert_eq!(linear.header[0], GRADIENT_KIND_LINEAR);
        assert_eq!(linear.header[2], 2.0);
        assert_eq!(linear.geometry[0], 0.0);
        assert_eq!(linear.geometry[2], 80.0);

        let circle = GradientData::new(
            &Gradient::radial([Color::WHITE, Color::BLACK]).shape(RadialGradientShape::Circle),
            bounds,
        );
        assert_eq!(circle.header[0], GRADIENT_KIND_RADIAL);
        assert_eq!(circle.geometry[0], 40.0);
        assert_eq!(circle.geometry[1], 20.0);
        // A farthest-corner circle reaches the box corner.
        assert!((circle.geometry[2] - (40.0_f32.hypot(20.0))).abs() < 0.001);
        assert_eq!(circle.geometry[2], circle.geometry[3]);

        let side = GradientData::new(
            &Gradient::radial([Color::WHITE, Color::BLACK])
                .extent(RadialGradientExtent::FarthestSide),
            bounds,
        );
        assert_eq!([side.geometry[2], side.geometry[3]], [40.0, 20.0]);

        let conic = GradientData::new(
            &Gradient::conic(90.0, [Color::WHITE, Color::BLACK])
                .center(GradientCenter::new(0.25, 0.75)),
            bounds,
        );
        assert_eq!(conic.header[0], GRADIENT_KIND_CONIC);
        assert_eq!([conic.geometry[0], conic.geometry[1]], [20.0, 30.0]);
        assert!((conic.geometry[2] - std::f32::consts::FRAC_PI_2).abs() < 0.0001);
    }

    #[test]
    fn two_stop_linear_gradients_reuse_the_multi_stop_representation() {
        let background = linear_gradient(
            45.0,
            linear_color_stop(Color::BLACK, 0.25),
            linear_color_stop(Color::WHITE, 0.75),
        );
        let gradient = background.as_gradient().unwrap();
        assert_eq!(gradient.stops().len(), 2);
        assert_eq!(gradient.stops().as_slice()[0].position, 0.25);
        assert!(matches!(gradient.kind(), GradientKind::Linear { .. }));
        assert!(Background::from(Color::WHITE).as_gradient().is_none());
    }

    #[test]
    fn gradient_alpha_multiplies_every_stop() {
        let faded = Background::Gradient(Gradient::linear(0.0, [Color::WHITE, Color::BLACK]))
            .multiply_alpha(0.5);
        let gradient = faded.as_gradient().unwrap();
        assert!(
            gradient
                .stops()
                .as_slice()
                .iter()
                .all(|stop| stop.color.a == 0.5)
        );
        assert!(faded.is_visible());
        assert!(!faded.multiply_alpha(0.0).is_visible());
    }

    #[test]
    fn fill_tessellates_once_and_clones_share_identity() {
        let mut builder = PathBuilder::fill();
        builder.move_to(Point::new(10.0, 20.0));
        builder.line_to(Point::new(50.0, 20.0));
        builder.line_to(Point::new(50.0, 60.0));
        builder.close();
        let path = builder.build().unwrap();

        assert_eq!(path.triangle_count(), 1);
        assert_eq!(path.vertex_count(), 3);
        assert_eq!(path.bounds(), Rect::new(10.0, 20.0, 40.0, 40.0));
        assert_eq!(path, path.clone());
    }

    #[test]
    fn internal_triangle_edges_are_not_antialiased() {
        let mut builder = PathBuilder::fill();
        builder.add_polygon(
            &[
                Point::new(0.0, 0.0),
                Point::new(20.0, 0.0),
                Point::new(20.0, 20.0),
                Point::new(0.0, 20.0),
            ],
            true,
        );
        let path = builder.build().unwrap();

        assert_eq!(path.triangle_count(), 2);
        assert_eq!(
            path.boundary_masks()
                .iter()
                .map(|mask| mask.count_ones())
                .sum::<u32>(),
            4
        );
    }

    #[test]
    fn strokes_support_safe_dash_patterns_and_curve_commands() {
        let mut builder = PathBuilder::stroke(3.0)
            .with_style(PathStyle::Stroke(
                StrokeOptions::default()
                    .with_line_width(3.0)
                    .with_line_cap(LineCap::Round)
                    .with_line_join(LineJoin::Bevel),
            ))
            .dash_array(&[6.0, 3.0]);
        builder.move_to(Point::new(0.0, 10.0));
        builder.quadratic_to(Point::new(25.0, -10.0), Point::new(50.0, 10.0));
        builder.cubic_to(
            Point::new(60.0, 30.0),
            Point::new(80.0, -10.0),
            Point::new(100.0, 10.0),
        );
        let path = builder.build().unwrap();
        assert!(!path.is_empty());
        assert!(path.bounds().height > 3.0);
    }

    #[test]
    fn zero_or_non_finite_dash_values_are_rejected_without_looping() {
        let mut builder = PathBuilder::stroke(1.0).dash_array(&[4.0, 0.0]);
        builder.move_to(Point::ZERO);
        builder.line_to(Point::new(10.0, 0.0));
        assert!(matches!(
            builder.build(),
            Err(PathError::InvalidDashPattern)
        ));

        let mut builder = PathBuilder::stroke(1.0).dash_array(&[f32::NAN]);
        builder.move_to(Point::ZERO);
        builder.line_to(Point::new(10.0, 0.0));
        assert!(matches!(
            builder.build(),
            Err(PathError::InvalidDashPattern)
        ));
    }

    #[test]
    fn invalid_geometry_and_styles_fail_at_the_api_boundary() {
        let mut invalid_point = PathBuilder::fill();
        invalid_point.move_to(Point::new(f32::INFINITY, 0.0));
        assert!(matches!(
            invalid_point.build(),
            Err(PathError::InvalidCoordinate)
        ));

        let invalid_stroke = PathBuilder::stroke(0.0).build();
        assert!(matches!(
            invalid_stroke,
            Err(PathError::InvalidStrokeWidth(0.0))
        ));

        let invalid_tolerance = PathBuilder::fill()
            .with_style(PathStyle::Fill(FillOptions::default().with_tolerance(0.0)))
            .build();
        assert!(matches!(
            invalid_tolerance,
            Err(PathError::InvalidTolerance(0.0))
        ));
    }

    #[test]
    fn gradients_sanitize_angles_stops_and_colors() {
        let gradient = LinearGradient::new(
            f32::NAN,
            LinearColorStop::new(Color::linear(f32::NAN, 2.0, -1.0, 2.0), 2.0),
            LinearColorStop::new(Color::WHITE, -1.0),
        );
        assert_eq!(gradient.angle_degrees(), 180.0);
        assert_eq!(gradient.stops()[0].position, 0.0);
        assert_eq!(gradient.stops()[1].position, 1.0);
        assert_eq!(gradient.stops()[1].color, Color::linear(0.0, 1.0, 0.0, 1.0));
    }

    #[test]
    fn empty_paths_are_valid_and_allocation_small() {
        let path = PathBuilder::fill().build().unwrap();
        assert!(path.is_empty());
        assert_eq!(path.bounds(), Rect::ZERO);
        assert_eq!(path.byte_len(), 0);
    }

    #[test]
    fn constants_keep_retained_and_gpu_expansion_bounded() {
        assert_eq!(MAX_PATH_VERTICES % 3, 0);
        assert!(MAX_PATH_VERTICES * mem::size_of::<[f32; 2]>() <= MAX_PATH_BYTES);
    }
}
