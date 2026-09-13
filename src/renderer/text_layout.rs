use super::*;

pub(super) fn create_font_system() -> FontSystem {
    #[cfg(target_os = "macos")]
    {
        let mut font_system = FontSystem::new();
        // Match the web's system-ui default even when Open Sans happens to be installed.
        // FontDB otherwise uses Open Sans for generic sans-serif on every platform.
        if font_system
            .db()
            .faces()
            .any(|face| face.families.iter().any(|(name, _)| name == ".SF NS"))
        {
            font_system.db_mut().set_sans_serif_family(".SF NS");
        }
        // NISC18030.ttf advertises `GB18030 Bitmap` as a monospaced CJK face, but its
        // non-scalable metrics produce infinite advances in Cosmic Text and Swash cannot
        // rasterize its glyphs. Cosmic Text otherwise ranks it ahead of the scalable macOS CJK
        // fallbacks for `Family::Monospace`, which invalidates the complete wrapped buffer.
        // Remove the unusable face once, before any font-match cache is populated; this adds no
        // shaping or frame-time work and allows the normal script fallback to select a scalable
        // face such as PingFang.
        let incompatible = font_system
            .db()
            .faces()
            .filter(|face| face.monospaced && face.post_script_name == "GB18030Bitmap")
            .map(|face| face.id)
            .collect::<Vec<_>>();
        if !incompatible.is_empty() {
            let database = font_system.db_mut();
            for id in incompatible {
                database.remove_face(id);
            }
        }
        font_system
    }
    #[cfg(not(target_os = "macos"))]
    {
        FontSystem::new()
    }
}

/// One application-wide mutable shaping database. Winit serializes every renderer callback on the
/// main thread, so sharing it removes per-window system-font metadata without adding locks or
/// cross-thread traffic. Text buffers, glyph atlases, and eviction caches remain window-local.
pub(crate) type SharedFontSystem = Rc<RefCell<FontSystem>>;

pub(crate) fn create_shared_font_system(
    assets: &Assets,
    fonts: &[FontSource],
) -> Result<SharedFontSystem, AssetError> {
    let resolved = resolve_fonts(assets, fonts)?;
    let mut font_system = create_font_system();
    for (index, (source, expected_faces)) in resolved
        .sources
        .iter()
        .zip(resolved.declared_faces)
        .enumerate()
    {
        let loaded =
            font_system
                .db_mut()
                .load_font_source(glyphon::cosmic_text::fontdb::Source::Binary(
                    source.backing(),
                ));
        if loaded.len() != expected_faces {
            return Err(AssetError::InvalidFont { index });
        }
    }
    Ok(Rc::new(RefCell::new(font_system)))
}

pub(super) fn text_cursor_for_byte_index(content: &str, index: usize) -> Cursor {
    let mut index = index.min(content.len());
    while !content.is_char_boundary(index) {
        index -= 1;
    }

    let mut offset = 0;
    for (line, text) in content.split('\n').enumerate() {
        let line_end = offset + text.len();
        if index <= line_end {
            return Cursor::new(line, index - offset);
        }
        offset = line_end.saturating_add(1);
    }
    Cursor::new(content.lines().count(), 0)
}

/// Return the visual selection spans for one retained layout run.
///
/// Cosmic Text 0.19's `LayoutRun::highlight` treats every line outside a same-line selection as
/// selected because it only compares line equality. Keep its grapheme/BiDi span construction but
/// reject lines outside the ordered cursor interval explicitly.
pub(super) fn text_selection_spans(
    run: &LayoutRun<'_>,
    cursor_start: Cursor,
    cursor_end: Cursor,
) -> Vec<(f32, f32)> {
    if run.line_i < cursor_start.line || run.line_i > cursor_end.line {
        return Vec::new();
    }
    let selection_start = if run.line_i == cursor_start.line {
        cursor_start.index.min(run.text.len())
    } else {
        0
    };
    let selection_end = if run.line_i == cursor_end.line {
        cursor_end.index.min(run.text.len())
    } else {
        run.text.len()
    };
    if selection_start >= selection_end {
        return Vec::new();
    }

    let mut results = Vec::new();
    let mut visual_range: Option<(f32, f32)> = None;
    for glyph in run.glyphs {
        let cluster = &run.text[glyph.start..glyph.end];
        let grapheme_count = cluster.grapheme_indices(true).count().max(1);
        let grapheme_width = glyph.w / grapheme_count as f32;
        let mut grapheme_x = glyph.x;
        for (offset, grapheme) in cluster.grapheme_indices(true) {
            let grapheme_start = glyph.start + offset;
            let grapheme_end = grapheme_start + grapheme.len();
            if grapheme_end > selection_start && grapheme_start < selection_end {
                visual_range = Some(match visual_range {
                    Some((minimum, maximum)) => (
                        minimum.min(grapheme_x),
                        maximum.max(grapheme_x + grapheme_width),
                    ),
                    None => (grapheme_x, grapheme_x + grapheme_width),
                });
            } else if let Some((minimum, maximum)) = visual_range.take()
                && maximum > minimum
            {
                results.push((minimum, maximum - minimum));
            }
            grapheme_x += grapheme_width;
        }
    }
    if let Some((minimum, maximum)) = visual_range
        && maximum > minimum
    {
        results.push((minimum, maximum - minimum));
    }
    results
}

