//! Retained native code and unified-diff documents.
//!
//! Parsing and theme metrics follow GPUIX's Comet port; language adapters only
//! send source and configuration. Parsing is retained until source changes.
pub mod color;
pub mod diff;
pub mod syntax;
pub mod theme;

use crate::{
    Color, Element, ElementId, FontFamily, HighlightStyle, IntoElement, StyledText, div, text,
};
use std::{collections::HashSet, sync::Arc};
use syntax::{HighlightSpan, HighlightedDocument, cache::highlight_cached};
use theme::Theme;

#[derive(Default)]
pub struct CodeDocument {
    source: String,
    language: Option<String>,
    path: Option<String>,
    highlight: Option<Arc<HighlightedDocument>>,
}
impl CodeDocument {
    pub fn set_source(&mut self, source: &str, language: Option<&str>, path: Option<&str>) {
        if self.source == source
            && self.language.as_deref() == language
            && self.path.as_deref() == path
        {
            return;
        }
        self.source = source.replace("\r\n", "\n");
        self.language = language.map(str::to_owned);
        self.path = path.map(str::to_owned);
        self.highlight = highlight_cached(&self.source, path, language);
    }
    pub fn element(
        &self,
        id: ElementId,
        theme: &Theme,
        numbers: bool,
        size: f32,
        line_height: f32,
    ) -> Element {
        let count = self.source.split('\n').count();
        let m = &theme.metrics;
        let gutter = (count.max(1).to_string().len() as f32 * m.code_gutter_digit_width
            + m.code_gutter_padding_right)
            .max(m.code_gutter_min_width);
        let rows = self.source.split('\n').enumerate().map(|(i, line)| {
            let spans = self
                .highlight
                .as_ref()
                .and_then(|h| h.lines.get(i))
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            let mut row = div()
                .id(ElementId::new(
                    id.as_u64()
                        .wrapping_mul(0x9e3779b97f4a7c15)
                        .wrapping_add(i as u64 + 1)
                        | (1 << 63),
                ))
                .h(line_height)
                .flex_none()
                .flex_row();
            if numbers {
                row = row.child(
                    text((i + 1).to_string())
                        .w(gutter)
                        .padding(0., m.code_gutter_padding_right, 0., 0.)
                        .text_right()
                        .text_color(theme.text_faint.into())
                        .user_select_none(),
                );
            }
            row.child(
                styled_line(line, spans, theme)
                    .into_element()
                    .user_select_text(),
            )
        });
        div().id(id).min_w(0.).flex_row().overflow_x_scroll().child(
            div()
                .flex_col()
                .flex_none()
                .font_family(FontFamily::from(theme.font_mono.clone()))
                .text_size(size)
                .line_height(line_height)
                .text_color(theme.text.into())
                .whitespace_nowrap()
                .children(rows),
        )
    }
}

fn styled_line(line: &str, spans: &[HighlightSpan], theme: &Theme) -> StyledText {
    let mut styled = StyledText::new(Arc::<str>::from(line));
    for span in spans.iter().take(crate::MAX_TEXT_HIGHLIGHTS) {
        styled = styled.highlight(
            span.range.clone(),
            HighlightStyle::default().color(theme.syntax.color(span.kind).into()),
        );
    }
    styled
}

#[derive(Clone, Debug)]
pub enum DiffAction {
    ShowMore(u32),
    ToggleFile(String),
    LineClick {
        text: String,
        old_line: Option<u32>,
        new_line: Option<u32>,
    },
}

