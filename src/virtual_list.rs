use std::{
    cell::RefCell,
    collections::BTreeMap,
    fmt,
    mem::size_of,
    ops::Range,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicU32, AtomicUsize, Ordering},
    },
};

use crate::{Element, ElementId, IntoElement, Rect, Size, div};

/// The visible slice of a [`VirtualList`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VisibleRows {
    pub range: Range<usize>,
}

impl VisibleRows {
    pub fn len(&self) -> usize {
        self.range.len()
    }

    pub fn is_empty(&self) -> bool {
        self.range.is_empty()
    }
}

/// Constant-height virtual scrolling with bounded memory and O(1) range calculation.
#[derive(Debug)]
pub struct VirtualList {
    len: usize,
    row_height: f32,
    overscan: usize,
    viewport_height: f32,
    scroll_offset: Arc<AtomicU32>,
}

/// Shared offset storage used by a retained virtual-scroll viewport.
///
/// This stays crate-private: applications bind a [`VirtualList`] directly with
/// [`crate::Element::virtual_scroll`].
#[derive(Clone)]
pub(crate) enum VirtualScrollHandle {
    Fixed(Arc<AtomicU32>),
    Variable(ListScrollHandle),
}

/// Geometry captured when a virtual-list declaration mounts its current slice.
///
/// Input may move within `content_range` without rebuilding that declaration: paint translates
/// the retained slice by the difference from `layout_offset_y`. Once the viewport leaves the
/// mounted coverage, the view rebuilds to replace the slice.
#[derive(Clone, Debug)]
pub(crate) struct VirtualScrollMount {
    pub(crate) layout_offset_y: f32,
    content_range: Option<Range<f32>>,
}

impl VirtualScrollMount {
    fn new(layout_offset_y: f32, content_range: Option<Range<f32>>) -> Self {
        Self {
            layout_offset_y,
            content_range,
        }
    }

    pub(crate) fn retains_viewport(&self, offset_y: f32, viewport_height: f32) -> bool {
        let Some(content) = &self.content_range else {
            return false;
        };
        let viewport_height = sane_list_dimension(viewport_height);
        let viewport_bottom = offset_y + viewport_height;
        offset_y + LIST_MEASUREMENT_EPSILON >= content.start
            && viewport_bottom <= content.end + LIST_MEASUREMENT_EPSILON
    }
}

impl VirtualScrollHandle {
    pub(crate) fn offset(&self) -> f32 {
        match self {
            Self::Fixed(offset) => f32::from_bits(offset.load(Ordering::Relaxed)),
            Self::Variable(handle) => handle.offset(),
        }
    }

    /// Content may arrive after input in a hosted collection. Keep drawing supplied rows while
    /// preserving the requested offset for subsequent input and range notifications.
    pub(crate) fn presented_offset(&self, requested: f32) -> f32 {
        match self {
            Self::Fixed(_) => requested,
            Self::Variable(handle) => handle.0.borrow().presented_offset(requested),
        }
    }

    pub(crate) fn retains_viewport(
        &self,
        mount: &VirtualScrollMount,
        offset: f32,
        height: f32,
    ) -> bool {
        match self {
            Self::Fixed(_) => mount.retains_viewport(offset, height),
            Self::Variable(handle) => handle.0.borrow().retains_viewport(mount, offset, height),
        }
    }

    pub(crate) fn set_offset_from_input(&self, offset: f32) {
        debug_assert!(offset.is_finite() && offset >= 0.0);
        match self {
            Self::Fixed(current) => current.store(offset.to_bits(), Ordering::Relaxed),
            Self::Variable(handle) => handle.set_offset_from_input(offset),
        }
    }

    pub(crate) fn set_offset_silent(&self, offset: f32) {
        debug_assert!(offset.is_finite() && offset >= 0.0);
        match self {
            Self::Fixed(current) => current.store(offset.to_bits(), Ordering::Relaxed),
            Self::Variable(handle) => handle.set_offset_silent(offset),
        }
    }

    pub(crate) fn measurement_revision(&self) -> u64 {
        match self {
            Self::Fixed(_) => 0,
            Self::Variable(handle) => handle.measurement_revision(),
        }
    }

    pub(crate) fn max_offset(&self, declared: f32) -> f32 {
        match self {
            Self::Fixed(_) => declared,
            Self::Variable(handle) => handle.max_offset(),
        }
    }

    pub(crate) fn report_viewport(&self, size: Size) -> bool {
        match self {
            Self::Fixed(_) => false,
            Self::Variable(handle) => handle.report_viewport(size),
        }
    }

    pub(crate) fn scrollbar_drag_started(&self) {
        if let Self::Variable(handle) = self {
            handle.scrollbar_drag_started();
        }
    }

    pub(crate) fn scrollbar_drag_ended(&self) -> bool {
        match self {
            Self::Fixed(_) => false,
            Self::Variable(handle) => handle.scrollbar_drag_ended(),
        }
    }

    /// Refresh measured mounted coverage while preserving the declaration's layout offset.
    /// Returns whether the currently mounted slice still covers the current viewport.
    pub(crate) fn refresh_mount_after_measurement(&self, mount: &mut VirtualScrollMount) -> bool {
        match self {
            Self::Fixed(_) => true,
            Self::Variable(handle) => handle.refresh_mount_after_measurement(mount),
        }
    }
}

impl fmt::Debug for VirtualScrollHandle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("VirtualScrollHandle")
            .field(&self.offset())
            .finish()
    }
}

impl Clone for VirtualList {
    fn clone(&self) -> Self {
        // Preserve VirtualList's value-like clone semantics. Element bindings clone only the
        // private handle above, while cloning the list itself starts an independent scroll state.
        Self {
            len: self.len,
            row_height: self.row_height,
            overscan: self.overscan,
            viewport_height: self.viewport_height,
            scroll_offset: Arc::new(AtomicU32::new(self.scroll_offset().to_bits())),
        }
    }
}

impl VirtualList {
    pub fn new(len: usize, row_height: f32) -> Self {
        assert!(
            row_height.is_finite() && row_height > 0.0,
            "row height must be positive"
        );
        Self {
            len,
            row_height,
            overscan: 2,
            viewport_height: 0.0,
            scroll_offset: Arc::new(AtomicU32::new(0.0_f32.to_bits())),
        }
    }

