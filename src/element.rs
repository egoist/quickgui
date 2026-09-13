use std::{
    any::{Any, TypeId},
    borrow::Cow,
    cell::RefCell,
    fmt,
    rc::Rc,
    sync::Arc,
};

use glyphon::{Style as GlyphStyle, Weight};
use taffy::{
    Style,
    geometry::{Line as TaffyLine, Point as TaffyPoint, Rect as TaffyRect, Size as TaffySize},
    prelude::{
        AlignContent, AlignItems, AlignSelf, Dimension, Display, FlexDirection, FlexWrap,
        GridAutoFlow, GridPlacement, GridTemplateComponent, JustifyContent, LengthPercentage,
        LengthPercentageAuto, Position, TrackSizingFunction,
    },
    style::Overflow,
    style_helpers::{
        auto, fit_content, flex, fr, length, line, max_content, min_content, minmax, percent,
        repeat,
    },
};

use crate::{
    AnimatedImage, Background, BorderStyle, BoxShadow, Canvas, Color, ColorStops, Corners,
    CursorStyle, CustomShader, DispatchPhase, Filter, Filters, Font, FontFallbacks, FontFamily,
    FontFeatures, Gradient, GradientAngle, GradientCenter, Image, ImageSource, Insets, KeyContext,
    MAX_VALIDATION_MESSAGE_BYTES, ObjectFit, Path, RadialGradientShape, Rect, ScenePlane,
    ShaderParameters, StyledText, Svg, SvgTransform, TextAlign, TextCheckingOverrides,
    TextHighlight, TextOverflow, TextShaping, TextStyle, TextUnderline, TextWrap, Tooltip,
    Transition, WhiteSpace,
    action::{ActionListenerBinding, MAX_ACTION_LISTENERS_PER_ELEMENT},
    animation::ElementAnimation,
    font::{assert_valid_font_family, normalize_fallbacks},
    spring::ElementSpring,
    virtual_list::{
        ListItemMeasurement, ListState, VirtualList, VirtualScrollHandle, VirtualScrollMount,
    },
};

// Direction, sticky positioning, scroll snapping, and extended text styling.
use crate::{
    Hyphens, OverflowWrap, TextDirection, TextShadow, TextTransform, WordBreak,
    scene::sane_text_spacing,
};

// Compositing layers: transforms, subtree filters, backdrop effects, and blend modes.
use crate::{BlendMode, DropShadow, Point, Transform2D};

#[cfg(target_os = "macos")]
use crate::native_view::MacNativeView;
#[cfg(target_os = "macos")]
use objc2_app_kit::NSView;

const SPACING_UNIT: f32 = 4.0;
const DEFAULT_ANCHOR_GAP: f32 = 8.0;
const DEFAULT_VIEWPORT_MARGIN: f32 = 8.0;

macro_rules! state_cursor_helper {
    ($method:ident, $variant:ident, $css:literal) => {
        #[doc = concat!("Use the native `", $css, "` cursor while this state is active.")]
        pub fn $method(self) -> Self {
            self.cursor(CursorStyle::$variant)
        }
    };
}

macro_rules! spacing_scale_methods {
    ($setter:ident; $($method:ident => $units:expr),+ $(,)?) => {
        $(
            pub fn $method(self) -> Self {
                self.$setter(SPACING_UNIT * $units)
            }
        )+
    };
}

/// Maximum CSS-like box shadows retained by one element or interaction-state override.
pub const MAX_BOX_SHADOWS_PER_ELEMENT: usize = 8;

/// Longest name a group can be declared with or targeted by, in bytes.
pub const MAX_HOVER_GROUP_NAME_BYTES: usize = 256;
/// Most group states — `group_hover` and `group_active` entries together — one element follows.
pub const MAX_GROUP_STYLES_PER_ELEMENT: usize = 8;

/// Largest accepted outline width in logical pixels.
pub const MAX_OUTLINE_WIDTH: f32 = 1_024.0;
/// Largest number of repeated background-image tiles painted for one element.
///
/// A tiling that would exceed this bound falls back to a single tile rather than emitting an
/// unbounded number of per-frame image instances.
pub const MAX_BACKGROUND_IMAGE_TILES: usize = 256;
/// Largest accepted absolute outline offset in logical pixels.
pub const MAX_OUTLINE_OFFSET: f32 = 1_024.0;
/// Largest accepted corner radius in logical pixels.
///
/// Radii are additionally reduced by the CSS uniform-scale rule so two radii sharing one edge
/// can never overlap.
pub const MAX_CORNER_RADIUS: f32 = 4_096.0;

/// Maximum explicit grid tracks accepted on either axis.
///
/// The public grid helpers retain a compact `repeat()` definition, but layout cost still scales
/// with the resolved track count. Keeping this below Taffy's much larger internal safety limit
/// prevents dynamic application data from accidentally creating an expensive desktop layout.
pub const MAX_GRID_TRACKS: u16 = 1_024;

/// Maximum CSS-like container query nodes retained by one window declaration.
pub const MAX_CONTAINER_QUERIES_PER_WINDOW: usize = 1_024;

/// Maximum nested container-query depth resolved in one declaration.
pub const MAX_CONTAINER_QUERY_DEPTH: usize = 16;

/// Maximum targeted desktop mouse declarations attached to one retained element.
///
/// Ordinary elements retain only one optional pointer and allocate nothing for mouse dispatch.
pub const MAX_MOUSE_LISTENERS_PER_ELEMENT: usize = 16;

