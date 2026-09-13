use std::{
    any::{Any, TypeId},
    collections::{HashMap, HashSet},
    ops::Range,
    sync::Arc,
};
use web_time::{Duration, Instant};

use accesskit::{
    Action, Affine, AutoComplete as NativeAccessibilityAutoComplete,
    HasPopup as AccessibilityHasPopover, Invalid as AccessibilityInvalid, Live,
    Node as AccessibilityNode, NodeId as AccessibilityNodeId,
    Orientation as NativeAccessibilityOrientation, Rect as AccessibilityRect, Role,
    SortDirection as NativeAccessibilitySortDirection, TextPosition, TextSelection,
    Toggled as AccessibilityToggled, Tree, TreeId, TreeUpdate, Vec2 as AccessibilityVector,
};
use taffy::{
    geometry::Size as TaffySize,
    prelude::{AvailableSpace, NodeId, TaffyTree},
    style::{CompactLength, Dimension, Overflow, Style as TaffyStyle},
};
use thiserror::Error;
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    AccessibilityAutoComplete, AccessibilityPopover, AccessibilityRole, AccessibilitySortDirection,
    AnchorAlign, AnchorPlacement, AnchorPlacementHandle, AnchorSide, AnimatedImage, AppRegion,
    BackgroundImage, BoxShadow, Canvas, Color, ColorMatrix, Corners, CursorStyle,
    CustomShaderPrimitive, DispatchPhase, Element, ElementId, ImagePrimitive, Insets, Interpolate,
    KeyContext, LayoutBoundsHandle, MAX_BACKGROUND_IMAGE_TILES, MAX_BOX_SHADOWS_PER_ELEMENT,
    MAX_CONTAINER_QUERIES_PER_WINDOW, MAX_CONTAINER_QUERY_DEPTH,
    MAX_DECLARATIVE_ANIMATIONS_PER_WINDOW, MAX_STYLE_TRANSITIONS_PER_WINDOW,
    MAX_TOOLTIPS_PER_WINDOW, MouseButton, ObjectFit, Path, PathPrimitive, Point, Quad, Rect,
    ResolvedAnchorPlacement, Scene, Shadow, Size, SvgPrimitive, TextHighlight, TextId, TextRun,
    TextStyle, TextWrap, ToggleState, Tooltip, Transition, TransitionProperties, UserSelect,
    Vector,
    action::ActionListenerBinding,
    animated_image::AnimatedImageId,
    animation::{Animation, ElementAnimation},
    element::{
        AccessibilityOrientation, AnchorStyle, AnchorTarget, DismissPolicy, DropPredicateCallback,
        ElementKind, ElementStateStyle, GroupState, ImageResolution, KeyListenerBinding,
        KeyListenerKind, MouseListenerBinding, MouseListenerKey, MouseListenerKind,
    },
    event::{
        FormField, FormSubmitEvent, MAX_FORM_FIELDS, MAX_VALIDATION_ISSUES,
        MAX_VALIDATION_MESSAGE_BYTES, ValidationIssue, ValidationReport,
    },
    image::fit_image,
    renderer::{TextLayoutEngine, TextPaintKind, TextPaintRect},
    scene::{EdgeQuad, PaintLayerKey, WavyUnderline},
    spring::{ElementSpring, SpringConfig, SpringPlayback, SpringState},
    text_input::{
        TextInputState, accessibility_byte_index, accessibility_character_index,
        accessibility_character_index_from_lengths, boundary_at_or_before,
        selectable_character_lengths,
    },
    virtual_list::{VirtualScrollHandle, VirtualScrollMount},
};

// Direction-relative layout, sticky positioning, scroll snapping, and text styling additions.
use crate::{
    TextAlign, TextDirection,
    element::{Direction, ScrollSnapStyle, SnapAlign, SnapStrictness},
};

// Compositing layers: transforms, subtree filters, backdrop effects, and blend modes.
use crate::{LayerEffects, Transform2D, scene::GroupHandle};

#[cfg(target_os = "macos")]
use crate::{ScenePlane, native_view::NativeViewPlacement};

const ACCESSIBILITY_ROOT_ID: AccessibilityNodeId = AccessibilityNodeId(u64::MAX);
const SCROLLBAR_AUTO_HIDE_DELAY: Duration = Duration::from_millis(900);
const STATIC_TEXT_MULTI_CLICK_INTERVAL: Duration = Duration::from_millis(500);
const STATIC_TEXT_MULTI_CLICK_DISTANCE: f32 = 4.0;

/// Maximum retained ancestor depth traversed by one targeted desktop mouse event.
pub const MAX_MOUSE_EVENT_PATH: usize = 256;

/// Maximum retained ancestor depth traversed by one focused action or key event.
pub const MAX_FOCUSED_EVENT_PATH: usize = 256;