    pub fn with_overscan(mut self, rows: usize) -> Self {
        self.overscan = rows;
        self
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn row_height(&self) -> f32 {
        self.row_height
    }

    pub fn content_height(&self) -> f32 {
        self.row_height * self.len as f32
    }

    pub fn viewport_height(&self) -> f32 {
        self.viewport_height
    }

    pub fn set_viewport_height(&mut self, height: f32) -> bool {
        let height = if height.is_finite() {
            height.max(0.0)
        } else {
            0.0
        };
        let changed = self.viewport_height != height;
        self.viewport_height = height;
        self.clamp_offset();
        changed
    }

    pub fn set_len(&mut self, len: usize) -> bool {
        let changed = self.len != len;
        self.len = len;
        self.clamp_offset();
        changed
    }

    pub fn scroll_offset(&self) -> f32 {
        f32::from_bits(self.scroll_offset.load(Ordering::Relaxed))
    }

    pub fn max_scroll_offset(&self) -> f32 {
        (self.content_height() - self.viewport_height).max(0.0)
    }

    pub fn scroll_by(&mut self, delta: f32) -> bool {
        self.scroll_to(self.scroll_offset() + delta)
    }

    pub fn scroll_to(&mut self, offset: f32) -> bool {
        let old = self.scroll_offset();
        let next = if offset.is_finite() {
            offset.clamp(0.0, self.max_scroll_offset())
        } else {
            old
        };
        if old == next {
            false
        } else {
            self.scroll_offset.store(next.to_bits(), Ordering::Relaxed);
            true
        }
    }

    /// Scroll the smallest distance needed to expose one complete row.
    pub fn scroll_to_reveal(&mut self, index: usize) -> bool {
        if index >= self.len || self.viewport_height <= 0.0 {
            return false;
        }
        let top = index as f32 * self.row_height;
        let bottom = top + self.row_height;
        let viewport_top = self.scroll_offset();
        let viewport_bottom = viewport_top + self.viewport_height;
        if top < viewport_top {
            self.scroll_to(top)
        } else if bottom > viewport_bottom {
            self.scroll_to(bottom - self.viewport_height)
        } else {
            false
        }
    }

    pub fn visible_rows(&self) -> VisibleRows {
        if self.len == 0 || self.viewport_height <= 0.0 {
            return VisibleRows { range: 0..0 };
        }

        let scroll_offset = self.scroll_offset();
        let first = (scroll_offset / self.row_height).floor() as usize;
        let visible_end =
            ((scroll_offset + self.viewport_height) / self.row_height).ceil() as usize;
        VisibleRows {
            range: first.saturating_sub(self.overscan)
                ..visible_end.saturating_add(self.overscan).min(self.len),
        }
    }

    /// The logical rectangle for a row inside a viewport whose top is `viewport_y`.
    pub fn row_rect(&self, index: usize, viewport_x: f32, viewport_y: f32, width: f32) -> Rect {
        debug_assert!(index < self.len);
        Rect::new(
            viewport_x,
            viewport_y + index as f32 * self.row_height - self.scroll_offset(),
            width,
            self.row_height,
        )
    }

    /// Returns `(thumb_offset, thumb_height)` in viewport-local coordinates.
    pub fn scrollbar_thumb(&self, minimum_height: f32) -> Option<(f32, f32)> {
        let content = self.content_height();
        if content <= self.viewport_height || self.viewport_height <= 0.0 {
            return None;
        }
        let thumb_height = (self.viewport_height * self.viewport_height / content)
            .max(minimum_height)
            .min(self.viewport_height);
        let travel = self.viewport_height - thumb_height;
        let offset = travel * (self.scroll_offset() / self.max_scroll_offset());
        Some((offset, thumb_height))
    }

    fn clamp_offset(&mut self) {
        let offset = self.scroll_offset().clamp(0.0, self.max_scroll_offset());
        self.scroll_offset
            .store(offset.to_bits(), Ordering::Relaxed);
    }

    pub(crate) fn scroll_handle(&self) -> VirtualScrollHandle {
        VirtualScrollHandle::Fixed(Arc::clone(&self.scroll_offset))
    }

    pub(crate) fn scroll_mount(&self) -> VirtualScrollMount {
        let range = self.visible_rows().range;
        let content_range = (!range.is_empty())
            .then_some(range.start as f32 * self.row_height..range.end as f32 * self.row_height);
        VirtualScrollMount::new(self.scroll_offset(), content_range)
    }
}

/// Maximum number of items retained by one variable-height [`ListState`].
///
/// Height metadata is chunked, so an entirely unmeasured million-item list retains only its
/// compact block index. Fully measuring the maximum still has a hard finite bound.
pub const MAX_LIST_ITEMS: usize = 1_000_000;

/// Maximum number of extra items mounted on either side of a variable-height viewport.
pub const MAX_LIST_OVERSCAN_ITEMS: usize = 1_024;

/// Maximum number of variable-height items one declaration may mount.
///
/// A sane estimate and QuickGUI's bounded window dimensions keep normal viewports far below this
/// value. The limit remains a final guard against a zero-height data set or an accidental full-list
/// range.
pub const MAX_MOUNTED_LIST_ITEMS: usize = 16_384;

const LIST_HEIGHT_BLOCK_ITEMS: usize = 64;
const DEFAULT_UNSIZED_LIST_ITEMS: usize = 16;
const LIST_MEASUREMENT_EPSILON: f32 = 1.0 / 64.0;
const MAX_LIST_ITEM_HEIGHT: f32 = 1_048_576.0;

static NEXT_LIST_ID: AtomicUsize = AtomicUsize::new(1);

/// The edge from which a variable-height list is naturally anchored.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum ListAlignment {
    /// Ordinary top-to-bottom content.
    #[default]
    Top,
    /// Bottom-aligned content such as a chat transcript.
    Bottom,
}

/// Whether content appended at the end should continue following the tail.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum FollowMode {
    /// Preserve the current logical item and pixel inset as content changes.
    #[default]
    Normal,
    /// Follow appended or growing tail content until the user scrolls away.
    Tail,
}

/// A stable logical position inside a variable-height list.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ListOffset {
    /// The item containing the top of the viewport. `item_ix == item_count` denotes the end.
    pub item_ix: usize,
    /// Logical pixels hidden above the viewport within `item_ix`.
    pub offset_in_item: f32,
}

/// Retained variable-list metric usage for diagnostics and regression tests.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ListStateStats {
    /// Items whose real laid-out height has replaced the estimate.
    pub measured_items: usize,
    /// Sparse 64-item metric blocks currently retained.
    pub measured_blocks: usize,
    /// Payload bytes retained by the sparse height blocks and compact block prefix index.
    pub retained_height_bytes: usize,
}

/// Intrusive state for efficiently rendering differently sized list items.
///
/// Store this on the owning view and bind it with [`crate::Element::variable_virtual_scroll`].
/// [`Self::render_rows`] mounts only a [`Self::visible_rows`] range in a normal Flexbox column, so
/// wrapped items stack using their real height in the same frame. The retained tree feeds those
/// heights back into this state and requests at most one correcting view rebuild; clean lists add
/// no frame deadline or idle work.
///
/// Clones share one scroll and measurement state, matching GPUI's intrusive `ListState` model. One
/// shared state should therefore be mounted as one list at a time.
#[derive(Clone)]
pub struct ListState(Rc<RefCell<ListStateInner>>);

impl fmt::Debug for ListState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state = self.0.borrow();
        formatter
            .debug_struct("ListState")
            .field("item_count", &state.metrics.len)
            .field("estimated_item_height", &state.metrics.estimate)
            .field("viewport", &state.viewport)
            .field("scroll_offset", &state.scroll_offset)
            .field("alignment", &state.alignment)
            .field("follow_mode", &state.follow_mode)
            .finish()
    }
}

#[derive(Clone)]
pub(crate) struct ListScrollHandle(Rc<RefCell<ListStateInner>>);

impl fmt::Debug for ListScrollHandle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("ListScrollHandle")
            .field(&self.offset())
            .finish()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ListItemMeasurement {
    handle: ListScrollHandle,
    index: usize,
    generation: u64,
}

impl ListItemMeasurement {
    pub(crate) fn report_height(&self, height: f32) -> bool {
        self.handle
            .report_item_height(self.index, self.generation, height)
    }
}

#[derive(Clone)]
struct ListStateInner {
    overscan_pixels: f32,
    id: usize,
    metrics: HeightMetrics,
    overscan: usize,
    viewport: Size,
    scroll_offset: f32,
    alignment: ListAlignment,
    follow_mode: FollowMode,
    following_tail: bool,
    anchor_end: bool,
    generation: u64,
    measurement_revision: u64,
    scrollbar_drag_max_offset: Option<f32>,
    mounted_range: Option<Range<usize>>,
    available_range: Option<Range<usize>>,
}

