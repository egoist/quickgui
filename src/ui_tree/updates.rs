use super::*;
use crate::ElementUpdate;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum ElementUpdateKind {
    #[default]
    None,
    Paint,
    Layout,
}

impl UiTree {
    /// Apply a batch without re-declaring siblings or replacing event registrations. All targets
    /// are validated first; a missing target or callback-owned subtree rejects the whole batch.
    pub(crate) fn update_elements(
        &mut self,
        updates: &[ElementUpdate],
    ) -> Result<Option<ElementUpdateKind>, UiError> {
        let started = Instant::now();
        let reconciled_before = self.work.reconciliation_time;
        let result = self.apply_element_updates(updates);
        self.work.mutation_time += started
            .elapsed()
            .saturating_sub(self.work.reconciliation_time - reconciled_before);
        result
    }

    fn apply_element_updates(
        &mut self,
        updates: &[ElementUpdate],
    ) -> Result<Option<ElementUpdateKind>, UiError> {
        let Some(root) = self.root.as_ref() else {
            return Ok(None);
        };
        let mut paths = Vec::with_capacity(updates.len());
        for update in updates {
            let mut id = update.id();
            let mut path = Vec::new();
            while id != root.runtime_id {
                let Some(&(parent, index)) = self.layout_nodes.positions.get(&id) else {
                    return Ok(None);
                };
                path.push(index);
                id = parent;
            }
            path.reverse();
            let mut element = root;
            for index in path.iter().copied().map(Some).chain(std::iter::once(None)) {
                // A later callback evaluation would overwrite an imperative change. Let the
                // source view re-declare these subtrees with its updated state instead.
                if element.animation.is_some()
                    || element.spring.is_some()
                    || element.resolved_motion
                    || matches!(element.kind, ElementKind::ContainerQuery(_))
                {
                    return Ok(None);
                }
                if let Some(index) = index {
                    let Some(child) = element.children.get(index) else {
                        return Ok(None);
                    };
                    element = child;
                }
            }
            if element.runtime_id != update.id()
                || (matches!(update, ElementUpdate::Text { .. })
                    && !matches!(element.kind, ElementKind::Text(_)))
            {
                return Ok(None);
            }
            if let ElementUpdate::Replace {
                id,
                element: replacement,
            } = update
                && (replacement.explicit_id != Some(*id)
                    || callback_owned(element)
                    || callback_owned(replacement))
            {
                return Ok(None);
            }
            paths.push(path);
        }

        // Structural roots must be disjoint from every other mutation. The source runtime can
        // coalesce overlapping changes into their nearest replacement ancestor before submitting.
        for (index, update) in updates.iter().enumerate() {
            if !matches!(update, ElementUpdate::Replace { .. }) {
                continue;
            }
            for (other, path) in paths.iter().enumerate() {
                if index != other && path.starts_with(&paths[index]) {
                    return Ok(None);
                }
            }
        }
        let mut kind = self.replace_declarations(updates, &paths)?;
        let mut transform_changed = false;
        for (update, path) in updates.iter().zip(paths) {
            let mut element = self.root.as_mut().unwrap();
            for index in path {
                element = &mut element.children[index];
            }
            let changed = match update {
                ElementUpdate::Replace { .. } => continue,
                ElementUpdate::Text { id, content } => {
                    let ElementKind::Text(previous) = &mut element.kind else {
                        unreachable!("batch targets were validated before mutation")
                    };
                    if previous == content {
                        continue;
                    }
                    *previous = content.clone();
                    let node = element.taffy_node.unwrap();
                    if let Some(MeasureContext::Text {
                        content: measured, ..
                    }) = self.taffy.get_node_context_mut(node)
                    {
                        *measured = content.clone();
                    }
                    self.taffy.mark_dirty(node)?;
                    if let Some(&index) = self.selectable_text_indices.get(id) {
                        let entry = &mut self.selectable_texts[index];
                        entry.content = content.clone();
                        entry.character_lengths = selectable_character_lengths(content).into();
                    }
                    self.retained_semantics_dirty = true;
                    kind = ElementUpdateKind::Layout;
                    true
                }
                ElementUpdate::BackgroundColor { color, .. } => {
                    if element.visual.background == Some(*color) {
                        false
                    } else {
                        element.visual.background = Some(*color);
                        true
                    }
                }
                ElementUpdate::TextColor { color, .. } => {
                    if element.typography.color == Some(*color) {
                        false
                    } else {
                        element.typography.color = Some(*color);
                        update_inherited_color(element, *color, &mut self.taffy);
                        true
                    }
                }
                ElementUpdate::Opacity { opacity, .. } => {
                    let opacity = if opacity.is_finite() {
                        opacity.clamp(0.0, 1.0)
                    } else {
                        1.0
                    };
                    if element.visual.opacity == opacity {
                        false
                    } else {
                        element.visual.opacity = opacity;
                        true
                    }
                }
                ElementUpdate::Transform { transform, .. } => {
                    if element.visual.transform == *transform {
                        false
                    } else {
                        element.visual.transform = *transform;
                        transform_changed = true;
                        true
                    }
                }
            };
            if changed && kind == ElementUpdateKind::None {
                kind = ElementUpdateKind::Paint;
            }
            if changed {
                self.paint_cache
                    .invalidate_element(update.id(), &self.parents);
            }
        }
        if transform_changed {
            self.retained_placement_dirty = true;
            self.geometry_cache.invalidate_paint();
            self.paint_cache.mount(self.root.as_ref());
        }
        if kind == ElementUpdateKind::Layout {
            sync_static_text_selection(
                &mut self.static_text_selection,
                &self.selectable_texts,
                &self.selectable_text_indices,
            );
        }
        Ok(Some(kind))
    }

