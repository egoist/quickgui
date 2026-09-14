//! Retained native Markdown parsing and rendering.
//!
//! The parser keeps a settled top-level prefix and reparses only the last unstable groups when
//! text is appended. Rendering is ordinary QuickGUI elements and retained [`StyledText`] values;
//! there is no webview, HTML layout engine, or per-frame parser.

mod mend;
mod parser;

use std::{collections::HashMap, sync::Arc};

#[cfg(feature = "editor")]
use std::collections::HashSet;

use crate::{
    AccessibilityRole, Color, Element, ElementId, FontFamily, FontWeight, HighlightStyle,
    IntoElement, MAX_TEXT_HIGHLIGHTS, StyledText, TextAlign, div, text,
};

use parser::{IncrementalParser, TopBlock};
pub use parser::{
    MarkdownBlock, MarkdownInlineRun, MarkdownInlineStyle, MarkdownListItem, MarkdownTableAlign,
};

#[cfg(feature = "editor")]
use crate::{CodeBlock, CodeBlockStyle, SyntaxLanguage};

/// Maximum UTF-8 source retained by one Markdown component.
pub const MAX_MARKDOWN_SOURCE_BYTES: usize = 4 * 1024 * 1024;
/// Maximum parsed top-level blocks retained by one Markdown component.
pub const MAX_MARKDOWN_BLOCKS: usize = 32_768;
/// Maximum recursive Markdown container depth.
pub const MAX_MARKDOWN_NESTING_DEPTH: usize = 64;
const MAX_MARKDOWN_TABLE_COLUMNS: usize = 64;

/// Semantic Markdown metrics and optional colors.
///
/// `None` colors inherit from the surrounding element. The default remains neutral across light
/// and dark application palettes while retaining heading hierarchy, spacing, code typography,
/// and list/table structure.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MarkdownStyle {
    pub text_color: Option<Color>,
    pub muted_color: Option<Color>,
    pub link_color: Option<Color>,
    pub code_text_color: Option<Color>,
    pub code_background: Option<Color>,
    pub border_color: Option<Color>,
    pub font_size: f32,
    pub line_height: f32,
    pub block_gap: f32,
    pub code_font_size: f32,
}

impl MarkdownStyle {
    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = Some(color);
        self
    }

    pub fn muted_color(mut self, color: Color) -> Self {
        self.muted_color = Some(color);
        self
    }

    pub fn link_color(mut self, color: Color) -> Self {
        self.link_color = Some(color);
        self
    }

    pub fn code_text_color(mut self, color: Color) -> Self {
        self.code_text_color = Some(color);
        self
    }

    pub fn code_background(mut self, color: Color) -> Self {
        self.code_background = Some(color);
        self
    }

    pub fn border_color(mut self, color: Color) -> Self {
        self.border_color = Some(color);
        self
    }

    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = finite_at_least(size, 1.0, 15.0);
        self
    }

    pub fn line_height(mut self, height: f32) -> Self {
        self.line_height = finite_at_least(height, 1.0, 23.0);
        self
    }

    pub fn block_gap(mut self, gap: f32) -> Self {
        self.block_gap = finite_at_least(gap, 0.0, 12.0);
        self
    }

    pub fn code_font_size(mut self, size: f32) -> Self {
        self.code_font_size = finite_at_least(size, 1.0, 13.0);
        self
    }

    fn sanitized(mut self) -> Self {
        self.font_size = finite_at_least(self.font_size, 1.0, 15.0);
        self.line_height = finite_at_least(self.line_height, 1.0, 23.0);
        self.block_gap = finite_at_least(self.block_gap, 0.0, 12.0);
        self.code_font_size = finite_at_least(self.code_font_size, 1.0, 13.0);
        self
    }
}