#[derive(Clone, Copy)]
struct ItemAnchor {
    index: usize,
    inset: f32,
}

impl ListStateInner {
    fn presented_offset(&self, requested: f32) -> f32 {
        if self.available_range.is_none() {
            return requested;
        }
        let Some(range) = &self.mounted_range else {
            return requested;
        };
        let start = self.metrics.item_top(range.start.min(self.metrics.len));
        let end = self.metrics.item_top(range.end.min(self.metrics.len));
        let minimum = start.min(self.actual_max_scroll_offset());
        let maximum = (end - self.viewport.height).max(minimum);
        requested.clamp(minimum, maximum)
    }

    fn retains_viewport(&self, mount: &VirtualScrollMount, offset: f32, height: f32) -> bool {
        if self.available_range.is_none() {
            return mount.retains_viewport(offset, height);
        }
        // Refill before the viewport consumes the supplied buffer. The remaining half viewport
        // lets ordinary scrolling continue while the frontend handles the asynchronous request.
        let margin = height * 0.5;
        let start = (offset - margin).max(0.0);
        let end = (offset + height + margin).min(self.metrics.content_height());
        mount.retains_viewport(start, (end - start).max(0.0))
    }

    fn actual_max_scroll_offset(&self) -> f32 {
        (self.metrics.content_height() - self.viewport.height).max(0.0)
    }

    fn effective_max_scroll_offset(&self) -> f32 {
        self.scrollbar_drag_max_offset
            .unwrap_or_else(|| self.actual_max_scroll_offset())
            .max(0.0)
    }

    fn capture_anchor(&self) -> Option<ItemAnchor> {
        if self.metrics.len == 0 || self.anchor_end {
            return None;
        }
        let index = self.metrics.item_at_offset(self.scroll_offset);
        Some(ItemAnchor {
            index,
            inset: (self.scroll_offset - self.metrics.item_top(index)).max(0.0),
        })
    }

    fn restore_anchor(&mut self, anchor: Option<ItemAnchor>) {
        if self.metrics.len == 0 {
            self.scroll_offset = 0.0;
            return;
        }
        let maximum = self.effective_max_scroll_offset();
        self.scroll_offset = if self.anchor_end || self.following_tail {
            maximum
        } else if let Some(anchor) = anchor {
            let index = anchor.index.min(self.metrics.len - 1);
            let inset = anchor.inset.min(self.metrics.item_height(index));
            (self.metrics.item_top(index) + inset).clamp(0.0, maximum)
        } else {
            self.scroll_offset.clamp(0.0, maximum)
        };
    }

    fn bump_measurement_revision(&mut self) {
        self.measurement_revision = self.measurement_revision.wrapping_add(1);
    }

    fn report_viewport(&mut self, size: Size) -> bool {
        let mut next = Size::new(
            sane_list_dimension(size.width),
            sane_list_dimension(size.height),
        );
        let width_changed = (next.width - self.viewport.width).abs() > LIST_MEASUREMENT_EPSILON;
        let height_changed = (next.height - self.viewport.height).abs() > LIST_MEASUREMENT_EPSILON;
        if !width_changed && !height_changed {
            return false;
        }
        if !width_changed {
            next.width = self.viewport.width;
        }
        if !height_changed {
            next.height = self.viewport.height;
        }
        let anchor = self.capture_anchor();
        let mounted = self.visible_rows().range;
        self.viewport = next;
        if width_changed {
            // Rows in the current declaration are about to be laid out at the new width, so keep
            // their measurements and let their existing handles replace them during this paint.
            // Discard only offscreen measurements, whose wrapped height can no longer be trusted.
            // This avoids an estimate-only intermediate view rebuild on every resize step.
            self.metrics.clear_range(0..mounted.start);
            self.metrics.clear_range(mounted.end..self.metrics.len);
        }
        self.restore_anchor(anchor);
        if self.visible_rows().range != mounted {
            self.bump_measurement_revision();
        }
        true
    }

    fn report_item_height(&mut self, index: usize, generation: u64, height: f32) -> bool {
        if generation != self.generation || index >= self.metrics.len || !height.is_finite() {
            return false;
        }
        let height = height.clamp(0.0, MAX_LIST_ITEM_HEIGHT);
        if (self.metrics.item_height(index) - height).abs() <= LIST_MEASUREMENT_EPSILON {
            return false;
        }
        let anchor = self.capture_anchor();
        if !self.metrics.set_item_height(index, height) {
            return false;
        }
        self.restore_anchor(anchor);
        self.bump_measurement_revision();
        true
    }

    fn set_offset_from_input(&mut self, offset: f32) {
        let maximum = self.effective_max_scroll_offset();
        let next = if offset.is_finite() {
            offset.clamp(0.0, maximum)
        } else {
            self.scroll_offset
        };
        self.scroll_offset = next;
        let at_end = maximum <= LIST_MEASUREMENT_EPSILON
            || (maximum - next).abs() <= LIST_MEASUREMENT_EPSILON;
        match self.follow_mode {
            FollowMode::Normal => self.following_tail = false,
            FollowMode::Tail => self.following_tail = at_end,
        }
        self.anchor_end =
            at_end && (self.following_tail || matches!(self.alignment, ListAlignment::Bottom));
    }

    fn set_offset_silent(&mut self, offset: f32) {
        if offset.is_finite() {
            self.scroll_offset = offset.clamp(0.0, self.effective_max_scroll_offset());
        }
    }

    fn content_origin_at(&self, first_item: usize, offset: f32) -> f32 {
        let content_height = self.metrics.content_height();
        let bottom_inset =
            if self.alignment == ListAlignment::Bottom && content_height < self.viewport.height {
                self.viewport.height - content_height
            } else {
                0.0
            };
        bottom_inset + self.metrics.item_top(first_item) - offset
    }

    fn visible_rows(&self) -> VisibleRows {
        if self.metrics.len == 0 {
            return VisibleRows { range: 0..0 };
        }
        if self.viewport.height <= 0.0 {
            let count = DEFAULT_UNSIZED_LIST_ITEMS
                .saturating_add(self.overscan)
                .min(MAX_MOUNTED_LIST_ITEMS)
                .min(self.metrics.len);
            let range = match self.alignment {
                ListAlignment::Top => 0..count,
                ListAlignment::Bottom => self.metrics.len - count..self.metrics.len,
            };
            return VisibleRows { range };
        }

        let first = self
            .metrics
            .item_at_offset((self.scroll_offset - self.overscan_pixels).max(0.0));
        let viewport_bottom = (self.scroll_offset + self.viewport.height + self.overscan_pixels)
            .min(self.metrics.content_height());
        let visible_end = if viewport_bottom >= self.metrics.content_height() {
            self.metrics.len
        } else {
            self.metrics
                .item_at_offset((viewport_bottom - LIST_MEASUREMENT_EPSILON).max(0.0))
                .saturating_add(1)
                .min(self.metrics.len)
        };
        let overscan = if self.available_range.is_some() {
            self.overscan
                .max(visible_end.saturating_sub(first))
                .min(MAX_LIST_OVERSCAN_ITEMS)
        } else {
            self.overscan
        };
        let start = first.saturating_sub(overscan);
        let end = visible_end.saturating_add(overscan).min(self.metrics.len);
        VisibleRows {
            range: start..end.min(start.saturating_add(MAX_MOUNTED_LIST_ITEMS)),
        }
    }
}

impl ListScrollHandle {
    fn offset(&self) -> f32 {
        self.0.borrow().scroll_offset
    }

    fn set_offset_from_input(&self, offset: f32) {
        self.0.borrow_mut().set_offset_from_input(offset);
    }