    pub(crate) fn take_retained_placement_dirty(&mut self) -> bool {
        std::mem::take(&mut self.retained_placement_dirty)
    }

    pub(crate) fn take_retained_semantics_dirty(&mut self) -> bool {
        std::mem::take(&mut self.retained_semantics_dirty)
    }
}

fn callback_owned(element: &Element) -> bool {
    element.animation.is_some() || element.spring.is_some() || element.resolved_motion
        || matches!(element.kind, ElementKind::ContainerQuery(_))
        // Resource loaders can materialize fallback subtrees and own declaration-wide cache
        // liveness. Re-enter that established resource frame instead of keeping removed loaders
        // active indefinitely across a sequence of scoped replacements.
        || matches!(&element.kind, ElementKind::Image(image) if image.source.resource().is_some())
        || element.children.iter().any(callback_owned)
}

fn element_at_mut<'a>(mut element: &'a mut Element, path: &[usize]) -> &'a mut Element {
    for &index in path {
        element = &mut element.children[index];
    }
    element
}

fn remove_subtree_ids(element: &Element, ids: &mut HashSet<ElementId>) {
    ids.remove(&element.runtime_id);
    for child in &element.children {
        remove_subtree_ids(child, ids);
    }
}

impl UiTree {
    fn replace_declarations(
        &mut self,
        updates: &[ElementUpdate],
        paths: &[Vec<usize>],
    ) -> Result<ElementUpdateKind, UiError> {
        if !updates
            .iter()
            .any(|update| matches!(update, ElementUpdate::Replace { .. }))
        {
            return Ok(ElementUpdateKind::None);
        }
        let started = Instant::now();
        let mut previous = Vec::new();
        let root = self.root.as_mut().unwrap();
        for (update, path) in updates.iter().zip(paths) {
            if let ElementUpdate::Replace { element, .. } = update {
                previous.push((
                    path,
                    std::mem::replace(element_at_mut(root, path), *element.clone()),
                ));
            }
        }
        // Validate the final declaration before touching layout or mount state. In particular,
        // moving an ID between two replacement roots is legal, duplicating it is not.
        let mut retained_ids = self.seen_ids.clone();
        for (_, element) in &previous {
            remove_subtree_ids(element, &mut retained_ids);
        }
        let mut validate = || -> Result<(), UiError> {
            let mut ids = HashSet::from([ElementId::new(ACCESSIBILITY_ROOT_ID.0)]);
            collect_explicit_ids(root, &mut ids)?;
            validate_style_transition_count(root, &mut 0)?;
            validate_container_query_limits(root)?;
            validate_sticky_limits(root)?;
            for (path, _) in &previous {
                let mut element = &*root;
                for &index in path.iter() {
                    element = &element.children[index];
                }
                collect_explicit_ids(element, &mut retained_ids)?;
            }
            Ok(())
        };
        if let Err(error) = validate() {
            for (path, element) in previous {
                *element_at_mut(root, path) = element;
            }
            return Err(error);
        }
        self.seen_ids = retained_ids;
        let before = self.layout_nodes.reconciled_nodes;
        for (path, _) in previous {
            let mut inherited = TextStyle::default();
            let mut parent = ElementId::new(0x9e37_79b9_7f4a_7c15);
            let mut user_select = true;
            let mut direction = Direction::Ltr;
            let mut element = &mut *root;
            for &index in path {
                // Re-resolve the logical inheritance chain: the mounted text style has already
                // converted start/end alignment into physical alignment for its own direction.
                inherited = element.typography.resolve(&inherited);
                parent = element.runtime_id;
                user_select = element.resolved_user_select;
                direction = element.resolved_direction;
                element = &mut element.children[index];
            }
            let node = build_retained_layout_node(
                &mut self.taffy,
                &mut self.layout_nodes,
                &mut self.seen_ids,
                element,
                parent,
                path.last().copied().unwrap_or(0),
                &inherited,
                user_select,
                direction,
            )?;
            if path.is_empty() {
                self.root_node = Some(node);
            }
        }
        self.layout_nodes.commit_children(&mut self.taffy)?;
        self.mounted_state_dirty = true;
        self.retained_semantics_dirty = true;
        self.geometry_cache.invalidate();
        self.work.reconciled_nodes += self.layout_nodes.reconciled_nodes - before;
        self.work.reconciliation_time += started.elapsed();
        Ok(ElementUpdateKind::Layout)
    }
}

fn update_inherited_color(
    element: &mut Element,
    color: Color,
    taffy: &mut TaffyTree<MeasureContext>,
) {
    element.resolved_typography.color = color;
    if let Some(node) = element.taffy_node
        && let Some(MeasureContext::Text { style, .. }) = taffy.get_node_context_mut(node)
    {
        style.color = color;
    }
    for child in &mut element.children {
        // An explicitly colored child is an inheritance boundary for its complete subtree.
        if child.typography.color.is_none() {
            update_inherited_color(child, color, taffy);
        }
    }
}