impl Default for MarkdownStyle {
    fn default() -> Self {
        Self {
            text_color: None,
            muted_color: None,
            link_color: None,
            code_text_color: None,
            code_background: Some(Color::rgba8(127, 127, 127, 32)),
            border_color: Some(Color::rgba8(127, 127, 127, 72)),
            font_size: 15.0,
            line_height: 23.0,
            block_gap: 12.0,
            code_font_size: 13.0,
        }
    }
}

/// Result of changing retained Markdown source.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MarkdownUpdate {
    pub changed: bool,
    pub appended: bool,
    pub reparsed_from: usize,
    pub truncated: bool,
}

/// One parsed fenced or indented code block handed to a custom Markdown renderer.
///
/// The renderer should use `element_id` for stable retained identity. `language` contains the
/// optional fence info string and is `None` for an indented code block.
#[derive(Clone, Copy, Debug)]
pub struct MarkdownCodeBlock<'a> {
    pub language: Option<&'a str>,
    pub code: &'a str,
    pub element_id: ElementId,
}

type CodeBlockRenderer<'renderer> =
    dyn for<'source> FnMut(MarkdownCodeBlock<'source>) -> Element + 'renderer;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct FlatKey {
    source_start: usize,
    top_index: usize,
    slot: u32,
}

/// Caller-owned retained Markdown document.
///
/// Keep one instance per logical document and call [`Self::set_text`] as streamed content grows.
/// [`Self::element`] rebuilds only the cheap declarative element shell; parsed blocks and shaped
/// inline run inputs remain retained across frames.
pub struct Markdown {
    parser: IncrementalParser,
    display_tail: Option<Vec<TopBlock>>,
    flat_cache: HashMap<FlatKey, StyledText>,
    style: MarkdownStyle,
    streaming: bool,
    truncated: bool,
    #[cfg(feature = "editor")]
    code_blocks: HashMap<ElementId, CodeBlock>,
}

impl Default for Markdown {
    fn default() -> Self {
        Self::new()
    }
}

impl Markdown {
    pub fn new() -> Self {
        Self {
            parser: IncrementalParser::new(),
            display_tail: None,
            flat_cache: HashMap::new(),
            style: MarkdownStyle::default(),
            streaming: false,
            truncated: false,
            #[cfg(feature = "editor")]
            code_blocks: HashMap::new(),
        }
    }

    pub fn with_text(text: &str) -> Self {
        let mut markdown = Self::new();
        markdown.set_text(text);
        markdown
    }

    pub fn text(&self) -> &str {
        self.parser.text()
    }

    pub fn block_count(&self) -> usize {
        self.canonical_display_prefix_len() + self.display_tail.as_ref().map_or(0, Vec::len)
    }

    pub const fn is_streaming(&self) -> bool {
        self.streaming
    }

    pub const fn is_truncated(&self) -> bool {
        self.truncated
    }

    pub const fn style(&self) -> MarkdownStyle {
        self.style
    }

    pub fn set_style(&mut self, style: MarkdownStyle) -> bool {
        let style = style.sanitized();
        if self.style == style {
            return false;
        }
        self.style = style;
        self.flat_cache.clear();
        true
    }

    pub fn set_streaming(&mut self, streaming: bool) -> bool {
        if self.streaming == streaming {
            return false;
        }
        self.streaming = streaming;
        self.invalidate_display_tail();
        true
    }

    /// Point the retained document at new controlled source.
    ///
    /// Append-only changes reuse the settled prefix. Rewrites and link reference definitions fall
    /// back to a bounded full parse. Input beyond [`MAX_MARKDOWN_SOURCE_BYTES`] is truncated at a
    /// UTF-8 boundary.
    pub fn set_text(&mut self, text: &str) -> MarkdownUpdate {
        let (text, truncated) = bounded_source(text);
        let previous = self.parser.text();
        let appended = text.len() >= previous.len() && text.starts_with(previous);
        let changed = self.parser.set_text(text);
        self.truncated = truncated;
        if !changed {
            return MarkdownUpdate {
                changed: false,
                appended,
                reparsed_from: self.parser.reparsed_from(),
                truncated,
            };
        }
        let reparsed_from = self.parser.reparsed_from();
        if appended {
            self.flat_cache
                .retain(|key, _| key.source_start < reparsed_from);
        } else {
            self.flat_cache.clear();
        }
        self.refresh_display_tail();
        MarkdownUpdate {
            changed: true,
            appended,
            reparsed_from,
            truncated,
        }
    }

