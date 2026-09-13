use std::ops::Range;

use pulldown_cmark::{Alignment, CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag};

use super::{MAX_MARKDOWN_BLOCKS, MAX_MARKDOWN_NESTING_DEPTH, MAX_MARKDOWN_TABLE_COLUMNS};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MarkdownInlineStyle {
    pub bold: bool,
    pub italic: bool,
    pub code: bool,
    pub strikethrough: bool,
    pub link: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MarkdownInlineRun {
    pub text: String,
    pub style: MarkdownInlineStyle,
}

impl MarkdownInlineRun {
    fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style: MarkdownInlineStyle::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MarkdownTableAlign {
    #[default]
    Left,
    Center,
    Right,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MarkdownListItem {
    pub task: Option<bool>,
    pub blocks: Vec<MarkdownBlock>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MarkdownBlock {
    Paragraph(Vec<MarkdownInlineRun>),
    Heading {
        level: u8,
        runs: Vec<MarkdownInlineRun>,
    },
    CodeBlock {
        language: Option<String>,
        code: String,
    },
    BlockQuote(Vec<MarkdownBlock>),
    List {
        ordered_start: Option<u64>,
        items: Vec<MarkdownListItem>,
    },
    Table {
        header: Vec<Vec<MarkdownInlineRun>>,
        rows: Vec<Vec<Vec<MarkdownInlineRun>>>,
        align: Vec<MarkdownTableAlign>,
    },
    Image {
        url: String,
        alt: String,
    },
    Rule,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TopBlock {
    pub range: Range<usize>,
    pub block: MarkdownBlock,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct BlockTree {
    pub blocks: Vec<TopBlock>,
}

fn options() -> Options {
    Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS
}

pub(crate) fn parse(source: &str) -> BlockTree {
    parse_with_images(source, false)
}

fn parse_with_images(source: &str, images_as_links: bool) -> BlockTree {
    let events = Parser::new_ext(source, options())
        .into_offset_iter()
        .collect::<Vec<_>>();
    let mut cursor = Cursor {
        events: &events,
        index: 0,
        images_as_links,
    };
    let mut blocks = Vec::new();
    while blocks.len() < MAX_MARKDOWN_BLOCKS {
        let Some((event, range)) = cursor.peek() else {
            break;
        };
        let range = range.clone();
        match event {
            Event::Rule => {
                cursor.bump();
                blocks.push(TopBlock {
                    range,
                    block: MarkdownBlock::Rule,
                });
            }
            Event::Start(_) => {
                for block in parse_started_block(&mut cursor, 0) {
                    if blocks.len() == MAX_MARKDOWN_BLOCKS {
                        break;
                    }
                    blocks.push(TopBlock {
                        range: range.clone(),
                        block,
                    });
                }
            }
            _ => cursor.bump(),
        }
    }
    BlockTree { blocks }
}

struct Cursor<'a, 'event> {
    images_as_links: bool,
    events: &'a [(Event<'event>, Range<usize>)],
    index: usize,
}

impl<'event> Cursor<'_, 'event> {
    fn peek(&self) -> Option<&(Event<'event>, Range<usize>)> {
        self.events.get(self.index)
    }

    fn peek_event(&self) -> Option<&Event<'event>> {
        self.peek().map(|(event, _)| event)
    }

    fn bump(&mut self) {
        self.index += 1;
    }

    fn next_event(&mut self) -> Option<Event<'event>> {
        let event = self.events.get(self.index).map(|(event, _)| event.clone());
        if event.is_some() {
            self.index += 1;
        }
        event
    }
}

fn is_block_tag(tag: &Tag<'_>) -> bool {
    matches!(
        tag,
        Tag::Paragraph
            | Tag::Heading { .. }
            | Tag::CodeBlock(_)
            | Tag::BlockQuote(_)
            | Tag::List(_)
            | Tag::Item
            | Tag::Table(_)
            | Tag::HtmlBlock
            | Tag::FootnoteDefinition(_)
    )
}

fn heading_level(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

fn parse_started_block(cursor: &mut Cursor<'_, '_>, depth: usize) -> Vec<MarkdownBlock> {
    let Some(Event::Start(tag)) = cursor.next_event() else {
        return Vec::new();
    };
    if depth >= MAX_MARKDOWN_NESTING_DEPTH {
        return parse_depth_limited_container(cursor);
    }
    match tag {
        Tag::Paragraph => pieces_into_blocks(parse_inline_container(cursor)),
        Tag::Heading { level, .. } => vec![MarkdownBlock::Heading {
            level: heading_level(level),
            runs: pieces_into_runs(parse_inline_container(cursor)),
        }],
        Tag::CodeBlock(kind) => {
            let language = match kind {
                CodeBlockKind::Fenced(info) => info
                    .split_whitespace()
                    .next()
                    .filter(|language| !language.is_empty())
                    .map(str::to_owned),
                CodeBlockKind::Indented => None,
            };
            let mut code = String::new();
            loop {
                match cursor.next_event() {
                    Some(Event::Text(text)) => code.push_str(&text),
                    Some(Event::End(_)) | None => break,
                    Some(_) => {}
                }
            }
            if code.ends_with('\n') {
                code.pop();
            }
            vec![MarkdownBlock::CodeBlock { language, code }]
        }
        Tag::BlockQuote(_) => vec![MarkdownBlock::BlockQuote(parse_block_sequence(
            cursor,
            depth + 1,
        ))],
        Tag::List(ordered_start) => {
            let mut items = Vec::new();
            loop {
                match cursor.peek_event() {
                    Some(Event::Start(Tag::Item)) => {
                        cursor.bump();
                        let task = match cursor.peek_event() {
                            Some(Event::TaskListMarker(checked)) => {
                                let checked = *checked;
                                cursor.bump();
                                Some(checked)
                            }
                            _ => None,
                        };
                        items.push(MarkdownListItem {
                            task,
                            blocks: parse_block_sequence(cursor, depth + 1),
                        });
                    }
                    Some(Event::End(_)) | None => {
                        cursor.bump();
                        break;
                    }
                    Some(_) => cursor.bump(),
                }
            }
            vec![MarkdownBlock::List {
                ordered_start,
                items,
            }]
        }
        Tag::Table(alignments) => {
            let align = alignments
                .iter()
                .take(MAX_MARKDOWN_TABLE_COLUMNS)
                .map(|alignment| match alignment {
                    Alignment::Center => MarkdownTableAlign::Center,
                    Alignment::Right => MarkdownTableAlign::Right,
                    Alignment::None | Alignment::Left => MarkdownTableAlign::Left,
                })
                .collect();
            vec![parse_table(cursor, align)]
        }
        Tag::HtmlBlock => {
            let mut text = String::new();
            loop {
                match cursor.next_event() {
                    Some(Event::Html(chunk) | Event::Text(chunk)) => text.push_str(&chunk),
                    Some(Event::End(_)) | None => break,
                    Some(_) => {}
                }
            }
            let text = text.trim_end_matches('\n').to_owned();
            (!text.is_empty())
                .then(|| MarkdownBlock::Paragraph(vec![MarkdownInlineRun::plain(text)]))
                .into_iter()
                .collect()
        }
        _ => parse_block_sequence(cursor, depth + 1),
    }
}

fn parse_depth_limited_container(cursor: &mut Cursor<'_, '_>) -> Vec<MarkdownBlock> {
    let mut nesting = 0usize;
    let mut text = String::new();
    while let Some(event) = cursor.next_event() {
        match event {
            Event::Start(_) => nesting = nesting.saturating_add(1),
            Event::End(_) if nesting == 0 => break,
            Event::End(_) => nesting -= 1,
            Event::Text(value)
            | Event::Code(value)
            | Event::Html(value)
            | Event::InlineHtml(value) => text.push_str(&value),
            Event::SoftBreak | Event::HardBreak => text.push('\n'),
            Event::TaskListMarker(checked) => {
                text.push_str(if checked { "[x] " } else { "[ ] " });
            }
            Event::FootnoteReference(label) => {
                text.push('[');
                text.push_str(&label);
                text.push(']');
            }
            Event::Rule => text.push_str("---"),
            Event::InlineMath(value) | Event::DisplayMath(value) => text.push_str(&value),
        }
    }
    (!text.is_empty())
        .then(|| MarkdownBlock::Paragraph(vec![MarkdownInlineRun::plain(text)]))
        .into_iter()
        .collect()
}

fn parse_block_sequence(cursor: &mut Cursor<'_, '_>, depth: usize) -> Vec<MarkdownBlock> {
    let mut blocks = Vec::new();
    let mut inline = Vec::new();
    while blocks.len() < MAX_MARKDOWN_BLOCKS {
        let Some(event) = cursor.peek_event() else {
            break;
        };
        match event {
            Event::End(_) => {
                cursor.bump();
                break;
            }
            Event::Start(tag) if is_block_tag(tag) => {
                flush_paragraph(&mut blocks, &mut inline);
                blocks.extend(parse_started_block(cursor, depth));
            }
            Event::Rule => {
                flush_paragraph(&mut blocks, &mut inline);
                cursor.bump();
                blocks.push(MarkdownBlock::Rule);
            }
            _ => parse_inline_event(cursor, &mut inline, &MarkdownInlineStyle::default()),
        }
    }
    flush_paragraph(&mut blocks, &mut inline);
    blocks.truncate(MAX_MARKDOWN_BLOCKS);
    blocks
}

fn flush_paragraph(blocks: &mut Vec<MarkdownBlock>, inline: &mut Vec<InlinePiece>) {
    if !inline.is_empty() {
        blocks.extend(pieces_into_blocks(merge_pieces(std::mem::take(inline))));
    }
}

fn parse_table(cursor: &mut Cursor<'_, '_>, align: Vec<MarkdownTableAlign>) -> MarkdownBlock {
    let mut header = Vec::new();
    let mut rows = Vec::new();
    loop {
        match cursor.peek_event() {
            Some(Event::Start(Tag::TableHead)) => {
                cursor.bump();
                header = parse_table_row(cursor);
            }
            Some(Event::Start(Tag::TableRow)) => {
                cursor.bump();
                rows.push(parse_table_row(cursor));
            }
            Some(Event::End(_)) | None => {
                cursor.bump();
                break;
            }
            Some(_) => cursor.bump(),
        }
    }
    MarkdownBlock::Table {
        header,
        rows,
        align,
    }
}

fn parse_table_row(cursor: &mut Cursor<'_, '_>) -> Vec<Vec<MarkdownInlineRun>> {
    let mut cells = Vec::new();
    loop {
        match cursor.peek_event() {
            Some(Event::Start(Tag::TableCell)) => {
                cursor.bump();
                let runs = pieces_into_runs(parse_inline_container(cursor));
                if cells.len() < MAX_MARKDOWN_TABLE_COLUMNS {
                    cells.push(runs);
                }
            }
            Some(Event::End(_)) | None => {
                cursor.bump();
                break;
            }
            Some(_) => cursor.bump(),
        }
    }
    cells
}

fn parse_inline_container(cursor: &mut Cursor<'_, '_>) -> Vec<InlinePiece> {
    let mut pieces = Vec::new();
    while let Some(event) = cursor.peek_event() {
        if matches!(event, Event::End(_)) {
            cursor.bump();
            break;
        }
        parse_inline_event(cursor, &mut pieces, &MarkdownInlineStyle::default());
    }
    merge_pieces(pieces)
}

#[derive(Clone, Debug)]
enum InlinePiece {
    Run(MarkdownInlineRun),
    Image { url: String, alt: String },
}

fn pieces_into_blocks(pieces: Vec<InlinePiece>) -> Vec<MarkdownBlock> {
    let mut blocks = Vec::new();
    let mut runs = Vec::new();
    for piece in pieces {
        match piece {
            InlinePiece::Run(run) => runs.push(run),
            InlinePiece::Image { url, alt } => {
                if !runs.is_empty() {
                    blocks.push(MarkdownBlock::Paragraph(std::mem::take(&mut runs)));
                }
                blocks.push(MarkdownBlock::Image { url, alt });
            }
        }
    }
    if !runs.is_empty() {
        blocks.push(MarkdownBlock::Paragraph(runs));
    }
    blocks
}

fn pieces_into_runs(pieces: Vec<InlinePiece>) -> Vec<MarkdownInlineRun> {
    merge_runs(
        pieces
            .into_iter()
            .map(|piece| match piece {
                InlinePiece::Run(run) => run,
                InlinePiece::Image { alt, .. } => MarkdownInlineRun::plain(alt),
            })
            .collect(),
    )
}

fn parse_inline_event(
    cursor: &mut Cursor<'_, '_>,
    pieces: &mut Vec<InlinePiece>,
    style: &MarkdownInlineStyle,
) {
    let Some(event) = cursor.next_event() else {
        return;
    };
    match event {
        Event::Text(text) => pieces.push(InlinePiece::Run(MarkdownInlineRun {
            text: text.to_string(),
            style: style.clone(),
        })),
        Event::Code(text) => {
            let mut style = style.clone();
            style.code = true;
            pieces.push(InlinePiece::Run(MarkdownInlineRun {
                text: text.to_string(),
                style,
            }));
        }
        Event::SoftBreak | Event::HardBreak => {
            pieces.push(InlinePiece::Run(MarkdownInlineRun {
                text: "\n".to_owned(),
                style: style.clone(),
            }));
        }
        Event::Start(Tag::Image {
            dest_url, title, ..
        }) => {
            let mut alt_pieces = Vec::new();
            while let Some(event) = cursor.peek_event() {
                if matches!(event, Event::End(_)) {
                    cursor.bump();
                    break;
                }
                parse_inline_event(cursor, &mut alt_pieces, &MarkdownInlineStyle::default());
            }
            let alt = pieces_into_runs(alt_pieces)
                .into_iter()
                .map(|run| run.text)
                .collect::<String>();
            if cursor.images_as_links {
                let mut image_style = style.clone();
                image_style.link = Some(dest_url.to_string());
                pieces.push(InlinePiece::Run(MarkdownInlineRun {
                    text: alt,
                    style: image_style,
                }));
            } else {
                pieces.push(InlinePiece::Image {
                    url: dest_url.to_string(),
                    alt: if alt.trim().is_empty() {
                        title.to_string()
                    } else {
                        alt
                    },
                });
            }
        }
        Event::Start(tag) => {
            let mut nested = style.clone();
            match &tag {
                Tag::Emphasis => nested.italic = true,
                Tag::Strong => nested.bold = true,
                Tag::Strikethrough => nested.strikethrough = true,
                Tag::Link { dest_url, .. } => nested.link = Some(dest_url.to_string()),
                _ => {}
            }
            while let Some(event) = cursor.peek_event() {
                if matches!(event, Event::End(_)) {
                    cursor.bump();
                    break;
                }
                parse_inline_event(cursor, pieces, &nested);
            }
        }
        Event::Html(text) | Event::InlineHtml(text) => {
            pieces.push(InlinePiece::Run(MarkdownInlineRun {
                text: text.to_string(),
                style: style.clone(),
            }));
        }
        Event::FootnoteReference(label) => pieces.push(InlinePiece::Run(MarkdownInlineRun {
            text: format!("[{label}]"),
            style: style.clone(),
        })),
        Event::TaskListMarker(checked) => pieces.push(InlinePiece::Run(MarkdownInlineRun {
            text: if checked { "[x] " } else { "[ ] " }.to_owned(),
            style: style.clone(),
        })),
        Event::End(_) | Event::Rule | Event::InlineMath(_) | Event::DisplayMath(_) => {}
    }
}

fn merge_pieces(pieces: Vec<InlinePiece>) -> Vec<InlinePiece> {
    let mut merged: Vec<InlinePiece> = Vec::with_capacity(pieces.len());
    for piece in pieces {
        match piece {
            InlinePiece::Run(run) if run.text.is_empty() => {}
            InlinePiece::Run(run) => match merged.last_mut() {
                Some(InlinePiece::Run(last)) if last.style == run.style => {
                    last.text.push_str(&run.text);
                }
                _ => merged.push(InlinePiece::Run(run)),
            },
            image => merged.push(image),
        }
    }
    merged
}

fn merge_runs(runs: Vec<MarkdownInlineRun>) -> Vec<MarkdownInlineRun> {
    let mut merged: Vec<MarkdownInlineRun> = Vec::with_capacity(runs.len());
    for run in runs {
        if run.text.is_empty() {
            continue;
        }
        match merged.last_mut() {
            Some(last) if last.style == run.style => last.text.push_str(&run.text),
            _ => merged.push(run),
        }
    }
    merged
}

pub(crate) struct IncrementalParser {
    images_as_links: bool,
    text: String,
    tree: BlockTree,
    stable_prefix: usize,
    full_reparse_only: bool,
    reparsed_from: usize,
}

impl Default for IncrementalParser {
    fn default() -> Self {
        Self::new()
    }
}

impl IncrementalParser {
    pub fn new() -> Self {
        Self {
            images_as_links: false,
            text: String::new(),
            tree: BlockTree::default(),
            stable_prefix: 0,
            full_reparse_only: false,
            reparsed_from: 0,
        }
    }

    pub fn set_images_as_links(&mut self, enabled: bool) {
        if self.images_as_links != enabled {
            self.images_as_links = enabled;
            let source = self.text.clone();
            self.reset(&source);
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn tree(&self) -> &BlockTree {
        &self.tree
    }

    pub fn reparsed_from(&self) -> usize {
        self.reparsed_from
    }

    pub fn set_text(&mut self, text: &str) -> bool {
        if text == self.text {
            return false;
        }
        match text.strip_prefix(self.text.as_str()) {
            Some(delta) if !self.text.is_empty() && !self.full_reparse_only => {
                self.append(delta);
            }
            _ => self.reset(text),
        }
        true
    }

    pub fn reset(&mut self, text: &str) {
        self.text.clear();
        self.text.push_str(text);
        self.tree = parse_with_images(&self.text, self.images_as_links);
        self.full_reparse_only = has_link_definition(&self.text);
        self.stable_prefix = self.settled_prefix();
        self.reparsed_from = 0;
    }

    pub fn append(&mut self, delta: &str) {
        if delta.is_empty() {
            return;
        }
        if self.full_reparse_only {
            let mut text = std::mem::take(&mut self.text);
            text.push_str(delta);
            self.reset(&text);
            return;
        }
        let block_start = self
            .tree
            .blocks
            .get(self.stable_prefix)
            .map_or(0, |block| block.range.start);
        // pulldown-cmark offsets may begin after syntax that changes the block kind. For
        // example, an indented code block's start offset excludes its four spaces. Reparse
        // from the start of the source line so the tail parser sees that syntax too.
        let boundary = self.text[..block_start]
            .rfind('\n')
            .map_or(0, |newline| newline + 1);
        let definition_scan_start = self.text.rfind('\n').map_or(0, |newline| newline + 1);
        self.text.push_str(delta);
        // Include the old unfinished line: a streamed reference definition is commonly split
        // between `[name]` and `: destination`, and can restyle references in the settled prefix.
        if has_link_definition(&self.text[definition_scan_start..]) {
            let text = std::mem::take(&mut self.text);
            self.reset(&text);
            return;
        }
        let tail = parse_with_images(&self.text[boundary..], self.images_as_links);
        self.tree.blocks.truncate(self.stable_prefix);
        self.tree
            .blocks
            .extend(tail.blocks.into_iter().map(|mut block| {
                block.range.start += boundary;
                block.range.end += boundary;
                block
            }));
        self.tree.blocks.truncate(MAX_MARKDOWN_BLOCKS);
        self.stable_prefix = self.settled_prefix();
        self.reparsed_from = boundary;
    }

    pub fn display_tail(&self) -> Option<Vec<TopBlock>> {
        let last = self.tree.blocks.last()?;
        if matches!(last.block, MarkdownBlock::CodeBlock { .. }) {
            return None;
        }
        let mended = super::mend::close_hanging(&self.text[last.range.start..])?;
        let offset = last.range.start;
        Some(
            parse_with_images(&mended, self.images_as_links)
                .blocks
                .into_iter()
                .map(|mut block| {
                    block.range.start += offset;
                    block.range.end = (block.range.end + offset).min(self.text.len());
                    block
                })
                .collect(),
        )
    }

    fn settled_prefix(&self) -> usize {
        let blocks = &self.tree.blocks;
        let mut index = blocks.len();
        for _ in 0..2 {
            let Some(group_start) = index.checked_sub(1).map(|last| blocks[last].range.start)
            else {
                break;
            };
            while index > 0 && blocks[index - 1].range.start == group_start {
                index -= 1;
            }
        }
        index
    }
}

fn has_link_definition(text: &str) -> bool {
    text.lines().any(|line| {
        let line = line.trim_start();
        let Some(rest) = line.strip_prefix('[') else {
            return false;
        };
        rest.find("]: ").is_some_and(|end| end > 0) || rest.find("]:").is_some_and(|end| end > 0)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain_text(block: &MarkdownBlock) -> String {
        match block {
            MarkdownBlock::Paragraph(runs) | MarkdownBlock::Heading { runs, .. } => {
                runs.iter().map(|run| run.text.as_str()).collect()
            }
            other => panic!("expected text block, got {other:?}"),
        }
    }

    #[test]
    fn parses_rich_commonmark_blocks() {
        let tree = parse(
            "# Title\n\nText with **bold**, *em*, `code`, and [link](https://example.com).\n\n- [x] done\n- pending\n\n| A | B |\n|:-|--:|\n| 1 | 2 |\n",
        );
        assert!(matches!(
            tree.blocks[0].block,
            MarkdownBlock::Heading { level: 1, .. }
        ));
        let MarkdownBlock::Paragraph(runs) = &tree.blocks[1].block else {
            panic!("expected paragraph");
        };
        assert!(runs.iter().any(|run| run.style.bold));
        assert!(runs.iter().any(|run| run.style.italic));
        assert!(runs.iter().any(|run| run.style.code));
        assert!(runs.iter().any(|run| run.style.link.is_some()));
        let MarkdownBlock::List { items, .. } = &tree.blocks[2].block else {
            panic!("expected list");
        };
        assert_eq!(items[0].task, Some(true));
        assert!(matches!(tree.blocks[3].block, MarkdownBlock::Table { .. }));
    }

    #[test]
    fn table_columns_are_hard_bounded() {
        let row = std::iter::repeat_n("cell", MAX_MARKDOWN_TABLE_COLUMNS + 8)
            .collect::<Vec<_>>()
            .join("|");
        let divider = std::iter::repeat_n("---", MAX_MARKDOWN_TABLE_COLUMNS + 8)
            .collect::<Vec<_>>()
            .join("|");
        let tree = parse(&format!("|{row}|\n|{divider}|\n|{row}|"));
        let MarkdownBlock::Table {
            header,
            rows,
            align,
        } = &tree.blocks[0].block
        else {
            panic!("expected table");
        };
        assert_eq!(header.len(), MAX_MARKDOWN_TABLE_COLUMNS);
        assert_eq!(rows[0].len(), MAX_MARKDOWN_TABLE_COLUMNS);
        assert_eq!(align.len(), MAX_MARKDOWN_TABLE_COLUMNS);
    }

    #[test]
    fn incremental_appends_match_full_parses() {
        for source in [
            "# Heading\n\nA paragraph with **bold**.\n\n- one\n- two\n\n```js\nlet x = 1;\n```\n\nTail.",
            "    indented code\n\nTail.",
            "[early][ref]\n\nSecond.\n\nThird.\n\n[ref]: https://example.com",
            "| A | B |\n|---|---|\n| 1 | 2 |\n| 3 | 4 |\n\nTail.",
            "before ![a shot](https://example.com/x.png) after, and more prose.",
        ] {
            for chunk_size in [1, 3, 7, 64] {
                let mut incremental = IncrementalParser::new();
                let mut built = String::new();
                let mut characters = source.chars().peekable();
                while characters.peek().is_some() {
                    let chunk = characters.by_ref().take(chunk_size).collect::<String>();
                    built.push_str(&chunk);
                    incremental.append(&chunk);
                    assert_eq!(incremental.tree(), &parse(&built));
                }
            }
        }
    }

    #[test]
    fn incremental_tree_never_exceeds_the_top_level_block_limit() {
        let source = "x\n\n".repeat(MAX_MARKDOWN_BLOCKS + 8);
        let mut parser = IncrementalParser::new();
        parser.set_text(&source);
        parser.append("\n\nmore");
        assert_eq!(parser.tree().blocks.len(), MAX_MARKDOWN_BLOCKS);
    }

    #[test]
    fn nesting_is_flattened_at_the_hard_limit() {
        let source = format!("{}deep", "> ".repeat(MAX_MARKDOWN_NESTING_DEPTH + 32));
        let tree = parse(&source);
        let mut block = &tree.blocks[0].block;
        let mut depth = 0;
        while let MarkdownBlock::BlockQuote(children) = block {
            depth += 1;
            block = &children[0];
        }
        assert_eq!(depth, MAX_MARKDOWN_NESTING_DEPTH);
        assert!(matches!(block, MarkdownBlock::Paragraph(_)));
    }

    #[test]
    fn append_reuses_the_settled_prefix() {
        let mut parser = IncrementalParser::new();
        parser.set_text("First.\n\nSecond.\n\nThird");
        parser.set_text("First.\n\nSecond.\n\nThird grows.");
        assert!(parser.reparsed_from() > 0);
        assert_eq!(plain_text(&parser.tree().blocks[0].block), "First.");
    }

    #[test]
    fn streaming_tail_mends_incomplete_emphasis() {
        let mut parser = IncrementalParser::new();
        parser.set_text("Settled.\n\nNow **bold");
        let tail = parser.display_tail().expect("mended tail");
        let MarkdownBlock::Paragraph(runs) = &tail[0].block else {
            panic!("expected paragraph");
        };
        assert!(runs.iter().any(|run| run.style.bold && run.text == "bold"));
    }
}

#[cfg(test)]
mod image_link_tests {
    use super::*;

    #[test]
    fn native_document_images_remain_inline_links_during_incremental_parsing() {
        let mut parser = IncrementalParser::new();
        parser.set_images_as_links(true);
        parser.set_text("Before ![diagram](https://example.com/diagram.png)");
        parser.append(" after.");
        let MarkdownBlock::Paragraph(runs) = &parser.tree().blocks[0].block else {
            panic!("inline paragraph expected")
        };
        assert_eq!(parser.tree().blocks.len(), 1);
        assert_eq!(
            runs.iter().map(|r| r.text.as_str()).collect::<String>(),
            "Before diagram after."
        );
        assert!(runs.iter().any(|r| r.text == "diagram"
            && r.style.link.as_deref() == Some("https://example.com/diagram.png")));
        parser.set_images_as_links(false);
        assert!(
            parser
                .tree()
                .blocks
                .iter()
                .any(|b| matches!(b.block, MarkdownBlock::Image { .. }))
        );
    }
}