#[derive(Default)]
pub struct DiffDocument {
    source: String,
    files: Vec<diff::FileDiff>,
    highlights: Vec<Option<FileHighlight>>,
    list: Option<crate::ListState>,
    row_heights: Vec<f32>,
    scale_factor: f32,
}
impl DiffDocument {
    pub fn set_scale_factor(&mut self, scale_factor: f32) {
        self.scale_factor = if scale_factor.is_finite() {
            scale_factor.max(1.)
        } else {
            1.
        };
    }
    pub fn set_source(&mut self, source: &str) {
        if self.source == source {
            return;
        }
        self.source = source.to_owned();
        self.files = diff::parse_patch(source);
        diff::annotate_word_diffs(&mut self.files);
        self.highlights = self.files.iter().map(file_highlight).collect();
    }
    pub fn element(
        &mut self,
        id: ElementId,
        theme: &Theme,
        collapsed: &HashSet<String>,
        max_lines: Option<usize>,
        word_diff: bool,
        scroll: bool,
        mut decorate: impl FnMut(ElementId, Element, DiffAction) -> Element,
    ) -> Element {
        use diff::{DiffRow, LineKind};
        let m = &theme.metrics;
        let scale = self.scale_factor.max(1.);
        let line_height = |font_size: f32| (font_size * 1.618_034 * scale - 0.5).ceil() / scale;
        let ink = |a| {
            if theme.bg.l < 0.5 {
                Color::WHITE.with_alpha(a)
            } else {
                Color::BLACK.with_alpha(a)
            }
        };
        let rows = diff::flatten_rows(&self.files, |p| collapsed.contains(p), max_lines);
        let list = if scroll {
            let heights: Vec<_> = rows.iter().map(|r| r.height(m)).collect();
            let list = self.list.get_or_insert_with(|| {
                crate::ListState::new(rows.len(), m.diff_line_height).with_overscan_pixels(600.)
            });
            if self.row_heights != heights {
                list.set_item_count(rows.len());
                list.set_item_heights(&heights);
                self.row_heights = heights;
            }
            Some(list.clone())
        } else {
            None
        };
        let range = list
            .as_ref()
            .map(|l| l.visible_rows().range)
            .unwrap_or(0..rows.len());
        let mut content = Vec::with_capacity(range.len());
        for (i, row) in rows
            .into_iter()
            .enumerate()
            .skip(range.start)
            .take(range.len())
        {
            let row_id = ElementId::new(
                id.as_u64()
                    .wrapping_mul(0x9e3779b97f4a7c15)
                    .wrapping_add(i as u64 + 1)
                    | (1 << 63),
            );
            let base = div()
                .id(row_id)
                .w_full()
                .h(row.height(m))
                .flex_none()
                .flex_row()
                .items_center();
            let element = match row {
                DiffRow::FileHeader { file } => {
                    let f = &self.files[file as usize];
                    base.cursor_pointer()
                        .hover(|s| s.bg(ink(0.05)))
                        .gap(8.)
                        .px(12.)
                        .bg(ink(0.025))
                        .border_top(1., ink(0.04))
                        .child(
                            text(f.path.clone())
                                .flex_1()
                                .min_w(0.)
                                .text_size(12.)
                                .line_height(line_height(12.))
                                .text_color(theme.text_dim.into()),
                        )
                        .child(
                            text(format!("+{}", f.additions))
                                .text_size(11.)
                                .line_height(line_height(11.))
                                .text_color(theme.diff_add.into()),
                        )
                        .child(
                            text(format!("−{}", f.deletions))
                                .text_size(11.)
                                .line_height(line_height(11.))
                                .text_color(theme.diff_del.into()),
                        )
                }
                DiffRow::HunkHeader { file, hunk } => base
                    .px(m.diff_row_padding_x)
                    .bg(theme.diff_hunk_bg.into())
                    .text_size(11.)
                    .line_height(line_height(11.))
                    .text_color(theme.text_faint.into())
                    .child(text(
                        self.files[file as usize].hunks[hunk as usize]
                            .header
                            .clone(),
                    )),
                DiffRow::Notice { file, notice } => base
                    .px(m.diff_row_padding_x)
                    .text_size(11.)
                    .line_height(line_height(11.))
                    .text_color(theme.text_faint.into())
                    .child(text(
                        diff::file_notices(&self.files[file as usize])[notice as usize].clone(),
                    )),
                DiffRow::BodyPad { .. } => base,
                DiffRow::ShowMore { remaining } => base
                    .cursor_pointer()
                    .hover(|s| s.bg(ink(0.05)))
                    .justify_center()
                    .text_color(theme.text_dim.into())
                    .child(text(if remaining == 1 {
                        "Show 1 more line".into()
                    } else {
                        format!("Show {remaining} more lines")
                    })),
                DiffRow::Line { file, hunk, line } => {
                    let f = &self.files[file as usize];
                    let l = &f.hunks[hunk as usize].lines[line as usize];
                    let gutter = diff::gutter_width(f, m);
                    let tint: Color = match l.kind {
                        LineKind::Add => theme.diff_add.into(),
                        LineKind::Del => theme.diff_del.into(),
                        _ => theme.text_faint.into(),
                    };
                    let changed = matches!(l.kind, LineKind::Add | LineKind::Del);
                    let spans = self.highlights[file as usize]
                        .as_ref()
                        .map(|h| h.spans_for(l))
                        .unwrap_or(&[]);
                    let mut edges = vec![0, l.text.len()];
                    for span in spans {
                        edges.extend([span.range.start, span.range.end]);
                    }
                    if word_diff {
                        for range in &l.word_ranges {
                            edges.extend([range.start, range.end]);
                        }
                    }
                    edges.sort_unstable();
                    edges.dedup();
                    let mut runs = Vec::new();
                    let (mut syntax_index, mut word_index) = (0, 0);
                    for pair in edges.windows(2) {
                        let (start, end) = (pair[0], pair[1]);
                        while syntax_index < spans.len() && spans[syntax_index].range.end <= start {
                            syntax_index += 1;
                        }
                        while word_index < l.word_ranges.len()
                            && l.word_ranges[word_index].end <= start
                        {
                            word_index += 1;
                        }
                        let mut style = HighlightStyle::default();
                        if let Some(span) =
                            spans.get(syntax_index).filter(|s| s.range.start <= start)
                        {
                            style = style.color(theme.syntax.color(span.kind).into());
                        }
                        if word_diff
                            && l.word_ranges
                                .get(word_index)
                                .is_some_and(|r| r.start <= start)
                        {
                            style = style
                                .background(tint.with_alpha(0.28))
                                .background_shape(3.0, 1.0, 1.5);
                        }
                        if style != HighlightStyle::default()
                            && runs.len() < crate::MAX_TEXT_HIGHLIGHTS
                        {
                            runs.push((start..end, style));
                        }
                    }
                    let styled =
                        StyledText::new(Arc::<str>::from(l.text.as_str())).with_highlights(runs);
                    if l.kind == LineKind::Meta {
                        base.padding(
                            0.,
                            0.,
                            0.,
                            m.diff_accent_bar_width + 2. * gutter + m.diff_marker_width + 12.,
                        )
                        .text_size(10.5)
                        .line_height(line_height(10.5))
                        .italic()
                        .text_color(theme.text_faint.into())
                        .child(text(l.text.clone()))
                    } else {
                        let row = if changed {
                            base.bg(tint.with_alpha(0.055))
                        } else {
                            base
                        };
                        let number = |value: Option<u32>, active| {
                            div()
                                .flex_row()
                                .justify_end()
                                .w(gutter)
                                .padding(0., 8., 0., 0.)
                                .flex_none()
                                .text_size(11.)
                                .line_height(line_height(11.))
                                .text_color(if active {
                                    tint.with_alpha(0.9)
                                } else {
                                    Color::from(theme.text_faint).with_alpha(0.8)
                                })
                                .user_select_none()
                                .child(text(value.map(|n| n.to_string()).unwrap_or_default()))
                        };
                        row.child(div().w(m.diff_accent_bar_width).h_full().flex_none().bg(
                            if changed {
                                tint.with_alpha(0.55)
                            } else {
                                Color::TRANSPARENT
                            },
                        ))
                        .child(number(l.old_no, l.kind == LineKind::Del))
                        .child(number(l.new_no, l.kind == LineKind::Add))
                        .child(
                            div()
                                .flex_row()
                                .justify_center()
                                .w(m.diff_marker_width)
                                .flex_none()
                                .text_color(tint)
                                .user_select_none()
                                .child(text(match l.kind {
                                    LineKind::Add => "+",
                                    LineKind::Del => "−",
                                    _ => "·",
                                })),
                        )
                        .child(
                            styled
                                .into_element()
                                .padding(0., 0., 0., 12.)
                                .flex_1()
                                .min_w(0.)
                                .overflow_hidden()
                                .user_select_text()
                                .text_color(Color::from(theme.text).with_alpha(0.92)),
                        )
                    }
                }
            };
            let element = match row {
                DiffRow::ShowMore { remaining } => {
                    decorate(row_id, element, DiffAction::ShowMore(remaining))
                }
                DiffRow::FileHeader { file } => decorate(
                    row_id,
                    element,
                    DiffAction::ToggleFile(self.files[file as usize].path.clone()),
                ),
                DiffRow::Line { file, hunk, line } => {
                    let line = &self.files[file as usize].hunks[hunk as usize].lines[line as usize];
                    decorate(
                        row_id,
                        element,
                        DiffAction::LineClick {
                            text: line.text.clone(),
                            old_line: line.old_no,
                            new_line: line.new_no,
                        },
                    )
                }
                _ => element,
            };
            content.push(element);
        }
        let root = div()
            .id(id)
            .w_full()
            .min_w(0.)
            .flex_col()
            .font_family(theme.font_mono.clone())
            .text_size(m.diff_text_size)
            .line_height(line_height(m.diff_text_size))
            .whitespace_nowrap();
        if let Some(list) = list {
            let mut content = content.into_iter();
            root.min_h(0.)
                .overflow_y_scroll()
                .child(list.render_rows(range, |_| content.next().expect("rendered visible row")))
                .variable_virtual_scroll(&list)
        } else {
            root.children(content)
        }
    }
}