/// The device delivering the input event a window is dispatching.
///
/// Focus styles follow the device like CSS `:focus-visible`: a pointer that lands focus paints no
/// ring, because the user is already looking at what they clicked, while a key that lands focus
/// paints one, because the ring is how a keyboard user finds focus at all.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum InputModality {
    /// A mouse, trackpad, or touch press or release is being dispatched.
    Pointer,
    /// A key press or release is being dispatched.
    Keyboard,
}

/// Receipt for one [`UiTree::begin_input_dispatch`], handed back to
/// [`UiTree::end_input_dispatch`].
///
/// Only the outermost scope clears the modality. Enter activating a button dispatches that click
/// inside the key event, and the focus the click lands must still count as keyboard-driven.
#[derive(Clone, Copy, Debug)]
#[must_use = "an input dispatch scope must be closed with `UiTree::end_input_dispatch`"]
pub(crate) struct InputDispatchScope {
    outermost: bool,
}

/// A focus request kept through exactly one rebuild, with the device that made it.
///
/// `EventContext::focus` may name an element the same event's rebuild introduces. The request is
/// applied after that rebuild, outside the dispatch that produced it, so it carries the modality
/// along and the focus it finally lands paints exactly as an already-mounted target would have.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PendingFocus {
    pub(crate) element: ElementId,
    pub(crate) modality: Option<InputModality>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct MouseHoverChange {
    pub(crate) key: MouseListenerKey,
    pub(crate) hovered: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TabNavigationTarget {
    pub(crate) id: ElementId,
    pub(crate) activate: bool,
}

/// Maximum retained hit regions consulted synchronously by one native macOS drag.
///
/// Regions are copied topmost-first, so reaching the bound conservatively rejects only targets
/// below the retained stack instead of allowing a drag through an omitted pointer blocker.
#[cfg(target_os = "macos")]
pub(crate) const MAX_EXTERNAL_DROP_HIT_REGIONS: usize = 4_096;

/// Maximum exact-type listener acceptances copied into one native macOS drag snapshot.
#[cfg(target_os = "macos")]
pub(crate) const MAX_EXTERNAL_DROP_ACCEPTANCES: usize = 8_192;

/// Maximum UTF-8 payload materialized by one immutable-text copy operation.
///
/// The mounted strings stay shared; this limit only bounds the temporary joined clipboard value.
pub const MAX_STATIC_TEXT_COPY_BYTES: usize = 8 * 1024 * 1024;

pub(crate) enum FormAttempt {
    Valid(FormSubmitEvent),
    Invalid(ValidationReport),
}

struct ValidationAnnouncement {
    form: ElementId,
    node: AccessibilityNodeId,
    message: Arc<str>,
}

pub(crate) fn static_selection_color() -> Color {
    Color::rgba8(48, 120, 196, 105)
}

#[derive(Debug, Error)]
pub(crate) enum UiError {
    #[error("flexbox layout failed: {0}")]
    Layout(#[from] taffy::TaffyError),
    #[error("element id {0:?} appears more than once in the same view")]
    DuplicateId(ElementId),
    #[error("element id {0:?} is reserved by the accessibility root")]
    ReservedId(ElementId),
    #[error("animation id {0:?} appears more than once in the same view")]
    DuplicateAnimationId(ElementId),
    #[error(
        "a window cannot retain more than {MAX_DECLARATIVE_ANIMATIONS_PER_WINDOW} declarative animations"
    )]
    TooManyDeclarativeAnimations,
    #[error(
        "a window cannot retain more than {MAX_STYLE_TRANSITIONS_PER_WINDOW} style transitions"
    )]
    TooManyStyleTransitions,
    #[error("anchored element {element:?} refers to missing element {anchor:?}")]
    MissingAnchor {
        element: ElementId,
        anchor: ElementId,
    },
    #[error("a window cannot retain more than {MAX_TOOLTIPS_PER_WINDOW} tooltips")]
    TooManyTooltips,
    #[error("a window cannot retain more than {MAX_STICKY_ELEMENTS_PER_WINDOW} sticky elements")]
    TooManyStickyElements,
    #[error(
        "a window cannot retain more than {MAX_CONTAINER_QUERIES_PER_WINDOW} container queries"
    )]
    TooManyContainerQueries,
    #[error("container queries cannot nest deeper than {MAX_CONTAINER_QUERY_DEPTH} levels")]
    ContainerQueryDepthExceeded,
    #[error("container query layout did not converge within the bounded nesting limit")]
    ContainerQueryDidNotConverge,
    #[cfg(target_os = "macos")]
    #[error("native AppKit view {0:?} must stay in the base composition plane")]
    NativeViewInOverlay(ElementId),
}

