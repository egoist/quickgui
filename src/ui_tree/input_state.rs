use super::*;

pub(super) fn sync_text_inputs(
    element: &Element,
    inputs: &mut HashMap<ElementId, TextInputState>,
    ids: &mut HashSet<ElementId>,
) {
    if element.is_display_none() {
        return;
    }
    if let ElementKind::TextInput(input) = &element.kind {
        ids.insert(element.runtime_id);
        inputs
            .entry(element.runtime_id)
            .and_modify(|state| {
                state.sync_external_styled(
                    &input.value,
                    input.multiline,
                    &input.constraints,
                    &input.highlights,
                )
            })
            .or_insert_with(|| {
                TextInputState::with_styling(
                    &input.value,
                    input.multiline,
                    input.constraints.clone(),
                    input.highlights.clone(),
                )
            });
    }
    for child in &element.children {
        sync_text_inputs(child, inputs, ids);
    }
}

pub(super) fn selectable_text_content(element: &Element) -> Option<&Arc<str>> {
    if !element.resolved_user_select {
        return None;
    }
    match &element.kind {
        ElementKind::Text(content) => Some(content),
        ElementKind::StyledText(styled) => Some(styled.content()),
        _ => None,
    }
}

pub(super) fn collect_selectable_texts(
    element: &Element,
    entries: &mut Vec<SelectableTextEntry>,
    indices: &mut HashMap<ElementId, usize>,
    previous: &mut HashMap<ElementId, SelectableTextEntry>,
) {
    if element.is_display_none() || element.is_visibility_hidden() {
        return;
    }
    if let Some(content) = selectable_text_content(element) {
        let document_index = entries.len();
        let character_lengths = previous
            .remove(&element.runtime_id)
            .filter(|entry| entry.content == *content)
            .map(|entry| entry.character_lengths)
            .unwrap_or_else(|| selectable_character_lengths(content).into());
        entries.push(SelectableTextEntry {
            id: element.runtime_id,
            content: content.clone(),
            character_lengths,
        });
        indices.insert(element.runtime_id, document_index);
    }
    for child in &element.children {
        collect_selectable_texts(child, entries, indices, previous);
    }
}

pub(super) fn sync_static_text_selection(
    selection: &mut Option<StaticTextSelection>,
    entries: &[SelectableTextEntry],
    indices: &HashMap<ElementId, usize>,
) {
    let Some(current) = selection.as_mut() else {
        return;
    };
    let Some(anchor_index) = indices.get(&current.anchor.id).copied() else {
        *selection = None;
        return;
    };
    let Some(focus_index) = indices.get(&current.focus.id).copied() else {
        *selection = None;
        return;
    };
    current.anchor.offset = boundary_at_or_before(
        &entries[anchor_index].content,
        current
            .anchor
            .offset
            .min(entries[anchor_index].content.len()),
    );
    current.focus.offset = boundary_at_or_before(
        &entries[focus_index].content,
        current.focus.offset.min(entries[focus_index].content.len()),
    );
}

pub(super) fn normalized_static_selection(
    selection: StaticTextSelection,
    indices: &HashMap<ElementId, usize>,
) -> Option<((usize, usize), (usize, usize))> {
    let anchor = (*indices.get(&selection.anchor.id)?, selection.anchor.offset);
    let focus = (*indices.get(&selection.focus.id)?, selection.focus.offset);
    Some(if anchor <= focus {
        (anchor, focus)
    } else {
        (focus, anchor)
    })
}

pub(super) fn static_position_key(
    position: StaticTextPosition,
    indices: &HashMap<ElementId, usize>,
) -> Option<(usize, usize)> {
    Some((*indices.get(&position.id)?, position.offset))
}

pub(super) fn static_selection_unit_range(
    position: StaticTextPosition,
    unit: StaticTextSelectionUnit,
    entries: &[SelectableTextEntry],
    indices: &HashMap<ElementId, usize>,
) -> Option<(StaticTextPosition, StaticTextPosition)> {
    let document_index = *indices.get(&position.id)?;
    let content = &entries.get(document_index)?.content;
    let range = match unit {
        StaticTextSelectionUnit::Character => position.offset..position.offset,
        StaticTextSelectionUnit::Word => word_range_at(content, position.offset),
        StaticTextSelectionUnit::Line => line_range_at(content, position.offset),
    };
    Some((
        StaticTextPosition {
            id: position.id,
            offset: range.start,
        },
        StaticTextPosition {
            id: position.id,
            offset: range.end,
        },
    ))
}