    /// Render the document as ordinary native QuickGUI elements.
    pub fn element(&mut self, id: impl Into<ElementId>) -> Element {
        self.render_element(id.into(), None)
    }

    /// Render code blocks with a caller-supplied component factory.
    ///
    /// Parsing remains owned by this retained Markdown model; the callback only replaces the
    /// element for each parsed code block. Keep any component state used by the callback in the
    /// owning view and key it by [`MarkdownCodeBlock::element_id`].
    pub fn element_with_code_blocks<R>(
        &mut self,
        id: impl Into<ElementId>,
        mut renderer: R,
    ) -> Element
    where
        R: for<'source> FnMut(MarkdownCodeBlock<'source>) -> Element,
    {
        self.render_element(id.into(), Some(&mut renderer))
    }

    /// Render parsed code blocks through retained [`CodeBlock`] components.
    ///
    /// This convenience is available when both `markdown` and `editor` are enabled. Each parsed
    /// block keeps independent virtual-scroll state. `max_height` is sanitized to a finite value;
    /// shorter blocks use their natural logical-line height.
    #[cfg(feature = "editor")]
    pub fn element_with_highlighted_code_blocks(
        &mut self,
        id: impl Into<ElementId>,
        style: CodeBlockStyle,
        max_height: f32,
    ) -> Element {
        let mut code_blocks = std::mem::take(&mut self.code_blocks);
        let mut live = HashSet::new();
        let max_height = if max_height.is_finite() {
            max_height.clamp(40.0, 4_096.0)
        } else {
            320.0
        };
        let element = self.element_with_code_blocks(id, |request| {
            live.insert(request.element_id);
            let block = code_blocks
                .entry(request.element_id)
                .or_insert_with(|| CodeBlock::with_text(request.code));
            block.set_text(request.code);
            block.set_language(
                request
                    .language
                    .and_then(SyntaxLanguage::from_name)
                    .unwrap_or(SyntaxLanguage::PlainText),
            );
            block.set_style(style);
            let style = block.style();
            let natural_height =
                block.line_count() as f32 * style.line_height + style.content_padding_y * 2.0 + 2.0;
            block
                .element(request.element_id)
                .h(natural_height.min(max_height))
        });
        code_blocks.retain(|id, _| live.contains(id));
        self.code_blocks = code_blocks;
        element
    }

    fn render_element(
        &mut self,
        root_id: ElementId,
        mut code_block_renderer: Option<&mut CodeBlockRenderer<'_>>,
    ) -> Element {
        let mut root = div()
            .id(root_id)
            .w_full()
            .min_w(0.0)
            .flex_col()
            .gap(self.style.block_gap)
            .user_select_text()
            .accessibility_role(AccessibilityRole::Group);
        if let Some(color) = self.style.text_color {
            root = root.text_color(color);
        }

        let style = self.style;
        let canonical_len = self.canonical_display_prefix_len();
        let cache = &mut self.flat_cache;
        let canonical = &self.parser.tree().blocks;
        let mut elements = Vec::new();
        for (index, block) in canonical[..canonical_len].iter().enumerate() {
            let mut slot = 0;
            elements.push(render_block(
                &block.block,
                block.range.start,
                &mut slot,
                root_id,
                index,
                0,
                style,
                cache,
                &mut code_block_renderer,
            ));
        }
        if let Some(tail) = &self.display_tail {
            for (tail_index, block) in tail.iter().enumerate() {
                let mut slot = 0;
                elements.push(render_block(
                    &block.block,
                    block.range.start,
                    &mut slot,
                    root_id,
                    canonical_len + tail_index,
                    0,
                    style,
                    cache,
                    &mut code_block_renderer,
                ));
            }
        }
        root.children(elements)
    }

    fn refresh_display_tail(&mut self) {
        self.display_tail = self.streaming.then(|| self.parser.display_tail()).flatten();
    }

    fn canonical_display_prefix_len(&self) -> usize {
        let canonical = &self.parser.tree().blocks;
        let Some(tail_start) = self
            .display_tail
            .as_ref()
            .and_then(|tail| tail.first())
            .map(|block| block.range.start)
        else {
            return canonical.len();
        };
        canonical.partition_point(|block| block.range.start < tail_start)
    }

    fn invalidate_display_tail(&mut self) {
        if let Some(start) = self
            .parser
            .tree()
            .blocks
            .last()
            .map(|block| block.range.start)
        {
            self.flat_cache.retain(|key, _| key.source_start < start);
        }
        self.refresh_display_tail();
    }
}