/// Maximum focused key listeners attached to one retained element.
pub const MAX_KEY_LISTENERS_PER_ELEMENT: usize = 16;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct MouseListenerKey(pub(crate) u64);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MouseListenerKind {
    Down,
    Up,
    Move,
    Exit,
    Hover,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct MouseListenerBinding {
    pub(crate) key: MouseListenerKey,
    pub(crate) kind: MouseListenerKind,
    pub(crate) phase: DispatchPhase,
    pub(crate) button: Option<crate::MouseButton>,
    pub(crate) outside: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct KeyListenerKey(pub(crate) u64);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum KeyListenerKind {
    Down,
    Up,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct KeyListenerBinding {
    pub(crate) key: KeyListenerKey,
    pub(crate) kind: KeyListenerKind,
    pub(crate) phase: DispatchPhase,
}

const DISMISS_ON_ESCAPE: u8 = 1 << 0;
const DISMISS_ON_POINTER_OUTSIDE: u8 = 1 << 1;

/// Independent event boundaries for one dismissible retained surface.
///
/// Keeping this as one byte lets modal components distinguish ordinary dialogs from alert
/// dialogs without adding another pair of booleans to every [`Element`].
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct DismissPolicy(u8);

impl DismissPolicy {
    pub(crate) const BOTH: Self = Self(DISMISS_ON_ESCAPE | DISMISS_ON_POINTER_OUTSIDE);

    pub(crate) const fn on_escape(self) -> bool {
        self.0 & DISMISS_ON_ESCAPE != 0
    }

    pub(crate) const fn on_pointer_outside(self) -> bool {
        self.0 & DISMISS_ON_POINTER_OUTSIDE != 0
    }

    pub(crate) const fn with_escape(self) -> Self {
        Self(self.0 | DISMISS_ON_ESCAPE)
    }

    pub(crate) const fn with_pointer_outside(self) -> Self {
        Self(self.0 | DISMISS_ON_POINTER_OUTSIDE)
    }

    pub(crate) const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

const MAX_GRID_LINE: i16 = MAX_GRID_TRACKS as i16 + 1;

/// One explicit CSS-grid track used by [`Element::grid_template_columns`] and
/// [`Element::grid_template_rows`].
///
/// Fractional tracks use `minmax(0, Nfr)`, which matches the web-friendly behavior of GPUI's
/// equal-column helpers and allows content to shrink without forcing overflow.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridTrack(TrackSizingFunction);

impl GridTrack {
    /// A content-sized `auto` track.
    pub fn auto() -> Self {
        Self(auto())
    }

    /// A track sized to its minimum content contribution.
    pub fn min_content() -> Self {
        Self(min_content())
    }

    /// A track sized to its maximum content contribution.
    pub fn max_content() -> Self {
        Self(max_content())
    }

    /// A fixed logical-pixel track.
    pub fn px(value: f32) -> Self {
        Self(length(finite_nonnegative(value)))
    }

    /// A percentage track expressed as a `0.0..=1.0` fraction of the grid container.
    pub fn percent(fraction: f32) -> Self {
        Self(percent(finite_nonnegative(fraction).min(1.0)))
    }

    /// A flexible `minmax(0, Nfr)` track.
    pub fn fr(fraction: f32) -> Self {
        Self(flex(finite_nonnegative(fraction)))
    }

    /// The common responsive web track `minmax(<minimum px>, <fraction>fr)`.
    pub fn minmax_px_fr(minimum: f32, fraction: f32) -> Self {
        Self(minmax(
            length(finite_nonnegative(minimum)),
            fr(finite_nonnegative(fraction)),
        ))
    }

    /// An `auto` minimum with a fixed fit-content limit in logical pixels.
    pub fn fit_content_px(limit: f32) -> Self {
        Self(fit_content(LengthPercentage::length(finite_nonnegative(
            limit,
        ))))
    }
}

#[derive(Clone, Copy, Debug)]
enum EqualGridTrackSizing {
    Zero,
    MinContent,
    MaxContent,
}

fn finite_nonnegative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn finite_opacity(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        1.0
    }
}

fn finite_length(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

fn equal_grid_tracks(
    count: u16,
    sizing: EqualGridTrackSizing,
) -> Vec<GridTemplateComponent<String>> {
    let count = count.min(MAX_GRID_TRACKS);
    if count == 0 {
        return Vec::new();
    }
    let track: TrackSizingFunction = match sizing {
        EqualGridTrackSizing::Zero => minmax(length(0.0_f32), fr(1.0_f32)),
        EqualGridTrackSizing::MinContent => minmax(min_content(), fr(1.0_f32)),
        EqualGridTrackSizing::MaxContent => minmax(length(0.0_f32), max_content()),
    };
    vec![repeat(count, vec![track])]
}

fn bounded_grid_line(index: i16) -> GridPlacement<String> {
    if index == 0 {
        GridPlacement::Auto
    } else {
        line(index.clamp(-MAX_GRID_LINE, MAX_GRID_LINE))
    }
}

fn bounded_grid_span(span: u16) -> GridPlacement<String> {
    GridPlacement::Span(span.clamp(1, MAX_GRID_TRACKS))
}

/// A stable identifier used for hit testing and retained state.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ElementId(pub(crate) u64);

/// Web-style window drag behavior for a laid-out element.
///
/// A `Drag` region hands primary-button drags to the native window and ignores element-level
/// pointer input. Descendants such as buttons opt back into normal input with `NoDrag`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AppRegion {
    Drag,
    NoDrag,
}

impl ElementId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn named(value: &str) -> Self {
        Self(stable_hash(value.as_bytes()))
    }

    pub const fn as_u64(self) -> u64 {
        self.0
    }

    pub(crate) const fn value(self) -> u64 {
        self.as_u64()
    }
}

impl From<u64> for ElementId {
    fn from(value: u64) -> Self {
        Self::new(value)
    }
}

impl From<usize> for ElementId {
    fn from(value: usize) -> Self {
        Self::new(value as u64)
    }
}

impl From<&str> for ElementId {
    fn from(value: &str) -> Self {
        Self::named(value)
    }
}

impl From<String> for ElementId {
    fn from(value: String) -> Self {
        Self::named(&value)
    }
}

/// A stable handle for programmatic and keyboard focus.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FocusHandle(ElementId);

impl FocusHandle {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self(id.into())
    }

    pub const fn id(self) -> ElementId {
        self.0
    }
}

impl From<FocusHandle> for ElementId {
    fn from(value: FocusHandle) -> Self {
        value.id()
    }
}

/// Preferred placement for a floating element relative to its anchor.
///
/// Placement automatically flips to the opposite side when it has more usable space, then shifts
/// inside the window's content viewport.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AnchorPlacement {
    TopStart,
    Top,
    TopEnd,
    #[default]
    BottomStart,
    Bottom,
    BottomEnd,
    LeftStart,
    Left,
    LeftEnd,
    RightStart,
    Right,
    RightEnd,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum AnchorTarget {
    Element(ElementId),
    Point(crate::Point),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct AnchorStyle {
    pub rounding_scale: Option<f32>,
    pub target: AnchorTarget,
    pub placement: AnchorPlacement,
    pub gap: f32,
    pub align_offset: f32,
    pub viewport_margin: f32,
    pub flip: bool,
    pub sticky: bool,
}

/// Where an anchored element actually landed on the most recently painted frame.
///
/// A declared [`AnchorPlacement`] is only a preference: QuickGUI flips to the opposite side and
/// re-aligns on the cross axis whenever the preferred side does not fit inside the collision
/// viewport. Presentation that has to follow the real placement — a popover arrow, a
/// side-dependent transform origin, a popup sized to the space it was actually given — must read
/// the resolved value instead of guessing from the preference.
///
/// Every rectangle is in window logical coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResolvedAnchorPlacement {
    /// The side and cross-axis alignment the element was actually placed with.
    pub placement: AnchorPlacement,
    /// The anchor rectangle the element was placed against.
    pub anchor: crate::Rect,
    /// The placed rectangle of the anchored element itself.
    pub bounds: crate::Rect,
    /// Space left for the element between the anchor and the collision viewport.
    ///
    /// The primary axis measures the gap-adjusted room on [`Self::placement`]'s resolved side; the
    /// cross axis measures the full margin-inset viewport extent. An application sizes a scrolling
    /// popup from this instead of measuring the window itself.
    pub available: crate::Size,
    /// Whether the anchor rectangle left the collision viewport entirely.
    ///
    /// This is the moment Base UI hides a popup whose anchor scrolled out of view; QuickGUI reports
    /// it and leaves the decision to the application.
    pub anchor_hidden: bool,
}

impl ResolvedAnchorPlacement {
    /// The resolved side, discarding the cross-axis alignment.
    pub const fn side(self) -> AnchorSide {
        AnchorSide::of(self.placement)
    }

    /// The resolved cross-axis alignment, discarding the side.
    pub const fn align(self) -> AnchorAlign {
        AnchorAlign::of(self.placement)
    }
}

impl Default for ResolvedAnchorPlacement {
    fn default() -> Self {
        Self {
            placement: AnchorPlacement::BottomStart,
            anchor: crate::Rect::ZERO,
            bounds: crate::Rect::ZERO,
            available: crate::Size::ZERO,
            anchor_hidden: false,
        }
    }
}

#[derive(Debug, Default)]
struct AnchorPlacementInner {
    resolved: Option<ResolvedAnchorPlacement>,
    revision: u64,
}

/// An application-owned receiver for the placement QuickGUI resolved for one anchored element.
///
/// Store the handle on the view next to the state that opens the surface, bind it with
/// [`Element::report_anchor_placement`], and read [`Self::resolved`] while declaring the next
/// frame. The handle retains one small allocation and no task, timer, observer, or idle scheduler
/// source: QuickGUI writes it during the paint it was already performing, and requests exactly one
/// correcting frame when the resolved placement changed, so a settled window stays settled.
///
/// ```
/// use quickgui::{AnchorPlacement, AnchorPlacementHandle, div};
///
/// let placement = AnchorPlacementHandle::new();
/// assert_eq!(placement.resolved(), None);
/// let positioner = div()
///     .anchor_to("trigger", AnchorPlacement::BottomStart)
///     .report_anchor_placement(placement.clone());
/// assert!(positioner.reports_anchor_placement());
/// ```
#[derive(Clone, Default)]
pub struct AnchorPlacementHandle(Rc<RefCell<AnchorPlacementInner>>);

impl AnchorPlacementHandle {
    pub fn new() -> Self {
        Self::default()
    }

    /// The placement resolved on the most recent painted frame, or `None` before the first one.
    pub fn resolved(&self) -> Option<ResolvedAnchorPlacement> {
        self.0.borrow().resolved
    }

    /// The resolved placement, falling back to `preferred` before the first painted frame.
    pub fn placement_or(&self, preferred: AnchorPlacement) -> AnchorPlacement {
        self.resolved()
            .map_or(preferred, |resolved| resolved.placement)
    }

    /// Forget the last resolved placement so a reopened surface cannot read a stale side.
    pub fn clear(&self) {
        let mut inner = self.0.borrow_mut();
        if inner.resolved.is_some() {
            inner.resolved = None;
            inner.revision = inner.revision.wrapping_add(1);
        }
    }

    pub(crate) fn revision(&self) -> u64 {
        self.0.borrow().revision
    }

    pub(crate) fn report(&self, resolved: ResolvedAnchorPlacement) {
        let mut inner = self.0.borrow_mut();
        if inner.resolved == Some(resolved) {
            return;
        }
        inner.resolved = Some(resolved);
        inner.revision = inner.revision.wrapping_add(1);
    }
}

impl fmt::Debug for AnchorPlacementHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AnchorPlacementHandle")
            .field("resolved", &self.resolved())
            .finish_non_exhaustive()
    }
}

impl PartialEq for AnchorPlacementHandle {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Default)]
struct LayoutBoundsInner {
    bounds: Option<Rect>,
    revision: u64,
}

/// An application-owned receiver for the window-relative bounds QuickGUI laid out for one element.
///
/// Store the handle next to the state that depends on the element's real size — a splitter that
/// must match its container, a scroll area that must know its viewport and content extents — bind
/// it with [`Element::report_bounds`], and read [`Self::bounds`] while declaring the next frame.
/// Like [`AnchorPlacementHandle`], it retains one small allocation and no task, timer, observer, or
/// idle scheduler source: QuickGUI writes it during the paint it was already performing and
/// requests exactly one correcting frame when the bounds changed, so a settled window stays
/// settled.
///
/// ```
/// use quickgui::{LayoutBoundsHandle, div};
///
/// let bounds = LayoutBoundsHandle::new();
/// assert_eq!(bounds.bounds(), None);
/// let pane = div().report_bounds(bounds.clone());
/// assert!(pane.reports_bounds());
/// ```
#[derive(Clone, Default)]
pub struct LayoutBoundsHandle(Rc<RefCell<LayoutBoundsInner>>);

impl LayoutBoundsHandle {
    pub fn new() -> Self {
        Self::default()
    }

    /// The bounds painted on the most recent frame, in logical window coordinates, or `None`
    /// before the first painted frame.
    pub fn bounds(&self) -> Option<Rect> {
        self.0.borrow().bounds
    }

    /// Forget the last painted bounds so a remounted element cannot read a stale size.
    pub fn clear(&self) {
        let mut inner = self.0.borrow_mut();
        if inner.bounds.is_some() {
            inner.bounds = None;
            inner.revision = inner.revision.wrapping_add(1);
        }
    }

    pub(crate) fn revision(&self) -> u64 {
        self.0.borrow().revision
    }

    pub(crate) fn report(&self, bounds: Rect) {
        let mut inner = self.0.borrow_mut();
        if inner.bounds == Some(bounds) {
            return;
        }
        inner.bounds = Some(bounds);
        inner.revision = inner.revision.wrapping_add(1);
    }
}

impl fmt::Debug for LayoutBoundsHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LayoutBoundsHandle")
            .field("bounds", &self.bounds())
            .finish_non_exhaustive()
    }
}

impl PartialEq for LayoutBoundsHandle {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// The side of its anchor an element was placed on.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum AnchorSide {
    Top,
    #[default]
    Bottom,
    Left,
    Right,
}

