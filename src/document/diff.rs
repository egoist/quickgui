//! Unified-patch parsing and row flattening.
//!
//! Ported from Comet (https://github.com/zeronsh/comet), MIT.
//! Original: the pure sections of `crates/ui/src/changes.rs`.
//!
//! Deliberately gpui-free so the parser can be unit tested without a window.
//! The rendering half lives in `custom_elements/diff.rs`.

use std::ops::Range;

use super::theme::Metrics;

// ── Model ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    Context,
    Add,
    Del,
    /// `\ No newline at end of file` and friends.
    Meta,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiffLine {
    pub kind: LineKind,
    pub old_no: Option<u32>,
    pub new_no: Option<u32>,
    pub text: String,
    /// Byte ranges of the words that differ from the paired line. Empty when
    /// word diffing is off or the line has no counterpart.
    pub word_ranges: Vec<Range<usize>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Hunk {
    pub header: String,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileStatus {
    Added,
    Deleted,
    Modified,
    Renamed,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FileDiff {
    /// Display path, the post-change side.
    pub path: String,
    /// Pre-rename path, when different.
    pub old_path: Option<String>,
    pub status: FileStatus,
    pub binary: bool,
    /// Parser-collected notices such as mode changes.
    pub notices: Vec<String>,
    pub hunks: Vec<Hunk>,
    pub additions: u32,
    pub deletions: u32,
    /// Largest line number on either side. Sizes the gutters analytically, so
    /// the code column never shifts while scrolling.
    pub max_line: u32,
}

impl FileDiff {
    fn new(path: String, old_path: Option<String>) -> Self {
        Self {
            path,
            old_path,
            status: FileStatus::Modified,
            binary: false,
            notices: Vec::new(),
            hunks: Vec::new(),
            additions: 0,
            deletions: 0,
            max_line: 0,
        }
    }
}

/// Width of one line-number gutter, fitted to the file's largest line number.
/// 11px mono is about 6.6px per digit, plus an 8px right pad and a 6px left gap
/// so the number never abuts the accent bar.
pub fn gutter_width(file: &FileDiff, metrics: &Metrics) -> f32 {
    let digits = file.max_line.max(1).ilog10() + 1;
    (digits as f32 * 6.6 + 8.0 + 6.0).max(metrics.diff_gutter_width)
}

// ── Parser ───────────────────────────────────────────────────────────

fn strip_git_prefix(path: &str) -> &str {
    path.strip_prefix("a/")
        .or_else(|| path.strip_prefix("b/"))
        .unwrap_or(path)
}

/// Split the tail of a `diff --git a/… b/…` line into (old, new) paths.
/// Quoted paths with spaces or unicode are handled; for unquoted paths with
/// spaces the split favours the last ` b/`, which is git's own convention.
fn parse_git_paths(rest: &str) -> (String, String) {
    fn unquote(s: &str) -> String {
        let trimmed = s.trim();
        if trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"') {
            trimmed[1..trimmed.len() - 1]
                .replace("\\\"", "\"")
                .replace("\\\\", "\\")
        } else {
            trimmed.to_string()
        }
    }
    if let Some(pos) = rest.rfind(" b/").or_else(|| rest.rfind(" \"b/")) {
        let old = unquote(&rest[..pos]);
        let new = unquote(&rest[pos + 1..]);
        (
            strip_git_prefix(&old).to_string(),
            strip_git_prefix(&new).to_string(),
        )
    } else {
        let p = strip_git_prefix(&unquote(rest)).to_string();
        (p.clone(), p)
    }
}

/// Parse one `@@ -a[,b] +c[,d] @@ …` header into starting line numbers.
fn parse_hunk_header(line: &str) -> Option<(u32, u32)> {
    let rest = line.strip_prefix("@@")?;
    let minus = rest.find('-')?;
    let old: u32 = rest[minus + 1..]
        .split(|c: char| c == ',' || c.is_whitespace())
        .next()?
        .parse()
        .ok()?;
    let plus = rest.find('+')?;
    let new: u32 = rest[plus + 1..]
        .split(|c: char| c == ',' || c.is_whitespace())
        .next()?
        .parse()
        .ok()?;
    Some((old, new))
}

/// Parse a unified git patch into file sections.
///
/// Tolerant by design: unknown header lines are skipped and a truncated hunk
/// keeps whatever parsed. A diff viewer that refuses to render a slightly
/// malformed patch is useless, and patches are often truncated at a byte cap.
pub fn parse_patch(patch: &str) -> Vec<FileDiff> {
    let mut files: Vec<FileDiff> = Vec::new();
    let mut in_hunk = false;
    let mut old_no: u32 = 0;
    let mut new_no: u32 = 0;

    for raw in patch.lines() {
        if let Some(rest) = raw.strip_prefix("diff --git ") {
            let (old, new) = parse_git_paths(rest);
            let old_path = (old != new).then_some(old);
            files.push(FileDiff::new(new, old_path));
            in_hunk = false;
            continue;
        }
        // A patch without a `diff --git` preamble (plain `diff -u` output, or
        // a bare hunk) still gets a file so the rows have somewhere to live.
        if files.is_empty() && (raw.starts_with("@@") || raw.starts_with("--- ")) {
            files.push(FileDiff::new(String::new(), None));
        }
        let Some(file) = files.last_mut() else {
            continue;
        };

        if raw.starts_with("@@") {
            if let Some((o, n)) = parse_hunk_header(raw) {
                old_no = o;
                new_no = n;
                file.hunks.push(Hunk {
                    header: raw.to_string(),
                    lines: Vec::new(),
                });
                in_hunk = true;
            }
            continue;
        }

        if in_hunk {
            let mut chars = raw.chars();
            let marker = chars.next();
            let body: String = chars.collect();
            let line = match marker {
                Some('+') => {
                    file.additions += 1;
                    let l = DiffLine {
                        kind: LineKind::Add,
                        old_no: None,
                        new_no: Some(new_no),
                        text: body,
                        word_ranges: Vec::new(),
                    };
                    new_no += 1;
                    Some(l)
                }
                Some('-') => {
                    file.deletions += 1;
                    let l = DiffLine {
                        kind: LineKind::Del,
                        old_no: Some(old_no),
                        new_no: None,
                        text: body,
                        word_ranges: Vec::new(),
                    };
                    old_no += 1;
                    Some(l)
                }
                Some(' ') | None => {
                    let l = DiffLine {
                        kind: LineKind::Context,
                        old_no: Some(old_no),
                        new_no: Some(new_no),
                        text: body,
                        word_ranges: Vec::new(),
                    };
                    old_no += 1;
                    new_no += 1;
                    Some(l)
                }
                Some('\\') => Some(DiffLine {
                    kind: LineKind::Meta,
                    old_no: None,
                    new_no: None,
                    text: raw.trim_start_matches('\\').trim().to_string(),
                    word_ranges: Vec::new(),
                }),
                _ => {
                    // A non-hunk line ends the hunk; reprocess it as a header.
                    in_hunk = false;
                    None
                }
            };
            if let Some(line) = line {
                if let Some(hunk) = file.hunks.last_mut() {
                    file.max_line = file
                        .max_line
                        .max(line.old_no.unwrap_or(0))
                        .max(line.new_no.unwrap_or(0));
                    hunk.lines.push(line);
                    continue;
                }
            }
            if in_hunk {
                continue;
            }
        }

        // File header territory.
        if raw.starts_with("new file mode") {
            file.status = FileStatus::Added;
        } else if raw.starts_with("deleted file mode") {
            file.status = FileStatus::Deleted;
        } else if let Some(from) = raw.strip_prefix("rename from ") {
            file.status = FileStatus::Renamed;
            file.old_path = Some(from.trim().to_string());
        } else if let Some(to) = raw.strip_prefix("rename to ") {
            file.status = FileStatus::Renamed;
            file.path = to.trim().to_string();
        } else if raw.starts_with("Binary files") || raw.starts_with("GIT binary patch") {
            file.binary = true;
        } else if let Some(mode) = raw.strip_prefix("new mode ") {
            file.notices
                .push(format!("Mode changed to {}", mode.trim()));
        } else if let Some(new) = raw.strip_prefix("+++ ") {
            let new = new.trim();
            if new == "/dev/null" {
                file.status = FileStatus::Deleted;
            } else if file.old_path.is_none() {
                file.path = strip_git_prefix(new).to_string();
            }
        } else if let Some(old) = raw.strip_prefix("--- ") {
            if old.trim() == "/dev/null" {
                file.status = FileStatus::Added;
            }
        }
        // "index …", "similarity index …", "old mode …": skipped.
    }
    files
}

/// Derived notice rows: new / deleted / renamed / binary plus parser notices.
pub fn file_notices(file: &FileDiff) -> Vec<String> {
    let mut notices = Vec::new();
    match file.status {
        FileStatus::Added => notices.push("New file".to_string()),
        FileStatus::Deleted => notices.push("Deleted file".to_string()),
        FileStatus::Renamed => {
            let from = file.old_path.as_deref().unwrap_or("?");
            notices.push(format!("Renamed from {from}"));
        }
        FileStatus::Modified => {}
    }
    if file.binary {
        notices.push("Binary file — contents not shown".to_string());
    }
    notices.extend(file.notices.iter().cloned());
    notices
}

// ── Word-level intra-line diff ───────────────────────────────────────
//
// Comet has none of this; its added and deleted rows carry a whole-line wash.
// A word-level highlight is what makes a one-character change readable, so it
// is worth the extra pass.

/// Annotate paired delete/add runs with the byte ranges that actually changed.
///
/// Only runs of the same length are paired. Pairing an unequal run by index
/// produces confident, wrong highlights (line 3 compared against line 7), which
/// is worse than no highlight at all.
pub fn annotate_word_diffs(files: &mut [FileDiff]) {
    for file in files {
        for hunk in &mut file.hunks {
            let mut i = 0;
            while i < hunk.lines.len() {
                let del_start = i;
                while i < hunk.lines.len() && hunk.lines[i].kind == LineKind::Del {
                    i += 1;
                }
                let del_end = i;
                let add_start = i;
                while i < hunk.lines.len() && hunk.lines[i].kind == LineKind::Add {
                    i += 1;
                }
                let add_end = i;

                let dels = del_end - del_start;
                let adds = add_end - add_start;
                if dels > 0 && dels == adds {
                    for offset in 0..dels {
                        let old = hunk.lines[del_start + offset].text.clone();
                        let new = hunk.lines[add_start + offset].text.clone();
                        let (old_ranges, new_ranges) = word_diff(&old, &new);
                        hunk.lines[del_start + offset].word_ranges = old_ranges;
                        hunk.lines[add_start + offset].word_ranges = new_ranges;
                    }
                }
                if i == del_start {
                    i += 1;
                }
            }
        }
    }
}

/// Byte ranges that differ between two lines, as (old ranges, new ranges).
///
/// A common-prefix / common-suffix trim on word boundaries. Not a full LCS:
/// for single-line edits the trim produces the same answer far more cheaply,
/// and a diff viewer runs this on every visible row.
pub fn word_diff(old: &str, new: &str) -> (Vec<Range<usize>>, Vec<Range<usize>>) {
    if old == new {
        return (Vec::new(), Vec::new());
    }
    let old_words = split_words(old);
    let new_words = split_words(new);

    let mut prefix = 0;
    while prefix < old_words.len()
        && prefix < new_words.len()
        && old[old_words[prefix].clone()] == new[new_words[prefix].clone()]
    {
        prefix += 1;
    }
    let mut suffix = 0;
    while suffix < old_words.len() - prefix
        && suffix < new_words.len() - prefix
        && old[old_words[old_words.len() - 1 - suffix].clone()]
            == new[new_words[new_words.len() - 1 - suffix].clone()]
    {
        suffix += 1;
    }

    let collapse = |words: &[Range<usize>]| -> Vec<Range<usize>> {
        let changed = &words[prefix..words.len() - suffix];
        match (changed.first(), changed.last()) {
            (Some(first), Some(last)) => vec![first.start..last.end],
            _ => Vec::new(),
        }
    };
    (collapse(&old_words), collapse(&new_words))
}

/// Byte ranges of the tokens in a line. Runs of word characters are one token;
/// every other character is its own token, so `foo(x)` differs from `foo(y)` in
/// exactly one token.
fn split_words(line: &str) -> Vec<Range<usize>> {
    let mut words = Vec::new();
    let mut start: Option<usize> = None;
    for (ix, ch) in line.char_indices() {
        let is_word = ch.is_alphanumeric() || ch == '_';
        match (is_word, start) {
            (true, None) => start = Some(ix),
            (false, Some(s)) => {
                words.push(s..ix);
                start = None;
                words.push(ix..ix + ch.len_utf8());
            }
            (false, None) => words.push(ix..ix + ch.len_utf8()),
            (true, Some(_)) => {}
        }
    }
    if let Some(s) = start {
        words.push(s..line.len());
    }
    words
}

// ── Row flattening ───────────────────────────────────────────────────

/// One virtual row. The list is addressed by index, so every row must know how
/// to find its own data without walking the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffRow {
    FileHeader {
        file: u32,
    },
    Notice {
        file: u32,
        notice: u32,
    },
    HunkHeader {
        file: u32,
        hunk: u32,
    },
    Line {
        file: u32,
        hunk: u32,
        line: u32,
    },
    /// Bottom padding under an expanded file body.
    BodyPad {
        file: u32,
    },
    /// Preview was cut by `maxLines`. `remaining` is the hidden line count.
    ShowMore {
        remaining: u32,
    },
}

impl DiffRow {
    pub fn height(&self, metrics: &Metrics) -> f32 {
        match self {
            DiffRow::FileHeader { .. } | DiffRow::ShowMore { .. } => {
                metrics.diff_file_header_height
            }
            DiffRow::Notice { .. } => metrics.diff_notice_height,
            DiffRow::HunkHeader { .. } => metrics.diff_hunk_header_height,
            DiffRow::Line { .. } => metrics.diff_line_height,
            DiffRow::BodyPad { .. } => metrics.diff_body_bottom_pad,
        }
    }
}

/// Flatten files into virtual rows. Collapsed files contribute only a header.
///
/// Collapsing REMOVES the body rows rather than hiding them, so a collapsed
/// 10k-line file costs one row. Hiding would keep the list's height model
/// paying for content nobody can see.
///
/// `max_lines` stops after that many `Line` rows and appends `ShowMore`.
/// Later files are not emitted. The cap is a preview, not a per-file fold.
pub fn flatten_rows(
    files: &[FileDiff],
    collapsed: impl Fn(&str) -> bool,
    max_lines: Option<usize>,
) -> Vec<DiffRow> {
    let mut rows = Vec::new();
    let mut line_count = 0usize;

    'files: for (file_ix, file) in files.iter().enumerate() {
        // Cap is checked here too: a file that ends exactly on `max_lines`
        // must not emit the next file's header before ShowMore.
        if file_ix > 0 && max_lines.is_some_and(|max| line_count >= max) {
            let remaining = remaining_line_rows(files, &collapsed, file_ix, 0, 0);
            if remaining > 0 {
                rows.push(DiffRow::ShowMore { remaining });
            }
            break;
        }
        let file_ix_u = file_ix as u32;
        rows.push(DiffRow::FileHeader { file: file_ix_u });
        if collapsed(&file.path) {
            continue;
        }
        for notice_ix in 0..file_notices(file).len() {
            rows.push(DiffRow::Notice {
                file: file_ix_u,
                notice: notice_ix as u32,
            });
        }
        for (hunk_ix, hunk) in file.hunks.iter().enumerate() {
            if max_lines.is_some_and(|max| line_count >= max) {
                rows.push(DiffRow::ShowMore {
                    remaining: remaining_line_rows(files, &collapsed, file_ix, hunk_ix, 0),
                });
                break 'files;
            }
            rows.push(DiffRow::HunkHeader {
                file: file_ix_u,
                hunk: hunk_ix as u32,
            });
            for line_ix in 0..hunk.lines.len() {
                if max_lines.is_some_and(|max| line_count >= max) {
                    rows.push(DiffRow::ShowMore {
                        remaining: remaining_line_rows(
                            files, &collapsed, file_ix, hunk_ix, line_ix,
                        ),
                    });
                    break 'files;
                }
                rows.push(DiffRow::Line {
                    file: file_ix_u,
                    hunk: hunk_ix as u32,
                    line: line_ix as u32,
                });
                line_count += 1;
            }
        }
        rows.push(DiffRow::BodyPad { file: file_ix_u });
    }
    rows
}