pub(super) fn byte_index_for_text_cursor(content: &str, cursor: Cursor) -> usize {
    let mut offset = 0;
    for (line, text) in content.split('\n').enumerate() {
        if line == cursor.line {
            let mut index = cursor.index.min(text.len());
            while !text.is_char_boundary(index) {
                index -= 1;
            }
            return offset + index;
        }
        offset = offset.saturating_add(text.len()).saturating_add(1);
    }
    content.len()
}

#[derive(Clone, Copy, Debug)]
pub(super) enum TruncationPlacement {
    End {
        prefix_end: usize,
    },
    Start {
        suffix_start: usize,
    },
    Middle {
        prefix_end: usize,
        suffix_start: usize,
    },
}

/// Whether this style actually rewrites `content` before shaping.
///
/// Soft-hyphen removal only matters for content that contains one, so ordinary runs keep the
/// identity projection and every existing truncation and caching path unchanged.
pub(super) fn rewrites_text_content(content: &str, style: &TextStyle) -> bool {
    style.transform.is_some() || (style.hyphens == Hyphens::None && content.contains(SOFT_HYPHEN))
}

/// The soft hyphen: an author-placed break opportunity that is invisible off a break.
pub(super) const SOFT_HYPHEN: char = '\u{00ad}';

/// Build the shaping input for one text run, mapping it back to the source string.
///
/// A case mapping or soft-hyphen removal changes what is shaped but never what the application,
/// selection, clipboard, or accessibility observe: the returned projection maps every display
/// index back onto a real boundary of `content`.
pub(super) fn project_text_content(
    content: &Arc<str>,
    highlights: Option<&Arc<[TextHighlight]>>,
    style: &TextStyle,
) -> ProjectedText {
    if !rewrites_text_content(content, style) {
        return ProjectedText::identity(content.clone(), highlights.cloned());
    }
    let strip_soft_hyphens = style.hyphens == Hyphens::None;
    let mut display = String::with_capacity(content.len());
    let mut spans: Vec<ProjectionSpan> = Vec::with_capacity(1);
    let mut at_word_start = true;
    let mut linear_start: Option<(usize, usize)> = None;
    let mut buffer = [0u8; 4];

    for (index, character) in content.char_indices() {
        let original = index..index + character.len_utf8();
        let display_start = display.len();
        let linear = if strip_soft_hyphens && character == SOFT_HYPHEN {
            false
        } else {
            match style.transform {
                None => display.push(character),
                Some(TextTransform::Uppercase) => {
                    for mapped in character.to_uppercase() {
                        display.push_str(mapped.encode_utf8(&mut buffer));
                    }
                }
                Some(TextTransform::Lowercase) => {
                    for mapped in character.to_lowercase() {
                        display.push_str(mapped.encode_utf8(&mut buffer));
                    }
                }
                Some(TextTransform::Capitalize) => {
                    if at_word_start {
                        for mapped in character.to_uppercase() {
                            display.push_str(mapped.encode_utf8(&mut buffer));
                        }
                    } else {
                        display.push(character);
                    }
                }
            }
            display.len() - display_start == original.len()
        };
        at_word_start = character.is_whitespace() || character == '-' || character == '_';

        if linear {
            linear_start.get_or_insert((display_start, original.start));
            continue;
        }
        if let Some((display_from, original_from)) = linear_start.take()
            && display_from < display_start
        {
            spans.push(ProjectionSpan {
                display: display_from..display_start,
                original: original_from..original.start,
            });
        }
        spans.push(ProjectionSpan {
            display: display_start..display.len(),
            original: original.clone(),
        });
    }
    if let Some((display_from, original_from)) = linear_start
        && display_from < display.len()
    {
        spans.push(ProjectionSpan {
            display: display_from..display.len(),
            original: original_from..content.len(),
        });
    }

    let projected_highlights = project_mapped_highlights(highlights, &spans);
    ProjectedText {
        content: Arc::from(display),
        highlights: projected_highlights,
        mapping: TextProjection::Mapped {
            original_len: content.len(),
            spans: spans.into(),
        },
    }
}

/// Move highlight ranges from source indices onto the rewritten shaping input.
pub(super) fn project_mapped_highlights(
    highlights: Option<&Arc<[TextHighlight]>>,
    spans: &[ProjectionSpan],
) -> Option<Arc<[TextHighlight]>> {
    let highlights = highlights?.as_ref();
    if highlights.is_empty() {
        return Some(Arc::from([]));
    }
    let mut projected: Vec<TextHighlight> = Vec::with_capacity(highlights.len());
    for highlight in highlights {
        for span in spans {
            let start = span.original.start.max(highlight.range.start);
            let end = span.original.end.min(highlight.range.end);
            if start >= end {
                continue;
            }
            let range = if span.display.len() == span.original.len() {
                span.display.start + start - span.original.start
                    ..span.display.start + end - span.original.start
            } else {
                span.display.clone()
            };
            if range.is_empty() {
                continue;
            }
            if let Some(previous) = projected.last_mut()
                && previous.range.end == range.start
                && previous.style == highlight.style
            {
                previous.range.end = range.end;
            } else if projected.len() < MAX_TEXT_HIGHLIGHTS {
                projected.push(TextHighlight {
                    range,
                    style: highlight.style.clone(),
                });
            }
        }
    }
    projected.sort_unstable_by_key(|highlight| highlight.range.start);
    Some(projected.into())
}