impl AnchorSide {
    /// The side half of a full placement.
    pub const fn of(placement: AnchorPlacement) -> Self {
        match placement {
            AnchorPlacement::TopStart | AnchorPlacement::Top | AnchorPlacement::TopEnd => Self::Top,
            AnchorPlacement::BottomStart | AnchorPlacement::Bottom | AnchorPlacement::BottomEnd => {
                Self::Bottom
            }
            AnchorPlacement::LeftStart | AnchorPlacement::Left | AnchorPlacement::LeftEnd => {
                Self::Left
            }
            AnchorPlacement::RightStart | AnchorPlacement::Right | AnchorPlacement::RightEnd => {
                Self::Right
            }
        }
    }

    /// The side a flip would move to.
    pub const fn opposite(self) -> Self {
        match self {
            Self::Top => Self::Bottom,
            Self::Bottom => Self::Top,
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }

    /// Whether the side runs along the vertical axis, so the cross axis is horizontal.
    pub const fn is_vertical(self) -> bool {
        matches!(self, Self::Top | Self::Bottom)
    }
}

/// The cross-axis alignment an element was placed with.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum AnchorAlign {
    #[default]
    Start,
    Center,
    End,
}

impl AnchorAlign {
    /// The alignment half of a full placement.
    pub const fn of(placement: AnchorPlacement) -> Self {
        match placement {
            AnchorPlacement::TopStart
            | AnchorPlacement::BottomStart
            | AnchorPlacement::LeftStart
            | AnchorPlacement::RightStart => Self::Start,
            AnchorPlacement::Top
            | AnchorPlacement::Bottom
            | AnchorPlacement::Left
            | AnchorPlacement::Right => Self::Center,
            AnchorPlacement::TopEnd
            | AnchorPlacement::BottomEnd
            | AnchorPlacement::LeftEnd
            | AnchorPlacement::RightEnd => Self::End,
        }
    }
}

/// Compose a side and a cross-axis alignment into one [`AnchorPlacement`].
pub const fn anchor_placement(side: AnchorSide, align: AnchorAlign) -> AnchorPlacement {
    match (side, align) {
        (AnchorSide::Top, AnchorAlign::Start) => AnchorPlacement::TopStart,
        (AnchorSide::Top, AnchorAlign::Center) => AnchorPlacement::Top,
        (AnchorSide::Top, AnchorAlign::End) => AnchorPlacement::TopEnd,
        (AnchorSide::Bottom, AnchorAlign::Start) => AnchorPlacement::BottomStart,
        (AnchorSide::Bottom, AnchorAlign::Center) => AnchorPlacement::Bottom,
        (AnchorSide::Bottom, AnchorAlign::End) => AnchorPlacement::BottomEnd,
        (AnchorSide::Left, AnchorAlign::Start) => AnchorPlacement::LeftStart,
        (AnchorSide::Left, AnchorAlign::Center) => AnchorPlacement::Left,
        (AnchorSide::Left, AnchorAlign::End) => AnchorPlacement::LeftEnd,
        (AnchorSide::Right, AnchorAlign::Start) => AnchorPlacement::RightStart,
        (AnchorSide::Right, AnchorAlign::Center) => AnchorPlacement::Right,
        (AnchorSide::Right, AnchorAlign::End) => AnchorPlacement::RightEnd,
    }
}

/// Platform-neutral semantics used to build the native accessibility tree.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AccessibilityRole {
    #[default]
    GenericContainer,
    Label,
    Button,
    Link,
    Image,
    List,
    ListItem,
    Heading,
    CheckBox,
    RadioButton,
    RadioGroup,
    Switch,
    TextInput,
    PasswordInput,
    MultilineTextInput,
    Dialog,
    AlertDialog,
    Menu,
    MenuItem,
    MenuItemCheckBox,
    MenuItemRadio,
    /// A non-interactive visual or structural divider.
    ///
    /// AccessKit names the platform-neutral role `Splitter`; without value-changing actions it
    /// projects as the native separator/divider role exposed by each platform adapter.
    Separator,
    Group,
    Region,
    ListBox,
    ListBoxOption,
    ComboBox,
    EditableComboBox,
    Table,
    Tree,
    Grid,
    Row,
    ColumnHeader,
    RowHeader,
    GridCell,
    TreeItem,
    Tab,
    TabList,
    TabPanel,
    Tooltip,
    Form,
    /// A continuous or stepped range control whose value the user changes directly.
    Slider,
    /// A numeric text control with paired increment and decrement affordances.
    SpinButton,
    /// A determinate or indeterminate task-completion indicator.
    ProgressIndicator,
    /// A static measurement inside a known range, such as disk usage.
    Meter,
    /// A movable divider between two resizable panes.
    ///
    /// AccessKit names the platform-neutral role `Splitter`. Unlike
    /// [`AccessibilityRole::Separator`], this variant is focusable and carries a numeric value.
    SplitterHandle,
    /// A grouping of controls presented as one compact set of application commands.
    Toolbar,
    /// A button with retained pressed state, distinct from a checkbox's checked state.
    ToggleButton,
    /// An in-window horizontal set of menu triggers.
    MenuBar,
    /// An assertive live region for urgent, time-sensitive messages.
    Alert,
    /// A polite live region for advisory status messages.
    Status,
    /// A collection of navigation links presented as one landmark.
    Navigation,
    /// A clipped scroll viewport whose content is larger than its box.
    ScrollView,
    /// A scrollbar whose numeric value is the current scroll position.
    ScrollBar,
}

/// Axis projected for accessibility roles whose behavior changes with orientation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccessibilityOrientation {
    Horizontal,
    Vertical,
}

/// Announcement urgency projected for one live region.
///
/// QuickGUI never polls a live region. The projection is rebuilt only when the application
/// rebuilds the mounted tree, so an unchanged region announces exactly once.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccessibilityLive {
    /// Announce after the current utterance finishes.
    Polite,
    /// Interrupt the current utterance.
    Assertive,
}

/// Numeric range projected by slider, spin-button, progress, meter, and splitter roles.
///
/// Every field is optional so an indeterminate progress indicator can expose bounds without a
/// current value. Non-finite inputs are dropped rather than projected.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AccessibilityValueRange {
    pub value: Option<f64>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub step: Option<f64>,
}

impl AccessibilityValueRange {
    /// Create a determinate range. Non-finite components are omitted from the native projection.
    pub fn new(value: f64, min: f64, max: f64) -> Self {
        Self {
            value: finite_value(value),
            min: finite_value(min),
            max: finite_value(max),
            step: None,
        }
    }

    /// Create a range with no current value, used by indeterminate progress indicators.
    pub fn indeterminate(min: f64, max: f64) -> Self {
        Self {
            value: None,
            min: finite_value(min),
            max: finite_value(max),
            step: None,
        }
    }

    pub fn step(mut self, step: f64) -> Self {
        self.step = finite_value(step).filter(|step| *step > 0.0);
        self
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.value.is_none() && self.min.is_none() && self.max.is_none() && self.step.is_none()
    }
}

fn finite_value(value: f64) -> Option<f64> {
    value.is_finite().then_some(value)
}

/// Kind of popover exposed by a trigger to assistive technology.
///
/// This mirrors the finite ARIA/AccessKit `has-popup` vocabulary. It describes the controlled
/// surface without imposing a visual component implementation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccessibilityPopover {
    Menu,
    ListBox,
    Tree,
    Grid,
    Dialog,
}

/// Sort order exposed by a table or grid column header.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccessibilitySortDirection {
    Ascending,
    Descending,
    Other,
}

/// Suggestion presentation exposed by an editable combobox or text input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccessibilityAutoComplete {
    Inline,
    List,
    Both,
}

/// The controlled checked state exposed by checkbox-like accessibility roles.
///
/// [`ToggleState::Mixed`] corresponds to the web `indeterminate` state and is intended primarily
/// for checkboxes that summarize a partially selected collection.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ToggleState {
    #[default]
    Off,
    On,
    Mixed,
}

impl ToggleState {
    pub const fn is_on(self) -> bool {
        matches!(self, Self::On)
    }

    pub const fn is_mixed(self) -> bool {
        matches!(self, Self::Mixed)
    }
}

impl From<bool> for ToggleState {
    fn from(checked: bool) -> Self {
        if checked { Self::On } else { Self::Off }
    }
}

/// CSS-like policy for selecting immutable text with the pointer.
///
/// [`UserSelect::Auto`] keeps ordinary text selectable while inheriting suppression from controls
/// such as buttons and drag sources. [`UserSelect::Text`] explicitly re-enables selection and
/// [`UserSelect::None`] disables it for the complete subtree.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum UserSelect {
    #[default]
    Auto,
    Text,
    None,
}

/// Whether an element subtree is painted while retaining its layout box.
///
/// This mirrors GPUI's visibility model: [`Visibility::Hidden`] suppresses the element and its
/// descendants from paint, input, focus, and accessibility, but their layout still participates.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Visibility {
    #[default]
    Visible,
    Hidden,
}

/// Converts common values into an [`Element`] for `.child(...)` and `.children(...)`.
pub trait IntoElement {
    fn into_element(self) -> Element;
}

impl IntoElement for Element {
    fn into_element(self) -> Element {
        self
    }
}

impl IntoElement for String {
    fn into_element(self) -> Element {
        text(self)
    }
}

impl IntoElement for Arc<str> {
    fn into_element(self) -> Element {
        text(self)
    }
}

impl IntoElement for &str {
    fn into_element(self) -> Element {
        text(Arc::<str>::from(self))
    }
}

impl IntoElement for Cow<'_, str> {
    fn into_element(self) -> Element {
        text(Arc::<str>::from(self.as_ref()))
    }
}

impl IntoElement for StyledText {
    fn into_element(self) -> Element {
        Element::styled_text(self)
    }
}

#[derive(Clone, Debug)]
pub(crate) enum ElementKind {
    Container,
    ContainerQuery(ContainerQueryElement),
    Text(Arc<str>),
    StyledText(StyledText),
    Image(ImageElement),
    Svg(SvgElement),
    Path(PathElement),
    Canvas(CanvasElement),
    CustomShader(ShaderElement),
    TextInput(TextInputElement),
    #[cfg(target_os = "macos")]
    NativeView(MacNativeView),
}