#[derive(Clone)]
enum MeasureContext {
    Text {
        id: TextId,
        content: Arc<str>,
        /// Boxed so one measurement context stays small: the resolved text style carries every
        /// inherited typography property and dwarfs the image variant otherwise.
        style: Box<TextStyle>,
        highlights: Option<Arc<[TextHighlight]>>,
    },
    Image {
        intrinsic: Size,
    },
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct HitRegion {
    pub(crate) id: ElementId,
    pub(crate) bounds: Rect,
    pub(crate) clip: Rect,
    pub(crate) clickable: bool,
    pub(crate) pointer_listener: bool,
    pub(crate) drag_source: bool,
    pub(crate) drop_target: bool,
    pub(crate) focusable: bool,
    pub(crate) cursor_style: Option<CursorStyle>,
    cursor_states: CursorStateStyles,
    pub(crate) stateful: bool,
    pub(crate) blocks_pointer: bool,
    pub(crate) app_region: Option<AppRegion>,
    pub(crate) order: PaintOrder,
    /// Accumulated window-space transform of the compositing groups this region paints inside.
    ///
    /// `bounds` and `clip` stay in the untransformed layout space they were measured in, so a
    /// pointer position is inverse-mapped through this before it is tested.
    pub(crate) transform: Option<Transform2D>,
}

#[derive(Clone, Copy, Debug, Default)]
struct CursorStateStyles {
    hover: Option<CursorStyle>,
    active: Option<CursorStyle>,
    focus: Option<CursorStyle>,
    invalid: Option<CursorStyle>,
    selected: Option<CursorStyle>,
    dragging: Option<CursorStyle>,
    drag_over: Option<CursorStyle>,
}

impl HitRegion {
    /// Map a window point into this region's own coordinate system.
    ///
    /// Returns `None` when the accumulated transform collapses an axis, which makes the subtree
    /// invisible and therefore untargetable.
    pub(crate) fn local_point(&self, point: Point) -> Option<Point> {
        match self.transform {
            None => Some(point),
            Some(transform) => transform.inverse().map(|inverse| inverse.apply(point)),
        }
    }

    fn contains(self, point: Point) -> bool {
        self.local_point(point)
            .is_some_and(|point| self.bounds.contains(point) && self.clip.contains(point))
    }
}

fn expand_hit_bounds(bounds: Rect, slop: Insets) -> Rect {
    Rect::new(
        bounds.x - slop.left,
        bounds.y - slop.top,
        bounds.width + slop.left + slop.right,
        bounds.height + slop.top + slop.bottom,
    )
}

#[cfg(target_os = "macos")]
#[derive(Clone)]
enum ExternalDropAcceptance {
    Always,
    Predicate(DropPredicateCallback),
}

#[cfg(target_os = "macos")]
impl ExternalDropAcceptance {
    fn accepts(&self, value: &dyn Any) -> bool {
        match self {
            Self::Always => true,
            // Application predicates must never unwind through an Objective-C drag callback.
            Self::Predicate(predicate) => {
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| predicate(value)))
                    .unwrap_or(false)
            }
        }
    }
}

#[cfg(target_os = "macos")]
#[derive(Clone)]
struct ExternalDropHitRegion {
    id: ElementId,
    bounds: Rect,
    clip: Rect,
    drop_target: bool,
    blocks_pointer: bool,
    pointer_listener: bool,
}

#[cfg(target_os = "macos")]
impl ExternalDropHitRegion {
    fn contains(&self, point: Point) -> bool {
        self.bounds.contains(point) && self.clip.contains(point)
    }
}

/// Reusable, bounded copy of the retained drop topology used by AppKit's synchronous callbacks.
#[cfg(target_os = "macos")]
#[derive(Default)]
pub(crate) struct ExternalDropSnapshot {
    regions: Vec<ExternalDropHitRegion>,
    acceptances: HashMap<(ElementId, TypeId), ExternalDropAcceptance>,
    target_ids: HashSet<ElementId>,
    regions_truncated: bool,
    acceptances_truncated: bool,
}

#[cfg(target_os = "macos")]
impl ExternalDropSnapshot {
    pub(crate) fn new() -> Self {
        Self {
            regions: Vec::with_capacity(64),
            acceptances: HashMap::with_capacity(64),
            target_ids: HashSet::with_capacity(32),
            regions_truncated: false,
            acceptances_truncated: false,
        }
    }

    pub(crate) fn clear(&mut self) {
        self.regions.clear();
        self.acceptances.clear();
        self.target_ids.clear();
        self.regions_truncated = false;
        self.acceptances_truncated = false;
    }