use diff::{DiffLine, FileDiff, LineKind};
const MAX_HIGHLIGHT_LINES: usize = 200_000;
struct FileHighlight {
    old: Vec<Vec<HighlightSpan>>,
    new: Vec<Vec<HighlightSpan>>,
}

impl FileHighlight {
    /// Spans for one diff line. Context lines prefer the post-change side,
    /// which is what the reader is looking at.
    fn spans_for(&self, line: &DiffLine) -> &[HighlightSpan] {
        fn pick<'a>(
            lines: &'a [Vec<HighlightSpan>],
            no: Option<u32>,
        ) -> Option<&'a [HighlightSpan]> {
            let ix = no?.saturating_sub(1) as usize;
            lines.get(ix).map(Vec::as_slice)
        }
        match line.kind {
            LineKind::Del => pick(&self.old, line.old_no),
            LineKind::Add => pick(&self.new, line.new_no),
            _ => pick(&self.new, line.new_no).or_else(|| pick(&self.old, line.old_no)),
        }
        .unwrap_or(&[])
    }
}

fn file_highlight(file: &FileDiff) -> Option<FileHighlight> {
    if file.binary || file.path.is_empty() {
        return None;
    }
    let mut old_visible: Vec<(u32, &str)> = Vec::new();
    let mut new_visible: Vec<(u32, &str)> = Vec::new();
    let (mut old_max, mut new_max) = (0u32, 0u32);
    for hunk in &file.hunks {
        for line in &hunk.lines {
            if line.kind == LineKind::Meta {
                continue;
            }
            if let Some(no) = line.old_no.filter(|no| *no > 0) {
                old_visible.push((no, &line.text));
                old_max = old_max.max(no);
            }
            if let Some(no) = line.new_no.filter(|no| *no > 0) {
                new_visible.push((no, &line.text));
                new_max = new_max.max(no);
            }
        }
    }

    // A rename can change the extension, so each side is detected from its own
    // path. `old.js -> new.py` must not parse the deleted JavaScript as Python.
    let old_path = file.old_path.as_deref().unwrap_or(&file.path);
    let old = highlight_side(&old_visible, old_max, old_path);
    let new = highlight_side(&new_visible, new_max, &file.path);
    (!old.is_empty() || !new.is_empty()).then_some(FileHighlight { old, new })
}