pub(super) fn prepare_text_buffer(
    font_system: &mut FontSystem,
    content: &Arc<str>,
    highlights: Option<&Arc<[TextHighlight]>>,
    style: &TextStyle,
    width: Option<f32>,
    scale: f32,
) -> (ProjectedText, Buffer) {
    let identity = project_text_content(content, highlights, style);
    let original_buffer = shape_projected_text(font_system, &identity, style, width, scale);
    let Some(overflow) = style.text_overflow.as_ref() else {
        return (identity, original_buffer);
    };
    // An affix truncation projection is expressed over source indices. Composing it with a
    // content rewrite is not worth the mapping complexity, so a rewritten run keeps its complete
    // shaped layout and relies on clipping; the default `…` ellipsis is unaffected because Cosmic
    // Text applies it inside the buffer.
    if rewrites_text_content(content, style) {
        return (identity, original_buffer);
    }
    if uses_cosmic_ellipsis(overflow) {
        // Cosmic Text retains the original shaped runs and applies Unicode-aware ellipsizing as
        // part of line layout. It therefore handles the common GPUI `…` path in one buffer and
        // can reflow that same buffer in place as the assigned width changes.
        return (identity, original_buffer);
    }
    let Some(width) = width
        .filter(|width| width.is_finite())
        .map(|width| width.max(0.0))
    else {
        return (identity, original_buffer);
    };
    let max_lines = style.line_clamp.unwrap_or(1).max(1);
    if text_buffer_fits(&original_buffer, width, max_lines, scale) {
        return (identity, original_buffer);
    }

    let mut boundaries = content
        .grapheme_indices(true)
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    if boundaries.first().copied() != Some(0) {
        boundaries.insert(0, 0);
    }
    if boundaries.last().copied() != Some(content.len()) {
        boundaries.push(content.len());
    }
    let graphemes = boundaries.len().saturating_sub(1);
    let affix = match overflow {
        TextOverflow::Truncate(affix)
        | TextOverflow::TruncateStart(affix)
        | TextOverflow::TruncateMiddle(affix) => affix,
    };

    let projection_for_count = |retained: usize| {
        let placement = match overflow {
            TextOverflow::Truncate(_) => TruncationPlacement::End {
                prefix_end: boundaries[retained],
            },
            TextOverflow::TruncateStart(_) => TruncationPlacement::Start {
                suffix_start: boundaries[graphemes - retained],
            },
            TextOverflow::TruncateMiddle(_) => {
                // GPUI biases middle truncation toward the recognizable prefix while still
                // retaining the path/identifier suffix.
                let prefix_graphemes = retained.saturating_mul(2).div_ceil(3);
                let suffix_graphemes = retained.saturating_sub(prefix_graphemes);
                TruncationPlacement::Middle {
                    prefix_end: boundaries[prefix_graphemes],
                    suffix_start: boundaries[graphemes - suffix_graphemes],
                }
            }
        };
        build_text_projection(content, highlights, affix, placement)
    };

    // The original did not fit, so at least one grapheme must be omitted. Each probe shapes an
    // immutable candidate only on this cache miss; stable frames reuse the selected buffer.
    let mut low = 0usize;
    let mut high = graphemes.saturating_sub(1);
    while low < high {
        let middle = low + (high - low).div_ceil(2);
        let projection = projection_for_count(middle);
        let candidate = shape_projected_text(font_system, &projection, style, Some(width), scale);
        if text_buffer_fits(&candidate, width, max_lines, scale) {
            low = middle;
        } else {
            high = middle - 1;
        }
    }
    let projection = projection_for_count(low);
    let buffer = shape_projected_text(font_system, &projection, style, Some(width), scale);
    (projection, buffer)
}

pub(super) fn shape_projected_text(
    font_system: &mut FontSystem,
    projection: &ProjectedText,
    style: &TextStyle,
    width: Option<f32>,
    scale: f32,
) -> Buffer {
    let metrics = Metrics::new(style.font_size * scale, style.line_height * scale);
    let mut buffer = Buffer::new(font_system, metrics);
    configure_text_buffer(
        &mut buffer,
        font_system,
        &projection.content,
        style,
        projection.highlights.as_deref(),
        width,
        scale,
    );
    buffer
}

pub(super) fn text_buffer_fits(buffer: &Buffer, width: f32, max_lines: usize, scale: f32) -> bool {
    let maximum = width.max(0.0) * scale + 0.5;
    let mut lines = 0usize;
    for run in buffer.layout_runs() {
        lines += 1;
        if lines > max_lines || run.line_w > maximum {
            return false;
        }
    }
    true
}