#[derive(Clone)]
pub(crate) struct ContainerQueryElement {
    render: Rc<dyn Fn(crate::Size) -> Element>,
    pub(crate) resolved_size: Option<crate::Size>,
    /// Declarative motion mounted directly by this callback. Descendant query callbacks retain
    /// their own IDs, which lets a size change unmount exactly the replaced subtree without
    /// disturbing motion elsewhere in the window.
    pub(crate) resolved_motion_ids: Vec<ElementId>,
    pub(crate) layout_pending: bool,
}

impl ContainerQueryElement {
    pub(crate) fn render(&self, size: crate::Size) -> Element {
        (self.render)(size)
    }
}

impl fmt::Debug for ContainerQueryElement {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ContainerQueryElement")
            .field("resolved_size", &self.resolved_size)
            .field("resolved_motion_count", &self.resolved_motion_ids.len())
            .field("layout_pending", &self.layout_pending)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ImageElement {
    pub source: ImageSource,
    pub object_fit: ObjectFit,
    pub resolved: ImageResolution,
    pub loading: Option<ImageReplacement>,
    pub fallback: Option<ImageReplacement>,
}

#[derive(Clone, Debug)]
pub(crate) struct SvgElement {
    pub svg: Svg,
    pub object_fit: ObjectFit,
    pub transform: SvgTransform,
}

#[derive(Clone, Debug)]
pub(crate) struct PathElement {
    pub path: Path,
    pub object_fit: ObjectFit,
    pub background: Option<Background>,
}

#[derive(Clone)]
pub(crate) struct CanvasElement {
    pub painter: Rc<CanvasPainter>,
}

#[derive(Clone, Debug)]
pub(crate) struct ShaderElement {
    pub shader: CustomShader,
    pub parameters: ShaderParameters,
}

type CanvasPainter = dyn for<'a> Fn(Rect, &mut Canvas<'a>);

impl fmt::Debug for CanvasElement {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("CanvasElement(..)")
    }
}

#[derive(Clone, Debug)]
pub(crate) enum ImageResolution {
    Ready(Image),
    Animated(AnimatedImage),
    Loading,
    Failed,
}

#[derive(Clone)]
pub(crate) struct ImageReplacement(Rc<dyn Fn() -> Element>);

impl ImageReplacement {
    fn new<E: IntoElement + 'static>(render: impl Fn() -> E + 'static) -> Self {
        Self(Rc::new(move || render().into_element()))
    }

    pub(crate) fn render(&self) -> Element {
        (self.0)()
    }
}

impl fmt::Debug for ImageReplacement {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ImageReplacement(..)")
    }
}

#[derive(Clone, Debug)]
pub(crate) struct TextInputElement {
    pub presentation: InputPresentation,
    pub value: Arc<str>,
    pub highlights: Arc<[TextHighlight]>,
    pub placeholder: Arc<str>,
    pub multiline: bool,
    pub password: bool,
    pub submit_on_enter: bool,
    pub constraints: InputConstraints,
}

/// Optional editor geometry and paint overrides, shared by painting and hit testing.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct InputPresentation {
    pub content_insets: Option<Insets>,
    pub caret_width: Option<f32>,
    pub caret_height_em: Option<f32>,
    pub caret_color: Option<Color>,
    pub placeholder_color: Option<Color>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ScrollRequest {
    pub revision: u64,
    pub offset: crate::Vector,
    pub child: Option<usize>,
}

pub(crate) type InputFilterCallback = Arc<dyn Fn(&str) -> bool>;

#[derive(Clone, Default)]
pub(crate) struct InputConstraints {
    pub read_only: bool,
    pub max_length: Option<usize>,
    pub filter: Option<InputFilterCallback>,
    /// Per-input text checking overrides layered over the application policy.
    pub text_checking: TextCheckingOverrides,
}

impl fmt::Debug for InputConstraints {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("InputConstraints")
            .field("max_length", &self.max_length)
            .field("filter", &self.filter.as_ref().map(|_| "InputFilter(..)"))
            .field("text_checking", &self.text_checking)
            .finish()
    }
}

/// How a background image is scaled inside its element box.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum BackgroundSize {
    /// Use the decoded pixel size as the logical tile size.
    #[default]
    Auto,
    /// Scale preserving aspect ratio until the tile covers the whole box.
    Cover,
    /// Scale preserving aspect ratio until the tile fits inside the box.
    Contain,
    /// An explicit logical tile size.
    Fixed(f32, f32),
}

/// Which axes a background image tiles along.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum BackgroundRepeat {
    /// One tile only.
    #[default]
    NoRepeat,
    RepeatX,
    RepeatY,
    Repeat,
}

impl BackgroundRepeat {
    pub(crate) fn repeats_x(self) -> bool {
        matches!(self, Self::RepeatX | Self::Repeat)
    }

    pub(crate) fn repeats_y(self) -> bool {
        matches!(self, Self::RepeatY | Self::Repeat)
    }
}

/// Where a background tile is anchored, as a fraction of the free space in the element box.
///
/// `0.0` aligns with the start edge, `0.5` centers, and `1.0` aligns with the end edge. Values
/// outside `0.0..=1.0` are clamped so a background image can never escape its own tiling grid.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BackgroundPosition {
    pub x: f32,
    pub y: f32,
}

impl Default for BackgroundPosition {
    fn default() -> Self {
        Self::CENTER
    }
}

impl BackgroundPosition {
    pub const TOP_LEFT: Self = Self { x: 0.0, y: 0.0 };
    pub const TOP: Self = Self { x: 0.5, y: 0.0 };
    pub const TOP_RIGHT: Self = Self { x: 1.0, y: 0.0 };
    pub const LEFT: Self = Self { x: 0.0, y: 0.5 };
    pub const CENTER: Self = Self { x: 0.5, y: 0.5 };
    pub const RIGHT: Self = Self { x: 1.0, y: 0.5 };
    pub const BOTTOM_LEFT: Self = Self { x: 0.0, y: 1.0 };
    pub const BOTTOM: Self = Self { x: 0.5, y: 1.0 };
    pub const BOTTOM_RIGHT: Self = Self { x: 1.0, y: 1.0 };

    pub fn new(x: f32, y: f32) -> Self {
        Self {
            x: if x.is_finite() {
                x.clamp(0.0, 1.0)
            } else {
                0.5
            },
            y: if y.is_finite() {
                y.clamp(0.0, 1.0)
            } else {
                0.5
            },
        }
    }
}

/// A raster background painted behind an element's children and inside its rounded corners.
#[derive(Clone, Debug, PartialEq)]
pub struct BackgroundImage {
    pub image: Image,
    pub size: BackgroundSize,
    pub repeat: BackgroundRepeat,
    pub position: BackgroundPosition,
}

impl BackgroundImage {
    pub fn new(image: Image) -> Self {
        Self {
            image,
            size: BackgroundSize::Auto,
            repeat: BackgroundRepeat::NoRepeat,
            position: BackgroundPosition::CENTER,
        }
    }

    /// The logical tile size for `bounds`, or `None` when nothing can be painted.
    pub(crate) fn tile_size(&self, bounds: Rect) -> Option<crate::Size> {
        let intrinsic = self.image.size();
        if intrinsic.width <= 0.0 || intrinsic.height <= 0.0 {
            return None;
        }
        let size = match self.size {
            BackgroundSize::Auto => intrinsic,
            BackgroundSize::Fixed(width, height) => crate::Size {
                width: if width.is_finite() {
                    width.max(0.0)
                } else {
                    0.0
                },
                height: if height.is_finite() {
                    height.max(0.0)
                } else {
                    0.0
                },
            },
            BackgroundSize::Cover | BackgroundSize::Contain => {
                if bounds.width <= 0.0 || bounds.height <= 0.0 {
                    return None;
                }
                let horizontal = bounds.width / intrinsic.width;
                let vertical = bounds.height / intrinsic.height;
                let scale = if matches!(self.size, BackgroundSize::Cover) {
                    horizontal.max(vertical)
                } else {
                    horizontal.min(vertical)
                };
                crate::Size {
                    width: intrinsic.width * scale,
                    height: intrinsic.height * scale,
                }
            }
        };
        (size.width > 0.0 && size.height > 0.0 && size.width.is_finite() && size.height.is_finite())
            .then_some(size)
    }
}

/// A ring painted outside the border box without participating in layout.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Outline {
    pub width: f32,
    pub color: Color,
    /// Gap between the border box and the inner edge of the ring. May be negative.
    pub offset: f32,
    pub style: BorderStyle,
}

impl Outline {
    /// A solid outline with no offset.
    pub fn new(width: f32, color: Color) -> Self {
        Self {
            width: finite_nonnegative(width).min(MAX_OUTLINE_WIDTH),
            color,
            offset: 0.0,
            style: BorderStyle::Solid,
        }
    }

    pub fn offset(mut self, offset: f32) -> Self {
        self.offset = if offset.is_finite() {
            offset.clamp(-MAX_OUTLINE_OFFSET, MAX_OUTLINE_OFFSET)
        } else {
            0.0
        };
        self
    }

    pub fn style(mut self, style: BorderStyle) -> Self {
        self.style = style;
        self
    }