    pub(crate) fn offer_target_at<'a, I>(
        &self,
        point: Point,
        offers: I,
    ) -> Option<(ElementId, usize)>
    where
        I: Iterator<Item = (TypeId, &'a dyn Any)> + Clone,
    {
        // An omitted acceptance could belong to a higher target than one retained below it.
        // Reject the complete native offer rather than allowing it to reach through that target.
        if self.acceptances_truncated {
            return None;
        }
        for region in &self.regions {
            if !region.contains(point) {
                continue;
            }
            if region.drop_target {
                for (index, (value_type, value)) in offers.clone().enumerate() {
                    if self
                        .acceptances
                        .get(&(region.id, value_type))
                        .is_some_and(|acceptance| acceptance.accepts(value))
                    {
                        return Some((region.id, index));
                    }
                }
            }
            if region.blocks_pointer || region.pointer_listener {
                return None;
            }
        }
        None
    }

    #[cfg(test)]
    pub(crate) fn target_at(
        &self,
        point: Point,
        value_type: TypeId,
        value: &dyn Any,
    ) -> Option<ElementId> {
        self.offer_target_at(point, std::iter::once((value_type, value)))
            .map(|(id, _)| id)
    }

    #[cfg(test)]
    fn is_truncated(&self) -> bool {
        self.regions_truncated || self.acceptances_truncated
    }
}

#[derive(Clone, Copy, Debug)]
struct ScrollRegion {
    id: ElementId,
    /// Whether the container's inline start edge is on the right, inverting horizontal wheels.
    rtl: bool,
    /// Pointer/wheel hit bounds for this scrolling container.
    bounds: Rect,
    /// Geometry used by the overlay scrollbar; normally equal to `bounds`.
    scrollbar_bounds: Rect,
    clip: Rect,
    max_offset: Vector,
    /// Whether changing this offset must update a bound virtual list and rebuild the view.
    virtual_scroll: bool,
    order: PaintOrder,
    scrollbar_order: PaintOrder,
}

#[derive(Clone, Debug)]
struct RetainedVirtualScroll {
    handle: VirtualScrollHandle,
    measurement_revision: u64,
    mount: VirtualScrollMount,
}

impl RetainedVirtualScroll {
    fn update_from_input(&self, offset_y: f32, viewport_height: f32) -> bool {
        self.handle.set_offset_from_input(offset_y);
        !self
            .handle
            .retains_viewport(&self.mount, offset_y, viewport_height)
    }
}

