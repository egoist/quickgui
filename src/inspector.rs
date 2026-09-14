//! Feature-gated retained-tree inspection.
//!
//! The complete module is omitted unless the `inspector` Cargo feature is enabled. An enabled
//! window owns one bounded snapshot and one independent overlay tree; a closed inspector retains
//! neither and never schedules work.

use std::{collections::HashMap, sync::Arc};

use crate::{
    AccessibilityAutoComplete, AccessibilityPopover, AccessibilityRole, AccessibilitySortDirection,
    AppRegion, Color, CursorStyle, Element, ElementId, FrameMetrics, Point, Rect, Scene,
    ScenePlane, Size, ToggleState, Vector,
    element::ElementKind,
    renderer::TextLayoutEngine,
    ui_tree::{HitRegion, UiError, UiTree},
};

/// Maximum painted application nodes retained by one inspector snapshot.
pub const MAX_INSPECTOR_NODES: usize = 8_192;
/// Maximum UTF-8 bytes retained for one inspector label, value, or description.
pub const MAX_INSPECTOR_TEXT_BYTES: usize = 1_024;
/// Preferred logical width of the inspector panel.
pub const INSPECTOR_PANEL_WIDTH: f32 = 360.0;

const INSPECTOR_HEADER_HEIGHT: f32 = 44.0;
const INSPECTOR_HEADER_BUTTON_WIDTH: f32 = 48.0;
const INSPECTOR_WHEEL_STEP: f32 = 18.0;
const MAX_INSPECTOR_WHEEL_STEPS_PER_EVENT: usize = 8;

/// Current interaction state of the inspector.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum InspectorMode {
    /// Pointer movement picks the topmost painted application element.
    #[default]
    Picking,
    /// The selected element is frozen while ordinary application input remains available outside
    /// the panel.
    Selected,
}

/// Stable, public classification of an internal retained element.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InspectorElementKind {
    Container,
    ContainerQuery,
    Text,
    StyledText,
    Image,
    Svg,
    Path,
    Canvas,
    CustomShader,
    TextInput,
    #[cfg(target_os = "macos")]
    NativeView,
}

impl InspectorElementKind {
    pub(crate) fn from_element(kind: &ElementKind) -> Self {
        match kind {
            ElementKind::Container => Self::Container,
            ElementKind::ContainerQuery(_) => Self::ContainerQuery,
            ElementKind::Text(_) => Self::Text,
            ElementKind::StyledText(_) => Self::StyledText,
            ElementKind::Image(_) => Self::Image,
            ElementKind::Svg(_) => Self::Svg,
            ElementKind::Path(_) => Self::Path,
            ElementKind::Canvas(_) => Self::Canvas,
            ElementKind::CustomShader(_) => Self::CustomShader,
            ElementKind::TextInput(_) => Self::TextInput,
            #[cfg(target_os = "macos")]
            ElementKind::NativeView(_) => Self::NativeView,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Container => "container",
            Self::ContainerQuery => "container-query",
            Self::Text => "text",
            Self::StyledText => "styled-text",
            Self::Image => "image",
            Self::Svg => "svg",
            Self::Path => "path",
            Self::Canvas => "canvas",
            Self::CustomShader => "custom-shader",
            Self::TextInput => "text-input",
            #[cfg(target_os = "macos")]
            Self::NativeView => "native-view",
        }
    }
}

/// Exact retained hit-test metadata for an inspectable node, when that node contributes a region.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InspectorHitRegion {
    pub bounds: Rect,
    pub clip: Rect,
    pub clickable: bool,
    pub pointer_listener: bool,
    pub drag_source: bool,
    pub drop_target: bool,
    pub focusable: bool,
    pub cursor: Option<CursorStyle>,
    pub stateful: bool,
    pub blocks_pointer: bool,
    pub app_region: Option<AppRegion>,
    pub plane: ScenePlane,
    pub z_index: i16,
    pub source_order: usize,
}

impl InspectorHitRegion {
    pub(crate) fn from_region(region: HitRegion) -> Self {
        Self {
            bounds: region.bounds,
            clip: region.clip,
            clickable: region.clickable,
            pointer_listener: region.pointer_listener,
            drag_source: region.drag_source,
            drop_target: region.drop_target,
            focusable: region.focusable,
            cursor: region.cursor_style,
            stateful: region.stateful,
            blocks_pointer: region.blocks_pointer,
            app_region: region.app_region,
            plane: region.order.layer.plane,
            z_index: region.order.layer.z_index,
            source_order: region.order.source,
        }
    }
}