    fn set_offset_silent(&self, offset: f32) {
        self.0.borrow_mut().set_offset_silent(offset);
    }

    fn measurement_revision(&self) -> u64 {
        self.0.borrow().measurement_revision
    }

    fn max_offset(&self) -> f32 {
        self.0.borrow().effective_max_scroll_offset()
    }

    fn report_viewport(&self, size: Size) -> bool {
        self.0.borrow_mut().report_viewport(size)
    }

    fn report_item_height(&self, index: usize, generation: u64, height: f32) -> bool {
        self.0
            .borrow_mut()
            .report_item_height(index, generation, height)
    }

    fn scrollbar_drag_started(&self) {
        let mut state = self.0.borrow_mut();
        if state.scrollbar_drag_max_offset.is_none() {
            state.scrollbar_drag_max_offset = Some(state.actual_max_scroll_offset());
        }
    }

    fn scrollbar_drag_ended(&self) -> bool {
        let mut state = self.0.borrow_mut();
        let Some(frozen) = state.scrollbar_drag_max_offset.take() else {
            return false;
        };
        let actual = state.actual_max_scroll_offset();
        state.scroll_offset = state.scroll_offset.clamp(0.0, actual);
        let changed = (frozen - actual).abs() > LIST_MEASUREMENT_EPSILON;
        if changed {
            state.bump_measurement_revision();
        }
        changed
    }

    fn refresh_mount_after_measurement(&self, mount: &mut VirtualScrollMount) -> bool {
        let state = self.0.borrow();
        mount.content_range = state.mounted_range.as_ref().and_then(|range| {
            (!range.is_empty())
                .then(|| state.metrics.item_top(range.start)..state.metrics.item_top(range.end))
        });
        state.retains_viewport(mount, state.scroll_offset, state.viewport.height)
    }
}

impl ListState {
    /// Create top-aligned variable-height list state with one uniform offscreen estimate.
    pub fn new(item_count: usize, estimated_item_height: f32) -> Self {
        assert!(
            item_count <= MAX_LIST_ITEMS,
            "a variable list retains at most {MAX_LIST_ITEMS} items"
        );
        assert!(
            estimated_item_height.is_finite() && estimated_item_height > 0.0,
            "estimated item height must be positive"
        );
        let id = NEXT_LIST_ID.fetch_add(1, Ordering::Relaxed);
        Self(Rc::new(RefCell::new(ListStateInner {
            overscan_pixels: 0.0,
            id,
            metrics: HeightMetrics::new(item_count, estimated_item_height),
            overscan: 2,
            viewport: Size::ZERO,
            scroll_offset: 0.0,
            alignment: ListAlignment::Top,
            follow_mode: FollowMode::Normal,
            following_tail: false,
            anchor_end: false,
            generation: 0,
            measurement_revision: 0,
            scrollbar_drag_max_offset: None,
            mounted_range: None,
            available_range: None,
        })))
    }

    /// Set the natural list edge. Bottom alignment keeps short content against the lower edge.
    pub fn with_alignment(self, alignment: ListAlignment) -> Self {
        {
            let mut state = self.0.borrow_mut();
            if state.alignment != alignment {
                state.alignment = alignment;
                state.anchor_end = alignment == ListAlignment::Bottom;
                let anchor = state.capture_anchor();
                state.restore_anchor(anchor);
                state.bump_measurement_revision();
            }
        }
        self
    }

    pub fn with_overscan(self, items: usize) -> Self {
        self.0.borrow_mut().overscan = items.min(MAX_LIST_OVERSCAN_ITEMS);
        self
    }

    /// Extend the mounted range by measured logical distance rather than an estimated row count.
    pub fn with_overscan_pixels(self, pixels: f32) -> Self {
        self.0.borrow_mut().overscan_pixels = if pixels.is_finite() {
            pixels.clamp(0.0, 1_048_576.0)
        } else {
            0.0
        };
        self
    }

    pub fn with_follow_mode(self, mode: FollowMode) -> Self {
        self.set_follow_mode(mode);
        self
    }

    pub fn item_count(&self) -> usize {
        self.0.borrow().metrics.len
    }

    pub fn is_empty(&self) -> bool {
        self.item_count() == 0
    }

    pub fn estimated_item_height(&self) -> f32 {
        self.0.borrow().metrics.estimate
    }

    pub fn viewport_size(&self) -> Size {
        self.0.borrow().viewport
    }

    /// Seed or update the list viewport before rendering visible rows.
    ///
    /// The retained tree also reports the final laid-out size, so calling this is optional. Seeding
    /// it from `ViewContext::size()` avoids a one-rebuild estimate when the list's size is known.
    pub fn set_viewport_size(&self, width: f32, height: f32) -> bool {
        self.0
            .borrow_mut()
            .report_viewport(Size::new(width, height))
    }

    pub fn set_viewport_height(&self, height: f32) -> bool {
        let width = self.0.borrow().viewport.width;
        self.set_viewport_size(width, height)
    }

    /// Supply exact row heights when a document's metrics determine them without layout.
    /// Updating known heights preserves the current logical scroll anchor.
    pub fn set_item_heights(&self, heights: &[f32]) {
        assert!(heights.iter().all(|h| h.is_finite() && *h > 0.0));
        let mut state = self.0.borrow_mut();
        assert_eq!(heights.len(), state.metrics.len);
        let anchor = state.capture_anchor();
        let mut changed = false;
        for (i, height) in heights.iter().enumerate() {
            changed |= state.metrics.set_item_height(i, *height);
        }
        if changed {
            state.restore_anchor(anchor);
            state.bump_measurement_revision();
        }
    }

    /// Update the item count while retaining measurements for unchanged prefix items.
    ///
    /// Use [`Self::reset`] instead when the item identities or contents were replaced.
    pub fn set_item_count(&self, item_count: usize) -> bool {
        assert!(
            item_count <= MAX_LIST_ITEMS,
            "a variable list retains at most {MAX_LIST_ITEMS} items"
        );
        let mut state = self.0.borrow_mut();
        if state.metrics.len == item_count {
            return false;
        }
        let anchor = state.capture_anchor();
        state.metrics.resize(item_count);
        state.generation = state.generation.wrapping_add(1);
        state.restore_anchor(anchor);
        state.bump_measurement_revision();
        true
    }

    /// Replace the data set, discard all measurements, and restore the natural alignment edge.
    pub fn reset(&self, item_count: usize) {
        assert!(
            item_count <= MAX_LIST_ITEMS,
            "a variable list retains at most {MAX_LIST_ITEMS} items"
        );
        let mut state = self.0.borrow_mut();
        let estimate = state.metrics.estimate;
        state.metrics = HeightMetrics::new(item_count, estimate);
        state.generation = state.generation.wrapping_add(1);
        state.anchor_end =
            state.alignment == ListAlignment::Bottom || state.follow_mode == FollowMode::Tail;
        state.following_tail = state.follow_mode == FollowMode::Tail;
        state.scroll_offset = if state.anchor_end {
            state.actual_max_scroll_offset()
        } else {
            0.0
        };
        state.scrollbar_drag_max_offset = None;
        state.bump_measurement_revision();
    }

    /// Discard measured heights after a width, font, or content change while preserving the
    /// current logical top item and pixel inset.
    pub fn remeasure(&self) {
        let mut state = self.0.borrow_mut();
        let anchor = state.capture_anchor();
        state.metrics.clear_measurements();
        state.generation = state.generation.wrapping_add(1);
        state.restore_anchor(anchor);
        state.bump_measurement_revision();
    }