pub(super) fn build_text_projection(
    content: &Arc<str>,
    highlights: Option<&Arc<[TextHighlight]>>,
    affix: &Arc<str>,
    placement: TruncationPlacement,
) -> ProjectedText {
    let mut display = String::with_capacity(content.len().saturating_add(affix.len()));
    let mut retained = Vec::with_capacity(2);
    let (insertion_display, omitted_original) = match placement {
        TruncationPlacement::End { prefix_end } => {
            display.push_str(&content[..prefix_end]);
            if prefix_end > 0 {
                retained.push(ProjectionSpan {
                    display: 0..prefix_end,
                    original: 0..prefix_end,
                });
            }
            let insertion_start = display.len();
            display.push_str(affix);
            (insertion_start..display.len(), prefix_end..content.len())
        }
        TruncationPlacement::Start { suffix_start } => {
            display.push_str(affix);
            let insertion_display = 0..display.len();
            let suffix_display_start = display.len();
            display.push_str(&content[suffix_start..]);
            if suffix_start < content.len() {
                retained.push(ProjectionSpan {
                    display: suffix_display_start..display.len(),
                    original: suffix_start..content.len(),
                });
            }
            (insertion_display, 0..suffix_start)
        }
        TruncationPlacement::Middle {
            prefix_end,
            suffix_start,
        } => {
            display.push_str(&content[..prefix_end]);
            if prefix_end > 0 {
                retained.push(ProjectionSpan {
                    display: 0..prefix_end,
                    original: 0..prefix_end,
                });
            }
            let insertion_start = display.len();
            display.push_str(affix);
            let insertion_display = insertion_start..display.len();
            let suffix_display_start = display.len();
            display.push_str(&content[suffix_start..]);
            if suffix_start < content.len() {
                retained.push(ProjectionSpan {
                    display: suffix_display_start..display.len(),
                    original: suffix_start..content.len(),
                });
            }
            (insertion_display, prefix_end..suffix_start)
        }
    };
    let projected_highlights = project_text_highlights(
        highlights,
        &retained,
        insertion_display.clone(),
        omitted_original.clone(),
        placement,
    );
    ProjectedText {
        content: Arc::from(display),
        highlights: projected_highlights,
        mapping: TextProjection::Truncated {
            original_len: content.len(),
            retained: retained.into(),
            insertion_display,
            omitted_original,
        },
    }
}

pub(super) fn project_text_highlights(
    highlights: Option<&Arc<[TextHighlight]>>,
    retained: &[ProjectionSpan],
    insertion_display: Range<usize>,
    omitted_original: Range<usize>,
    placement: TruncationPlacement,
) -> Option<Arc<[TextHighlight]>> {
    let highlights = highlights?.as_ref();
    if highlights.is_empty() {
        return Some(Arc::from([]));
    }
    let mut projected = Vec::with_capacity(highlights.len().min(MAX_TEXT_HIGHLIGHTS));
    for span in retained {
        for highlight in highlights {
            let start = span.original.start.max(highlight.range.start);
            let end = span.original.end.min(highlight.range.end);
            if start < end {
                projected.push(TextHighlight {
                    range: span.display.start + start - span.original.start
                        ..span.display.start + end - span.original.start,
                    style: highlight.style.clone(),
                });
            }
        }
    }

    if !insertion_display.is_empty()
        && let Some(style) = affix_highlight_style(highlights, &omitted_original, placement)
    {
        projected.push(TextHighlight {
            range: insertion_display,
            style,
        });
    }
    projected.sort_unstable_by_key(|highlight| highlight.range.start);

    let mut merged: Vec<TextHighlight> = Vec::with_capacity(projected.len());
    for highlight in projected {
        if let Some(previous) = merged.last_mut()
            && previous.range.end == highlight.range.start
            && previous.style == highlight.style
        {
            previous.range.end = highlight.range.end;
        } else if merged.len() < MAX_TEXT_HIGHLIGHTS {
            merged.push(highlight);
        }
    }
    Some(merged.into())
}

pub(super) fn affix_highlight_style(
    highlights: &[TextHighlight],
    omitted: &Range<usize>,
    placement: TruncationPlacement,
) -> Option<crate::HighlightStyle> {
    let prefer_previous = !matches!(placement, TruncationPlacement::Start { .. });
    if prefer_previous {
        highlights
            .iter()
            .rev()
            .find(|highlight| {
                highlight.range.start < omitted.start && highlight.range.end >= omitted.start
            })
            .or_else(|| {
                highlights.iter().find(|highlight| {
                    highlight.range.start <= omitted.start && highlight.range.end > omitted.start
                })
            })
            .map(|highlight| highlight.style.clone())
    } else {
        highlights
            .iter()
            .find(|highlight| {
                highlight.range.start <= omitted.end && highlight.range.end > omitted.end
            })
            .or_else(|| {
                highlights.iter().rev().find(|highlight| {
                    highlight.range.start < omitted.end && highlight.range.end >= omitted.end
                })
            })
            .map(|highlight| highlight.style.clone())
    }
}

pub(super) fn should_fragment_basic_text(content: &str, style: &TextStyle) -> bool {
    style.shaping == TextShaping::Basic
        && style.wrap == TextWrap::None
        && matches!(style.align, TextAlign::Left | TextAlign::Start)
        && style.text_overflow.is_none()
        && style.line_clamp.is_none()
        && content.len() >= BASIC_FRAGMENT_MIN_BYTES
        && content
            .bytes()
            .all(|byte| byte.is_ascii_graphic() || byte == b' ')
        && content.bytes().any(|byte| byte.is_ascii_digit())
}

pub(super) struct BasicTextFragments<'a> {
    content: &'a str,
    cursor: usize,
}

impl<'a> BasicTextFragments<'a> {
    pub(super) fn new(content: &'a str) -> Self {
        Self { content, cursor: 0 }
    }
}

impl<'a> Iterator for BasicTextFragments<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        let bytes = self.content.as_bytes();
        let start = self.cursor;
        let first = *bytes.get(start)?;
        if first.is_ascii_digit() {
            self.cursor += 1;
            if self.cursor < bytes.len() && bytes[self.cursor].is_ascii_digit() {
                self.cursor += 1;
            }
        } else {
            self.cursor += 1;
            while self.cursor < bytes.len() && !bytes[self.cursor].is_ascii_digit() {
                self.cursor += 1;
            }
        }
        Some(&self.content[start..self.cursor])
    }
}