#[derive(Clone, Copy, Debug)]
struct ScrollbarDrag {
    id: ElementId,
    pointer_origin_y: f32,
    scroll_origin_y: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct ScrollbarState {
    hovered: bool,
    dragging: bool,
    visible_until: Option<Instant>,
}

impl ScrollbarState {
    fn visible(self, now: Instant) -> bool {
        self.hovered || self.dragging || self.visible_until.is_some_and(|deadline| deadline > now)
    }
}

#[derive(Clone, Copy, Debug)]
struct VerticalScrollbarGeometry {
    track: Rect,
    thumb: Rect,
    travel: f32,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct PaintOrder {
    pub(crate) layer: PaintLayerKey,
    pub(crate) source: usize,
}

#[derive(Clone, Copy, Debug)]
struct DismissRegion {
    id: ElementId,
    bounds: Rect,
    clip: Rect,
    policy: DismissPolicy,
    restore_focus: Option<ElementId>,
    order: PaintOrder,
}

impl DismissRegion {
    fn contains(self, point: Point) -> bool {
        self.bounds.contains(point) && self.clip.contains(point)
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct DismissRequest {
    pub id: ElementId,
    pub restore_focus: Option<ElementId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FocusRestoration {
    surface: ElementId,
    target: ElementId,
}

#[derive(Clone)]
struct TextInputRegion {
    id: ElementId,
    bounds: Rect,
    clip: Rect,
    content: Arc<str>,
    password: Option<PasswordDisplay>,
    highlights: Option<Arc<[TextHighlight]>>,
    style: TextStyle,
    scroll: Vector,
    max_scroll: Vector,
    caret_bounds: Rect,
}

const PASSWORD_MASK: &str = "•";

/// A display-only password projection with exact source/display boundary translation.
///
/// The retained editor always owns the real string. Rendering one mask glyph per grapheme keeps
/// Unicode cursor and selection behavior web-like without placing the secret in a scene command.
#[derive(Clone)]
struct PasswordDisplay {
    content: Arc<str>,
    source_boundaries: Arc<[usize]>,
}

impl PasswordDisplay {
    fn new(source: &str) -> Self {
        let graphemes = source.grapheme_indices(true).collect::<Vec<_>>();
        let mut masked = String::with_capacity(graphemes.len() * PASSWORD_MASK.len());
        let mut source_boundaries = Vec::with_capacity(graphemes.len() + 1);
        source_boundaries.push(0);
        for (start, grapheme) in graphemes {
            masked.push_str(PASSWORD_MASK);
            source_boundaries.push(start + grapheme.len());
        }
        Self {
            content: Arc::from(masked),
            source_boundaries: Arc::from(source_boundaries),
        }
    }

    fn display_index(&self, source_index: usize) -> usize {
        self.source_boundaries
            .partition_point(|boundary| *boundary <= source_index)
            .saturating_sub(1)
            .min(self.source_boundaries.len().saturating_sub(1))
            * PASSWORD_MASK.len()
    }

    fn source_index(&self, display_index: usize) -> usize {
        let grapheme = (display_index / PASSWORD_MASK.len())
            .min(self.source_boundaries.len().saturating_sub(1));
        self.source_boundaries[grapheme]
    }
}

#[derive(Clone, Debug)]
struct SelectableTextEntry {
    id: ElementId,
    content: Arc<str>,
    character_lengths: Arc<[u8]>,
}

#[derive(Clone)]
struct SelectableTextRegion {
    links: Vec<(Rect, Arc<str>)>,
    document_index: usize,
    bounds: Rect,
    clip: Rect,
    style: TextStyle,
    highlights: Option<Arc<[TextHighlight]>>,
    order: PaintOrder,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct StaticTextPosition {
    id: ElementId,
    offset: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct StaticTextSelection {
    anchor: StaticTextPosition,
    focus: StaticTextPosition,
}

#[derive(Clone, Copy, Debug)]
struct StaticTextGesture {
    origin: Point,
    moved: bool,
    unit: StaticTextSelectionUnit,
    base_start: StaticTextPosition,
    base_end: StaticTextPosition,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StaticTextSelectionUnit {
    Character,
    Word,
    Line,
}

#[derive(Clone, Copy, Debug)]
struct StaticTextClick {
    position: Point,
    id: ElementId,
    at: Instant,
    count: u8,
}

#[derive(Clone, Debug)]
pub(crate) struct InputChange {
    pub id: ElementId,
    pub value: Arc<str>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct InputResult {
    pub repaint: bool,
    pub change: Option<InputChange>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct ScrollResult {
    pub changed: bool,
    pub view_dirty: bool,
}

/// One mounted binding between an anchored element and the handle it publishes into.
///
/// The retained revision is compared after paint so a resolved placement that actually changed
/// requests exactly one correcting frame, and an unchanged one requests none.
#[derive(Clone, Debug)]
struct RetainedAnchorPlacement {
    handle: AnchorPlacementHandle,
    revision: u64,
}

struct RetainedLayoutBounds {
    handle: LayoutBoundsHandle,
    revision: u64,
}

#[derive(Clone, Copy, Debug)]
struct ScrollEndState {
    revision: u64,
    previous_max_y: f32,
}

pub(crate) struct UiTree {
    work: crate::PipelineMetrics,
    geometry_cache: retained::GeometryCache,
    paint_cache: paint_cache::PaintCache,
    root: Option<Element>,
    taffy: TaffyTree<MeasureContext>,
    layout_nodes: LayoutNodeCache,
    root_node: Option<NodeId>,
    mounted_state_dirty: bool,
    retained_semantics_dirty: bool,
    retained_placement_dirty: bool,
    seen_ids: HashSet<ElementId>,
    /// IDs whose complete ancestor chain participates in layout (`display != none`).
    displayed_ids: HashSet<ElementId>,
    /// Displayed IDs whose complete ancestor chain is painted (`visibility != hidden`).
    visible_ids: HashSet<ElementId>,
    scroll_offsets: HashMap<ElementId, Vector>,
    scroll_end_states: HashMap<ElementId, ScrollEndState>,
    virtual_scroll_handles: HashMap<ElementId, RetainedVirtualScroll>,
    anchor_placement_handles: Vec<RetainedAnchorPlacement>,
    layout_bounds_handles: Vec<RetainedLayoutBounds>,
    natural_bounds: HashMap<ElementId, Rect>,
    element_bounds: HashMap<ElementId, Rect>,
    hit_regions: Vec<HitRegion>,
    drop_predicates: HashMap<(ElementId, TypeId), DropPredicateCallback>,
    context_menu_ids: HashSet<ElementId>,
    mouse_listener_bindings: Vec<MouseListenerBinding>,
    mouse_listener_ranges: HashMap<ElementId, Range<usize>>,
    mouse_listener_elements: Vec<ElementId>,
    key_listener_bindings: Vec<KeyListenerBinding>,
    key_listener_ranges: HashMap<ElementId, Range<usize>>,
    action_listener_bindings: Vec<ActionListenerBinding>,
    action_listener_ranges: HashMap<ElementId, Range<usize>>,
    scroll_wheel_ids: HashSet<ElementId>,
    touch_ids: HashSet<ElementId>,
    mouse_pressure_ids: HashSet<ElementId>,
    pinch_ids: HashSet<ElementId>,
    rotation_ids: HashSet<ElementId>,
    smart_magnify_ids: HashSet<ElementId>,
    tooltips: HashMap<ElementId, Tooltip>,
    pointer_tooltip: Option<ElementId>,
    hovered_tooltip: Option<ElementId>,
    pending_tooltip: Option<PendingTooltip>,
    visible_tooltip: Option<ElementId>,
    tooltip_overlay: Option<TooltipOverlay>,
    scroll_regions: Vec<ScrollRegion>,
    /// Bounded snap geometry rebuilt in place by every geometry pass.
    scroll_snap_geometry: ScrollSnapGeometry,
    /// The single in-flight snap settle and animation for this window's scroll gesture.
    scroll_snap: ScrollSnapState,
    scrollbar_drag: Option<ScrollbarDrag>,
    scrollbar_states: HashMap<ElementId, ScrollbarState>,
    hovered_scrollbar: Option<ElementId>,
    dismiss_regions: Vec<DismissRegion>,
    focus_restorations: Vec<FocusRestoration>,
    #[cfg(target_os = "macos")]
    native_views: Vec<NativeViewPlacement>,
    text_input_regions: Vec<TextInputRegion>,
    text_inputs: HashMap<ElementId, TextInputState>,
    selectable_texts: Vec<SelectableTextEntry>,
    selectable_text_indices: HashMap<ElementId, usize>,
    selectable_text_regions: Vec<SelectableTextRegion>,
    static_text_selection: Option<StaticTextSelection>,
    static_text_gesture: Option<StaticTextGesture>,
    pressed_link: Option<(ElementId, Arc<str>)>,
    last_static_text_click: Option<StaticTextClick>,
    accessibility_text_ids: HashMap<ElementId, AccessibilityNodeId>,
    accessibility_snapshot: std::cell::RefCell<Option<AccessibilitySnapshot>>,
    next_accessibility_text_id: u64,
    animations: HashMap<ElementId, AnimationPlayback>,
    animation_ids: HashSet<ElementId>,
    declarative_animations: HashMap<ElementId, DeclarativeAnimationPlayback>,
    declarative_time_animation_ids: HashSet<ElementId>,
    declarative_springs: HashMap<ElementId, DeclarativeSpringPlayback>,
    declarative_spring_ids: HashSet<ElementId>,
    /// Shared namespace across time animations and springs for one declaration.
    declarative_animation_ids: HashSet<ElementId>,
    declarative_animation_frame_requested: bool,
    declarative_animation_deadline: Option<Instant>,
    style_transitions: HashMap<ElementId, StyleTransitionPlayback>,
    style_transition_ids: HashSet<ElementId>,
    style_transition_frame_requested: bool,
    animation_epoch: Instant,
    animations_enabled: bool,
    reduce_motion: bool,
    selecting_input: Option<ElementId>,
    hovered: HashSet<ElementId>,
    hover_scratch: HashSet<ElementId>,
    mouse_hover_path: Vec<ElementId>,
    mouse_hover_path_scratch: Vec<ElementId>,
    pending_mouse_hover_changes: Vec<MouseHoverChange>,
    pressed: Option<ElementId>,
    dragging: Option<ElementId>,
    external_drag_active: bool,
    drag_over: Option<ElementId>,
    drag_preview: Option<DragPreview>,
    focused: Option<ElementId>,
    /// Whether the focused element paints its focus styles, like CSS `:focus-visible`.
    ///
    /// Pointer-driven focus hides them, keyboard-driven focus shows them, and a programmatic focus
    /// outside any input keeps the previous answer. Text inputs are exempt and always paint theirs.
    focus_visible: bool,
    /// The device delivering the input event being dispatched, if one is in flight.
    ///
    /// Set once when a pointer or key dispatch starts, so every focus change it causes — including
    /// one a listener requests through `EventContext::focus` — resolves its visibility from the
    /// device instead of from flags threaded through each listener.
    input_modality: Option<InputModality>,
    active_focus_trap: Option<ElementId>,
    focusable_ids: HashSet<ElementId>,
    clickable_ids: HashSet<ElementId>,
    activation_targets: HashMap<ElementId, ElementId>,
    invalid_ids: HashSet<ElementId>,
    form_ids: HashSet<ElementId>,
    form_submitter_ids: HashSet<ElementId>,
    focus_order: Vec<ElementId>,
    parents: HashMap<ElementId, ElementId>,
    key_contexts: HashMap<ElementId, KeyContext>,
    focus_initialized: bool,
    validation_announcement: Option<ValidationAnnouncement>,
    viewport: Size,
    scale_factor: f32,
}

struct AnimationPlayback {
    asset: AnimatedImage,
    asset_id: AnimatedImageId,
    frame_index: usize,
    elapsed: Duration,
    last_advanced_at: Instant,
    active: bool,
    seen: bool,
    completed: bool,
}

#[derive(Clone, Copy, Debug)]
struct DeclarativeAnimationSample {
    animation_ix: usize,
    value: f32,
    request_frame: bool,
    deadline: Option<Instant>,
}

#[derive(Clone, Debug)]
struct DeclarativeAnimationPlayback {
    animation_ix: usize,
    elapsed: Duration,
    last_advanced_at: Instant,
    next_frame_at: Option<Instant>,
    scheduled_stage: usize,
    last_value: f32,
    completed: bool,
    active: bool,
}

#[derive(Clone, Copy, Debug)]
struct DeclarativeSpringSample {
    value: f32,
    request_frame: bool,
}

#[derive(Clone, Debug)]
struct DeclarativeSpringPlayback {
    state: SpringState,
    target: f32,
    config: SpringConfig,
    initial: f32,
    playback: SpringPlayback,
    updated_at: Instant,
    active: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct TransitionShadowList {
    values: [BoxShadow; MAX_BOX_SHADOWS_PER_ELEMENT],
    len: u8,
}

impl TransitionShadowList {
    fn empty() -> Self {
        Self {
            values: [neutral_transition_shadow(false); MAX_BOX_SHADOWS_PER_ELEMENT],
            len: 0,
        }
    }

    fn from_slice(shadows: &[BoxShadow]) -> Self {
        debug_assert!(shadows.len() <= MAX_BOX_SHADOWS_PER_ELEMENT);
        let mut list = Self::empty();
        for (index, shadow) in shadows
            .iter()
            .copied()
            .take(MAX_BOX_SHADOWS_PER_ELEMENT)
            .enumerate()
        {
            list.values[index] = sane_transition_shadow(shadow);
            list.len += 1;
        }
        list
    }

    fn as_slice(&self) -> &[BoxShadow] {
        &self.values[..usize::from(self.len)]
    }

    fn interpolate(from: Self, to: Self, phase: f32) -> Self {
        if phase == 0.0 {
            return from;
        }
        if phase == 1.0 {
            return to;
        }
        let len = usize::from(from.len.max(to.len));
        let mut result = Self::empty();
        result.len = len as u8;
        for index in 0..len {
            let from_shadow = if index < usize::from(from.len) {
                from.values[index]
            } else {
                neutral_transition_shadow(to.values[index].is_inset())
            };
            let to_shadow = if index < usize::from(to.len) {
                to.values[index]
            } else {
                neutral_transition_shadow(from.values[index].is_inset())
            };
            result.values[index] = interpolate_transition_shadow(from_shadow, to_shadow, phase);
        }
        result
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct TransitionPaintStyle {
    background: Color,
    border_color: Color,
    border_widths: Insets,
    radius: f32,
    opacity: f32,
    transform: Transform2D,
    text_color: Option<Color>,
    text_fallback: Color,
    shadows: TransitionShadowList,
}

impl TransitionPaintStyle {
    fn interpolate(from: Self, to: Self, phase: f32, properties: TransitionProperties) -> Self {
        let selected = |property| properties.contains(property);
        Self {
            background: if selected(TransitionProperties::BACKGROUND) {
                Color::interpolate_premultiplied(from.background, to.background, phase)
            } else {
                to.background
            },
            border_color: if selected(TransitionProperties::BORDER_COLOR) {
                Color::interpolate_premultiplied(from.border_color, to.border_color, phase)
            } else {
                to.border_color
            },
            border_widths: if selected(TransitionProperties::BORDER_WIDTH) {
                interpolate_insets(from.border_widths, to.border_widths, phase)
            } else {
                to.border_widths
            },
            radius: if selected(TransitionProperties::BORDER_RADIUS) {
                f32::interpolate(from.radius, to.radius, phase).max(0.0)
            } else {
                to.radius
            },
            opacity: if selected(TransitionProperties::OPACITY) {
                f32::interpolate(from.opacity, to.opacity, phase).clamp(0.0, 1.0)
            } else {
                to.opacity
            },
            transform: if selected(TransitionProperties::TRANSFORM) {
                interpolate_transition_transform(from.transform, to.transform, phase)
            } else {
                to.transform
            },
            text_color: if selected(TransitionProperties::TEXT_COLOR) {
                if phase == 0.0 {
                    from.text_color
                } else if phase == 1.0 {
                    to.text_color
                } else {
                    Some(Color::interpolate_premultiplied(
                        from.text_color.unwrap_or(from.text_fallback),
                        to.text_color.unwrap_or(to.text_fallback),
                        phase,
                    ))
                }
            } else {
                to.text_color
            },
            text_fallback: if selected(TransitionProperties::TEXT_COLOR) {
                Color::interpolate_premultiplied(from.text_fallback, to.text_fallback, phase)
            } else {
                to.text_fallback
            },
            shadows: if selected(TransitionProperties::BOX_SHADOW) {
                TransitionShadowList::interpolate(from.shadows, to.shadows, phase)
            } else {
                to.shadows
            },
        }
    }

    fn differs_for(self, other: Self, properties: TransitionProperties) -> bool {
        (properties.contains(TransitionProperties::BACKGROUND)
            && self.background != other.background)
            || (properties.contains(TransitionProperties::BORDER_COLOR)
                && self.border_color != other.border_color)
            || (properties.contains(TransitionProperties::BORDER_WIDTH)
                && self.border_widths != other.border_widths)
            || (properties.contains(TransitionProperties::BORDER_RADIUS)
                && self.radius != other.radius)
            || (properties.contains(TransitionProperties::OPACITY) && self.opacity != other.opacity)
            || (properties.contains(TransitionProperties::TRANSFORM)
                && self.transform != other.transform)
            || (properties.contains(TransitionProperties::TEXT_COLOR)
                && (self.text_color.unwrap_or(self.text_fallback)
                    != other.text_color.unwrap_or(other.text_fallback)))
            || (properties.contains(TransitionProperties::BOX_SHADOW)
                && self.shadows != other.shadows)
    }
}

fn interpolate_insets(from: Insets, to: Insets, phase: f32) -> Insets {
    Insets {
        top: f32::interpolate(from.top, to.top, phase).max(0.0),
        right: f32::interpolate(from.right, to.right, phase).max(0.0),
        bottom: f32::interpolate(from.bottom, to.bottom, phase).max(0.0),
        left: f32::interpolate(from.left, to.left, phase).max(0.0),
    }
}

#[derive(Clone, Debug)]
struct StyleTransitionPlayback {
    from: TransitionPaintStyle,
    current: TransitionPaintStyle,
    target: TransitionPaintStyle,
    config: Transition,
    elapsed: Duration,
    last_advanced_at: Instant,
    next_frame_at: Option<Instant>,
    in_progress: bool,
    active: bool,
}

struct StyleTransitionPaintContext<'a> {
    playbacks: Option<&'a mut HashMap<ElementId, StyleTransitionPlayback>>,
    request_frame: &'a mut bool,
    enabled: bool,
    reduce_motion: bool,
    now: Instant,
}

/// A detached declaration owns its template and motion clock independently from the application
/// view. Dropping the enclosing tooltip or drag preview therefore drops every playback and exact
/// deadline at the same time.
struct DetachedMotionState {
    template: Element,
    animations: HashMap<ElementId, DeclarativeAnimationPlayback>,
    time_animation_ids: HashSet<ElementId>,
    springs: HashMap<ElementId, DeclarativeSpringPlayback>,
    spring_ids: HashSet<ElementId>,
    motion_ids: HashSet<ElementId>,
    frame_requested: bool,
    deadline: Option<Instant>,
    needs_resolve: bool,
    animation_epoch: Instant,
}

/// Retained layout and image state for a pointer-passive tree painted outside the main view.
struct DetachedTree {
    motion: DetachedMotionState,
    root: Element,
    taffy: TaffyTree<MeasureContext>,
    root_node: Option<NodeId>,
    root_seed: ElementId,
    seen_ids: HashSet<ElementId>,
    input_ids: HashSet<ElementId>,
    animation_ids: HashSet<ElementId>,
    scroll_offsets: HashMap<ElementId, Vector>,
    scroll_end_states: HashMap<ElementId, ScrollEndState>,
    natural_bounds: HashMap<ElementId, Rect>,
    paint_bounds: HashMap<ElementId, Rect>,
    text_inputs: HashMap<ElementId, TextInputState>,
    animations: HashMap<ElementId, AnimationPlayback>,
}

struct DragPreview {
    tree: DetachedTree,
    cursor_offset: Point,
    position: Point,
}

#[derive(Clone, Copy, Debug)]
struct PendingTooltip {
    target: ElementId,
    show_at: Instant,
}

struct TooltipOverlay {
    target: ElementId,
    content_identity: *const Element,
    placement: AnchorPlacement,
    gap: f32,
    viewport_margin: f32,
    tree: DetachedTree,
}

mod accessibility;
mod dispatch;
mod editing;
mod focus;
mod input_state;
mod layout;
mod lifecycle;
mod motion;
mod paint_cache;
mod painting;
mod pointer;
mod retained;
mod updates;

use accessibility::*;
use dispatch::*;
use input_state::*;
use layout::*;
use motion::*;
use painting::*;

pub(crate) use layout::PointerResult;
pub use layout::{
    MAX_SCROLL_SNAP_CONTAINERS_PER_WINDOW, MAX_SCROLL_SNAP_POINTS_PER_WINDOW,
    MAX_STICKY_ELEMENTS_PER_WINDOW,
};
pub(crate) use updates::ElementUpdateKind;

#[cfg(test)]
mod tests;