    /// Discard measurements for items whose content changed while retaining unaffected sparse
    /// blocks and the current logical top item.
    pub fn remeasure_items(&self, range: Range<usize>) {
        let mut state = self.0.borrow_mut();
        let start = range.start.min(state.metrics.len);
        let end = range.end.min(state.metrics.len);
        if start >= end {
            return;
        }
        let anchor = state.capture_anchor();
        state.metrics.clear_range(start..end);
        state.generation = state.generation.wrapping_add(1);
        state.restore_anchor(anchor);
        state.bump_measurement_revision();
    }

    pub fn content_height(&self) -> f32 {
        self.0.borrow().metrics.content_height()
    }

    pub fn scroll_offset(&self) -> f32 {
        self.0.borrow().scroll_offset
    }

    pub fn max_scroll_offset(&self) -> f32 {
        self.0.borrow().actual_max_scroll_offset()
    }

    pub fn logical_scroll_top(&self) -> ListOffset {
        let state = self.0.borrow();
        if state.anchor_end && state.metrics.len > 0 {
            return ListOffset {
                item_ix: state.metrics.len,
                offset_in_item: 0.0,
            };
        }
        if state.metrics.len == 0 {
            return ListOffset::default();
        }
        let item_ix = state.metrics.item_at_offset(state.scroll_offset);
        ListOffset {
            item_ix,
            offset_in_item: state.scroll_offset - state.metrics.item_top(item_ix),
        }
    }

    pub fn scroll_by(&self, distance: f32) -> bool {
        if !distance.is_finite() || distance == 0.0 {
            return false;
        }
        let current = self.scroll_offset();
        self.scroll_to_pixels(current + distance)
    }

    /// Scroll to an absolute estimated/measured content offset.
    pub fn scroll_to_pixels(&self, offset: f32) -> bool {
        let mut state = self.0.borrow_mut();
        if !offset.is_finite() {
            return false;
        }
        let next = offset.clamp(0.0, state.actual_max_scroll_offset());
        let changed = state.scroll_offset != next || state.anchor_end || state.following_tail;
        state.scroll_offset = next;
        state.anchor_end = false;
        state.following_tail = false;
        changed
    }

    /// Scroll to a logical item and pixel inset, clamped to current metrics.
    pub fn scroll_to(&self, offset: ListOffset) -> bool {
        let mut state = self.0.borrow_mut();
        if offset.item_ix >= state.metrics.len {
            let next = state.actual_max_scroll_offset();
            let changed = state.scroll_offset != next || !state.anchor_end;
            state.scroll_offset = next;
            state.anchor_end = true;
            state.following_tail = state.follow_mode == FollowMode::Tail;
            return changed;
        }
        let inset = if offset.offset_in_item.is_finite() {
            offset
                .offset_in_item
                .clamp(0.0, state.metrics.item_height(offset.item_ix))
        } else {
            0.0
        };
        let next = (state.metrics.item_top(offset.item_ix) + inset)
            .clamp(0.0, state.actual_max_scroll_offset());
        let changed = state.scroll_offset != next || state.anchor_end || state.following_tail;
        state.scroll_offset = next;
        state.anchor_end = false;
        state.following_tail = false;
        changed
    }

    pub fn scroll_to_end(&self) -> bool {
        let item_count = self.item_count();
        self.scroll_to(ListOffset {
            item_ix: item_count,
            offset_in_item: 0.0,
        })
    }

    /// Scroll the smallest distance needed to expose one complete item.
    pub fn scroll_to_reveal_item(&self, index: usize) -> bool {
        let mut state = self.0.borrow_mut();
        if index >= state.metrics.len || state.viewport.height <= 0.0 {
            return false;
        }
        let top = state.metrics.item_top(index);
        let item_height = state.metrics.item_height(index);
        let bottom = top + item_height;
        let viewport_top = state.scroll_offset;
        let viewport_bottom = viewport_top + state.viewport.height;
        let next = if item_height >= state.viewport.height {
            if (top - viewport_top).abs() <= LIST_MEASUREMENT_EPSILON {
                return false;
            }
            top
        } else if top < viewport_top {
            top
        } else if bottom > viewport_bottom {
            bottom - state.viewport.height
        } else {
            return false;
        }
        .clamp(0.0, state.actual_max_scroll_offset());
        state.scroll_offset = next;
        state.anchor_end = false;
        state.following_tail = false;
        true
    }

    pub fn set_follow_mode(&self, mode: FollowMode) {
        let mut state = self.0.borrow_mut();
        state.follow_mode = mode;
        match mode {
            FollowMode::Normal => {
                state.following_tail = false;
                state.anchor_end = false;
            }
            FollowMode::Tail => {
                state.following_tail = true;
                state.anchor_end = true;
                state.scroll_offset = state.actual_max_scroll_offset();
            }
        }
    }

    pub fn pause_following_tail(&self) {
        let mut state = self.0.borrow_mut();
        state.following_tail = false;
        state.anchor_end = false;
    }

    pub fn is_following_tail(&self) -> bool {
        self.0.borrow().following_tail
    }

    pub fn is_scrolled_to_end(&self) -> Option<bool> {
        let state = self.0.borrow();
        let maximum = state.actual_max_scroll_offset();
        (maximum > 0.0).then(|| {
            state.anchor_end || (maximum - state.scroll_offset).abs() <= LIST_MEASUREMENT_EPSILON
        })
    }

    pub fn visible_rows(&self) -> VisibleRows {
        self.0.borrow().visible_rows()
    }

    /// Limit mounted content to a contiguous range supplied asynchronously by the application.
    ///
    /// [`Self::visible_rows`] continues to report the requested scroll destination, including a
    /// bounded viewport of prefetch on either side. Rendering stays within the supplied range
    /// until its replacement arrives; wheel deltas and scrollbar dragging keep their requested
    /// offset. Pass `None` for ordinary synchronous row construction.
    pub fn set_available_range(&self, range: Option<Range<usize>>) {
        let mut state = self.0.borrow_mut();
        state.available_range = range.map(|range| {
            let start = range.start.min(state.metrics.len);
            let end = range.end.max(start).min(state.metrics.len);
            start..end
        });
    }

    /// Build a positioned normal-flow column for a contiguous visible range.
    ///
    /// Each rendered root receives stable list-scoped identity when it has no explicit ID. Its
    /// actual Taffy height is measured automatically after layout. Ranges are clamped to the list
    /// and [`MAX_MOUNTED_LIST_ITEMS`]. With [`Self::set_available_range`], the window is moved
    /// inside the supplied range so it never constructs rows that have not arrived yet.
    pub fn render_rows<E>(&self, range: Range<usize>, mut render: impl FnMut(usize) -> E) -> Element
    where
        E: IntoElement,
    {
        let (id, generation, start, end, origin, handle) = {
            let mut state = self.0.borrow_mut();
            let mut start = range.start.min(state.metrics.len);
            let mut end = range
                .end
                .max(start)
                .min(state.metrics.len)
                .min(start.saturating_add(MAX_MOUNTED_LIST_ITEMS));
            if let Some(available) = &state.available_range {
                let available_start = available.start.min(state.metrics.len);
                let available_end = available.end.min(state.metrics.len);
                let count = (end - start).min(available_end - available_start);
                start = start.clamp(available_start, available_end - count);
                end = start + count;
            }
            state.mounted_range = Some(start..end);
            (
                state.id,
                state.generation,
                start,
                end,
                state.content_origin_at(start, state.presented_offset(state.scroll_offset)),
                ListScrollHandle(Rc::clone(&self.0)),
            )
        };

        let mut children = Vec::with_capacity(end - start);
        for index in start..end {
            let mut child = render(index).into_element().flex_none().w_full();
            if child.explicit_id.is_none() {
                child.explicit_id = Some(list_internal_element_id(id, index, 0x4954_454d));
            }
            child.list_item_measurement = Some(ListItemMeasurement {
                handle: handle.clone(),
                index,
                generation,
            });
            children.push(child);
        }

        div()
            .id(list_internal_element_id(id, 0, 0x434f_4c55))
            .absolute()
            .top(origin)
            .left(0.0)
            .w_full()
            .flex_col()
            .children(children)
    }