/// Line rows from `(file_ix, hunk_ix, line_ix)` to the end, skipping collapsed
/// files. Includes the current line.
fn remaining_line_rows(
    files: &[FileDiff],
    collapsed: &impl Fn(&str) -> bool,
    file_ix: usize,
    hunk_ix: usize,
    line_ix: usize,
) -> u32 {
    let mut remaining = 0u32;
    for (fi, file) in files.iter().enumerate().skip(file_ix) {
        if collapsed(&file.path) {
            continue;
        }
        for (hi, hunk) in file.hunks.iter().enumerate() {
            if fi == file_ix && hi < hunk_ix {
                continue;
            }
            let start = if fi == file_ix && hi == hunk_ix {
                line_ix
            } else {
                0
            };
            remaining += hunk.lines.len().saturating_sub(start) as u32;
        }
    }
    remaining
}

/// Mean row height, used to seed the virtualized list's height estimate.
///
/// A flat `DIFF_LINE_HEIGHT` guess makes the scrollbar visibly wrong on a
/// many-file diff, where 36px headers are a large share of the document.
pub fn estimated_row_height(rows: &[DiffRow], metrics: &Metrics) -> f32 {
    if rows.is_empty() {
        return metrics.diff_line_height;
    }
    let total: f32 = rows.iter().map(|row| row.height(metrics)).sum();
    total / rows.len() as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    // NOTE: no `\` line continuations. Rust strips the leading whitespace of the
    // next line, which eats the single space that marks a context line, and the
    // patch silently parses as three unrelated headers.
    const SIMPLE: &str = concat!(
        "diff --git a/src/app.ts b/src/app.ts\n",
        "index 111..222 100644\n",
        "--- a/src/app.ts\n",
        "+++ b/src/app.ts\n",
        "@@ -1,4 +1,4 @@\n",
        " const a = 1\n",
        "-const b = 2\n",
        "+const b = 3\n",
        " const c = 4\n",
    );

    #[test]
    fn parses_a_simple_patch() {
        let files = parse_patch(SIMPLE);
        assert_eq!(files.len(), 1);
        let file = &files[0];
        assert_eq!(file.path, "src/app.ts");
        assert_eq!(file.status, FileStatus::Modified);
        assert_eq!(file.additions, 1);
        assert_eq!(file.deletions, 1);
        assert_eq!(file.hunks.len(), 1);
        assert_eq!(file.hunks[0].lines.len(), 4);
        // Three lines on each side, so the widest gutter number is 3.
        assert_eq!(file.max_line, 3);
    }

    #[test]
    fn line_numbers_advance_per_side() {
        let file = &parse_patch(SIMPLE)[0];
        let lines = &file.hunks[0].lines;
        assert_eq!((lines[0].old_no, lines[0].new_no), (Some(1), Some(1)));
        assert_eq!((lines[1].old_no, lines[1].new_no), (Some(2), None));
        assert_eq!((lines[2].old_no, lines[2].new_no), (None, Some(2)));
        assert_eq!((lines[3].old_no, lines[3].new_no), (Some(3), Some(3)));
    }

    #[test]
    fn detects_new_deleted_and_renamed_files() {
        let added = &parse_patch(
            "diff --git a/x.rs b/x.rs\nnew file mode 100644\n--- /dev/null\n+++ b/x.rs\n",
        )[0];
        assert_eq!(added.status, FileStatus::Added);

        let deleted = &parse_patch(
            "diff --git a/x.rs b/x.rs\ndeleted file mode 100644\n--- a/x.rs\n+++ /dev/null\n",
        )[0];
        assert_eq!(deleted.status, FileStatus::Deleted);

        let renamed =
            &parse_patch("diff --git a/old.rs b/new.rs\nrename from old.rs\nrename to new.rs\n")[0];
        assert_eq!(renamed.status, FileStatus::Renamed);
        assert_eq!(renamed.path, "new.rs");
        assert_eq!(renamed.old_path.as_deref(), Some("old.rs"));
    }

    #[test]
    fn detects_binary_files() {
        let file = &parse_patch(
            "diff --git a/logo.png b/logo.png\nBinary files a/logo.png and b/logo.png differ\n",
        )[0];
        assert!(file.binary);
        assert!(file_notices(file).iter().any(|n| n.contains("Binary")));
    }

    #[test]
    fn keeps_no_newline_markers_as_meta_rows() {
        let file =
            &parse_patch("diff --git a/a b/a\n@@ -1 +1 @@\n-x\n+y\n\\ No newline at end of file\n")
                [0];
        let last = file.hunks[0].lines.last().unwrap();
        assert_eq!(last.kind, LineKind::Meta);
        assert_eq!(last.text, "No newline at end of file");
    }

    #[test]
    fn handles_quoted_paths_with_spaces() {
        let file = &parse_patch("diff --git \"a/my file.txt\" \"b/my file.txt\"\n")[0];
        assert_eq!(file.path, "my file.txt");
    }

    #[test]
    fn a_truncated_patch_keeps_what_parsed() {
        let file = &parse_patch("diff --git a/a b/a\n@@ -1,3 +1,3 @@\n a\n-b\n")[0];
        assert_eq!(file.hunks[0].lines.len(), 2);
    }

    #[test]
    fn a_bare_hunk_without_a_git_header_still_parses() {
        let files = parse_patch("@@ -1,2 +1,2 @@\n a\n-b\n+c\n");
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].hunks[0].lines.len(), 3);
    }

    #[test]
    fn an_empty_patch_yields_no_files() {
        assert!(parse_patch("").is_empty());
    }

    #[test]
    fn gutter_widens_for_large_line_numbers() {
        let metrics = Metrics::default();
        let mut file = FileDiff::new("a".into(), None);
        file.max_line = 9;
        let narrow = gutter_width(&file, &metrics);
        file.max_line = 99999;
        assert!(gutter_width(&file, &metrics) > narrow);
        assert_eq!(narrow, metrics.diff_gutter_width);
    }

    #[test]
    fn gutter_follows_the_metrics_override() {
        let mut metrics = Metrics::default();
        metrics.diff_gutter_width = 80.0;
        let file = FileDiff::new("a".into(), None);
        assert_eq!(gutter_width(&file, &metrics), 80.0);
    }

    #[test]
    fn word_diff_isolates_the_changed_token() {
        let (old, new) = word_diff("const b = 2", "const b = 3");
        assert_eq!(old.len(), 1);
        assert_eq!(new.len(), 1);
        assert_eq!(&"const b = 2"[old[0].clone()], "2");
        assert_eq!(&"const b = 3"[new[0].clone()], "3");
    }

    #[test]
    fn word_diff_spans_a_middle_run() {
        let a = "foo(alpha, beta)";
        let b = "foo(gamma, beta)";
        let (old, new) = word_diff(a, b);
        assert_eq!(&a[old[0].clone()], "alpha");
        assert_eq!(&b[new[0].clone()], "gamma");
    }

    /// The trim walks `words.len() - prefix` and `words.len() - 1 - suffix`.
    /// Degenerate inputs are where an off-by-one becomes a panic.
    #[test]
    fn word_diff_survives_degenerate_inputs() {
        for (old, new) in [
            ("", "x"),
            ("x", ""),
            ("", ""),
            ("a", "a b"),
            ("a b", "a"),
            (" ", "  "),
            ("aaa", "aaaa"),
        ] {
            let (old_ranges, new_ranges) = word_diff(old, new);
            for range in &old_ranges {
                assert!(old.get(range.clone()).is_some(), "{old:?} {range:?}");
            }
            for range in &new_ranges {
                assert!(new.get(range.clone()).is_some(), "{new:?} {range:?}");
            }
        }
    }

    #[test]
    fn word_diff_of_identical_lines_is_empty() {
        let (old, new) = word_diff("same", "same");
        assert!(old.is_empty() && new.is_empty());
    }

    #[test]
    fn word_diff_handles_unicode_without_panicking() {
        let (old, new) = word_diff("let café = 1", "let café = 2");
        assert_eq!(&"let café = 1"[old[0].clone()], "1");
        assert_eq!(&"let café = 2"[new[0].clone()], "2");
    }

    #[test]
    fn annotation_pairs_equal_length_runs_only() {
        let mut files = parse_patch(SIMPLE);
        annotate_word_diffs(&mut files);
        let lines = &files[0].hunks[0].lines;
        assert_eq!(&lines[1].text[lines[1].word_ranges[0].clone()], "2");
        assert_eq!(&lines[2].text[lines[2].word_ranges[0].clone()], "3");

        // 1 deletion against 2 additions must not be paired by index.
        let mut uneven = parse_patch("diff --git a/a b/a\n@@ -1,1 +1,2 @@\n-x\n+y\n+z\n");
        annotate_word_diffs(&mut uneven);
        assert!(
            uneven[0].hunks[0]
                .lines
                .iter()
                .all(|l| l.word_ranges.is_empty())
        );
    }

    #[test]
    fn flatten_produces_one_row_per_line_plus_chrome() {
        let files = parse_patch(SIMPLE);
        let rows = flatten_rows(&files, |_| false, None);
        // header + 1 hunk header + 4 lines + body pad
        assert_eq!(rows.len(), 7);
        assert_eq!(rows[0], DiffRow::FileHeader { file: 0 });
        assert_eq!(rows[1], DiffRow::HunkHeader { file: 0, hunk: 0 });
        assert_eq!(*rows.last().unwrap(), DiffRow::BodyPad { file: 0 });
    }

    #[test]
    fn collapsing_removes_body_rows() {
        let files = parse_patch(SIMPLE);
        let rows = flatten_rows(&files, |_| true, None);
        assert_eq!(rows, vec![DiffRow::FileHeader { file: 0 }]);
    }

    #[test]
    fn notices_get_their_own_rows() {
        let files =
            parse_patch("diff --git a/x.rs b/x.rs\nnew file mode 100644\n@@ -0,0 +1 @@\n+hi\n");
        let rows = flatten_rows(&files, |_| false, None);
        assert!(rows.contains(&DiffRow::Notice { file: 0, notice: 0 }));
    }

    #[test]
    fn estimated_height_sits_between_the_extremes() {
        let m = Metrics::default();
        let files = parse_patch(SIMPLE);
        let rows = flatten_rows(&files, |_| false, None);
        let estimate = estimated_row_height(&rows, &m);
        assert!(
            estimate > m.diff_body_bottom_pad && estimate < m.diff_file_header_height,
            "{estimate}"
        );
        assert_eq!(estimated_row_height(&[], &m), m.diff_line_height);
    }

    #[test]
    fn row_heights_come_from_the_metrics() {
        let mut m = Metrics::default();
        m.diff_line_height = 40.0;
        assert_eq!(
            DiffRow::Line {
                file: 0,
                hunk: 0,
                line: 0
            }
            .height(&m),
            40.0
        );
        assert_eq!(
            DiffRow::FileHeader { file: 0 }.height(&m),
            m.diff_file_header_height
        );
    }

    #[test]
    fn max_lines_stops_and_counts_the_rest() {
        let files = parse_patch(concat!(
            "diff --git a/README.md b/README.md\n",
            "--- a/README.md\n",
            "+++ b/README.md\n",
            "@@ -1,2 +1,2 @@\n",
            " # Title\n",
            "-old line\n",
            "+new line\n",
            "diff --git a/src/lib.rs b/src/lib.rs\n",
            "new file mode 100644\n",
            "--- /dev/null\n",
            "+++ b/src/lib.rs\n",
            "@@ -0,0 +1,3 @@\n",
            "+pub fn hello() -> &'static str {\n",
            "+    \"hi\"\n",
            "+}\n",
        ));
        let rows = flatten_rows(&files, |_| false, Some(1));
        assert_eq!(rows.last(), Some(&DiffRow::ShowMore { remaining: 5 }));
        assert_eq!(
            rows.iter()
                .filter(|row| matches!(row, DiffRow::Line { .. }))
                .count(),
            1
        );
        assert!(
            !rows
                .iter()
                .any(|row| matches!(row, DiffRow::FileHeader { file: 1 }))
        );
    }

    #[test]
    fn max_lines_at_a_file_boundary_omits_later_headers() {
        let files = parse_patch(concat!(
            "diff --git a/README.md b/README.md\n",
            "--- a/README.md\n",
            "+++ b/README.md\n",
            "@@ -1,2 +1,2 @@\n",
            " # Title\n",
            "-old line\n",
            "+new line\n",
            "diff --git a/src/lib.rs b/src/lib.rs\n",
            "new file mode 100644\n",
            "--- /dev/null\n",
            "+++ b/src/lib.rs\n",
            "@@ -0,0 +1,3 @@\n",
            "+pub fn hello() -> &'static str {\n",
            "+    \"hi\"\n",
            "+}\n",
        ));
        let rows = flatten_rows(&files, |_| false, Some(3));
        assert_eq!(
            rows.iter()
                .filter(|row| matches!(row, DiffRow::Line { .. }))
                .count(),
            3
        );
        assert_eq!(rows.last(), Some(&DiffRow::ShowMore { remaining: 3 }));
        assert!(
            !rows
                .iter()
                .any(|row| matches!(row, DiffRow::FileHeader { file: 1 }))
        );
    }

    #[test]
    fn max_lines_at_a_hunk_boundary_omits_the_next_hunk_header() {
        let files = parse_patch(concat!(
            "diff --git a/a.ts b/a.ts\n",
            "--- a/a.ts\n",
            "+++ b/a.ts\n",
            "@@ -1,1 +1,1 @@\n",
            " keep\n",
            "@@ -10,1 +10,1 @@\n",
            "-old\n",
            "+new\n",
        ));
        let rows = flatten_rows(&files, |_| false, Some(1));
        assert_eq!(
            rows.iter()
                .filter(|row| matches!(row, DiffRow::HunkHeader { .. }))
                .count(),
            1
        );
        assert_eq!(rows.last(), Some(&DiffRow::ShowMore { remaining: 2 }));
    }
}