/// Bounded accessibility projection captured alongside an inspector node.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct InspectorAccessibility {
    pub hidden: bool,
    pub role: AccessibilityRole,
    pub label: Option<Arc<str>>,
    pub value: Option<Arc<str>>,
    pub description: Option<Arc<str>>,
    pub validation_message: Option<Arc<str>>,
    pub disabled: bool,
    pub selected: bool,
    pub toggled: Option<ToggleState>,
    pub expanded: Option<bool>,
    pub controls: Option<ElementId>,
    pub active_descendant: Option<ElementId>,
    pub labelled_by: Option<ElementId>,
    pub described_by: Option<ElementId>,
    pub described_by_secondary: Option<ElementId>,
    pub has_popover: Option<AccessibilityPopover>,
    pub auto_complete: Option<AccessibilityAutoComplete>,
    pub modal: bool,
    pub required: bool,
    pub row_count: Option<usize>,
    pub column_count: Option<usize>,
    pub row_index: Option<usize>,
    pub column_index: Option<usize>,
    pub level: Option<usize>,
    pub size_of_set: Option<usize>,
    pub position_in_set: Option<usize>,
    pub sort_direction: Option<AccessibilitySortDirection>,
    pub invalid: bool,
    pub text_truncated: bool,
}

/// One painted node from the application-owned retained tree.
#[derive(Clone, Debug, PartialEq)]
pub struct InspectorNode {
    pub id: ElementId,
    pub parent: Option<ElementId>,
    pub depth: usize,
    pub explicit_id: bool,
    pub kind: InspectorElementKind,
    pub bounds: Rect,
    pub clip: Rect,
    pub plane: ScenePlane,
    pub z_index: i16,
    pub source_order: usize,
    pub portal: bool,
    pub focused: bool,
    pub on_focus_path: bool,
    pub hit_region: Option<InspectorHitRegion>,
    pub accessibility: InspectorAccessibility,
}

impl InspectorNode {
    pub(crate) fn order(&self) -> (ScenePlane, i16, usize) {
        (self.plane, self.z_index, self.source_order)
    }

    fn contains(&self, point: Point) -> bool {
        self.bounds.contains(point) && self.clip.contains(point)
    }
}

/// Causes associated with the application frame represented by a snapshot.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct InspectorFrameDamage {
    pub view_rebuilt: bool,
    pub retained_scroll_changed: bool,
    pub variable_measurements_changed: bool,
    pub animation_requested: bool,
    pub declarative_animation_requested: bool,
    pub detached_animation_requested: bool,
    pub style_transition_requested: bool,
}

/// Bounded, owned inspector state for the most recently painted application tree.
#[derive(Clone, Debug, Default)]
pub struct InspectorSnapshot {
    pub mode: InspectorMode,
    pub nodes: Vec<InspectorNode>,
    pub selected: Option<ElementId>,
    pub pick_depth: usize,
    pub candidates_at_pointer: usize,
    pub focus_path: Vec<ElementId>,
    pub metrics: FrameMetrics,
    pub damage: InspectorFrameDamage,
    pub viewport: Size,
    pub scale_factor: f32,
    pub nodes_truncated: bool,
    pub hit_regions_truncated: bool,
}

impl InspectorSnapshot {
    fn with_capacity() -> Self {
        Self {
            nodes: Vec::with_capacity(256),
            focus_path: Vec::with_capacity(16),
            ..Self::default()
        }
    }