    /// Return the current estimated or measured item rectangle in viewport-local coordinates.
    pub fn item_rect(&self, index: usize) -> Option<Rect> {
        let state = self.0.borrow();
        if index >= state.metrics.len {
            return None;
        }
        Some(Rect::new(
            0.0,
            state.content_origin_at(index, state.presented_offset(state.scroll_offset)),
            state.viewport.width,
            state.metrics.item_height(index),
        ))
    }

    pub fn stats(&self) -> ListStateStats {
        self.0.borrow().metrics.stats()
    }

    pub(crate) fn scroll_binding(&self) -> (VirtualScrollHandle, f32, u64, VirtualScrollMount) {
        let state = self.0.borrow();
        let handle = ListScrollHandle(Rc::clone(&self.0));
        let content_range = state.mounted_range.as_ref().and_then(|range| {
            (!range.is_empty())
                .then(|| state.metrics.item_top(range.start)..state.metrics.item_top(range.end))
        });
        (
            VirtualScrollHandle::Variable(handle),
            state.effective_max_scroll_offset(),
            state.measurement_revision,
            VirtualScrollMount::new(state.presented_offset(state.scroll_offset), content_range),
        )
    }
}

#[derive(Clone)]
struct HeightBlock {
    heights: [f32; LIST_HEIGHT_BLOCK_ITEMS],
    measured: u64,
    total_delta: f32,
}

impl Default for HeightBlock {
    fn default() -> Self {
        Self {
            heights: [0.0; LIST_HEIGHT_BLOCK_ITEMS],
            measured: 0,
            total_delta: 0.0,
        }
    }
}

impl HeightBlock {
    fn item_height(&self, slot: usize, estimate: f32) -> f32 {
        if self.measured & (1_u64 << slot) == 0 {
            estimate
        } else {
            self.heights[slot]
        }
    }

    fn prefix_delta(&self, slots: usize, estimate: f32) -> f32 {
        let mut delta = 0.0;
        for slot in 0..slots.min(LIST_HEIGHT_BLOCK_ITEMS) {
            if self.measured & (1_u64 << slot) != 0 {
                delta += self.heights[slot] - estimate;
            }
        }
        delta
    }

    fn set_height(&mut self, slot: usize, height: f32, estimate: f32) -> f32 {
        let previous = self.item_height(slot, estimate);
        let height = if (height - estimate).abs() <= LIST_MEASUREMENT_EPSILON {
            estimate
        } else {
            height
        };
        if height == estimate {
            self.measured &= !(1_u64 << slot);
            self.heights[slot] = 0.0;
        } else {
            self.measured |= 1_u64 << slot;
            self.heights[slot] = height;
        }
        let change = height - previous;
        self.total_delta += change;
        change
    }

    fn trim_from(&mut self, slot: usize, estimate: f32) {
        for item in slot..LIST_HEIGHT_BLOCK_ITEMS {
            if self.measured & (1_u64 << item) != 0 {
                self.total_delta -= self.heights[item] - estimate;
                self.heights[item] = 0.0;
                self.measured &= !(1_u64 << item);
            }
        }
    }

    fn clear_range(&mut self, range: Range<usize>, estimate: f32) {
        for item in range.start.min(LIST_HEIGHT_BLOCK_ITEMS)..range.end.min(LIST_HEIGHT_BLOCK_ITEMS)
        {
            if self.measured & (1_u64 << item) != 0 {
                self.total_delta -= self.heights[item] - estimate;
                self.heights[item] = 0.0;
                self.measured &= !(1_u64 << item);
            }
        }
    }
}

#[derive(Clone)]
struct HeightMetrics {
    len: usize,
    estimate: f32,
    blocks: BTreeMap<usize, HeightBlock>,
    block_prefix: Vec<f32>,
}

impl HeightMetrics {
    fn new(len: usize, estimate: f32) -> Self {
        let mut metrics = Self {
            len,
            estimate: estimate.clamp(LIST_MEASUREMENT_EPSILON, MAX_LIST_ITEM_HEIGHT),
            blocks: BTreeMap::new(),
            block_prefix: Vec::new(),
        };
        metrics.resize_prefix_index();
        metrics
    }

    fn block_count(&self) -> usize {
        self.len.div_ceil(LIST_HEIGHT_BLOCK_ITEMS)
    }

    fn resize_prefix_index(&mut self) {
        self.block_prefix.clear();
        self.block_prefix.resize(self.block_count() + 1, 0.0);
        let totals = self
            .blocks
            .iter()
            .map(|(&index, block)| (index, block.total_delta))
            .collect::<Vec<_>>();
        for (index, total) in totals {
            self.add_block_delta(index, total);
        }
    }

    fn add_block_delta(&mut self, block: usize, delta: f32) {
        let mut index = block + 1;
        while index < self.block_prefix.len() {
            self.block_prefix[index] += delta;
            index += index & index.wrapping_neg();
        }
    }

    fn prefix_block_delta(&self, blocks: usize) -> f32 {
        let mut index = blocks.min(self.block_count());
        let mut result = 0.0;
        while index > 0 {
            result += self.block_prefix[index];
            index &= index - 1;
        }
        result
    }

    fn item_height(&self, index: usize) -> f32 {
        debug_assert!(index < self.len);
        let block = index / LIST_HEIGHT_BLOCK_ITEMS;
        let slot = index % LIST_HEIGHT_BLOCK_ITEMS;
        self.blocks.get(&block).map_or(self.estimate, |block| {
            block.item_height(slot, self.estimate)
        })
    }

    fn item_top(&self, index: usize) -> f32 {
        let index = index.min(self.len);
        let block = index / LIST_HEIGHT_BLOCK_ITEMS;
        let slot = index % LIST_HEIGHT_BLOCK_ITEMS;
        let within = self
            .blocks
            .get(&block)
            .map_or(0.0, |block| block.prefix_delta(slot, self.estimate));
        (index as f32 * self.estimate + self.prefix_block_delta(block) + within).max(0.0)
    }

    fn content_height(&self) -> f32 {
        self.item_top(self.len)
    }

    fn item_at_offset(&self, offset: f32) -> usize {
        if self.len == 0 {
            return 0;
        }
        let offset = if offset.is_finite() {
            offset.clamp(0.0, self.content_height())
        } else {
            0.0
        };
        let mut low = 0;
        let mut high = self.len;
        while low < high {
            let middle = low + (high - low) / 2;
            if self.item_top(middle + 1) <= offset {
                low = middle + 1;
            } else {
                high = middle;
            }
        }
        low.min(self.len - 1)
    }

    fn set_item_height(&mut self, index: usize, height: f32) -> bool {
        if index >= self.len {
            return false;
        }
        let block_index = index / LIST_HEIGHT_BLOCK_ITEMS;
        let slot = index % LIST_HEIGHT_BLOCK_ITEMS;
        let (change, empty) = {
            let block = self.blocks.entry(block_index).or_default();
            let change = block.set_height(slot, height, self.estimate);
            (change, block.measured == 0)
        };
        if change.abs() <= LIST_MEASUREMENT_EPSILON {
            if empty {
                self.blocks.remove(&block_index);
            }
            return false;
        }
        self.add_block_delta(block_index, change);
        if empty {
            self.blocks.remove(&block_index);
        }
        true
    }