pub(super) fn word_range_at(content: &str, offset: usize) -> std::ops::Range<usize> {
    if content.is_empty() {
        return 0..0;
    }
    let offset = boundary_at_or_before(content, offset.min(content.len()));
    let segments = content.split_word_bound_indices().collect::<Vec<_>>();
    let target = segments
        .iter()
        .position(|(start, segment)| {
            let end = *start + segment.len();
            offset < end || (offset == content.len() && end == content.len())
        })
        .unwrap_or(segments.len() - 1);
    let (mut start, segment) = segments[target];
    let mut end = start + segment.len();
    if segment_is_selectable_word(segment) {
        let mut index = target;
        while index > 0 {
            let (previous_start, previous) = segments[index - 1];
            if previous_start + previous.len() != start || !segment_is_selectable_word(previous) {
                break;
            }
            start = previous_start;
            index -= 1;
        }
        let mut index = target + 1;
        while let Some((next_start, next)) = segments.get(index).copied() {
            if next_start != end || !segment_is_selectable_word(next) {
                break;
            }
            end = next_start + next.len();
            index += 1;
        }
    }
    start..end
}

pub(super) fn segment_is_selectable_word(segment: &str) -> bool {
    segment
        .chars()
        .any(|character| character.is_alphanumeric() || character == '_')
}

pub(super) fn line_range_at(content: &str, offset: usize) -> std::ops::Range<usize> {
    let offset = boundary_at_or_before(content, offset.min(content.len()));
    let start = content[..offset].rfind('\n').map_or(0, |index| index + 1);
    let end = content[offset..]
        .find('\n')
        .map_or(content.len(), |index| offset + index + 1);
    start..end
}

pub(super) fn static_selection_range_for_entry(
    selection: Option<StaticTextSelection>,
    indices: &HashMap<ElementId, usize>,
    document_index: usize,
    content_len: usize,
) -> Option<std::ops::Range<usize>> {
    let (start, end) = normalized_static_selection(selection?, indices)?;
    if document_index < start.0 || document_index > end.0 {
        return None;
    }
    let range = if start.0 == end.0 {
        start.1.min(content_len)..end.1.min(content_len)
    } else if document_index == start.0 {
        start.1.min(content_len)..content_len
    } else if document_index == end.0 {
        0..end.1.min(content_len)
    } else {
        0..content_len
    };
    (!range.is_empty()).then_some(range)
}

pub(super) fn sync_virtual_scrolls(
    element: &Element,
    handles: &mut HashMap<ElementId, RetainedVirtualScroll>,
    offsets: &mut HashMap<ElementId, Vector>,
    scrollbar_states: &mut HashMap<ElementId, ScrollbarState>,
    now: Instant,
) {
    if element.is_display_none() {
        return;
    }
    if let Some(virtual_scroll) = &element.virtual_scroll {
        let max_offset = virtual_scroll
            .handle
            .max_offset(virtual_scroll.max_offset_y)
            .max(0.0);
        let next = virtual_scroll.handle.offset().clamp(0.0, max_offset);
        virtual_scroll.handle.set_offset_silent(next);
        let offset = offsets.entry(element.runtime_id).or_default();
        let previous_y = offset.y;
        offset.y = next;
        if previous_y != next {
            let state = scrollbar_states.entry(element.runtime_id).or_default();
            state.axis = ScrollbarAxis::Vertical;
            if !state.hovered && !state.dragging {
                state.visible_until = now.checked_add(SCROLLBAR_AUTO_HIDE_DELAY);
            }
        }
        handles.insert(
            element.runtime_id,
            RetainedVirtualScroll {
                handle: virtual_scroll.handle.clone(),
                measurement_revision: virtual_scroll.measurement_revision,
                mount: virtual_scroll.mount.clone(),
            },
        );
    }
    for child in &element.children {
        sync_virtual_scrolls(child, handles, offsets, scrollbar_states, now);
    }
}

/// Collect every mounted element that publishes its resolved anchor placement.
///
/// The walk keeps declaration order and records the revision each handle already carried, so the
/// post-paint comparison only reports placements that actually moved. A `display: none` subtree is
/// never placed, so it publishes nothing.
pub(super) fn collect_anchor_placement_handles(
    element: &Element,
    handles: &mut Vec<RetainedAnchorPlacement>,
) {
    if element.is_display_none() {
        return;
    }
    if let Some(handle) = &element.anchor_placement {
        let revision = handle.revision();
        handles.push(RetainedAnchorPlacement {
            handle: handle.clone(),
            revision,
        });
    }
    for child in &element.children {
        collect_anchor_placement_handles(child, handles);
    }
}

/// Collect every mounted element that publishes its painted bounds.
pub(super) fn collect_layout_bounds_handles(
    element: &Element,
    handles: &mut Vec<RetainedLayoutBounds>,
) {
    if element.is_display_none() {
        return;
    }
    if let Some(handle) = &element.layout_bounds {
        let revision = handle.revision();
        handles.push(RetainedLayoutBounds {
            handle: handle.clone(),
            revision,
        });
    }
    for child in &element.children {
        collect_layout_bounds_handles(child, handles);
    }
}