    /// The ring rectangle for a border box, or `None` when nothing is painted.
    pub(crate) fn ring(self, bounds: Rect) -> Option<Rect> {
        if self.width <= 0.0 || self.color.a <= 0.0 {
            return None;
        }
        let grow = self.offset + self.width;
        let left = bounds.x - grow;
        let top = bounds.y - grow;
        let width = bounds.width + grow * 2.0;
        let height = bounds.height + grow * 2.0;
        if width <= 0.0 || height <= 0.0 {
            return None;
        }
        Some(Rect::new(left, top, width, height))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct VisualStyle {
    pub background: Option<Color>,
    /// A bounded multi-stop gradient painted instead of `background`.
    pub background_gradient: Option<Gradient>,
    /// A raster background painted above the background color and behind children.
    pub background_image: Option<Box<BackgroundImage>>,
    /// A bounded color-filter chain applied to this element's own raster content.
    pub filters: Filters,
    pub border_color: Option<Color>,
    pub border_widths: Insets,
    pub border_style: BorderStyle,
    pub radius: f32,
    /// Per-corner radii. When present they replace `radius` and are not transitioned.
    pub corner_radii: Option<Corners>,
    pub outline: Option<Outline>,
    pub shadows: Option<Arc<[BoxShadow]>>,
    pub opacity: f32,
    /// A subtree transform. Anything but a whole-pixel translation opens a compositing group.
    pub transform: Transform2D,
    /// Where the transform acts, as a fraction of the element's border box.
    pub transform_origin: Point,
    /// Color filters applied to what is already painted behind this element.
    pub backdrop: Filters,
    /// How this element's subtree combines with what is already painted behind it.
    pub blend: BlendMode,
}

impl VisualStyle {
    /// The resolved corner radii, preferring explicit per-corner values.
    pub(crate) fn corners(&self, radius: f32) -> Corners {
        self.corner_radii.unwrap_or(Corners::all(radius))
    }
}

impl Default for VisualStyle {
    fn default() -> Self {
        Self {
            background: None,
            background_gradient: None,
            background_image: None,
            filters: Filters::none(),
            border_color: None,
            border_widths: Insets::default(),
            border_style: BorderStyle::Solid,
            radius: 0.0,
            corner_radii: None,
            outline: None,
            shadows: None,
            opacity: 1.0,
            transform: Transform2D::IDENTITY,
            transform_origin: Point::new(0.5, 0.5),
            backdrop: Filters::none(),
            blend: BlendMode::Normal,
        }
    }
}

/// Non-layout overrides for hover, pressed, focus, validation, and drag states.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ElementStateStyle {
    pub(crate) background: Option<Color>,
    pub(crate) background_gradient: Option<Gradient>,
    pub(crate) outline: Option<Outline>,
    pub(crate) border_color: Option<Color>,
    pub(crate) border_width: Option<f32>,
    pub(crate) radius: Option<f32>,
    pub(crate) text_color: Option<Color>,
    pub(crate) shadows: Option<Arc<[BoxShadow]>>,
    pub(crate) opacity: Option<f32>,
    pub(crate) cursor_style: Option<CursorStyle>,
    pub(crate) transform: Option<Transform2D>,
    pub(crate) transform_origin: Option<Point>,
}

impl ElementStateStyle {
    /// Set the native cursor while this state is active.
    pub fn cursor(mut self, cursor: CursorStyle) -> Self {
        self.cursor_style = Some(cursor);
        self
    }

    state_cursor_helper!(cursor_default, Arrow, "default");
    state_cursor_helper!(cursor_pointer, PointingHand, "pointer");
    state_cursor_helper!(cursor_text, IBeam, "text");
    state_cursor_helper!(cursor_move, ClosedHand, "move");
    state_cursor_helper!(cursor_not_allowed, OperationNotAllowed, "not-allowed");
    state_cursor_helper!(cursor_context_menu, ContextualMenu, "context-menu");
    state_cursor_helper!(cursor_crosshair, Crosshair, "crosshair");
    state_cursor_helper!(
        cursor_vertical_text,
        IBeamCursorForVerticalLayout,
        "vertical-text"
    );
    state_cursor_helper!(cursor_alias, DragLink, "alias");
    state_cursor_helper!(cursor_copy, DragCopy, "copy");
    state_cursor_helper!(cursor_no_drop, OperationNotAllowed, "no-drop");
    state_cursor_helper!(cursor_grab, OpenHand, "grab");
    state_cursor_helper!(cursor_grabbing, ClosedHand, "grabbing");
    state_cursor_helper!(cursor_ew_resize, ResizeLeftRight, "ew-resize");
    state_cursor_helper!(cursor_ns_resize, ResizeUpDown, "ns-resize");
    state_cursor_helper!(cursor_nesw_resize, ResizeUpRightDownLeft, "nesw-resize");
    state_cursor_helper!(cursor_nwse_resize, ResizeUpLeftDownRight, "nwse-resize");
    state_cursor_helper!(cursor_col_resize, ResizeColumn, "col-resize");
    state_cursor_helper!(cursor_row_resize, ResizeRow, "row-resize");
    state_cursor_helper!(cursor_n_resize, ResizeUp, "n-resize");
    state_cursor_helper!(cursor_e_resize, ResizeRight, "e-resize");
    state_cursor_helper!(cursor_s_resize, ResizeDown, "s-resize");
    state_cursor_helper!(cursor_w_resize, ResizeLeft, "w-resize");

    pub fn bg(mut self, color: Color) -> Self {
        self.background = Some(color);
        self.background_gradient = None;
        self
    }

    /// Transform the subtree while this state is active.
    ///
    /// Layout never moves; only paint and hit testing do, so a hover lift or press shrink costs
    /// no relayout. A transform in a state overrides the element's own
    /// [`Element::transform`](crate::Element::transform).
    pub fn transform(mut self, transform: Transform2D) -> Self {
        self.transform = Some(transform);
        self
    }

    /// Move the point this state's transform acts around, as a fraction of the border box.
    pub fn transform_origin(mut self, x: f32, y: f32) -> Self {
        self.transform_origin = Some(Point::new(x, y));
        self
    }

    /// Translate the subtree by logical pixels while this state is active.
    pub fn translate(self, x: f32, y: f32) -> Self {
        self.transform(Transform2D::translate(x, y))
    }

    /// Rotate the subtree around its transform origin while this state is active.
    pub fn rotate_degrees(self, degrees: f32) -> Self {
        self.transform(Transform2D::rotate_degrees(degrees))
    }

    /// Scale the subtree around its transform origin while this state is active.
    pub fn scale(self, x: f32, y: f32) -> Self {
        self.transform(Transform2D::scale(x, y))
    }

    /// Scale the subtree uniformly around its transform origin while this state is active.
    pub fn scale_uniform(self, scale: f32) -> Self {
        self.transform(Transform2D::scale_uniform(scale))
    }

    /// Replace the background with a bounded multi-stop gradient while this state is active.
    ///
    /// Gradients are swapped, not interpolated: only the transitionable solid background,
    /// border, radius, shadow, opacity, and text color values animate.
    pub fn bg_gradient(mut self, gradient: impl Into<Background>) -> Self {
        match gradient.into() {
            Background::Solid(color) => {
                self.background = Some(color);
                self.background_gradient = None;
            }
            other => {
                self.background_gradient = other.as_gradient();
            }
        }
        self
    }

    /// Paint an outline ring outside the border box while this state is active.
    pub fn outline(mut self, width: f32, color: Color) -> Self {
        self.outline = Some(Outline::new(width, color));
        self
    }

    /// Paint an offset outline ring outside the border box while this state is active.
    pub fn outline_offset(mut self, width: f32, color: Color, offset: f32) -> Self {
        self.outline = Some(Outline::new(width, color).offset(offset));
        self
    }

    /// Remove any inherited outline while this state is active.
    pub fn outline_none(mut self) -> Self {
        self.outline = Some(Outline::new(0.0, Color::TRANSPARENT));
        self
    }

    /// Paint this state's outline as evenly spaced dashes.
    pub fn outline_dashed(self) -> Self {
        self.outline_style(BorderStyle::Dashed)
    }

    /// Paint this state's outline as evenly distributed dots.
    pub fn outline_dotted(self) -> Self {
        self.outline_style(BorderStyle::Dotted)
    }

    fn outline_style(mut self, style: BorderStyle) -> Self {
        let outline = self
            .outline
            .unwrap_or_else(|| Outline::new(0.0, Color::TRANSPARENT));
        self.outline = Some(outline.style(style));
        self
    }

    pub fn border_color(mut self, color: Color) -> Self {
        self.border_color = Some(color);
        self
    }

    /// Override the paint-only border width while this state is active, keeping the border color.
    pub fn border_width(mut self, width: f32) -> Self {
        self.border_width = Some(finite_nonnegative(width));
        self
    }

    /// Paint an inside border without changing layout.
    pub fn border(mut self, width: f32, color: Color) -> Self {
        self.border_width = Some(width.max(0.0));
        self.border_color = Some(color);
        self
    }

    /// Override the paint-only corner radius while this state is active.
    pub fn rounded(mut self, radius: f32) -> Self {
        self.radius = Some(finite_nonnegative(radius));
        self
    }

    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = Some(color);
        self
    }

    /// Set the opacity of this element and its descendants while this state is active.
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = Some(finite_opacity(opacity));
        self
    }

    /// Replace the element's shadows while this state is active.
    pub fn shadow(mut self, shadow: BoxShadow) -> Self {
        self.shadows = Some(Arc::from([shadow]));
        self
    }

    /// Replace the element's shadows while this state is active.
    pub fn shadows(mut self, shadows: impl IntoIterator<Item = BoxShadow>) -> Self {
        self.shadows = Some(collect_box_shadows(shadows));
        self
    }

    /// Remove all shadows while this state is active.
    pub fn shadow_none(mut self) -> Self {
        self.shadows = Some(Arc::from([]));
        self
    }

    pub fn shadow_sm(self) -> Self {
        self.shadow(shadow_sm_preset())
    }

    pub fn shadow_md(self) -> Self {
        self.shadows(shadow_md_preset())
    }

    pub fn shadow_lg(self) -> Self {
        self.shadows(shadow_lg_preset())
    }

    pub fn shadow_xl(self) -> Self {
        self.shadows(shadow_xl_preset())
    }

    pub fn shadow_2xl(self) -> Self {
        self.shadow(shadow_2xl_preset())
    }

    /// Lay `other` over this style: every value it declares replaces the one here.
    pub(crate) fn overlay(&mut self, other: &Self) {
        if other.background.is_some() {
            self.background = other.background;
        }
        if other.background_gradient.is_some() {
            self.background_gradient = other.background_gradient;
        }
        if other.outline.is_some() {
            self.outline = other.outline;
        }
        if other.border_color.is_some() {
            self.border_color = other.border_color;
        }
        if other.border_width.is_some() {
            self.border_width = other.border_width;
        }
        if other.radius.is_some() {
            self.radius = other.radius;
        }
        if other.text_color.is_some() {
            self.text_color = other.text_color;
        }
        if other.shadows.is_some() {
            self.shadows = other.shadows.clone();
        }
        if other.opacity.is_some() {
            self.opacity = other.opacity;
        }
        if other.cursor_style.is_some() {
            self.cursor_style = other.cursor_style;
        }
        if other.transform.is_some() {
            self.transform = other.transform;
        }
        if other.transform_origin.is_some() {
            self.transform_origin = other.transform_origin;
        }
    }

    fn has_paint_overrides(&self) -> bool {
        self.background.is_some()
            || self.background_gradient.is_some()
            || self.outline.is_some()
            || self.border_color.is_some()
            || self.border_width.is_some()
            || self.radius.is_some()
            || self.text_color.is_some()
            || self.shadows.is_some()
            || self.opacity.is_some()
    }
}