    fn resize(&mut self, len: usize) {
        self.len = len;
        let block_count = self.block_count();
        self.blocks.retain(|index, _| *index < block_count);
        if len > 0 && !len.is_multiple_of(LIST_HEIGHT_BLOCK_ITEMS) {
            let block_index = len / LIST_HEIGHT_BLOCK_ITEMS;
            if let Some(block) = self.blocks.get_mut(&block_index) {
                block.trim_from(len % LIST_HEIGHT_BLOCK_ITEMS, self.estimate);
                if block.measured == 0 {
                    self.blocks.remove(&block_index);
                }
            }
        }
        self.resize_prefix_index();
    }

    fn clear_measurements(&mut self) {
        self.blocks.clear();
        self.block_prefix.fill(0.0);
    }

    fn clear_range(&mut self, range: Range<usize>) {
        let start = range.start.min(self.len);
        let end = range.end.min(self.len);
        if start >= end {
            return;
        }
        let first_block = start / LIST_HEIGHT_BLOCK_ITEMS;
        let last_block = (end - 1) / LIST_HEIGHT_BLOCK_ITEMS;
        let keys = self
            .blocks
            .range(first_block..=last_block)
            .map(|(&index, _)| index)
            .collect::<Vec<_>>();
        for block_index in keys {
            let block_start = block_index * LIST_HEIGHT_BLOCK_ITEMS;
            let local_start = start.saturating_sub(block_start);
            let local_end = end.saturating_sub(block_start);
            let (delta, empty) = {
                let block = self
                    .blocks
                    .get_mut(&block_index)
                    .expect("the block key was snapshotted above");
                let previous = block.total_delta;
                block.clear_range(local_start..local_end, self.estimate);
                (block.total_delta - previous, block.measured == 0)
            };
            self.add_block_delta(block_index, delta);
            if empty {
                self.blocks.remove(&block_index);
            }
        }
    }

    fn stats(&self) -> ListStateStats {
        ListStateStats {
            measured_items: self
                .blocks
                .values()
                .map(|block| block.measured.count_ones() as usize)
                .sum(),
            measured_blocks: self.blocks.len(),
            retained_height_bytes: self
                .block_prefix
                .capacity()
                .saturating_mul(size_of::<f32>())
                .saturating_add(
                    self.blocks
                        .len()
                        .saturating_mul(size_of::<(usize, HeightBlock)>()),
                ),
        }
    }
}

fn sane_list_dimension(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, MAX_LIST_ITEM_HEIGHT)
    } else {
        0.0
    }
}