pub(super) fn text_buffer_width(buffer: &Buffer) -> f32 {
    buffer
        .layout_runs()
        .next()
        .map_or(0.0, |layout| layout.line_w)
}

pub(super) fn canonical_text_width(
    wrap: TextWrap,
    align: TextAlign,
    has_text_overflow: bool,
    width: Option<f32>,
) -> Option<f32> {
    match (wrap, align, has_text_overflow) {
        (TextWrap::None, TextAlign::Left | TextAlign::Start, false) => None,
        (TextWrap::None, _, true) => width.map(f32::ceil),
        (
            TextWrap::None | TextWrap::Word | TextWrap::WordWithTrailingSpace | TextWrap::Glyph,
            _,
            _,
        ) => width,
    }
}

pub(super) fn collect_styled_text_geometry(
    buffer: &Buffer,
    highlights: &[TextHighlight],
    style: &TextStyle,
    scale: f32,
    visible_y: Range<f32>,
    line_clamp: Option<usize>,
) -> StyledTextGeometry {
    const MAX_TEXT_GEOMETRY_RECTS: usize = MAX_TEXT_HIGHLIGHTS * 4;

    let scale = scale.max(f32::EPSILON);
    let mut backgrounds: Vec<TextPaintRect> = Vec::with_capacity(highlights.len().min(32));
    let mut decorations = Vec::with_capacity(highlights.len().min(32));
    for run in buffer.layout_runs().take(line_clamp.unwrap_or(usize::MAX)) {
        let line_top = run.line_top / scale;
        let line_bottom = (run.line_top + run.line_height) / scale;
        if line_top >= visible_y.end || line_bottom <= visible_y.start {
            continue;
        }

        let mut start = 0;
        while start < run.glyphs.len() {
            let metadata = run.glyphs[start].metadata;
            let mut end = start + 1;
            while end < run.glyphs.len() && run.glyphs[end].metadata == metadata {
                end += 1;
            }
            if let Some(highlight) = metadata
                .checked_sub(1)
                .and_then(|index| highlights.get(index))
                .filter(|highlight| {
                    highlight
                        .style
                        .background
                        .is_some_and(|color| color.a > 0.0)
                })
            {
                let mut left = f32::INFINITY;
                let mut right = f32::NEG_INFINITY;
                for glyph in &run.glyphs[start..end] {
                    left = left.min(glyph.x);
                    right = right.max(glyph.x + glyph.w);
                }
                if right > left && backgrounds.len() < MAX_TEXT_GEOMETRY_RECTS {
                    let h = &highlight.style;
                    let inset = h.background_inset_y.min(run.line_height / scale * 0.5);
                    let next = TextPaintRect {
                        rect: Rect::new(
                            left / scale - h.background_padding_x,
                            line_top + inset,
                            (right - left) / scale + 2.0 * h.background_padding_x,
                            run.line_height / scale - 2.0 * inset,
                        ),
                        color: h.background.unwrap(),
                        kind: if h.background_radius > 0.0 {
                            TextPaintKind::Rounded(h.background_radius)
                        } else {
                            TextPaintKind::Solid
                        },
                    };
                    if let Some(previous) = backgrounds.last_mut().filter(|p| {
                        p.color == next.color
                            && p.kind == next.kind
                            && p.rect.y == next.rect.y
                            && p.rect.height == next.rect.height
                            && next.rect.x <= p.rect.right() + 0.001
                            && next.rect.x >= p.rect.x
                    }) {
                        previous.rect.width =
                            previous.rect.right().max(next.rect.right()) - previous.rect.x;
                    } else {
                        backgrounds.push(next);
                    }
                }
            }
            start = end;
        }
        collect_decoration_geometry(
            &run,
            highlights,
            style,
            scale,
            MAX_TEXT_GEOMETRY_RECTS,
            &mut decorations,
        );
    }
    StyledTextGeometry {
        backgrounds,
        decorations,
    }
}

#[derive(Clone, Copy)]
struct ResolvedUnderlinePaint {
    wavy: bool,
    thickness: f32,
}

fn resolved_underline_paint(
    metadata: usize,
    highlights: &[TextHighlight],
    style: &TextStyle,
) -> ResolvedUnderlinePaint {
    if let Some(highlight) = metadata
        .checked_sub(1)
        .and_then(|index| highlights.get(index))
        .filter(|highlight| highlight.style.underline != TextUnderline::None)
    {
        return ResolvedUnderlinePaint {
            wavy: highlight
                .style
                .underline_wavy
                .unwrap_or(style.underline_wavy),
            thickness: highlight
                .style
                .underline_thickness
                .unwrap_or(style.underline_thickness),
        };
    }
    ResolvedUnderlinePaint {
        wavy: style.underline_wavy,
        thickness: style.underline_thickness,
    }
}