/// Which state of an ancestor group a member's style follows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GroupState {
    /// The group is hovered.
    Hover,
    /// A press inside the group is held.
    Active,
}

/// One paint-only style a member paints while an ancestor group is in one state.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GroupStateStyle {
    pub(crate) state: GroupState,
    /// The named group to follow; `None` follows the nearest group.
    pub(crate) target: Option<Arc<str>>,
    pub(crate) style: ElementStateStyle,
}

fn shadow_sm_preset() -> BoxShadow {
    BoxShadow::new(0.0, 1.0, Color::rgba8(0, 0, 0, 13)).blur_radius(2.0)
}

fn shadow_md_preset() -> [BoxShadow; 2] {
    [
        BoxShadow::new(0.0, 4.0, Color::rgba8(0, 0, 0, 26))
            .blur_radius(6.0)
            .spread_radius(-1.0),
        BoxShadow::new(0.0, 2.0, Color::rgba8(0, 0, 0, 26))
            .blur_radius(4.0)
            .spread_radius(-2.0),
    ]
}

fn shadow_lg_preset() -> [BoxShadow; 2] {
    [
        BoxShadow::new(0.0, 10.0, Color::rgba8(0, 0, 0, 26))
            .blur_radius(15.0)
            .spread_radius(-3.0),
        BoxShadow::new(0.0, 4.0, Color::rgba8(0, 0, 0, 26))
            .blur_radius(6.0)
            .spread_radius(-4.0),
    ]
}

fn shadow_xl_preset() -> [BoxShadow; 2] {
    [
        BoxShadow::new(0.0, 20.0, Color::rgba8(0, 0, 0, 26))
            .blur_radius(25.0)
            .spread_radius(-5.0),
        BoxShadow::new(0.0, 8.0, Color::rgba8(0, 0, 0, 26))
            .blur_radius(10.0)
            .spread_radius(-6.0),
    ]
}

fn shadow_2xl_preset() -> BoxShadow {
    BoxShadow::new(0.0, 25.0, Color::rgba8(0, 0, 0, 64))
        .blur_radius(50.0)
        .spread_radius(-12.0)
}

fn collect_box_shadows(shadows: impl IntoIterator<Item = BoxShadow>) -> Arc<[BoxShadow]> {
    let mut retained = Vec::with_capacity(2);
    for shadow in shadows {
        assert!(
            retained.len() < MAX_BOX_SHADOWS_PER_ELEMENT,
            "one element retains at most {MAX_BOX_SHADOWS_PER_ELEMENT} box shadows"
        );
        retained.push(shadow);
    }
    Arc::from(retained)
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct AccessibilityStyle {
    pub hidden: bool,
    pub role: AccessibilityRole,
    pub label: Option<Arc<str>>,
    pub value: Option<Arc<str>>,
    pub disabled: bool,
    pub selected: bool,
    pub toggled: Option<ToggleState>,
    pub expanded: Option<bool>,
    pub relations: AccessibilityRelationsStyle,
    pub has_popover: Option<AccessibilityPopover>,
    pub auto_complete: Option<AccessibilityAutoComplete>,
    pub collection: AccessibilityCollectionStyle,
    pub modal: bool,
    pub required: bool,
    pub read_only: bool,
    pub invalid: bool,
    pub validation_message: Option<Arc<str>>,
    pub validation_message_truncated: bool,
    pub description: Option<Arc<str>>,
    pub orientation: Option<AccessibilityOrientation>,
    pub live: Option<AccessibilityLive>,
    pub value_range: Option<Box<AccessibilityValueRange>>,
    pub multiselectable: bool,
}

/// Retained keyboard policy for an unstyled tab list.
///
/// This is present only on [`AccessibilityRole::TabList`] elements. It is scanned directly from
/// the mounted tree on explicit keyboard input and retains no item registry or scheduler source.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TabListBehavior {
    pub(crate) vertical: bool,
    pub(crate) activate_on_focus: bool,
    pub(crate) loop_focus: bool,
}

/// Exact opt-in relationships stored out of line.
///
/// Most retained elements declare no relationship, so they pay one nullable pointer instead of
/// four discriminated `ElementId` words. Relationship-bearing controls allocate one fixed record
/// during declaration and can still represent every `u64` identity without sentinels.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct AccessibilityRelationsStyle(Option<Box<AccessibilityRelations>>);

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct AccessibilityRelations {
    controls: Option<ElementId>,
    active_descendant: Option<ElementId>,
    labelled_by: Option<ElementId>,
    described_by: Option<ElementId>,
    described_by_secondary: Option<ElementId>,
}

impl AccessibilityRelationsStyle {
    fn values_mut(&mut self) -> &mut AccessibilityRelations {
        self.0
            .get_or_insert_with(|| Box::new(AccessibilityRelations::default()))
    }

    pub(crate) fn controls(&self) -> Option<ElementId> {
        self.0.as_deref().and_then(|values| values.controls)
    }

    pub(crate) fn set_controls(&mut self, value: ElementId) {
        self.values_mut().controls = Some(value);
    }

    pub(crate) fn active_descendant(&self) -> Option<ElementId> {
        self.0
            .as_deref()
            .and_then(|values| values.active_descendant)
    }

    pub(crate) fn set_active_descendant(&mut self, value: ElementId) {
        self.values_mut().active_descendant = Some(value);
    }

    pub(crate) fn labelled_by(&self) -> Option<ElementId> {
        self.0.as_deref().and_then(|values| values.labelled_by)
    }

    pub(crate) fn set_labelled_by(&mut self, value: ElementId) {
        self.values_mut().labelled_by = Some(value);
    }

    pub(crate) fn described_by(&self) -> Option<ElementId> {
        self.0.as_deref().and_then(|values| values.described_by)
    }

    pub(crate) fn set_described_by(&mut self, value: ElementId) {
        let values = self.values_mut();
        values.described_by = Some(value);
        values.described_by_secondary = None;
    }

    pub(crate) fn described_by_secondary(&self) -> Option<ElementId> {
        self.0
            .as_deref()
            .and_then(|values| values.described_by_secondary)
    }

    pub(crate) fn set_described_by_pair(&mut self, first: ElementId, second: ElementId) {
        let values = self.values_mut();
        values.described_by = Some(first);
        values.described_by_secondary = (second != first).then_some(second);
    }
}

const ACCESSIBILITY_COLLECTION_UNSET: u32 = u32::MAX;

/// Compact inline storage for collection metadata.
///
/// A separate `Option<usize>` for every property would add more than one hundred bytes to every
/// retained element, including ordinary text. The reserved sentinel keeps the complete table/tree
/// vocabulary allocation-free in 32 bytes while still covering QuickGUI's much smaller hard
/// collection bounds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct AccessibilityCollectionStyle {
    pub row_count: u32,
    pub column_count: u32,
    pub row_index: u32,
    pub column_index: u32,
    pub level: u32,
    pub size_of_set: u32,
    pub position_in_set: u32,
    pub sort_direction: Option<AccessibilitySortDirection>,
}

impl Default for AccessibilityCollectionStyle {
    fn default() -> Self {
        Self {
            row_count: ACCESSIBILITY_COLLECTION_UNSET,
            column_count: ACCESSIBILITY_COLLECTION_UNSET,
            row_index: ACCESSIBILITY_COLLECTION_UNSET,
            column_index: ACCESSIBILITY_COLLECTION_UNSET,
            level: ACCESSIBILITY_COLLECTION_UNSET,
            size_of_set: ACCESSIBILITY_COLLECTION_UNSET,
            position_in_set: ACCESSIBILITY_COLLECTION_UNSET,
            sort_direction: None,
        }
    }
}

impl AccessibilityCollectionStyle {
    pub(crate) fn row_count(self) -> Option<usize> {
        accessibility_collection_value(self.row_count)
    }

    pub(crate) fn column_count(self) -> Option<usize> {
        accessibility_collection_value(self.column_count)
    }

    pub(crate) fn row_index(self) -> Option<usize> {
        accessibility_collection_value(self.row_index)
    }

    pub(crate) fn column_index(self) -> Option<usize> {
        accessibility_collection_value(self.column_index)
    }

    pub(crate) fn level(self) -> Option<usize> {
        accessibility_collection_value(self.level)
    }

    pub(crate) fn size_of_set(self) -> Option<usize> {
        accessibility_collection_value(self.size_of_set)
    }

    pub(crate) fn position_in_set(self) -> Option<usize> {
        accessibility_collection_value(self.position_in_set)
    }
}

fn accessibility_collection_storage(value: usize) -> u32 {
    assert!(
        value < ACCESSIBILITY_COLLECTION_UNSET as usize,
        "accessibility collection values must fit below u32::MAX"
    );
    value as u32
}

fn accessibility_collection_value(value: u32) -> Option<usize> {
    (value != ACCESSIBILITY_COLLECTION_UNSET).then_some(value as usize)
}