#[allow(clippy::too_many_arguments)]
fn render_block(
    block: &MarkdownBlock,
    source_start: usize,
    slot: &mut u32,
    root_id: ElementId,
    top_index: usize,
    depth: usize,
    style: MarkdownStyle,
    cache: &mut HashMap<FlatKey, StyledText>,
    code_block_renderer: &mut Option<&mut CodeBlockRenderer<'_>>,
) -> Element {
    let (id, block_slot) = allocate_id(root_id, top_index, depth, slot);
    match block {
        MarkdownBlock::Paragraph(runs) => inline_element(
            runs,
            source_start,
            top_index,
            block_slot,
            id,
            style,
            style.font_size,
            style.line_height,
            FontWeight::NORMAL,
            cache,
        ),
        MarkdownBlock::Heading { level, runs } => {
            let (font_size, line_height) = heading_metrics(*level, style);
            inline_element(
                runs,
                source_start,
                top_index,
                block_slot,
                id,
                style,
                font_size,
                line_height,
                FontWeight::BOLD,
                cache,
            )
            .accessibility_role(AccessibilityRole::Heading)
        }
        MarkdownBlock::CodeBlock { language, code } => {
            let (code_id, code_slot) = allocate_id(root_id, top_index, depth + 1, slot);
            let language = language.as_deref().filter(|language| !language.is_empty());
            let label_id = language.map(|_| allocate_id(root_id, top_index, depth + 1, slot).0);
            if let Some(renderer) = code_block_renderer.as_deref_mut() {
                return renderer(MarkdownCodeBlock {
                    language,
                    code,
                    element_id: id,
                })
                .id(id);
            }
            let code_key = FlatKey {
                source_start,
                top_index,
                slot: code_slot,
            };
            let styled = cache
                .entry(code_key)
                .or_insert_with(|| StyledText::new(Arc::<str>::from(code.as_str())))
                .clone();
            let mut code_text = styled
                .into_element()
                .id(code_id)
                .w_full()
                .min_w(0.0)
                .wrap()
                .font_family(FontFamily::Monospace)
                .text_size(style.code_font_size)
                .line_height((style.code_font_size * 1.55).max(style.code_font_size + 2.0));
            if let Some(color) = style.code_text_color.or(style.text_color) {
                code_text = code_text.text_color(color);
            }
            let mut container = div()
                .id(id)
                .w_full()
                .min_w(0.0)
                .flex_col()
                .gap_2()
                .p_3()
                .rounded_lg();
            if let Some(background) = style.code_background {
                container = container.bg(background);
            }
            if let Some(border) = style.border_color {
                container = container.border(1.0, border);
            }
            let mut children = Vec::with_capacity(2);
            if let Some(language) = language {
                let mut label = text(Arc::<str>::from(language))
                    .id(label_id.expect("a non-empty code language allocated a label id"))
                    .text_size(11.0)
                    .line_height(14.0)
                    .font_semibold();
                if let Some(color) = style.muted_color {
                    label = label.text_color(color);
                }
                children.push(label);
            }
            children.push(code_text);
            container.children(children)
        }
        MarkdownBlock::BlockQuote(children) => {
            let (marker_id, _) = allocate_id(root_id, top_index, depth + 1, slot);
            let mut marker = div().id(marker_id).w(3.0).self_stretch().rounded_sm();
            if let Some(color) = style.border_color {
                marker = marker.bg(color);
            }
            let content = render_blocks(
                children,
                source_start,
                slot,
                root_id,
                top_index,
                depth + 1,
                style,
                cache,
                code_block_renderer,
            )
            .flex_1()
            .min_w(0.0);
            let mut quote = div()
                .id(id)
                .w_full()
                .min_w(0.0)
                .flex_row()
                .items_stretch()
                .gap_3()
                .child(marker)
                .child(content);
            if let Some(color) = style.muted_color {
                quote = quote.text_color(color);
            }
            quote
        }
        MarkdownBlock::List {
            ordered_start,
            items,
        } => {
            let mut rows = Vec::with_capacity(items.len());
            for (item_index, item) in items.iter().enumerate() {
                let (marker_id, _) = allocate_id(root_id, top_index, depth + 1, slot);
                let marker = item.task.map_or_else(
                    || {
                        ordered_start.map_or_else(
                            || "•".to_owned(),
                            |start| format!("{}.", start.saturating_add(item_index as u64)),
                        )
                    },
                    |checked| if checked { "☑" } else { "☐" }.to_owned(),
                );
                let mut marker = text(marker)
                    .id(marker_id)
                    .w(26.0)
                    .flex_none()
                    .text_size(style.font_size)
                    .line_height(style.line_height)
                    .text_right();
                if let Some(color) = style.muted_color {
                    marker = marker.text_color(color);
                }
                let body = render_blocks(
                    &item.blocks,
                    source_start,
                    slot,
                    root_id,
                    top_index,
                    depth + 1,
                    style,
                    cache,
                    code_block_renderer,
                )
                .flex_1()
                .min_w(0.0);
                let (row_id, _) = allocate_id(root_id, top_index, depth + 1, slot);
                rows.push(
                    div()
                        .id(row_id)
                        .w_full()
                        .min_w(0.0)
                        .flex_row()
                        .items_start()
                        .gap_2()
                        .child(marker)
                        .child(body),
                );
            }
            div()
                .id(id)
                .w_full()
                .min_w(0.0)
                .flex_col()
                .gap_2()
                .children(rows)
        }
        MarkdownBlock::Table {
            header,
            rows,
            align,
        } => {
            let columns = header
                .len()
                .max(rows.iter().map(Vec::len).max().unwrap_or(0))
                .clamp(1, MAX_MARKDOWN_TABLE_COLUMNS);
            let mut cells = Vec::new();
            for row in std::iter::once(header).chain(rows.iter()) {
                let is_header = std::ptr::eq(row, header);
                for column in 0..columns {
                    let runs = row.get(column).map(Vec::as_slice).unwrap_or(&[]);
                    let (cell_id, cell_slot) = allocate_id(root_id, top_index, depth + 1, slot);
                    let mut cell = inline_element(
                        runs,
                        source_start,
                        top_index,
                        cell_slot,
                        cell_id,
                        style,
                        (style.font_size - 1.0).max(11.0),
                        style.line_height,
                        if is_header {
                            FontWeight::SEMIBOLD
                        } else {
                            FontWeight::NORMAL
                        },
                        cache,
                    )
                    .p_2()
                    .text_align(
                        match align.get(column).copied().unwrap_or_default() {
                            MarkdownTableAlign::Left => TextAlign::Left,
                            MarkdownTableAlign::Center => TextAlign::Center,
                            MarkdownTableAlign::Right => TextAlign::Right,
                        },
                    );
                    if let Some(border) = style.border_color {
                        cell = cell.border(1.0, border);
                    }
                    cells.push(cell);
                }
            }
            div()
                .id(id)
                .w_full()
                .min_w(0.0)
                .grid()
                .grid_cols(columns as u16)
                .children(cells)
        }
        MarkdownBlock::Image { url, alt } => {
            let label = if alt.trim().is_empty() { "Image" } else { alt };
            let runs = [MarkdownInlineRun {
                text: format!("{label} ({url})"),
                style: MarkdownInlineStyle {
                    italic: true,
                    link: Some(url.clone()),
                    ..MarkdownInlineStyle::default()
                },
            }];
            inline_element(
                &runs,
                source_start,
                top_index,
                block_slot,
                id,
                style,
                style.font_size,
                style.line_height,
                FontWeight::NORMAL,
                cache,
            )
        }
        MarkdownBlock::Rule => {
            let mut rule = div().id(id).w_full().h(1.0);
            if let Some(color) = style.border_color {
                rule = rule.bg(color);
            }
            rule
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render_blocks(
    blocks: &[MarkdownBlock],
    source_start: usize,
    slot: &mut u32,
    root_id: ElementId,
    top_index: usize,
    depth: usize,
    style: MarkdownStyle,
    cache: &mut HashMap<FlatKey, StyledText>,
    code_block_renderer: &mut Option<&mut CodeBlockRenderer<'_>>,
) -> Element {
    let (id, _) = allocate_id(root_id, top_index, depth, slot);
    let children = blocks
        .iter()
        .map(|block| {
            render_block(
                block,
                source_start,
                slot,
                root_id,
                top_index,
                depth,
                style,
                cache,
                code_block_renderer,
            )
        })
        .collect::<Vec<_>>();
    div()
        .id(id)
        .w_full()
        .min_w(0.0)
        .flex_col()
        .gap(style.block_gap.min(8.0))
        .children(children)
}

#[allow(clippy::too_many_arguments)]
fn inline_element(
    runs: &[MarkdownInlineRun],
    source_start: usize,
    top_index: usize,
    cache_slot: u32,
    id: ElementId,
    style: MarkdownStyle,
    font_size: f32,
    line_height: f32,
    weight: FontWeight,
    cache: &mut HashMap<FlatKey, StyledText>,
) -> Element {
    let key = FlatKey {
        source_start,
        top_index,
        slot: cache_slot,
    };
    let styled = cache
        .entry(key)
        .or_insert_with(|| flatten_runs(runs, style))
        .clone();
    let mut element = styled
        .into_element()
        .id(id)
        .w_full()
        .min_w(0.0)
        .wrap()
        .text_size(font_size)
        .line_height(line_height)
        .font_weight(weight);
    if let Some(color) = style.text_color {
        element = element.text_color(color);
    }
    element
}

fn flatten_runs(runs: &[MarkdownInlineRun], style: MarkdownStyle) -> StyledText {
    let bytes = runs.iter().map(|run| run.text.len()).sum();
    let mut content = String::with_capacity(bytes);
    let mut highlights = Vec::with_capacity(runs.len().min(MAX_TEXT_HIGHLIGHTS));
    for run in runs {
        let start = content.len();
        content.push_str(&run.text);
        let end = content.len();
        if start == end || highlights.len() == MAX_TEXT_HIGHLIGHTS {
            continue;
        }
        let mut highlight = HighlightStyle::default();
        if run.style.bold {
            highlight = highlight.font_bold();
        }
        if run.style.italic {
            highlight = highlight.italic();
        }
        if run.style.strikethrough {
            highlight = highlight.strikethrough();
        }
        if run.style.code {
            highlight = highlight.font_family(FontFamily::Monospace);
            if let Some(color) = style.code_text_color {
                highlight = highlight.color(color);
            }
            if let Some(background) = style.code_background {
                highlight = highlight.background(background);
            }
        }
        if run.style.link.is_some() {
            highlight = highlight.underline();
            if let Some(color) = style.link_color {
                highlight = highlight.color(color).underline_color(color);
            }
        }
        highlights.push((start..end, highlight));
    }
    StyledText::new(Arc::<str>::from(content)).with_highlights(highlights)
}

fn heading_metrics(level: u8, style: MarkdownStyle) -> (f32, f32) {
    let scale = match level {
        1 => 1.75,
        2 => 1.45,
        3 => 1.25,
        4 => 1.12,
        5 => 1.0,
        _ => 0.92,
    };
    let size = (style.font_size * scale).max(11.0);
    (size, (size * 1.3).max(style.line_height))
}

fn derived_id(root: ElementId, top_index: usize, depth: usize, slot: u32) -> ElementId {
    let mut hash = root.as_u64()
        ^ (top_index as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15)
        ^ (depth as u64).rotate_left(19)
        ^ u64::from(slot).rotate_left(37)
        ^ 0x6d61_726b_646f_776e;
    hash ^= hash >> 30;
    hash = hash.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    hash ^= hash >> 27;
    hash = hash.wrapping_mul(0x94d0_49bb_1331_11eb);
    hash ^= hash >> 31;
    if hash == u64::MAX || hash == root.as_u64() {
        hash ^= 0xa5a5_5a5a_d3c4_b2e1;
    }
    ElementId::new(hash)
}

fn allocate_id(
    root: ElementId,
    top_index: usize,
    depth: usize,
    slot: &mut u32,
) -> (ElementId, u32) {
    let allocated = *slot;
    *slot = slot.saturating_add(1);
    (derived_id(root, top_index, depth, allocated), allocated)
}

fn bounded_source(source: &str) -> (&str, bool) {
    if source.len() <= MAX_MARKDOWN_SOURCE_BYTES {
        return (source, false);
    }
    let mut end = MAX_MARKDOWN_SOURCE_BYTES;
    while !source.is_char_boundary(end) {
        end -= 1;
    }
    (&source[..end], true)
}

fn finite_at_least(value: f32, minimum: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.max(minimum)
    } else {
        fallback
    }
}

#[cfg(all(test, not(quickgui_component_extension)))]
mod tests {
    use super::*;

    #[cfg(target_os = "macos")]
    use crate::{Application, View, ViewContext, WindowOptions};

    #[cfg(target_os = "macos")]
    struct WrappedMarkdownView(Markdown);

    #[cfg(target_os = "macos")]
    impl View for WrappedMarkdownView {
        fn render(&mut self, _cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            div()
                .size_full()
                .p_2()
                .child(self.0.element("wrapped-markdown"))
        }
    }

    #[test]
    fn retained_document_reuses_settled_flattened_text() {
        let mut markdown = Markdown::with_text("First.\n\nSecond.\n\nThird");
        let _ = markdown.element("markdown");
        let before = markdown.flat_cache.len();
        let update = markdown.set_text("First.\n\nSecond.\n\nThird grows.");
        assert!(update.appended);
        assert!(update.reparsed_from > 0);
        assert!(!markdown.flat_cache.is_empty());
        assert!(markdown.flat_cache.len() < before);
        let _ = markdown.element("markdown");
        assert!(markdown.flat_cache.len() >= before);
    }

    #[test]
    fn streaming_display_mends_then_settles() {
        let mut markdown = Markdown::new();
        markdown.set_streaming(true);
        markdown.set_text("Now **bold");
        assert!(markdown.display_tail.is_some());
        markdown.set_streaming(false);
        assert!(markdown.display_tail.is_none());
    }

    #[test]
    fn streaming_tail_replaces_every_block_from_one_source_group() {
        let mut markdown = Markdown::new();
        markdown.set_streaming(true);
        markdown.set_text("before ![alt](https://example.com/image.png) after **bold");
        let display_count = markdown.block_count();
        let canonical_prefix = markdown.canonical_display_prefix_len();
        let tail_count = markdown.display_tail.as_ref().map_or(0, Vec::len);
        assert_eq!(display_count, canonical_prefix + tail_count);
        assert_eq!(canonical_prefix, 0);
    }

    #[test]
    fn one_source_group_keeps_distinct_flattened_blocks() {
        let mut markdown =
            Markdown::with_text("before ![alt](https://example.com/image.png) after");
        let _ = markdown.element("markdown");
        let content = markdown
            .flat_cache
            .values()
            .map(|styled| styled.content().as_ref())
            .collect::<Vec<_>>();
        assert_eq!(content.len(), 3);
        assert!(content.contains(&"before "));
        assert!(content.contains(&"alt (https://example.com/image.png)"));
        assert!(content.contains(&" after"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn native_markdown_wraps_to_its_available_width() {
        let markdown = Markdown::with_text(
            "This is a deliberately long Markdown paragraph that must wrap across several lines.",
        );
        let (mut cx, view) = Application::new()
            .into_test_context(
                WindowOptions::default().size(180.0, 140.0),
                WrappedMarkdownView(markdown),
            )
            .unwrap();
        let bounds = cx
            .visual(view.window_handle())
            .unwrap()
            .element_bounds("wrapped-markdown")
            .unwrap();
        assert!(bounds.width <= 164.0);
        assert!(bounds.height > MarkdownStyle::default().line_height * 2.0);
    }

    #[test]
    fn source_limit_preserves_utf8_boundaries() {
        let source = format!("{}🙂", "a".repeat(MAX_MARKDOWN_SOURCE_BYTES - 1));
        let mut markdown = Markdown::new();
        let update = markdown.set_text(&source);
        assert!(update.truncated);
        assert!(markdown.text().is_char_boundary(markdown.text().len()));
        assert!(markdown.text().len() <= MAX_MARKDOWN_SOURCE_BYTES);
    }

    #[test]
    fn style_changes_invalidate_only_render_cache() {
        let mut markdown = Markdown::with_text("A **bold** paragraph.");
        let _ = markdown.element("markdown");
        assert!(!markdown.flat_cache.is_empty());
        assert!(markdown.set_style(MarkdownStyle::default().font_size(18.0)));
        assert!(markdown.flat_cache.is_empty());
        assert_eq!(markdown.block_count(), 1);
    }

    #[test]
    fn custom_code_block_renderer_receives_parsed_fence_data_and_stable_id() {
        let mut markdown = Markdown::with_text("before\n\n```rust\nfn main() {}\n```\n");
        let mut rendered = Vec::new();
        let element = markdown.element_with_code_blocks("custom-code", |block| {
            rendered.push((
                block.language.map(str::to_owned),
                block.code.to_owned(),
                block.element_id,
            ));
            div().child("custom code")
        });
        assert_eq!(rendered.len(), 1);
        assert_eq!(rendered[0].0.as_deref(), Some("rust"));
        assert_eq!(rendered[0].1, "fn main() {}");
        assert_eq!(element.children.len(), 2);
        assert_eq!(element.children[1].explicit_id, Some(rendered[0].2));
    }

    #[cfg(feature = "editor")]
    #[test]
    fn highlighted_code_blocks_retain_one_code_block_model_per_parsed_fence() {
        let mut markdown = Markdown::with_text(
            "```rust\nfn main() {}\n```\n\n```typescript\nconst answer = 42;\n```\n",
        );
        let element = markdown.element_with_highlighted_code_blocks(
            "highlighted-code",
            CodeBlockStyle::default().line_numbers(true),
            240.0,
        );
        assert_eq!(element.children.len(), 2);
        assert_eq!(markdown.code_blocks.len(), 2);
        assert!(
            markdown
                .code_blocks
                .values()
                .all(|block| block.style().line_numbers)
        );
    }
}