pub(super) fn sync_animations(
    element: &Element,
    animations: &mut HashMap<ElementId, AnimationPlayback>,
    ids: &mut HashSet<ElementId>,
    now: Instant,
) {
    if element.is_display_none() || element.is_visibility_hidden() {
        return;
    }
    if let ElementKind::Image(image) = &element.kind
        && let ImageResolution::Animated(asset) = &image.resolved
    {
        ids.insert(element.runtime_id);
        let replace = animations
            .get(&element.runtime_id)
            .is_none_or(|playback| playback.asset_id != asset.id());
        if replace {
            animations.insert(
                element.runtime_id,
                AnimationPlayback::new(asset.clone(), now),
            );
        }
    }
    for child in &element.children {
        sync_animations(child, animations, ids, now);
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn resolve_declarative_animations(
    element: &mut Element,
    playbacks: &mut HashMap<ElementId, DeclarativeAnimationPlayback>,
    motion_ids: &mut HashSet<ElementId>,
    time_animation_ids: &mut HashSet<ElementId>,
    springs: &mut HashMap<ElementId, DeclarativeSpringPlayback>,
    spring_ids: &mut HashSet<ElementId>,
    request_frame: &mut bool,
    deadline: &mut Option<Instant>,
    now: Instant,
    animation_epoch: Instant,
    enabled: bool,
    reduce_motion: bool,
    resolved_motion_ids: &mut Vec<ElementId>,
) -> Result<(), UiError> {
    if element.is_display_none() || element.is_visibility_hidden() {
        return Ok(());
    }
    if let Some(ElementAnimation {
        id,
        stages,
        animator,
    }) = element.animation.take()
    {
        if motion_ids.contains(&id) {
            return Err(UiError::DuplicateAnimationId(id));
        }
        if motion_ids.len() == MAX_DECLARATIVE_ANIMATIONS_PER_WINDOW {
            return Err(UiError::TooManyDeclarativeAnimations);
        }
        motion_ids.insert(id);
        time_animation_ids.insert(id);
        resolved_motion_ids.push(id);
        let sample = playbacks
            .entry(id)
            .or_insert_with(|| DeclarativeAnimationPlayback::new(now))
            .sample(&stages, now, animation_epoch, enabled, reduce_motion);
        *request_frame |= sample.request_frame;
        if let Some(next) = sample.deadline {
            *deadline = Some(deadline.map_or(next, |current| current.min(next)));
        }

        let base = std::mem::replace(element, crate::div());
        let mut resolved = animator(base, sample.animation_ix, sample.value);
        // The wrapper is declaration metadata, not part of the retained paint tree. An animator
        // may still create independently animated descendants, which are resolved below.
        resolved.animation = None;
        resolved.resolved_motion = true;
        *element = resolved;
    }

    if let Some(spring) = element.spring.take() {
        let id = spring.id;
        if motion_ids.contains(&id) {
            return Err(UiError::DuplicateAnimationId(id));
        }
        if motion_ids.len() == MAX_DECLARATIVE_ANIMATIONS_PER_WINDOW {
            return Err(UiError::TooManyDeclarativeAnimations);
        }
        motion_ids.insert(id);
        spring_ids.insert(id);
        resolved_motion_ids.push(id);
        let sample = springs
            .entry(id)
            .or_insert_with(|| DeclarativeSpringPlayback::new(&spring, now))
            .sample(&spring, now, enabled, reduce_motion);
        *request_frame |= sample.request_frame;

        let base = std::mem::replace(element, crate::div());
        let mut resolved = (spring.animator)(base, sample.value);
        resolved.spring = None;
        resolved.resolved_motion = true;
        *element = resolved;
    }

    for child in &mut element.children {
        resolve_declarative_animations(
            child,
            playbacks,
            motion_ids,
            time_animation_ids,
            springs,
            spring_ids,
            request_frame,
            deadline,
            now,
            animation_epoch,
            enabled,
            reduce_motion,
            resolved_motion_ids,
        )?;
    }
    Ok(())
}

pub(super) fn sync_accessibility_text_ids(
    input_ids: &HashSet<ElementId>,
    element_ids: &HashSet<ElementId>,
    text_ids: &mut HashMap<ElementId, AccessibilityNodeId>,
    next_id: &mut u64,
) {
    text_ids.retain(|input, text_id| {
        input_ids.contains(input) && !element_ids.contains(&ElementId::new(text_id.0))
    });
    for input_id in input_ids {
        if text_ids.contains_key(input_id) {
            continue;
        }
        loop {
            let candidate = AccessibilityNodeId(*next_id);
            *next_id = next_id.wrapping_sub(1);
            if candidate != ACCESSIBILITY_ROOT_ID
                && !element_ids.contains(&ElementId::new(candidate.0))
                && !text_ids.values().any(|existing| *existing == candidate)
            {
                text_ids.insert(*input_id, candidate);
                break;
            }
        }
    }
}