#[derive(Clone, Debug)]
pub(crate) struct VirtualScrollStyle {
    pub handle: VirtualScrollHandle,
    pub max_offset_y: f32,
    pub measurement_revision: u64,
    pub mount: VirtualScrollMount,
}

pub(crate) type DropPredicateCallback = Arc<dyn Fn(&dyn Any) -> bool>;

#[derive(Clone)]
pub(crate) struct DropPredicate {
    pub type_id: TypeId,
    pub callback: DropPredicateCallback,
}

impl fmt::Debug for DropPredicate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DropPredicate")
            .field("type_id", &self.type_id)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct TypographyStyle {
    pub color: Option<Color>,
    pub font_size: Option<f32>,
    pub line_height: Option<f32>,
    pub monospace_width: Option<f32>,
    pub family: Option<FontFamily>,
    pub features: Option<FontFeatures>,
    pub fallbacks: Option<Option<FontFallbacks>>,
    pub weight: Option<Weight>,
    pub font_style: Option<GlyphStyle>,
    pub font_thicken: Option<bool>,
    pub underline: Option<TextUnderline>,
    pub underline_color: Option<Option<Color>>,
    pub underline_wavy: Option<bool>,
    pub underline_thickness: Option<f32>,
    pub strikethrough: Option<bool>,
    pub strikethrough_color: Option<Option<Color>>,
    pub overline: Option<bool>,
    pub overline_color: Option<Option<Color>>,
    pub direction: Option<TextDirection>,
    pub letter_spacing: Option<f32>,
    pub word_spacing: Option<f32>,
    pub transform: Option<Option<TextTransform>>,
    pub word_break: Option<WordBreak>,
    pub overflow_wrap: Option<OverflowWrap>,
    pub hyphens: Option<Hyphens>,
    pub shadow: Option<Option<TextShadow>>,
    pub align: Option<TextAlign>,
    pub wrap: Option<TextWrap>,
    pub text_overflow: Option<TextOverflow>,
    pub line_clamp: Option<usize>,
    pub shaping: Option<TextShaping>,
}

impl TypographyStyle {
    pub(crate) fn resolve(&self, inherited: &TextStyle) -> TextStyle {
        let font_size = self.font_size.unwrap_or(inherited.font_size);
        TextStyle {
            font_size,
            line_height: self.line_height.unwrap_or_else(|| {
                self.font_size
                    .map(|_| font_size * 1.35)
                    .unwrap_or(inherited.line_height)
            }),
            monospace_width: self.monospace_width.or(inherited.monospace_width),
            family: self
                .family
                .clone()
                .unwrap_or_else(|| inherited.family.clone()),
            features: self
                .features
                .clone()
                .unwrap_or_else(|| inherited.features.clone()),
            fallbacks: normalize_fallbacks(
                self.fallbacks
                    .clone()
                    .unwrap_or_else(|| inherited.fallbacks.clone()),
            ),
            weight: self.weight.unwrap_or(inherited.weight),
            font_style: self.font_style.unwrap_or(inherited.font_style),
            font_thicken: self.font_thicken.unwrap_or(inherited.font_thicken),
            underline: self.underline.unwrap_or(inherited.underline),
            underline_color: self.underline_color.unwrap_or(inherited.underline_color),
            underline_wavy: self.underline_wavy.unwrap_or(inherited.underline_wavy),
            underline_thickness: self
                .underline_thickness
                .unwrap_or(inherited.underline_thickness),
            strikethrough: self.strikethrough.unwrap_or(inherited.strikethrough),
            strikethrough_color: self
                .strikethrough_color
                .unwrap_or(inherited.strikethrough_color),
            overline: self.overline.unwrap_or(inherited.overline),
            overline_color: self.overline_color.unwrap_or(inherited.overline_color),
            direction: self.direction.unwrap_or(inherited.direction),
            letter_spacing: self.letter_spacing.unwrap_or(inherited.letter_spacing),
            word_spacing: self.word_spacing.unwrap_or(inherited.word_spacing),
            transform: self.transform.unwrap_or(inherited.transform),
            word_break: self.word_break.unwrap_or(inherited.word_break),
            overflow_wrap: self.overflow_wrap.unwrap_or(inherited.overflow_wrap),
            hyphens: self.hyphens.unwrap_or(inherited.hyphens),
            shadow: self.shadow.unwrap_or(inherited.shadow),
            align: self.align.unwrap_or(inherited.align),
            wrap: self.wrap.unwrap_or(inherited.wrap),
            text_overflow: self
                .text_overflow
                .clone()
                .or_else(|| inherited.text_overflow.clone()),
            line_clamp: self.line_clamp.or(inherited.line_clamp),
            shaping: self.shaping.unwrap_or(inherited.shaping),
            color: self.color.unwrap_or(inherited.color),
        }
    }
}

/// A declarative UI node with Tailwind-like fluent styling.
///
/// See [`ElementUpdate`] for targeted updates from an embedding runtime that already owns a
/// retained declaration.
#[derive(Clone, Debug)]
pub struct Element {
    pub(crate) explicit_id: Option<ElementId>,
    pub(crate) runtime_id: ElementId,
    pub(crate) kind: ElementKind,
    pub(crate) layout: Style,
    pub(crate) visibility: Visibility,
    pub(crate) visual: VisualStyle,
    pub(crate) typography: TypographyStyle,
    pub(crate) resolved_typography: TextStyle,
    pub(crate) hover: ElementStateStyle,
    pub(crate) active: ElementStateStyle,
    pub(crate) focus: ElementStateStyle,
    pub(crate) disabled_style: ElementStateStyle,
    pub(crate) invalid_style: ElementStateStyle,
    /// Paint while this element is marked selected, like a chosen row of a native list.
    pub(crate) selected_style: ElementStateStyle,
    pub(crate) dragging: ElementStateStyle,
    pub(crate) drag_over: ElementStateStyle,
    /// Paint while this element or a descendant owns keyboard focus, like CSS `:focus-within`.
    pub(crate) focus_within: ElementStateStyle,
    /// Styles following the state of an ancestor group, in declaration order; later ones win.
    pub(crate) group_styles: Vec<GroupStateStyle>,
    /// Whether descendants' `group_hover` and `group_active` styles follow this element.
    pub(crate) group: bool,
    /// The name descendants can target this group by.
    pub(crate) group_name: Option<Arc<str>>,
    pub(crate) clickable: bool,
    pub(crate) pointer_listener: bool,
    pub(crate) scroll_wheel_listener: bool,
    pub(crate) touch_listener: bool,
    pub(crate) context_menu_listener: bool,
    // This extra indirection is intentional: listeners are uncommon, and keeping the
    // non-interactive Element representation compact matters more than one opt-in allocation.
    #[allow(clippy::box_collection)]
    pub(crate) mouse_listeners: Option<Box<Vec<MouseListenerBinding>>>,
    #[allow(clippy::box_collection)]
    pub(crate) key_listeners: Option<Box<Vec<KeyListenerBinding>>>,
    #[allow(clippy::box_collection)]
    pub(crate) action_listeners: Option<Box<Vec<ActionListenerBinding>>>,
    pub(crate) mouse_pressure_listener: bool,
    pub(crate) pinch_listener: bool,
    pub(crate) rotation_listener: bool,
    pub(crate) smart_magnify_listener: bool,
    pub(crate) drag_source: bool,
    pub(crate) drop_target: bool,
    pub(crate) drop_predicates: Vec<DropPredicate>,
    pub(crate) cursor_style: Option<CursorStyle>,
    pub(crate) cursor_style_explicit: bool,
    pub(crate) user_select: UserSelect,
    pub(crate) resolved_user_select: bool,
    pub(crate) focusable: bool,
    pub(crate) focusable_when_disabled: bool,
    pub(crate) focus_on_pointer: bool,
    pub(crate) hit_slop: Insets,
    pub(crate) focus_trap: bool,
    pub(crate) restore_previous_focus: bool,
    pub(crate) key_context: Option<KeyContext>,
    pub(crate) tab_index: i16,
    pub(crate) auto_focus: bool,
    pub(crate) form: bool,
    pub(crate) form_submitter: bool,
    pub(crate) activation_target: Option<ElementId>,
    pub(crate) accessibility: AccessibilityStyle,
    pub(crate) tab_list_behavior: Option<TabListBehavior>,
    pub(crate) plane: Option<ScenePlane>,
    pub(crate) z_index: Option<i16>,
    pub(crate) portal: bool,
    pub(crate) anchor: Option<AnchorStyle>,
    pub(crate) anchor_placement: Option<AnchorPlacementHandle>,
    pub(crate) layout_bounds: Option<LayoutBoundsHandle>,
    pub(crate) tooltip: Option<Tooltip>,
    pub(crate) app_region: Option<AppRegion>,
    pub(crate) virtual_scroll: Option<VirtualScrollStyle>,
    pub(crate) scroll_to_end_revision: Option<u64>,
    pub(crate) scroll_request: Option<ScrollRequest>,
    pub(crate) layout_rounding: bool,
    pub(crate) list_item_measurement: Option<ListItemMeasurement>,
    pub(crate) animation: Option<ElementAnimation>,
    pub(crate) spring: Option<ElementSpring>,
    /// The declaration callback was consumed, but still owns this resolved subtree's values.
    pub(crate) resolved_motion: bool,
    pub(crate) transition: Option<Transition>,
    pub(crate) blocks_pointer: bool,
    pub(crate) dismiss_policy: DismissPolicy,
    pub(crate) restore_focus: Option<FocusHandle>,
    pub(crate) children: Vec<Element>,
    pub(crate) taffy_node: Option<taffy::NodeId>,
    /// Declared inline layout direction; inherited when absent.
    pub(crate) direction: Option<Direction>,
    /// Direction resolved against the ancestor chain during layout build.
    pub(crate) resolved_direction: Direction,
    /// Direction-relative padding and border overrides resolved during layout build.
    pub(crate) logical_insets: Option<Box<LogicalInsets>>,
    /// CSS-style sticky offsets relative to the nearest scroll container.
    pub(crate) sticky: Option<StickyInsets>,
    /// Scroll-snap strictness declared by this scroll container.
    pub(crate) scroll_snap: Option<ScrollSnapStyle>,
    /// Snap alignment declared by a scroll-snap child.
    pub(crate) snap_align: Option<SnapAlign>,
    /// Whether a scroll gesture may never skip past this snap child.
    pub(crate) snap_stop_always: bool,
}

