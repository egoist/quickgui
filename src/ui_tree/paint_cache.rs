use super::*;

const MIN_CACHED_SUBTREE_NODES: usize = 32;
const MAX_CACHED_SUBTREES: usize = 256;
const MAX_CACHED_PAINT_BYTES: usize = 32 * 1024 * 1024;

#[derive(Clone, Copy, PartialEq)]
pub(super) struct PaintCacheKey {
    pub frame: LayoutFrame,
    pub clip: Rect,
    pub layer: PaintLayerKey,
    pub opacity: f32,
    pub text_color: Option<Color>,
}

struct CachedPaint {
    key: PaintCacheKey,
    commands: crate::scene::SceneFragment,
    source_count: usize,
    bytes: usize,
    used: u64,
}

/// Cache substantial subtrees. Nested entries share immutable command chunks, while conservative
/// byte accounting charges each reference in full so retained storage cannot exceed the budget.
#[derive(Default)]
pub(super) struct PaintCache {
    eligible: HashSet<ElementId>,
    groups: HashSet<ElementId>,
    focus_containers: HashSet<ElementId>,
    entries: HashMap<ElementId, CachedPaint>,
    dirty: HashSet<ElementId>,
    paths: HashSet<ElementId>,
    bytes: usize,
    frame: u64,
    hovered: HashSet<ElementId>,
    pressed: Option<ElementId>,
    focused: Option<ElementId>,
    styled_focus: Option<ElementId>,
    dragging: Option<ElementId>,
    drag_over: Option<ElementId>,
    selection: Option<StaticTextSelection>,
    scrollbars: HashMap<ElementId, (ScrollbarState, bool)>,
}

impl PaintCache {
    pub(super) fn mount(&mut self, root: Option<&Element>) {
        self.entries.clear();
        self.eligible.clear();
        self.groups.clear();
        self.focus_containers.clear();
        self.bytes = 0;
        if let Some(root) = root {
            self.classify(root);
        }
    }

    fn classify(&mut self, element: &Element) -> (usize, bool) {
        let mut nodes = 1;
        let mut cacheable = !matches!(
            element.kind,
            ElementKind::Canvas(_) | ElementKind::CustomShader(_) | ElementKind::TextInput(_)
        ) && !matches!(&element.kind, ElementKind::Image(image)
                if matches!(image.resolved, ImageResolution::Animated(_)))
            && element.transition.is_none()
            && !element.resolved_motion
            && element.animation.is_none()
            && element.spring.is_none()
            && element.visual.transform.is_translation()
            && element.visual.filters.blur() == 0.0
            && element.visual.filters.drop_shadow().is_none()
            && element.visual.backdrop.is_empty()
            && element.visual.blend == crate::BlendMode::Normal;
        #[cfg(target_os = "macos")]
        {
            cacheable &= !matches!(element.kind, ElementKind::NativeView(_));
        }
        cacheable &= !retained::has_own_state_transform(element);
        if element.group {
            self.groups.insert(element.runtime_id);
        }
        if element.focus_within != ElementStateStyle::default() {
            self.focus_containers.insert(element.runtime_id);
        }
        for child in &element.children {
            let (child_nodes, child_cacheable) = self.classify(child);
            nodes += child_nodes;
            cacheable &= child_cacheable;
        }
        if cacheable
            && nodes >= MIN_CACHED_SUBTREE_NODES
            && self.eligible.len() < MAX_CACHED_SUBTREES
        {
            self.eligible.insert(element.runtime_id);
        }
        (nodes, cacheable)
    }

    pub(super) fn invalidate_element(
        &mut self,
        id: ElementId,
        parents: &HashMap<ElementId, ElementId>,
    ) {
        self.dirty.insert(id);
        let mut current = Some(id);
        while let Some(id) = current {
            if !self.paths.insert(id) {
                break;
            }
            current = parents.get(&id).copied();
        }
    }

    pub(super) fn dirty_subtree(&self, id: ElementId) -> bool {
        self.dirty.contains(&id)
    }

    pub(super) fn eligible(&self, id: ElementId, layer: PaintLayerKey) -> bool {
        layer.group == 0 && self.eligible.contains(&id)
    }

    pub(super) fn replay(
        &mut self,
        id: ElementId,
        key: PaintCacheKey,
        scene: &mut Scene,
    ) -> Option<usize> {
        if self.paths.contains(&id) {
            return None;
        }
        let entry = self.entries.get_mut(&id)?;
        if entry.key != key {
            return None;
        }
        entry.used = self.frame;
        scene.replay_fragment(&entry.commands);
        Some(entry.source_count)
    }

    pub(super) fn record(
        &mut self,
        id: ElementId,
        key: PaintCacheKey,
        commands: crate::scene::SceneFragment,
        source_count: usize,
    ) {
        if let Some(previous) = self.entries.remove(&id) {
            self.bytes -= previous.bytes;
        }
        let bytes = commands.bytes();
        if bytes > MAX_CACHED_PAINT_BYTES {
            return;
        }
        while self.bytes + bytes > MAX_CACHED_PAINT_BYTES {
            let Some(id) = self
                .entries
                .iter()
                .min_by_key(|(_, entry)| entry.used)
                .map(|(id, _)| *id)
            else {
                break;
            };
            self.bytes -= self.entries.remove(&id).unwrap().bytes;
        }
        self.bytes += bytes;
        self.entries.insert(
            id,
            CachedPaint {
                key,
                commands,
                source_count,
                bytes,
                used: self.frame,
            },
        );
    }
}

impl UiTree {
    pub(super) fn begin_cached_paint(&mut self, now: Instant) {
        let focus = self.styled_focus();
        let cache = &mut self.paint_cache;
        cache.frame = cache.frame.wrapping_add(1);
        let mut changed: Vec<_> = cache
            .hovered
            .symmetric_difference(&self.hovered)
            .copied()
            .collect();
        for (previous, current) in [
            (cache.pressed, self.pressed),
            (cache.focused, self.focused),
            (cache.styled_focus, focus),
            (cache.dragging, self.dragging),
            (cache.drag_over, self.drag_over),
        ] {
            if previous != current {
                changed.extend(previous);
                changed.extend(current);
            }
        }
        for id in changed {
            cache.invalidate_element(id, &self.parents);
            let mut current = self.parents.get(&id).copied();
            while let Some(parent) = current {
                if cache.groups.contains(&parent) || cache.focus_containers.contains(&parent) {
                    cache.invalidate_element(parent, &self.parents);
                }
                current = self.parents.get(&parent).copied();
            }
        }
        for (id, state) in &self.scrollbar_states {
            if cache.scrollbars.get(id) != Some(&(*state, state.visible_any(now))) {
                cache.invalidate_element(*id, &self.parents);
            }
        }
        if cache.selection != self.static_text_selection {
            cache.entries.clear();
            cache.bytes = 0;
        }
        cache.hovered.clone_from(&self.hovered);
        cache.pressed = self.pressed;
        cache.focused = self.focused;
        cache.styled_focus = focus;
        cache.dragging = self.dragging;
        cache.drag_over = self.drag_over;
        cache.selection = self.static_text_selection;
        cache.scrollbars.clear();
        cache.scrollbars.extend(
            self.scrollbar_states
                .iter()
                .map(|(id, state)| (*id, (*state, state.visible_any(now)))),
        );
    }

    pub(super) fn finish_cached_paint(&mut self) {
        self.paint_cache.dirty.clear();
        self.paint_cache.paths.clear();
        self.work.cached_paint_bytes = self.paint_cache.bytes;
    }
}