    pub fn selected_node(&self) -> Option<&InspectorNode> {
        let selected = self.selected?;
        self.nodes.iter().find(|node| node.id == selected)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum InspectorPointerAction {
    None,
    StartPicking,
    Close,
}

/// Window-owned implementation state. This type is itself compiled out of ordinary builds.
pub(crate) struct InspectorState {
    snapshot: InspectorSnapshot,
    overlay: UiTree,
    hit_lookup: HashMap<ElementId, HitRegion>,
    candidates: Vec<usize>,
    pointer: Option<Point>,
    pick_depth: usize,
    wheel_accumulator: f32,
    pointer_sequence_captured: bool,
    close_on_release: bool,
}

impl InspectorState {
    pub(crate) fn new(animation_epoch: web_time::Instant) -> Self {
        let mut overlay = UiTree::new_at(animation_epoch);
        overlay.set_animations_enabled(false, animation_epoch);
        overlay.set_reduce_motion(true);
        Self {
            snapshot: InspectorSnapshot::with_capacity(),
            overlay,
            hit_lookup: HashMap::with_capacity(128),
            candidates: Vec::with_capacity(32),
            pointer: None,
            pick_depth: 0,
            wheel_accumulator: 0.0,
            pointer_sequence_captured: false,
            close_on_release: false,
        }
    }

    #[cfg(any(test, feature = "test-support"))]
    pub(crate) fn snapshot(&self) -> &InspectorSnapshot {
        &self.snapshot
    }

    pub(crate) fn mode(&self) -> InspectorMode {
        self.snapshot.mode
    }

    pub(crate) fn refresh(
        &mut self,
        ui: &UiTree,
        metrics: FrameMetrics,
        damage: InspectorFrameDamage,
        viewport: Size,
        scale_factor: f32,
    ) {
        self.snapshot.metrics = metrics;
        self.snapshot.damage = damage;
        self.snapshot.viewport = viewport;
        self.snapshot.scale_factor = scale_factor;
        ui.inspector_snapshot_into(&mut self.snapshot, &mut self.hit_lookup);
        self.repick(false);
    }

    pub(crate) fn pointer_moved(&mut self, point: Point) -> bool {
        self.pointer = Some(point);
        if self.snapshot.mode != InspectorMode::Picking {
            return false;
        }
        if self.panel_bounds().contains(point) {
            return false;
        }
        self.pick_depth = 0;
        self.repick(true)
    }

    pub(crate) fn pointer_left(&mut self) -> bool {
        self.pointer = None;
        self.wheel_accumulator = 0.0;
        if self.snapshot.mode == InspectorMode::Picking {
            let changed = self.snapshot.selected.take().is_some()
                || self.snapshot.candidates_at_pointer != 0
                || self.snapshot.pick_depth != 0;
            self.candidates.clear();
            self.pick_depth = 0;
            self.snapshot.pick_depth = 0;
            self.snapshot.candidates_at_pointer = 0;
            changed
        } else {
            false
        }
    }

    pub(crate) fn captures_pointer(&self, point: Point) -> bool {
        self.pointer_sequence_captured
            || self.snapshot.mode == InspectorMode::Picking
            || self.panel_bounds().contains(point)
    }

    pub(crate) fn pointer_button(
        &mut self,
        point: Point,
        pressed: bool,
        primary: bool,
    ) -> (bool, bool, InspectorPointerAction) {
        if !primary {
            return (
                self.captures_pointer(point),
                false,
                InspectorPointerAction::None,
            );
        }
        if !pressed && self.pointer_sequence_captured {
            self.pointer_sequence_captured = false;
            let action = if self.close_on_release && self.close_button_bounds().contains(point) {
                InspectorPointerAction::Close
            } else {
                InspectorPointerAction::None
            };
            self.close_on_release = false;
            return (true, false, action);
        }
        if !pressed || !self.captures_pointer(point) {
            return (false, false, InspectorPointerAction::None);
        }
        self.pointer_sequence_captured = true;
        if self.close_button_bounds().contains(point) {
            self.close_on_release = true;
            return (true, false, InspectorPointerAction::None);
        }
        if self.pick_button_bounds().contains(point) {
            return (true, false, InspectorPointerAction::StartPicking);
        }
        if self.snapshot.mode == InspectorMode::Picking && !self.panel_bounds().contains(point) {
            self.snapshot.mode = InspectorMode::Selected;
            return (true, true, InspectorPointerAction::None);
        }
        (true, false, InspectorPointerAction::None)
    }

    pub(crate) fn start_picking(&mut self) -> bool {
        let changed = self.snapshot.mode != InspectorMode::Picking || self.pick_depth != 0;
        self.snapshot.mode = InspectorMode::Picking;
        self.pick_depth = 0;
        self.wheel_accumulator = 0.0;
        self.repick(true) | changed
    }

    pub(crate) fn scroll(&mut self, point: Point, delta: Vector) -> (bool, bool) {
        if !self.captures_pointer(point) {
            return (false, false);
        }
        if self.snapshot.mode != InspectorMode::Picking || self.panel_bounds().contains(point) {
            return (true, false);
        }
        let component = if delta.y.abs() >= delta.x.abs() {
            delta.y
        } else {
            delta.x
        };
        if !component.is_finite() || component == 0.0 {
            return (true, false);
        }
        self.wheel_accumulator = (self.wheel_accumulator + component).clamp(-1_024.0, 1_024.0);
        let mut changed = false;
        for _ in 0..MAX_INSPECTOR_WHEEL_STEPS_PER_EVENT {
            if self.wheel_accumulator.abs() < INSPECTOR_WHEEL_STEP {
                break;
            }
            let direction = self.wheel_accumulator.signum();
            self.wheel_accumulator -= direction * INSPECTOR_WHEEL_STEP;
            if !self.candidates.is_empty() {
                let previous = self.pick_depth;
                if direction < 0.0 {
                    self.pick_depth = (self.pick_depth + 1) % self.candidates.len();
                } else {
                    self.pick_depth = self
                        .pick_depth
                        .checked_sub(1)
                        .unwrap_or_else(|| self.candidates.len().saturating_sub(1));
                }
                changed |= previous != self.pick_depth;
            }
        }
        if changed {
            self.apply_candidate();
        }
        (true, changed)
    }

    pub(crate) fn panel_contains(&self, point: Point) -> bool {
        self.panel_bounds().contains(point)
    }

    pub(crate) fn paint(
        &mut self,
        scene: &mut Scene,
        renderer: &mut impl TextLayoutEngine,
        viewport: Size,
        scale_factor: f32,
        paint_time: web_time::Instant,
    ) -> Result<(), UiError> {
        let root = self.overlay_element();
        self.overlay
            .set_root_with_prepare(root, viewport, scale_factor, renderer, |_| {})?;
        self.overlay.paint_at(scene, renderer, paint_time)
    }

    fn repick(&mut self, reset_if_missing: bool) -> bool {
        let previous = self.snapshot.selected;
        self.candidates.clear();
        if self.snapshot.mode == InspectorMode::Picking
            && let Some(point) = self.pointer
            && !self.panel_bounds().contains(point)
        {
            for (index, node) in self.snapshot.nodes.iter().enumerate() {
                if node.contains(point) {
                    self.candidates.push(index);
                }
            }
            self.candidates
                .sort_unstable_by_key(|index| self.snapshot.nodes[*index].order());
            self.candidates.reverse();
            if self.pick_depth >= self.candidates.len() {
                self.pick_depth = if reset_if_missing {
                    0
                } else {
                    self.candidates.len().saturating_sub(1)
                };
            }
            self.apply_candidate();
        } else if self
            .snapshot
            .selected
            .is_some_and(|selected| !self.snapshot.nodes.iter().any(|node| node.id == selected))
        {
            self.snapshot.mode = InspectorMode::Picking;
            self.pick_depth = 0;
            self.snapshot.selected = None;
        }
        self.snapshot.pick_depth = self.pick_depth;
        self.snapshot.candidates_at_pointer = self.candidates.len();
        previous != self.snapshot.selected
    }

    fn apply_candidate(&mut self) {
        self.snapshot.selected = self
            .candidates
            .get(self.pick_depth)
            .and_then(|index| self.snapshot.nodes.get(*index))
            .map(|node| node.id);
        self.snapshot.pick_depth = self.pick_depth;
        self.snapshot.candidates_at_pointer = self.candidates.len();
    }

    fn panel_bounds(&self) -> Rect {
        let width = self
            .snapshot
            .viewport
            .width
            .clamp(0.0, INSPECTOR_PANEL_WIDTH);
        Rect::new(
            (self.snapshot.viewport.width - width).max(0.0),
            0.0,
            width,
            self.snapshot.viewport.height.max(0.0),
        )
    }

    fn close_button_bounds(&self) -> Rect {
        let panel = self.panel_bounds();
        Rect::new(
            panel.right() - INSPECTOR_HEADER_BUTTON_WIDTH,
            panel.y,
            INSPECTOR_HEADER_BUTTON_WIDTH,
            INSPECTOR_HEADER_HEIGHT,
        )
    }

    fn pick_button_bounds(&self) -> Rect {
        let panel = self.panel_bounds();
        Rect::new(
            panel.right() - INSPECTOR_HEADER_BUTTON_WIDTH * 2.0,
            panel.y,
            INSPECTOR_HEADER_BUTTON_WIDTH,
            INSPECTOR_HEADER_HEIGHT,
        )
    }

    fn overlay_element(&self) -> Element {
        let panel = self.panel_bounds();
        let mut root = crate::overlay().inset_0().size_full().z_index(i16::MAX);
        if let Some(node) = self.snapshot.selected_node()
            && let Some(visible) = node.bounds.intersection(node.clip)
        {
            root = root.child(
                crate::div()
                    .absolute()
                    .left(visible.x)
                    .top(visible.y)
                    .w(visible.width)
                    .h(visible.height)
                    .border(2.0, Color::rgb8(65, 194, 255))
                    .bg(Color::rgba8(65, 194, 255, 35)),
            );
        }

        let header = crate::div()
            .h(INSPECTOR_HEADER_HEIGHT)
            .px_3()
            .flex_row()
            .items_center()
            .justify_between()
            .border(1.0, Color::rgb8(54, 61, 73))
            .child(crate::text("QuickGUI Inspector").text_sm().font_semibold())
            .child(
                crate::div()
                    .h(INSPECTOR_HEADER_HEIGHT)
                    .flex_row()
                    .items_center()
                    .child(
                        crate::div()
                            .w(INSPECTOR_HEADER_BUTTON_WIDTH)
                            .text_center()
                            .child(crate::text("Pick").text_xs()),
                    )
                    .child(
                        crate::div()
                            .w(INSPECTOR_HEADER_BUTTON_WIDTH)
                            .text_center()
                            .child(crate::text("Close").text_xs()),
                    ),
            );

        let lines = self.panel_lines();
        let body = crate::div()
            .flex_1()
            .min_h(0.0)
            .overflow_hidden()
            .p_3()
            .flex_col()
            .gap_1()
            .children(lines.into_iter().map(|line| {
                crate::text(line)
                    .text_xs()
                    .line_height(15.0)
                    .whitespace_nowrap()
                    .text_ellipsis()
            }));

        root.child(
            crate::div()
                .absolute()
                .right(0.0)
                .top(0.0)
                .w(panel.width)
                .h(panel.height)
                .flex_col()
                .border(1.0, Color::rgb8(54, 61, 73))
                .bg(Color::rgba8(20, 23, 29, 246))
                .text_color(Color::rgb8(226, 232, 240))
                .child(header)
                .child(body),
        )
    }

    fn panel_lines(&self) -> Vec<String> {
        let mut lines = Vec::with_capacity(32);
        let mode = match self.snapshot.mode {
            InspectorMode::Picking => "picking (click to freeze; wheel changes depth)",
            InspectorMode::Selected => "selected (Pick resumes mouse picking)",
        };
        lines.push(format!("mode: {mode}"));
        lines.push(format!(
            "tree: {} nodes{} | candidates: {} | depth: {}",
            self.snapshot.nodes.len(),
            if self.snapshot.nodes_truncated || self.snapshot.hit_regions_truncated {
                " (truncated)"
            } else {
                ""
            },
            self.snapshot.candidates_at_pointer,
            self.snapshot.pick_depth,
        ));
        lines.push(format!(
            "frame: {} | cpu {:.3} ms ({:.3} avg) | wall {:.3} ms",
            self.snapshot.metrics.frame_number,
            self.snapshot.metrics.cpu_milliseconds(),
            self.snapshot.metrics.smoothed_cpu_milliseconds(),
            self.snapshot.metrics.frame_milliseconds(),
        ));
        let render = self.snapshot.metrics.render;
        lines.push(format!(
            "draw: {} calls | {} quads | {} text | {} images | {} paths",
            render.draw_calls, render.quads, render.text_areas, render.images, render.paths,
        ));
        lines.push(format!(
            "cache: text {}/{} | image gpu {} KiB cpu {} KiB | svg {} KiB",
            render.retained_text_areas,
            render.retained_text_layouts,
            render.gpu_image_cache_bytes / 1024,
            render.cpu_image_cache_bytes / 1024,
            render.gpu_svg_cache_bytes / 1024,
        ));
        let damage = self.snapshot.damage;
        lines.push(format!(
            "damage: view={} scroll={} measure={} anim={}/{}/{} transition={}",
            damage.view_rebuilt,
            damage.retained_scroll_changed,
            damage.variable_measurements_changed,
            damage.animation_requested,
            damage.declarative_animation_requested,
            damage.detached_animation_requested,
            damage.style_transition_requested,
        ));
        lines.push(String::new());

        if let Some(node) = self.snapshot.selected_node() {
            lines.push(format!(
                "selected: {:?} #{}{}",
                node.kind.label(),
                node.id.as_u64(),
                if node.explicit_id { " (explicit)" } else { "" },
            ));
            lines.push(format!(
                "bounds: x {:.1} y {:.1} w {:.1} h {:.1}",
                node.bounds.x, node.bounds.y, node.bounds.width, node.bounds.height,
            ));
            lines.push(format!(
                "clip: x {:.1} y {:.1} w {:.1} h {:.1}",
                node.clip.x, node.clip.y, node.clip.width, node.clip.height,
            ));
            lines.push(format!(
                "paint: {:?} z {} source {} | portal {}",
                node.plane, node.z_index, node.source_order, node.portal,
            ));
            lines.push(format!(
                "focus: focused={} path={} | depth {} parent {:?}",
                node.focused,
                node.on_focus_path,
                node.depth,
                node.parent.map(ElementId::as_u64),
            ));
            if let Some(hit) = node.hit_region {
                lines.push(format!(
                    "hit: click={} pointer={} focus={} block={} stateful={}",
                    hit.clickable,
                    hit.pointer_listener,
                    hit.focusable,
                    hit.blocks_pointer,
                    hit.stateful,
                ));
                lines.push(format!(
                    "hit extras: drag={}/{} cursor={:?} app-region={:?}",
                    hit.drag_source, hit.drop_target, hit.cursor, hit.app_region,
                ));
            } else {
                lines.push("hit: none".to_owned());
            }
            let accessibility = &node.accessibility;
            lines.push(format!(
                "a11y: {:?} hidden={} disabled={} selected={} toggled={:?} expanded={:?} modal={} invalid={}",
                accessibility.role,
                accessibility.hidden,
                accessibility.disabled,
                accessibility.selected,
                accessibility.toggled,
                accessibility.expanded,
                accessibility.modal,
                accessibility.invalid,
            ));
            if accessibility.controls.is_some()
                || accessibility.active_descendant.is_some()
                || accessibility.labelled_by.is_some()
                || accessibility.described_by.is_some()
                || accessibility.has_popover.is_some()
                || accessibility.auto_complete.is_some()
            {
                lines.push(format!(
                    "a11y relation: popover={:?} autocomplete={:?} controls={:?} active={:?} labelled_by={:?} described_by={:?}",
                    accessibility.has_popover,
                    accessibility.auto_complete,
                    accessibility.controls.map(ElementId::as_u64),
                    accessibility.active_descendant.map(ElementId::as_u64),
                    accessibility.labelled_by.map(ElementId::as_u64),
                    accessibility.described_by.map(ElementId::as_u64),
                ));
            }
            if accessibility.row_count.is_some()
                || accessibility.column_count.is_some()
                || accessibility.row_index.is_some()
                || accessibility.column_index.is_some()
                || accessibility.level.is_some()
                || accessibility.size_of_set.is_some()
                || accessibility.position_in_set.is_some()
                || accessibility.sort_direction.is_some()
            {
                lines.push(format!(
                    "a11y collection: rows={:?} cols={:?} row={:?} col={:?} level={:?} set={:?}/{:?} sort={:?}",
                    accessibility.row_count,
                    accessibility.column_count,
                    accessibility.row_index,
                    accessibility.column_index,
                    accessibility.level,
                    accessibility.position_in_set,
                    accessibility.size_of_set,
                    accessibility.sort_direction,
                ));
            }
            if let Some(label) = &accessibility.label {
                lines.push(format!("label: {label}"));
            }
            if let Some(value) = &accessibility.value {
                lines.push(format!("value: {value}"));
            }
            if let Some(description) = &accessibility.description {
                lines.push(format!("description: {description}"));
            }
            if let Some(message) = &accessibility.validation_message {
                lines.push(format!("validation: {message}"));
            }
            let mut hierarchy = Vec::with_capacity(node.depth.saturating_add(1).min(32));
            let mut current = Some(node.id);
            while let Some(id) = current {
                hierarchy.push(id.as_u64().to_string());
                current = self
                    .snapshot
                    .nodes
                    .iter()
                    .find(|candidate| candidate.id == id)
                    .and_then(|candidate| candidate.parent);
                if hierarchy.len() == 32 {
                    break;
                }
            }
            hierarchy.reverse();
            lines.push(format!("hierarchy: {}", hierarchy.join(" > ")));
        } else {
            lines.push("Move over application content to inspect an element.".to_owned());
        }
        lines
    }
}

pub(crate) fn bounded_text(value: &str) -> (Arc<str>, bool) {
    if value.len() <= MAX_INSPECTOR_TEXT_BYTES {
        return (Arc::from(value), false);
    }
    let mut end = MAX_INSPECTOR_TEXT_BYTES;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    (Arc::from(&value[..end]), true)
}

pub(crate) fn inspector_accessibility(element: &Element) -> InspectorAccessibility {
    let mut truncated = false;
    let (label, label_truncated) = inspector_accessibility_label(element);
    truncated |= label_truncated;
    let mut bound = |value: Option<&Arc<str>>| {
        value.map(|value| {
            let (value, was_truncated) = bounded_text(value);
            truncated |= was_truncated;
            value
        })
    };
    let value = bound(element.accessibility.value.as_ref());
    let description = bound(element.accessibility.description.as_ref());
    let validation_message = bound(element.accessibility.validation_message.as_ref());
    truncated |= element.accessibility.validation_message_truncated;
    InspectorAccessibility {
        hidden: element.accessibility.hidden,
        role: element.accessibility.role,
        label,
        value,
        description,
        validation_message,
        disabled: element.accessibility.disabled,
        selected: element.accessibility.selected,
        toggled: element.accessibility.toggled,
        expanded: element.accessibility.expanded,
        controls: element.accessibility.relations.controls(),
        active_descendant: element.accessibility.relations.active_descendant(),
        labelled_by: element.accessibility.relations.labelled_by(),
        described_by: element.accessibility.relations.described_by(),
        described_by_secondary: element.accessibility.relations.described_by_secondary(),
        has_popover: element.accessibility.has_popover,
        auto_complete: element.accessibility.auto_complete,
        modal: element.accessibility.modal,
        required: element.accessibility.required,
        row_count: element.accessibility.collection.row_count(),
        column_count: element.accessibility.collection.column_count(),
        row_index: element.accessibility.collection.row_index(),
        column_index: element.accessibility.collection.column_index(),
        level: element.accessibility.collection.level(),
        size_of_set: element.accessibility.collection.size_of_set(),
        position_in_set: element.accessibility.collection.position_in_set(),
        sort_direction: element.accessibility.collection.sort_direction,
        invalid: element.accessibility.invalid,
        text_truncated: truncated,
    }
}

fn inspector_accessibility_label(element: &Element) -> (Option<Arc<str>>, bool) {
    if let Some(label) = &element.accessibility.label {
        let (label, truncated) = bounded_text(label);
        return (Some(label), truncated);
    }
    match &element.kind {
        ElementKind::Text(content) => {
            let (label, truncated) = bounded_text(content);
            return (Some(label), truncated);
        }
        ElementKind::StyledText(content) => {
            let (label, truncated) = bounded_text(content.content());
            return (Some(label), truncated);
        }
        _ if element.accessibility.role == AccessibilityRole::GenericContainer => {
            return (None, false);
        }
        _ => {}
    }

    let mut label = String::with_capacity(MAX_INSPECTOR_TEXT_BYTES.min(128));
    let mut truncated = false;
    collect_inspector_text(element, &mut label, &mut truncated);
    ((!label.is_empty()).then(|| Arc::from(label)), truncated)
}

fn collect_inspector_text(element: &Element, output: &mut String, truncated: &mut bool) {
    if *truncated || element.is_display_none() || element.is_visibility_hidden() {
        return;
    }
    let content = match &element.kind {
        ElementKind::Text(content) => Some(content.as_ref()),
        ElementKind::StyledText(content) => Some(content.content().as_ref()),
        _ => None,
    };
    if let Some(content) = content {
        if !output.is_empty() && output.len() < MAX_INSPECTOR_TEXT_BYTES {
            output.push(' ');
        }
        let remaining = MAX_INSPECTOR_TEXT_BYTES.saturating_sub(output.len());
        if content.len() > remaining {
            let mut end = remaining;
            while !content.is_char_boundary(end) {
                end -= 1;
            }
            output.push_str(&content[..end]);
            *truncated = true;
            return;
        }
        output.push_str(content);
    }
    for child in &element.children {
        collect_inspector_text(child, output, truncated);
        if *truncated {
            return;
        }
    }
}

pub(crate) fn child_clip(
    element: &Element,
    parent_clip: Rect,
    padding_bounds: Rect,
) -> Option<Rect> {
    let clips_children = element.layout.overflow.x != taffy::Overflow::Visible
        || element.layout.overflow.y != taffy::Overflow::Visible;
    if clips_children {
        parent_clip.intersection(padding_bounds)
    } else {
        Some(parent_clip)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: u64, plane: ScenePlane, z_index: i16, source_order: usize) -> InspectorNode {
        InspectorNode {
            id: ElementId::new(id),
            parent: None,
            depth: 0,
            explicit_id: true,
            kind: InspectorElementKind::Container,
            bounds: Rect::new(0.0, 0.0, 100.0, 100.0),
            clip: Rect::new(0.0, 0.0, 100.0, 100.0),
            plane,
            z_index,
            source_order,
            portal: plane == ScenePlane::Overlay,
            focused: false,
            on_focus_path: false,
            hit_region: None,
            accessibility: InspectorAccessibility::default(),
        }
    }

    #[test]
    fn picking_uses_real_paint_order_and_wheel_cycles_occluded_nodes() {
        let mut inspector = InspectorState::new(web_time::Instant::now());
        inspector.snapshot.viewport = Size::new(500.0, 400.0);
        inspector.snapshot.nodes = vec![
            node(1, ScenePlane::Base, 10, 100),
            node(2, ScenePlane::Overlay, -10, 1),
        ];

        assert!(inspector.pointer_moved(Point::new(20.0, 20.0)));
        assert_eq!(inspector.snapshot.selected, Some(ElementId::new(2)));
        assert_eq!(inspector.snapshot.candidates_at_pointer, 2);

        let (consumed, changed) = inspector.scroll(Point::new(20.0, 20.0), Vector::new(0.0, -18.0));
        assert!(consumed && changed);
        assert_eq!(inspector.snapshot.selected, Some(ElementId::new(1)));
        assert_eq!(inspector.snapshot.pick_depth, 1);

        let (consumed, changed, action) =
            inspector.pointer_button(Point::new(20.0, 20.0), true, true);
        assert!(consumed && changed);
        assert_eq!(action, InspectorPointerAction::None);
        assert_eq!(inspector.mode(), InspectorMode::Selected);
        assert!(
            inspector
                .pointer_button(Point::new(20.0, 20.0), false, true)
                .0
        );
        assert!(!inspector.captures_pointer(Point::new(20.0, 20.0)));
    }

    #[test]
    fn panel_actions_are_bounded_to_the_visible_header() {
        let mut inspector = InspectorState::new(web_time::Instant::now());
        inspector.snapshot.viewport = Size::new(500.0, 400.0);
        assert_eq!(
            inspector
                .pointer_button(Point::new(430.0, 20.0), true, true)
                .2,
            InspectorPointerAction::StartPicking,
        );
        assert!(
            inspector
                .pointer_button(Point::new(430.0, 20.0), false, true)
                .0
        );
        assert_eq!(
            inspector
                .pointer_button(Point::new(480.0, 20.0), true, true)
                .2,
            InspectorPointerAction::None,
        );
        assert_eq!(
            inspector
                .pointer_button(Point::new(480.0, 20.0), false, true)
                .2,
            InspectorPointerAction::Close,
        );
    }

    #[test]
    fn inspector_text_limit_preserves_utf8_boundaries() {
        let input = "a".repeat(MAX_INSPECTOR_TEXT_BYTES - 1) + "é";
        let (bounded, truncated) = bounded_text(&input);
        assert!(truncated);
        assert_eq!(bounded.len(), MAX_INSPECTOR_TEXT_BYTES - 1);
        assert!(std::str::from_utf8(bounded.as_bytes()).is_ok());

        let element = crate::div()
            .accessibility_role(AccessibilityRole::Button)
            .child(crate::text("界".repeat(MAX_INSPECTOR_TEXT_BYTES)));
        let accessibility = inspector_accessibility(&element);
        let label = accessibility.label.expect("derived button label");
        assert!(accessibility.text_truncated);
        assert!(label.len() <= MAX_INSPECTOR_TEXT_BYTES);
        assert!(std::str::from_utf8(label.as_bytes()).is_ok());

        let toggle = inspector_accessibility(&crate::checkbox(ToggleState::Mixed));
        assert_eq!(toggle.role, AccessibilityRole::CheckBox);
        assert_eq!(toggle.toggled, Some(ToggleState::Mixed));

        let popover = crate::Popover::new("trigger", "surface", true).trigger();
        let accessibility = inspector_accessibility(&popover);
        assert_eq!(accessibility.expanded, Some(true));
        assert_eq!(accessibility.controls, Some("surface".into()));
        assert_eq!(
            accessibility.has_popover,
            Some(AccessibilityPopover::Dialog)
        );

        let group = inspector_accessibility(
            &crate::div()
                .accessibility_role(AccessibilityRole::Group)
                .accessibility_labelled_by("group-label")
                .accessibility_described_by("group-description")
                .accessibility_modal(true)
                .accessibility_hidden(true),
        );
        assert_eq!(group.labelled_by, Some("group-label".into()));
        assert_eq!(group.described_by, Some("group-description".into()));
        assert!(group.modal);
        assert!(group.hidden);
    }
}