/// A change to a mounted element, addressed by stable identity.
///
/// Embedding runtimes can submit a batch with [`crate::AppRunner::update_elements`] after
/// updating their source declaration. Text changes invalidate intrinsic layout along the
/// affected ancestor path. Colors and opacity require paint only. Unsupported or unmounted
/// targets reject the entire batch, allowing the caller to request an ordinary view rebuild.
#[derive(Clone, Debug)]
pub enum ElementUpdate {
    Text {
        id: ElementId,
        content: Arc<str>,
    },
    BackgroundColor {
        id: ElementId,
        color: Color,
    },
    TextColor {
        id: ElementId,
        color: Color,
    },
    Opacity {
        id: ElementId,
        opacity: f32,
    },
    /// Change placement without measuring or laying out the subtree again.
    Transform {
        id: ElementId,
        transform: Transform2D,
    },
    /// Replace a declaration at an existing identity, retaining keyed layout and input state.
    /// The replacement root must have the same explicit ID. Register replacement listeners
    /// through a component scope; this operation itself does not register callbacks.
    Replace {
        id: ElementId,
        element: Box<Element>,
    },
}

impl ElementUpdate {
    pub(crate) fn apply_to_declaration(&self, element: &mut Element) -> bool {
        if element.explicit_id != Some(self.id()) {
            return false;
        }
        match self {
            Self::Text { content, .. } => {
                let ElementKind::Text(value) = &mut element.kind else {
                    return false;
                };
                *value = content.clone();
            }
            Self::BackgroundColor { color, .. } => element.visual.background = Some(*color),
            Self::TextColor { color, .. } => element.typography.color = Some(*color),
            Self::Opacity { opacity, .. } => {
                element.visual.opacity = if opacity.is_finite() {
                    opacity.clamp(0.0, 1.0)
                } else {
                    1.0
                }
            }
            Self::Transform { transform, .. } => element.visual.transform = *transform,
            Self::Replace { .. } => return false,
        }
        true
    }

    pub fn id(&self) -> ElementId {
        match self {
            Self::Text { id, .. }
            | Self::BackgroundColor { id, .. }
            | Self::TextColor { id, .. }
            | Self::Opacity { id, .. }
            | Self::Transform { id, .. }
            | Self::Replace { id, .. } => *id,
        }
    }
}

impl Element {
    /// Whether keyboard focus may land on this element.
    ///
    /// A disabled control leaves the Tab sequence, which is the web default. Toolbars are the
    /// documented exception: an item that opts into [`Element::focusable_when_disabled`] stays
    /// reachable so a keyboard user can discover why it is unavailable.
    pub(crate) const fn is_keyboard_focusable(&self) -> bool {
        self.focusable && (!self.accessibility.disabled || self.focusable_when_disabled)
    }
}

/// Create a container element.
pub fn div() -> Element {
    Element::container()
}

/// Create a CSS-like container query whose contents are declared from its assigned size.
///
/// The query fills its parent by default. Its callback is evaluated after the query's own layout,
/// and the returned subtree is laid out independently inside that fixed box, so contents cannot
/// affect the query's intrinsic size.
///
/// An unchanged assigned size retains the existing subtree. A size change evaluates the callback
/// again, while stable declarative-animation IDs keep their playback state. Query contents must be
/// returned from this callback rather than attached with [`Element::child`] or
/// [`Element::children`].
///
/// # Example
///
/// ```
/// use quickgui::{container_query, div, text};
///
/// let responsive = container_query(|size| {
///     if size.width < 480.0 {
///         div().flex_col().child(text("Compact"))
///     } else {
///         div().grid().grid_cols(3).child(text("Wide"))
///     }
/// });
/// ```
pub fn container_query<E>(render: impl Fn(crate::Size) -> E + 'static) -> Element
where
    E: IntoElement,
{
    let mut element = Element::container();
    element.kind = ElementKind::ContainerQuery(ContainerQueryElement {
        render: Rc::new(move |size| render(size).into_element()),
        resolved_size: None,
        resolved_motion_ids: Vec::new(),
        layout_pending: false,
    });
    element.layout.size = TaffySize {
        width: Dimension::percent(1.0),
        height: Dimension::percent(1.0),
    };
    element
}

/// Create a semantic form container.
///
/// Attach callbacks with [`Element::on_form_submit`] and [`Element::on_form_invalid`]. Return in a
/// descendant single-line input and [`submit_button`] both validate the nearest form.
pub fn form() -> Element {
    let mut element = div().accessibility_role(AccessibilityRole::Form);
    element.form = true;
    element
}

/// Create a viewport-level element painted on the overlay plane.
///
/// Overlay elements are removed from normal flow, escape ancestor clipping, and block pointer
/// events inside their bounds. Use [`Element::anchor_to`] for popovers and menus.
pub fn overlay() -> Element {
    div().overlay()
}

/// Create a semantic, keyboard-focusable button container.
pub fn button() -> Element {
    let mut element = div()
        .accessibility_role(AccessibilityRole::Button)
        .focusable();
    element.set_implicit_cursor(CursorStyle::PointingHand);
    element
}

/// Create a button that validates and submits its nearest ancestor [`form`].
pub fn submit_button() -> Element {
    button().form_submitter()
}

/// Create a text element. Strings are owned through `Arc<str>` and cheap to retain.
pub fn text(content: impl Into<Arc<str>>) -> Element {
    Element::text(content.into())
}

/// Create an image element with intrinsic sizing and `object-fit: contain` behavior.
pub fn img(source: impl Into<ImageSource>) -> Element {
    Element::image(source.into())
}

/// Create a monochrome SVG element with intrinsic sizing and inherited text color.
pub fn svg(source: impl Into<Svg>) -> Element {
    Element::svg(source.into())
}

/// Create a retained vector path element with intrinsic sizing and inherited text color.
pub fn path(source: impl Into<Path>) -> Element {
    Element::path(source.into())
}

/// Create a web-like custom paint surface. The callback runs only when the view repaints.
///
/// Callback coordinates are local to the element and are clipped to its layout box.
pub fn canvas(painter: impl for<'a> Fn(Rect, &mut Canvas<'a>) + 'static) -> Element {
    Element::canvas(painter)
}

/// Create a retained rectangle painted by validated application WGSL.
///
/// Like the web canvas default, its initial size is 300 by 150 logical pixels. The shader remains
/// still until application state invalidates the view or explicitly requests another frame.
pub fn custom_shader(shader: impl Into<CustomShader>) -> Element {
    Element::custom_shader(shader.into())
}

/// Create a controlled, single-line text input.
///
/// Attach a stable [`crate::InputListener`] with [`Element::on_input`].
pub fn text_input(value: impl Into<Arc<str>>) -> Element {
    Element::text_input(value.into(), Arc::from([]), false)
}

/// Create a controlled, multiline text area with web-style soft wrapping.
///
/// Attach a stable [`crate::InputListener`] with [`Element::on_input`]. Use [`Element::no_wrap`]
/// for code-editor-style horizontal scrolling.
pub fn text_area(value: impl Into<Arc<str>>) -> Element {
    Element::text_input(value.into(), Arc::from([]), true)
}

/// Create a controlled, single-line input with bounded byte-range text styles.
///
/// Attach a stable [`crate::InputListener`] with [`Element::on_input`] and rebuild the
/// [`StyledText`] from the controlled value when it changes. Editing, selection, IME, hit testing,
/// and painting all reuse the same retained shaped buffer.
pub fn styled_text_input(value: StyledText) -> Element {
    let (value, highlights) = value.into_parts();
    Element::text_input(value, highlights, false)
}

/// Create a controlled, multiline text area with bounded byte-range text styles.
///
/// This is the attributed-text counterpart to [`text_area`]. It preserves wrapping and both-axis
/// scrolling while using the supplied styles for shaping, caret geometry, selection, and paint.
pub fn styled_text_area(value: StyledText) -> Element {
    let (value, highlights) = value.into_parts();
    Element::text_input(value, highlights, true)
}

/// Embed an AppKit view as a declarative leaf on macOS.
///
/// QuickGUI synchronizes layout, clipping, visibility, and sibling order while AppKit keeps
/// ownership of the view's rendering and input behavior.
#[cfg(target_os = "macos")]
pub fn native_view(view: &NSView) -> Element {
    Element::native_view(MacNativeView::new(view))
}

/// Embed an AppKit view whose frame extends `outset` points past its layout box on every side.
///
/// Layout, hit regions, and siblings see the element's own box; only the native frame and its clip
/// grow, so a control can draw an effect that spills past its bounds, such as Liquid Glass, without
/// that headroom pushing its neighbours apart. See [`MacNativeView::with_outset`].
#[cfg(target_os = "macos")]
pub fn native_view_with_outset(view: &NSView, outset: f32) -> Element {
    Element::native_view(MacNativeView::new(view).with_outset(outset))
}

mod construction_layout;
pub use construction_layout::{Direction, SnapAlign, SnapStrictness, StickyInsets};
pub(crate) use construction_layout::{LogicalInsets, ScrollSnapStyle};
mod interaction;
mod state;
mod style;

fn stable_hash(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn bounded_validation_message(message: Arc<str>) -> (Option<Arc<str>>, bool) {
    if message.is_empty() {
        return (None, false);
    }
    if message.len() <= MAX_VALIDATION_MESSAGE_BYTES {
        return (Some(message), false);
    }
    let mut end = MAX_VALIDATION_MESSAGE_BYTES;
    while !message.is_char_boundary(end) {
        end -= 1;
    }
    (Some(Arc::from(&message[..end])), true)
}

#[cfg(test)]
mod tests;