/// Parse the visible lines of one side and scatter the resulting spans into a
/// table indexed by real line number.
fn highlight_side(visible: &[(u32, &str)], max_line: u32, path: &str) -> Vec<Vec<HighlightSpan>> {
    if visible.is_empty() || max_line as usize > MAX_HIGHLIGHT_LINES {
        return Vec::new();
    }
    let source = visible
        .iter()
        .map(|(_, text)| *text)
        .collect::<Vec<_>>()
        .join("\n");
    let Some(document) = highlight_cached(&source, Some(path), None) else {
        return Vec::new();
    };
    let mut lines = vec![Vec::new(); max_line as usize];
    for ((number, _), spans) in visible.iter().zip(document.lines.iter()) {
        // `number` is 1-based and non-zero by construction above.
        lines[*number as usize - 1] = spans.clone();
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn syntax_and_word_washes_share_non_overlapping_unicode_ranges() {
        let mut document = DiffDocument::default();
        document.set_source("diff --git a/file.ts b/file.ts\n@@ -1 +1 @@\n-const text = \"héllo\";\n+const text = \"hëllo\";\n");
        let element = document.element(
            ElementId::new(9),
            &Theme::dark(),
            &HashSet::new(),
            None,
            true,
            false,
            |_, e, _| e,
        );
        assert_eq!(element.children.len(), 5);
    }

    #[test]
    fn code_normalizes_windows_lines_and_retains_highlighting() {
        let mut code = CodeDocument::default();
        code.set_source("const x = 1;\r\n", Some("typescript"), None);
        assert_eq!(code.source, "const x = 1;\n");
        let first = code.highlight.clone().unwrap();
        code.set_source("const x = 1;\n", Some("typescript"), None);
        assert!(Arc::ptr_eq(&first, code.highlight.as_ref().unwrap()));
    }
}