fn list_internal_element_id(list: usize, item: usize, salt: u64) -> ElementId {
    let mut value = (list as u64)
        .wrapping_mul(0x9e37_79b9_7f4a_7c15)
        .wrapping_add(item as u64)
        ^ salt.rotate_left(17);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^= value >> 31;
    if value == u64::MAX {
        value = u64::MAX - 1;
    }
    ElementId::new(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_document_heights_preserve_scroll_anchor_and_pixel_overscan() {
        let list = ListState::new(1000, 10.0)
            .with_overscan(0)
            .with_overscan_pixels(20.0);
        list.set_viewport_size(400.0, 40.0);
        let mut heights = vec![10.0; 1000];
        heights[1] = 100.0;
        list.set_item_heights(&heights);
        list.scroll_to_pixels(110.0);
        let range = list.visible_rows().range;
        assert_eq!(range.start, 1);
        assert!(range.end < 10);
        let anchor = list.logical_scroll_top();
        heights[0] = 20.0;
        list.set_item_heights(&heights);
        assert_eq!(list.logical_scroll_top(), anchor);
        assert_eq!(list.scroll_offset(), 120.0);
    }

    #[test]
    fn only_viewport_rows_and_overscan_are_returned() {
        let mut list = VirtualList::new(100_000, 20.0).with_overscan(2);
        list.set_viewport_height(100.0);
        list.scroll_to(10_000.0);
        assert_eq!(list.visible_rows().range, 498..507);
    }

    #[test]
    fn offsets_are_clamped_after_content_shrinks() {
        let mut list = VirtualList::new(100, 10.0);
        list.set_viewport_height(100.0);
        list.scroll_to(f32::MAX);
        assert_eq!(list.scroll_offset(), 900.0);
        list.set_len(5);
        assert_eq!(list.scroll_offset(), 0.0);
    }

    #[test]
    fn scrollbar_reaches_both_ends() {
        let mut list = VirtualList::new(100, 10.0);
        list.set_viewport_height(100.0);
        assert_eq!(list.scrollbar_thumb(10.0), Some((0.0, 10.0)));
        list.scroll_to(list.max_scroll_offset());
        assert_eq!(list.scrollbar_thumb(10.0), Some((90.0, 10.0)));
    }

    #[test]
    fn cloned_lists_keep_independent_scroll_offsets() {
        let mut list = VirtualList::new(100, 10.0);
        list.set_viewport_height(100.0);
        list.scroll_to(240.0);
        let mut clone = list.clone();

        clone.scroll_to(500.0);

        assert_eq!(list.scroll_offset(), 240.0);
        assert_eq!(clone.scroll_offset(), 500.0);
    }

    #[test]
    fn revealing_rows_moves_only_when_the_complete_row_is_outside() {
        let mut list = VirtualList::new(100, 20.0);
        list.set_viewport_height(100.0);

        assert!(!list.scroll_to_reveal(4));
        assert!(list.scroll_to_reveal(5));
        assert_eq!(list.scroll_offset(), 20.0);
        assert!(list.scroll_to_reveal(0));
        assert_eq!(list.scroll_offset(), 0.0);
        assert!(!list.scroll_to_reveal(100));
    }

    #[test]
    fn measured_items_preserve_the_logical_top_anchor() {
        let list = ListState::new(100, 20.0);
        list.set_viewport_size(400.0, 100.0);
        list.scroll_to(ListOffset {
            item_ix: 20,
            offset_in_item: 5.0,
        });
        assert_eq!(list.scroll_offset(), 405.0);

        let (generation, handle) = {
            let state = list.0.borrow();
            (state.generation, ListScrollHandle(Rc::clone(&list.0)))
        };
        assert!(handle.report_item_height(10, generation, 40.0));
        assert_eq!(list.scroll_offset(), 425.0);
        assert_eq!(
            list.logical_scroll_top(),
            ListOffset {
                item_ix: 20,
                offset_in_item: 5.0,
            }
        );

        assert!(handle.report_item_height(30, generation, 60.0));
        assert_eq!(list.scroll_offset(), 425.0);
        assert_eq!(list.stats().measured_items, 2);
    }

    #[test]
    fn asynchronous_ranges_mount_only_a_bounded_window_of_available_rows() {
        let list = ListState::new(MAX_LIST_ITEMS, 20.0);
        list.set_viewport_size(320.0, 100.0);
        list.set_available_range(Some(0..MAX_LIST_ITEMS));
        list.scroll_to_pixels(10_000.0);
        let requested = list.visible_rows().range;
        assert_eq!(requested, 495..510);
        let rows = list.render_rows(requested, |_| div().h(20.0));
        assert_eq!(
            rows.children.len(),
            15,
            "a supplied full data set must stay virtualized"
        );
        assert_eq!(list.0.borrow().mounted_range, Some(495..510));

        list.set_available_range(Some(0..10));
        let rows = list.render_rows(list.visible_rows().range, |_| div().h(20.0));
        assert_eq!(rows.children.len(), 10);
        assert_eq!(list.scroll_offset(), 10_000.0);
        assert_eq!(list.item_rect(5).unwrap().y, 0.0);

        // Shrinking or clearing a source cannot leave an invalid range or a stale offset.
        list.set_item_count(3);
        let rows = list.render_rows(list.visible_rows().range, |_| div().h(20.0));
        assert_eq!(rows.children.len(), 3);
        assert_eq!(list.scroll_offset(), 0.0);
        list.set_available_range(Some(usize::MAX..usize::MAX));
        assert!(
            list.render_rows(list.visible_rows().range, |_| div())
                .children
                .is_empty()
        );
        list.set_available_range(None);
        assert_eq!(
            list.render_rows(list.visible_rows().range, |_| div())
                .children
                .len(),
            3
        );
    }

    #[test]
    fn visible_range_uses_sparse_measured_heights_and_stays_bounded() {
        let list = ListState::new(100, 20.0).with_overscan(0);
        list.set_viewport_size(320.0, 60.0);
        let (generation, handle) = {
            let state = list.0.borrow();
            (state.generation, ListScrollHandle(Rc::clone(&list.0)))
        };
        handle.report_item_height(0, generation, 40.0);
        assert_eq!(list.visible_rows().range, 0..2);

        let huge = ListState::new(MAX_LIST_ITEMS, 0.5).with_overscan(MAX_LIST_OVERSCAN_ITEMS);
        huge.set_viewport_size(320.0, MAX_LIST_ITEM_HEIGHT);
        assert_eq!(huge.visible_rows().len(), MAX_MOUNTED_LIST_ITEMS);
        assert!(
            huge.stats().retained_height_bytes
                <= (MAX_LIST_ITEMS.div_ceil(LIST_HEIGHT_BLOCK_ITEMS) + 1) * size_of::<f32>()
        );
    }

    #[test]
    fn bottom_alignment_and_tail_following_have_distinct_user_semantics() {
        let list = ListState::new(3, 20.0).with_alignment(ListAlignment::Bottom);
        list.set_viewport_size(200.0, 40.0);
        assert_eq!(list.scroll_offset(), 20.0);
        assert_eq!(list.item_rect(0).unwrap().y, -20.0);

        let handle = ListScrollHandle(Rc::clone(&list.0));
        handle.set_offset_from_input(0.0);
        list.set_item_count(4);
        assert_eq!(list.scroll_offset(), 0.0);

        list.set_follow_mode(FollowMode::Tail);
        assert!(list.is_following_tail());
        assert_eq!(list.scroll_offset(), 40.0);
        list.set_item_count(5);
        assert_eq!(list.scroll_offset(), 60.0);

        handle.set_offset_from_input(20.0);
        assert!(!list.is_following_tail());
        list.set_item_count(6);
        assert_eq!(list.scroll_offset(), 20.0);
        handle.set_offset_from_input(list.max_scroll_offset());
        assert!(list.is_following_tail());
    }

    #[test]
    fn disabling_tail_following_preserves_the_viewport_when_content_grows() {
        let list = ListState::new(3, 20.0).with_follow_mode(FollowMode::Tail);
        list.set_viewport_size(200.0, 40.0);
        assert_eq!(list.scroll_offset(), 20.0);
        list.set_follow_mode(FollowMode::Normal);
        list.set_item_count(5);
        assert_eq!(list.scroll_offset(), 20.0);
        assert!(!list.is_following_tail());
    }

    #[test]
    fn resize_remeasurement_and_scrollbar_drag_keep_state_bounded_and_stable() {
        let list = ListState::new(100, 10.0);
        list.set_viewport_size(300.0, 100.0);
        list.scroll_to(ListOffset {
            item_ix: 20,
            offset_in_item: 3.0,
        });
        let handle = ListScrollHandle(Rc::clone(&list.0));
        let generation = list.0.borrow().generation;
        handle.report_item_height(5, generation, 30.0);
        assert_eq!(list.logical_scroll_top().item_ix, 20);
        assert_eq!(list.stats().measured_items, 1);

        handle.scrollbar_drag_started();
        let frozen_maximum = list.scroll_binding().1;
        handle.report_item_height(6, generation, 30.0);
        assert_eq!(list.scroll_binding().1, frozen_maximum);
        assert!(handle.scrollbar_drag_ended());
        assert!(list.max_scroll_offset() > frozen_maximum);

        let anchor = list.logical_scroll_top();
        list.set_viewport_size(280.0, 100.0);
        assert_eq!(list.stats().measured_items, 0);
        assert_eq!(list.logical_scroll_top(), anchor);
    }

    #[test]
    fn width_changes_keep_mounted_measurements_and_drop_only_offscreen_rows() {
        let list = ListState::new(100, 20.0).with_overscan(0);
        list.set_viewport_size(300.0, 100.0);
        let generation = list.0.borrow().generation;
        let handle = ListScrollHandle(Rc::clone(&list.0));
        assert!(handle.report_item_height(0, generation, 60.0));
        assert!(handle.report_item_height(50, generation, 80.0));
        assert_eq!(list.stats().measured_items, 2);

        let revision = handle.measurement_revision();
        list.set_viewport_size(280.0, 100.0);

        assert_eq!(list.0.borrow().generation, generation);
        assert_eq!(list.0.borrow().metrics.item_height(0), 60.0);
        assert_eq!(list.0.borrow().metrics.item_height(50), 20.0);
        assert_eq!(list.stats().measured_items, 1);
        assert_eq!(handle.measurement_revision(), revision);
        assert_eq!(handle.max_offset(), list.max_scroll_offset());
        assert!(handle.report_item_height(0, generation, 80.0));
    }

    #[test]
    fn targeted_remeasurement_retains_unaffected_sparse_blocks() {
        let list = ListState::new(1_000, 20.0);
        list.set_viewport_size(300.0, 100.0);
        let generation = list.0.borrow().generation;
        let handle = ListScrollHandle(Rc::clone(&list.0));
        handle.report_item_height(5, generation, 40.0);
        handle.report_item_height(70, generation, 60.0);
        handle.report_item_height(900, generation, 80.0);
        assert_eq!(list.stats().measured_items, 3);

        list.remeasure_items(64..128);
        assert_eq!(list.stats().measured_items, 2);
        assert_eq!(list.0.borrow().metrics.item_height(5), 40.0);
        assert_eq!(list.0.borrow().metrics.item_height(70), 20.0);
        assert_eq!(list.0.borrow().metrics.item_height(900), 80.0);
    }

    #[test]
    fn rendered_rows_are_normal_flow_measured_and_list_scoped() {
        let list = ListState::new(10, 24.0);
        list.set_viewport_size(300.0, 72.0);
        let visible = list.visible_rows();
        let rows = list.render_rows(visible.range.clone(), |index| div().h(20.0 + index as f32));

        assert_eq!(rows.children.len(), visible.len());
        assert!(rows.explicit_id.is_some());
        assert!(
            rows.children
                .iter()
                .all(|row| { row.explicit_id.is_some() && row.list_item_measurement.is_some() })
        );
        assert_eq!(rows.layout.position, taffy::prelude::Position::Absolute);
    }

    #[test]
    fn generated_list_ids_never_use_the_accessibility_root_id() {
        for list in [0, 1, usize::MAX] {
            for item in [0, 1, usize::MAX] {
                assert_ne!(
                    list_internal_element_id(list, item, u64::MAX).value(),
                    u64::MAX
                );
            }
        }
    }
}