pub(super) fn collect_decoration_geometry(
    run: &LayoutRun<'_>,
    highlights: &[TextHighlight],
    style: &TextStyle,
    scale: f32,
    limit: usize,
    decorations: &mut Vec<TextPaintRect>,
) {
    for span in run.decorations {
        let mut start = span.glyph_range.start;
        while start < span.glyph_range.end && decorations.len() < limit {
            let metadata = run.glyphs[start].metadata;
            let mut end = start + 1;
            while end < span.glyph_range.end && run.glyphs[end].metadata == metadata {
                end += 1;
            }
            collect_decoration_group(
                run,
                span,
                start,
                end,
                metadata,
                highlights,
                style,
                scale,
                limit,
                decorations,
            );
            start = end;
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn collect_decoration_group(
    run: &LayoutRun<'_>,
    span: &DecorationSpan,
    start: usize,
    end: usize,
    metadata: usize,
    highlights: &[TextHighlight],
    style: &TextStyle,
    scale: f32,
    limit: usize,
    decorations: &mut Vec<TextPaintRect>,
) {
    let glyphs = &run.glyphs[start..end];
    let Some((x, width)) = decoration_horizontal_span(glyphs) else {
        return;
    };
    let data = &span.data;
    let text_decoration = &data.text_decoration;
    let fallback_color = glyphs[0]
        .color_opt
        .unwrap_or_else(|| glyph_color(style.color));
    let font_size = glyphs[0].font_size;

    if !matches!(text_decoration.underline, GlyphUnderlineStyle::None) {
        let underline = resolved_underline_paint(metadata, highlights, style);
        let thickness = (underline.thickness.max(0.0) * scale).max(0.0);
        if thickness > 0.0 {
            let color = ui_color(
                text_decoration
                    .underline_color_opt
                    .unwrap_or(fallback_color),
            );
            let descent_fraction = metadata
                .checked_sub(1)
                .and_then(|index| highlights.get(index))
                .and_then(|highlight| highlight.style.underline_descent_fraction);
            let y = run.line_y
                + descent_fraction.map_or(-data.underline_metrics.offset, |fraction| {
                    data.descent * fraction
                }) * font_size;
            if descent_fraction.is_some() && !underline.wavy {
                decorations.push(TextPaintRect {
                    rect: Rect::new(
                        x / scale,
                        y / scale,
                        width.round() / scale,
                        thickness.round() / scale,
                    ),
                    color,
                    kind: TextPaintKind::SolidUnderlay,
                });
            } else {
                push_underline_geometry(
                    decorations,
                    limit,
                    x,
                    y,
                    width,
                    thickness,
                    color,
                    underline.wavy,
                    scale,
                );
            }
            if matches!(text_decoration.underline, GlyphUnderlineStyle::Double) {
                push_underline_geometry(
                    decorations,
                    limit,
                    x,
                    y + thickness * 2.0,
                    width,
                    thickness,
                    color,
                    underline.wavy,
                    scale,
                );
            }
        }
    }

    if text_decoration.overline && decorations.len() < limit {
        let color = ui_color(text_decoration.overline_color_opt.unwrap_or(fallback_color));
        let thickness = (data.underline_metrics.thickness * font_size)
            .max(1.0)
            .ceil();
        // `ascent` is in EM above the baseline; the overline sits on the ascent, like CSS.
        let y = run.line_y - data.ascent * font_size;
        push_solid_decoration(decorations, limit, x, y, width, thickness, color, scale);
    }

    if text_decoration.strikethrough && decorations.len() < limit {
        let color = ui_color(
            text_decoration
                .strikethrough_color_opt
                .unwrap_or(fallback_color),
        );
        let thickness = (data.strikethrough_metrics.thickness * font_size)
            .max(1.0)
            .ceil();
        let y = run.line_y - data.strikethrough_metrics.offset * font_size;
        push_solid_decoration(decorations, limit, x, y, width, thickness, color, scale);
    }
}

pub(super) fn decoration_horizontal_span(glyphs: &[LayoutGlyph]) -> Option<(f32, f32)> {
    let mut left = f32::INFINITY;
    let mut right = f32::NEG_INFINITY;
    for glyph in glyphs {
        left = left.min(glyph.x);
        right = right.max(glyph.x + glyph.w);
    }
    let width = right - left;
    (width > 0.0).then_some((left, width))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_underline_geometry(
    decorations: &mut Vec<TextPaintRect>,
    limit: usize,
    x: f32,
    y: f32,
    width: f32,
    thickness: f32,
    color: UiColor,
    wavy: bool,
    scale: f32,
) {
    if !wavy {
        push_solid_decoration(
            decorations,
            limit,
            x,
            y,
            width,
            thickness.ceil(),
            color,
            scale,
        );
        return;
    }
    if decorations.len() >= limit || color.a <= 0.0 {
        return;
    }

    let logical_x = x.floor() / scale;
    let logical_width = width.floor() / scale;
    let logical_thickness = thickness / scale;
    if logical_width <= 0.0 || logical_thickness <= 0.0 {
        return;
    }
    let baseline = y.floor() / scale + logical_thickness * 0.5;
    let amplitude = (logical_thickness * 1.5).clamp(1.0, 4.0);
    let wavelength = (amplitude * 4.0).max(4.0);
    let antialias_margin = scale.recip();
    decorations.push(TextPaintRect {
        rect: Rect::new(
            logical_x,
            baseline - amplitude - logical_thickness * 0.5 - antialias_margin,
            logical_width,
            amplitude * 2.0 + logical_thickness + antialias_margin * 2.0,
        ),
        color,
        kind: TextPaintKind::WavyUnderline {
            baseline,
            amplitude,
            thickness: logical_thickness,
            wavelength,
        },
    });
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_solid_decoration(
    decorations: &mut Vec<TextPaintRect>,
    limit: usize,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    color: UiColor,
    scale: f32,
) {
    let x = x as i32;
    let y = y as i32;
    let width = width as u32;
    let height = height as u32;
    if decorations.len() >= limit || width == 0 || height == 0 || color.a <= 0.0 {
        return;
    }
    decorations.push(TextPaintRect {
        rect: Rect::new(
            x as f32 / scale,
            y as f32 / scale,
            width as f32 / scale,
            height as f32 / scale,
        ),
        color,
        kind: TextPaintKind::Solid,
    });
}

pub(super) fn configure_text_buffer(
    buffer: &mut Buffer,
    font_system: &mut FontSystem,
    content: &str,
    style: &TextStyle,
    highlights: Option<&[TextHighlight]>,
    width: Option<f32>,
    scale: f32,
) {
    let metrics = Metrics::new(style.font_size * scale, style.line_height * scale);
    buffer.set_metrics_and_size(metrics, width.map(|value| value * scale), None);
    buffer.set_monospace_width(style.monospace_width.map(|width| width * scale));
    buffer.set_wrap(cosmic_wrap(style));
    buffer.set_ellipsize(cosmic_ellipsize(style));
    buffer.set_base_direction(match style.direction {
        TextDirection::Auto => BaseDirection::Auto,
        TextDirection::Ltr => BaseDirection::Ltr,
        TextDirection::Rtl => BaseDirection::Rtl,
    });
    let mut attrs = Attrs::new()
        .family(glyph_family(&style.family))
        .weight(style.weight)
        .optical_size(style.font_size)
        .style(style.font_style)
        .font_features(style.features.cosmic());
    let mut flags = CacheKeyFlags::empty();
    if style.font_thicken {
        flags |= CacheKeyFlags::FONT_THICKEN;
    }
    // Keep native rasterization separate from shaping: platform glyphs retain the exact font,
    // variations and subpixel positions selected by the shared layout engine.
    if cfg!(target_os = "macos") {
        flags |= CacheKeyFlags::DISABLE_HINTING | CacheKeyFlags::NATIVE_RASTERIZATION;
    }
    if cfg!(target_os = "windows") {
        flags |= CacheKeyFlags::NATIVE_RASTERIZATION;
    }
    if !flags.is_empty() {
        attrs = attrs.cache_key_flags(flags);
    }
    // Cosmic Text tracks spacing in EM, and the buffer's metrics are already scaled, so a logical
    // pixel amount converts with the unscaled font size.
    if style.letter_spacing != 0.0 && style.font_size > 0.0 {
        attrs = attrs.letter_spacing(style.letter_spacing / style.font_size);
    }
    if style.word_spacing != 0.0 && style.font_size > 0.0 {
        attrs = attrs.word_spacing(style.word_spacing / style.font_size);
    }
    if style.overline {
        attrs = attrs.overline();
    }
    if let Some(color) = style.overline_color {
        attrs = attrs.overline_color(glyph_color(color));
    }
    if let Some(fallbacks) = style.fallbacks.as_ref() {
        attrs = attrs.font_fallbacks(fallbacks.cosmic());
    }
    attrs = match style.underline {
        TextUnderline::None => attrs,
        TextUnderline::Single => attrs.underline(GlyphUnderlineStyle::Single),
        TextUnderline::Double => attrs.underline(GlyphUnderlineStyle::Double),
    };
    if let Some(color) = style.underline_color {
        attrs = attrs.underline_color(glyph_color(color));
    }
    if style.strikethrough {
        attrs = attrs.strikethrough();
    }
    if let Some(color) = style.strikethrough_color {
        attrs = attrs.strikethrough_color(glyph_color(color));
    }
    let shaping = match style.shaping {
        TextShaping::Advanced => GlyphShaping::Advanced,
        TextShaping::Basic => GlyphShaping::Basic,
    };
    if let Some(highlights) = highlights.filter(|highlights| !highlights.is_empty()) {
        let mut spans = Vec::with_capacity(highlights.len() * 2 + 1);
        let mut cursor = 0;
        for (index, highlight) in highlights.iter().enumerate() {
            if cursor < highlight.range.start {
                spans.push((&content[cursor..highlight.range.start], attrs.clone()));
            }
            let mut highlight_attrs = attrs
                .clone()
                .metadata(index + 1)
                .family(
                    highlight
                        .style
                        .family
                        .as_ref()
                        .map_or_else(|| glyph_family(&style.family), glyph_family),
                )
                .weight(highlight.style.weight.unwrap_or(style.weight))
                .style(highlight.style.glyph_style.unwrap_or(style.font_style));
            highlight_attrs.font_features = highlight
                .style
                .features
                .as_ref()
                .unwrap_or(&style.features)
                .cosmic();
            highlight_attrs.font_fallbacks = match highlight.style.fallbacks.as_ref() {
                Some(fallbacks) => fallbacks.as_ref().map(FontFallbacks::cosmic),
                None => style.fallbacks.as_ref().map(FontFallbacks::cosmic),
            };
            if let Some(color) = highlight.style.color {
                highlight_attrs = highlight_attrs.color(glyph_color(color));
            }
            highlight_attrs = match highlight.style.underline {
                TextUnderline::None => highlight_attrs,
                TextUnderline::Single => highlight_attrs.underline(GlyphUnderlineStyle::Single),
                TextUnderline::Double => highlight_attrs.underline(GlyphUnderlineStyle::Double),
            };
            if let Some(color) = highlight.style.underline_color {
                highlight_attrs = highlight_attrs.underline_color(glyph_color(color));
            }
            if highlight.style.strikethrough {
                highlight_attrs = highlight_attrs.strikethrough();
            }
            if let Some(color) = highlight.style.strikethrough_color {
                highlight_attrs = highlight_attrs.strikethrough_color(glyph_color(color));
            }
            spans.push((&content[highlight.range.clone()], highlight_attrs));
            cursor = highlight.range.end;
        }
        if cursor < content.len() {
            spans.push((&content[cursor..], attrs.clone()));
        }
        buffer.set_rich_text(spans, &attrs, shaping, Some(glyph_alignment(style.align)));
    } else {
        buffer.set_text(content, &attrs, shaping, Some(glyph_alignment(style.align)));
    }
    buffer.shape_until_scroll(font_system, false);
    #[cfg(target_os = "macos")]
    buffer.refine_native_wrapped_positions(font_system, scale);
}

/// Map QuickGUI's wrapping, word-break, and overflow-wrap declarations onto one Cosmic Text mode.
///
/// `word-break` wins over `overflow-wrap`, matching CSS. `KeepAll` is approximated by ordinary
/// word wrapping: QuickGUI never breaks inside a CJK run in that mode, but it also does not add
/// the extra CJK break opportunities `Normal` would allow.
pub(super) fn cosmic_wrap(style: &TextStyle) -> Wrap {
    let base = match style.wrap {
        TextWrap::None => return Wrap::None,
        TextWrap::Word => Wrap::Word,
        TextWrap::WordWithTrailingSpace => return Wrap::WordWithTrailingSpace,
        TextWrap::Glyph => Wrap::Glyph,
    };
    match (style.word_break, style.overflow_wrap) {
        (WordBreak::BreakAll, _) => Wrap::Glyph,
        (WordBreak::KeepAll, _) => Wrap::Word,
        (WordBreak::Normal, OverflowWrap::Anywhere) => Wrap::Glyph,
        (WordBreak::Normal, OverflowWrap::BreakWord) => Wrap::WordOrGlyph,
        (WordBreak::Normal, OverflowWrap::Normal) => base,
    }
}

pub(super) fn uses_cosmic_ellipsis(overflow: &TextOverflow) -> bool {
    match overflow {
        TextOverflow::Truncate(affix)
        | TextOverflow::TruncateStart(affix)
        | TextOverflow::TruncateMiddle(affix) => affix.as_ref() == "…",
    }
}

pub(super) fn cosmic_ellipsize(style: &TextStyle) -> Ellipsize {
    let limit = EllipsizeHeightLimit::Lines(style.line_clamp.unwrap_or(1).max(1));
    match style.text_overflow.as_ref() {
        Some(TextOverflow::Truncate(affix)) if affix.as_ref() == "…" => Ellipsize::End(limit),
        Some(TextOverflow::TruncateStart(affix)) if affix.as_ref() == "…" => {
            Ellipsize::Start(limit)
        }
        Some(TextOverflow::TruncateMiddle(affix)) if affix.as_ref() == "…" => {
            Ellipsize::Middle(limit)
        }
        None
        | Some(
            TextOverflow::Truncate(_)
            | TextOverflow::TruncateStart(_)
            | TextOverflow::TruncateMiddle(_),
        ) => Ellipsize::None,
    }
}

/// Reflow a configured buffer while preserving its shaped glyph-line cache.
pub(super) fn reflow_text_buffer(
    buffer: &mut Buffer,
    font_system: &mut FontSystem,
    width: Option<f32>,
    scale: f32,
) {
    buffer.set_size(width.map(|value| value * scale), None);
    buffer.shape_until_scroll(font_system, false);
    #[cfg(target_os = "macos")]
    buffer.refine_native_wrapped_positions(font_system, scale);
}

pub(super) fn glyph_alignment(align: TextAlign) -> GlyphAlign {
    match align {
        // Logical alignment is resolved against the element's direction while the layout tree is
        // built, so a shaped buffer only ever sees a physical edge. Mapping the logical variants
        // to their LTR resolution keeps a directly constructed `TextStyle` sane.
        TextAlign::Left | TextAlign::Start => GlyphAlign::Left,
        TextAlign::Center => GlyphAlign::Center,
        TextAlign::CenterIncludingWhitespace => GlyphAlign::CenterIncludingWhitespace,
        TextAlign::RightIncludingWhitespace => GlyphAlign::RightIncludingWhitespace,
        TextAlign::Right | TextAlign::End => GlyphAlign::Right,
        TextAlign::Justify => GlyphAlign::Justified,
    }
}

pub(super) fn glyph_color(color: UiColor) -> GlyphColor {
    let [red, green, blue, alpha] = color.to_srgba8();
    GlyphColor::rgba(red, green, blue, alpha)
}

pub(super) fn ui_color(color: GlyphColor) -> UiColor {
    let [red, green, blue, alpha] = color.as_rgba();
    UiColor::rgba8(red, green, blue, alpha)
}

pub(super) fn physical_text_bounds(rect: Rect, scale: f32) -> TextBounds {
    TextBounds {
        left: saturating_i32((rect.x * scale).floor()),
        top: saturating_i32((rect.y * scale).floor()),
        right: saturating_i32((rect.right() * scale).ceil()),
        bottom: saturating_i32((rect.bottom() * scale).ceil()),
    }
}

pub(super) fn saturating_i32(value: f32) -> i32 {
    value.clamp(i32::MIN as f32, i32::MAX as f32) as i32
}
