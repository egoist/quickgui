//! A retained, PTY-backed terminal component powered by libghostty-vt.

#[path = "terminal/graphics.rs"]
mod graphics;

use self::graphics::{CellMetrics, paint_block, paint_padding_extension};
#[cfg(any(feature = "terminal", quickgui_terminal_extension))]
use crate::terminal_process::DetectedAgentProcess;
#[cfg(not(quickgui_terminal_extension))]
use crate::{
    AccessibilityRole, ClipboardItem, Element, ElementId, EventContext, FontFamily, GesturePhase,
    IntoElement, KeyDownEvent, KeyUpEvent, MouseButton, PointerEvent, PointerPhase, Rect,
    StyledText, ViewContext, canvas, div,
};
use crate::{Color, HighlightStyle, Key, MAX_TEXT_HIGHLIGHTS, Modifiers, WindowInvalidator};
#[cfg(any(feature = "terminal", quickgui_terminal_extension))]
use libghostty_vt::{
    RenderState, Terminal as GhosttyTerminal, TerminalOptions as GhosttyTerminalOptions,
    fmt::Format as GhosttyFormat,
    key::{
        Action as GhosttyKeyAction, Encoder as GhosttyKeyEncoder, Event as GhosttyKeyEvent,
        Key as GhosttyKey, Mods as GhosttyMods, OptionAsAlt,
    },
    render::{CellIterator, CursorVisualStyle, RowIterator},
    screen::CellWide,
    selection::{
        FormatOptions as GhosttySelectionFormatOptions,
        gesture::{
            DragEvent as GhosttySelectionDragEvent, Geometry as GhosttySelectionGeometry,
            Gesture as GhosttySelectionGesture, PressEvent as GhosttySelectionPressEvent,
            ReleaseEvent as GhosttySelectionReleaseEvent,
        },
    },
    style::{RgbColor, Underline},
    terminal::{Mode, Point as GhosttyPoint, PointCoordinate, ScrollViewport},
};
#[cfg(any(feature = "terminal", quickgui_terminal_extension))]
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, Instant};
#[cfg(any(feature = "terminal", quickgui_terminal_extension))]
#[cfg(target_arch = "wasm32")]
use web_time::{Duration, Instant};

use std::{
    cell::RefCell,
    io::{Read, Write},
    rc::Rc,
    sync::{
        RwLock,
        atomic::AtomicBool,
        mpsc::{Receiver, SyncSender, TryRecvError, sync_channel},
    },
    thread,
};
use std::{
    ffi::{OsStr, OsString},
    ops::Range,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU32, AtomicU64, Ordering},
    },
};
#[path = "terminal/data.rs"]
mod data;
#[cfg(any(feature = "terminal", quickgui_terminal_extension))]
use crate::static_selection_color;
use data::EdgeBackgrounds;
#[cfg(any(feature = "terminal", quickgui_terminal_extension))]
use data::is_block_element;
/// Maximum UTF-8 bytes accepted for a program, argument, environment entry, or working directory.
pub const MAX_TERMINAL_STRING_BYTES: usize = 32 * 1024;
/// Maximum arguments retained by one terminal process declaration.
pub const MAX_TERMINAL_ARGUMENTS: usize = 256;
/// Maximum environment overrides retained by one terminal process declaration.
pub const MAX_TERMINAL_ENVIRONMENT: usize = 256;
/// Maximum scrollback rows retained by libghostty-vt for one terminal.
pub const MAX_TERMINAL_SCROLLBACK: usize = 100_000;
/// Maximum bytes accepted by one direct write or paste operation.
pub const MAX_TERMINAL_INPUT_BYTES: usize = 1024 * 1024;
/// Number of configurable colors in the standard and bright ANSI palette.
pub const TERMINAL_ANSI_COLOR_COUNT: usize = 16;

const DEFAULT_COLS: u16 = 80;
const DEFAULT_ROWS: u16 = 24;
const DEFAULT_CELL_WIDTH_RATIO: f32 = 0.6;
const MAX_COLS: u16 = 512;
const MAX_ROWS: u16 = 256;
#[cfg(any(feature = "terminal", quickgui_terminal_extension))]
const MESSAGE_CAPACITY: usize = 256;
#[cfg(any(feature = "terminal", quickgui_terminal_extension))]
const READ_CHUNK_BYTES: usize = 16 * 1024;
#[cfg(any(feature = "terminal", quickgui_terminal_extension))]
const WORKER_BATCH_LIMIT: usize = 256;
#[cfg(any(feature = "terminal", quickgui_terminal_extension))]
const AGENT_PROCESS_PROBE_INTERVAL: Duration = Duration::from_millis(500);
#[cfg(any(feature = "terminal", quickgui_terminal_extension))]
const AGENT_RECENT_ACTIVITY: Duration = Duration::from_millis(1_200);
#[cfg(any(feature = "terminal", quickgui_terminal_extension))]
const TERM_PROGRAM_VALUE: &str = "ghostty";
const TERMINAL_SCROLLBAR_HIT_WIDTH: f32 = 12.0;
const TERMINAL_SCROLLBAR_IDLE_WIDTH: f32 = 3.0;
const TERMINAL_SCROLLBAR_HOVER_WIDTH: f32 = 7.0;
const TERMINAL_SCROLLBAR_EDGE_INSET: f32 = 1.0;
const TERMINAL_SCROLLBAR_VERTICAL_INSET: f32 = 2.0;
const TERMINAL_SCROLLBAR_MIN_THUMB: f32 = 24.0;
const TERMINAL_SCROLLBAR_HIDE_DELAY: Duration = Duration::from_millis(900);
const TERMINAL_SCROLLBAR_ID_TAG: u64 = 0x7465_726d_7363_726c;
const TERMINAL_TEXT_ID_TAG: u64 = 0x7465_726d_7465_7874;
const TERMINAL_SELECTION_ID_TAG: u64 = 0x7465_726d_7365_6c65;
const TERMINAL_CURSOR_BLINK_HALF_PERIOD: Duration = Duration::from_millis(500);
#[cfg(any(feature = "terminal", quickgui_terminal_extension))]
const TERMINAL_MULTI_CLICK_INTERVAL: Duration = Duration::from_millis(500);
const TERMINAL_MULTI_CLICK_DISTANCE: f32 = 4.0;

/// Process and scrollback configuration for a terminal session.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TerminalOptions {
    /// Program to execute. `None` starts the platform's default shell.
    pub program: Option<OsString>,
    /// Arguments passed without shell interpolation.
    pub arguments: Vec<OsString>,
    /// Initial working directory. `None` inherits the application directory.
    pub working_directory: Option<PathBuf>,
    /// Environment variables added to or replacing the inherited environment.
    pub environment: Vec<(OsString, OsString)>,
    /// Initial terminal width in cells.
    pub cols: u16,
    /// Initial terminal height in cells.
    pub rows: u16,
    /// Maximum libghostty scrollback rows.
    pub max_scrollback: usize,
}

impl Default for TerminalOptions {
    fn default() -> Self {
        Self {
            program: None,
            arguments: Vec::new(),
            working_directory: None,
            environment: Vec::new(),
            cols: DEFAULT_COLS,
            rows: DEFAULT_ROWS,
            max_scrollback: 10_000,
        }
    }
}

impl TerminalOptions {
    pub(crate) fn validate(&self) -> Result<(), TerminalError> {
        if !(1..=MAX_COLS).contains(&self.cols) || !(1..=MAX_ROWS).contains(&self.rows) {
            return Err(TerminalError::InvalidSize);
        }
        if self.arguments.len() > MAX_TERMINAL_ARGUMENTS {
            return Err(TerminalError::TooManyArguments);
        }
        if self.environment.len() > MAX_TERMINAL_ENVIRONMENT {
            return Err(TerminalError::TooManyEnvironmentVariables);
        }
        if self.max_scrollback > MAX_TERMINAL_SCROLLBACK {
            return Err(TerminalError::ScrollbackTooLarge);
        }
        if self
            .program
            .as_deref()
            .is_some_and(|program| program.is_empty() || !valid_os_string(program))
            || self.arguments.iter().any(|value| !valid_os_string(value))
            || self
                .working_directory
                .as_deref()
                .is_some_and(|value| !valid_os_string(value.as_os_str()))
            || self.environment.iter().any(|(key, value)| {
                key.is_empty()
                    || key.to_string_lossy().contains('=')
                    || !valid_os_string(key)
                    || !valid_os_string(value)
            })
        {
            return Err(TerminalError::InvalidString);
        }
        Ok(())
    }
}

/// Complete application-supplied color theme for a terminal emulator.
///
/// The first eight ANSI entries are the normal colors in black-through-white order. The final
/// eight are their bright variants. Changing this theme updates libghostty's defaults without
/// restarting the PTY, while OSC color overrides emitted by the running program remain honored.
#[derive(Clone, Debug, PartialEq)]
pub struct TerminalTheme {
    pub foreground: Color,
    pub background: Color,
    pub cursor: Color,
    pub ansi: [Color; TERMINAL_ANSI_COLOR_COUNT],
}

impl TerminalTheme {
    pub fn new(
        foreground: Color,
        background: Color,
        ansi: [Color; TERMINAL_ANSI_COLOR_COUNT],
    ) -> Self {
        Self {
            foreground,
            background,
            cursor: foreground,
            ansi,
        }
    }

    pub fn cursor(mut self, cursor: Color) -> Self {
        self.cursor = cursor;
        self
    }
}

/// Terminal session creation errors detected before its worker starts.
#[derive(Debug, thiserror::Error)]
pub enum TerminalError {
    #[error("terminal dimensions must be between 1x1 and {MAX_COLS}x{MAX_ROWS}")]
    InvalidSize,
    #[error("terminal process declarations support at most {MAX_TERMINAL_ARGUMENTS} arguments")]
    TooManyArguments,
    #[error(
        "terminal process declarations support at most {MAX_TERMINAL_ENVIRONMENT} environment variables"
    )]
    TooManyEnvironmentVariables,
    #[error("terminal scrollback cannot exceed {MAX_TERMINAL_SCROLLBACK} rows")]
    ScrollbackTooLarge,
    #[error(
        "terminal program, argument, directory, and environment strings must be non-NUL and at most {MAX_TERMINAL_STRING_BYTES} bytes"
    )]
    InvalidString,
    #[error("could not start the terminal worker: {0}")]
    Worker(String),
}

/// Current lifecycle state of the process attached to a terminal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TerminalStatus {
    Starting,
    Running {
        process_id: Option<u32>,
    },
    Exited {
        exit_code: u32,
        signal: Option<Arc<str>>,
    },
    Failed {
        message: Arc<str>,
    },
}

/// Screen-derived activity for an agent detected in the terminal's foreground process group.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TerminalAgentStatus {
    Idle,
    Working,
    Blocked,
}

impl TerminalAgentStatus {
    pub const fn kind(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Working => "working",
            Self::Blocked => "blocked",
        }
    }
}

/// An agent process discovered from the real PTY foreground job.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TerminalAgent {
    pub kind: Arc<str>,
    pub status: TerminalAgentStatus,
    pub process_id: u32,
}

/// Scrollbar geometry reported by libghostty for the visible terminal viewport.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TerminalScrollState {
    /// Total rows in the active screen, including retained history.
    pub total_rows: u64,
    /// Visible viewport offset measured from the top of retained history.
    pub offset_rows: u64,
    /// Number of rows visible in the viewport.
    pub viewport_rows: u64,
}

impl TerminalScrollState {
    const fn initial(rows: u16) -> Self {
        Self {
            total_rows: rows as u64,
            offset_rows: 0,
            viewport_rows: rows as u64,
        }
    }

    const fn max_offset_rows(self) -> u64 {
        self.total_rows.saturating_sub(self.viewport_rows)
    }
}

/// Cursor shape selected by the running terminal application through libghostty.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TerminalCursorStyle {
    Bar,
    Block,
    Underline,
    BlockHollow,
}

/// Cursor metadata for the currently visible libghostty viewport.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TerminalCursor {
    pub column: u16,
    pub row: u16,
    pub style: TerminalCursorStyle,
    pub blinking: bool,
    pub color: Color,
}

impl TerminalStatus {
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Starting => "starting",
            Self::Running { .. } => "running",
            Self::Exited { .. } => "exited",
            Self::Failed { .. } => "failed",
        }
    }
}

/// Immutable render and process metadata produced by a terminal worker.
#[derive(Clone, Debug)]
pub struct TerminalSnapshot {
    pub revision: u64,
    pub cols: u16,
    pub rows: u16,
    pub content: Arc<str>,
    pub highlights: Arc<[(Range<usize>, HighlightStyle)]>,
    pub foreground: Color,
    pub background: Color,
    pub title: Arc<str>,
    pub working_directory: Arc<str>,
    pub scroll: TerminalScrollState,
    pub cursor: Option<TerminalCursor>,
    pub(crate) selected_text: Option<Arc<str>>,
    pub(crate) cursor_range: Option<Range<usize>>,
    pub(crate) graphics: Arc<[TerminalCellGraphic]>,
    pub(crate) edge_backgrounds: EdgeBackgrounds,
    pub status: TerminalStatus,
    pub agent: Option<TerminalAgent>,
}

impl TerminalSnapshot {
    fn starting(cols: u16, rows: u16) -> Self {
        Self {
            revision: 0,
            cols,
            rows,
            content: Arc::from(""),
            highlights: Arc::from([]),
            foreground: Color::rgb8(224, 224, 224),
            background: Color::rgb8(20, 20, 20),
            title: Arc::from(""),
            working_directory: Arc::from(""),
            scroll: TerminalScrollState::initial(rows),
            cursor: None,
            selected_text: None,
            cursor_range: None,
            graphics: Arc::from([]),
            edge_backgrounds: EdgeBackgrounds::empty(cols, rows),
            status: TerminalStatus::Starting,
            agent: None,
        }
    }
}

/// A Unicode block element rendered from terminal-cell geometry instead of a font outline.
///
/// Terminal emulators synthesize these glyphs so adjoining blocks cover the grid exactly. Keeping
/// the original character in `TerminalSnapshot::content` preserves selection and clipboard text.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TerminalCellGraphic {
    pub(crate) column: u16,
    pub(crate) row: u16,
    pub(crate) character: char,
    pub(crate) foreground: Option<Color>,
}

/// How a terminal paints the space between its grid and its element bounds.
#[cfg(not(quickgui_terminal_extension))]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TerminalPaddingColor {
    /// Paint padding with the terminal's default background.
    #[default]
    Background,
    /// Extend the nearest grid-edge cell background through the padding.
    Extend,
}

/// Visual metrics for a [`Terminal`] element.
#[cfg(not(quickgui_terminal_extension))]
#[derive(Clone, Debug, PartialEq)]
pub struct TerminalStyle {
    pub font_family: FontFamily,
    pub font_size: f32,
    pub line_height: f32,
    /// Optically thicken glyph stems without selecting a heavier font face.
    pub font_thicken: bool,
    /// Logical cell width as a multiple of `font_size`.
    pub cell_width_ratio: f32,
    /// Space between the terminal surface and its PTY-backed content.
    pub padding_top: f32,
    pub padding_right: f32,
    pub padding_bottom: f32,
    pub padding_left: f32,
    /// Background treatment for the space around the terminal grid.
    pub padding_color: TerminalPaddingColor,
    /// Override the emulator's default foreground without changing explicit ANSI colors.
    pub foreground: Option<Color>,
    /// Override the emulator's default background without changing explicit ANSI colors.
    pub background: Option<Color>,
    /// Core-owned libghostty color theme, including all normal and bright ANSI colors.
    pub theme: Option<TerminalTheme>,
}

#[cfg(not(quickgui_terminal_extension))]
impl Default for TerminalStyle {
    fn default() -> Self {
        Self {
            font_family: FontFamily::Monospace,
            font_size: 13.0,
            line_height: 18.0,
            font_thicken: false,
            cell_width_ratio: DEFAULT_CELL_WIDTH_RATIO,
            padding_top: 0.0,
            padding_right: 0.0,
            padding_bottom: 0.0,
            padding_left: 0.0,
            padding_color: TerminalPaddingColor::Background,
            foreground: None,
            background: None,
            theme: None,
        }
    }
}

#[cfg(not(quickgui_terminal_extension))]
impl TerminalStyle {
    fn normalized(&self) -> Self {
        Self {
            font_family: self.font_family.clone(),
            font_size: finite_clamp(self.font_size, 1.0, 128.0, 13.0),
            line_height: finite_clamp(self.line_height, 1.0, 256.0, 18.0),
            font_thicken: self.font_thicken,
            cell_width_ratio: finite_clamp(
                self.cell_width_ratio,
                0.2,
                2.0,
                DEFAULT_CELL_WIDTH_RATIO,
            ),
            padding_top: finite_clamp(self.padding_top, 0.0, 256.0, 0.0),
            padding_right: finite_clamp(self.padding_right, 0.0, 256.0, 0.0),
            padding_bottom: finite_clamp(self.padding_bottom, 0.0, 256.0, 0.0),
            padding_left: finite_clamp(self.padding_left, 0.0, 256.0, 0.0),
            padding_color: self.padding_color,
            foreground: self.foreground,
            background: self.background,
            theme: self.theme.clone(),
        }
    }
}

/// A real pseudoterminal session whose VT state is owned by a dedicated worker thread.
#[derive(Clone)]
pub struct Terminal {
    inner: Arc<TerminalInner>,
}

struct TerminalInner {
    #[cfg(any(feature = "terminal", quickgui_terminal_extension))]
    messages: SyncSender<WorkerMessage>,
    #[cfg(any(feature = "terminal", quickgui_terminal_extension))]
    snapshot: Arc<RwLock<Arc<TerminalSnapshot>>>,
    #[cfg(not(quickgui_terminal_extension))]
    last_size: AtomicU64,
    #[cfg(not(quickgui_terminal_extension))]
    viewport_height_bits: AtomicU32,
    #[cfg(not(quickgui_terminal_extension))]
    viewport_bounds: Mutex<Rect>,
    #[cfg(not(quickgui_terminal_extension))]
    selection_epoch: Instant,
    #[cfg(not(quickgui_terminal_extension))]
    wheel_remainder: Mutex<f32>,
    #[cfg(not(quickgui_terminal_extension))]
    scrollbar_drag_remainder: Mutex<f32>,
    #[cfg(not(quickgui_terminal_extension))]
    scrollbar_interaction: Mutex<TerminalScrollbarInteraction>,
    #[cfg(not(quickgui_terminal_extension))]
    cursor_blink: Mutex<TerminalCursorBlink>,
    #[cfg(not(quickgui_terminal_extension))]
    theme: Mutex<Option<TerminalTheme>>,
    #[cfg(any(feature = "terminal", quickgui_terminal_extension))]
    shutdown: Arc<AtomicBool>,
}

#[cfg(not(quickgui_terminal_extension))]
#[derive(Clone, Copy, Debug)]
struct TerminalCursorBlink {
    focused: bool,
    epoch: Instant,
}

#[cfg(not(quickgui_terminal_extension))]
impl TerminalCursorBlink {
    fn new(now: Instant) -> Self {
        Self {
            focused: false,
            epoch: now,
        }
    }

    fn reset(&mut self, now: Instant) {
        self.epoch = now;
    }

    fn update_focus(&mut self, focused: bool, now: Instant) -> Instant {
        if focused && !self.focused {
            self.reset(now);
        }
        self.focused = focused;
        self.epoch
    }
}

#[cfg(not(quickgui_terminal_extension))]
#[derive(Default)]
struct TerminalScrollbarInteraction {
    hovered: bool,
    dragging: bool,
    visible_until: Option<Instant>,
}

#[cfg(not(quickgui_terminal_extension))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct TerminalScrollbarPresentation {
    visible: bool,
    expanded: bool,
    hide_at: Option<Instant>,
}

#[cfg(not(quickgui_terminal_extension))]
impl TerminalScrollbarInteraction {
    fn reveal(&mut self, now: Instant) {
        self.visible_until = Some(now + TERMINAL_SCROLLBAR_HIDE_DELAY);
    }

    fn set_hovered(&mut self, hovered: bool, now: Instant) {
        self.hovered = hovered;
        self.reveal(now);
    }

    fn begin_drag(&mut self, now: Instant) {
        self.dragging = true;
        self.reveal(now);
    }

    fn end_drag(&mut self, now: Instant) {
        self.dragging = false;
        self.reveal(now);
    }

    fn presentation(&self, now: Instant) -> TerminalScrollbarPresentation {
        let expanded = self.hovered || self.dragging;
        let hide_at = self.visible_until.filter(|deadline| *deadline > now);
        TerminalScrollbarPresentation {
            visible: expanded || hide_at.is_some(),
            expanded,
            hide_at: if expanded { None } else { hide_at },
        }
    }
}

#[cfg(any(feature = "terminal", quickgui_terminal_extension))]
impl Drop for TerminalInner {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Release);
        let _ = self.messages.try_send(WorkerMessage::Shutdown);
    }
}

#[path = "terminal/api.rs"]
mod api;
#[cfg(quickgui_terminal_extension)]
#[path = "../extensions/terminal/src/component.rs"]
pub(crate) mod component;
#[cfg(not(quickgui_terminal_extension))]
#[path = "terminal/render.rs"]
mod render;
#[cfg(any(feature = "terminal", quickgui_terminal_extension))]
#[path = "terminal/snapshot.rs"]
mod snapshot;
#[cfg(any(feature = "terminal", quickgui_terminal_extension))]
#[path = "terminal/worker.rs"]
mod worker;

#[cfg(not(quickgui_terminal_extension))]
use render::*;
#[cfg(any(feature = "terminal", quickgui_terminal_extension))]
use snapshot::*;
#[cfg(any(feature = "terminal", quickgui_terminal_extension))]
use worker::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct TerminalSize {
    cols: u16,
    rows: u16,
    cell_width_px: u32,
    cell_height_px: u32,
}

enum WorkerMessage {
    #[cfg(any(feature = "terminal", quickgui_terminal_extension))]
    Output(Vec<u8>),
    #[cfg(any(feature = "terminal", quickgui_terminal_extension))]
    ReaderClosed,
    Input(Vec<u8>),
    Paste(String),
    Key(TerminalKeyInput),
    Resize(TerminalSize),
    Scroll(isize),
    Selection(TerminalSelectionInput),
    SelectAll,
    Theme(Option<Box<TerminalTheme>>),
    #[cfg(any(feature = "terminal", quickgui_terminal_extension))]
    Shutdown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TerminalSelectionPhase {
    Press,
    Drag,
    Release,
    Cancel,
}

#[derive(Clone, Copy, Debug)]
struct TerminalSelectionInput {
    phase: TerminalSelectionPhase,
    column: u16,
    row: u16,
    surface_x: f64,
    surface_y: f64,
    geometry: GhosttySelectionGeometry,
    time: Duration,
    repeat_distance: f64,
    rectangle: bool,
}

#[cfg(any(feature = "terminal", quickgui_terminal_extension))]
struct TerminalSelectionState {
    gesture: GhosttySelectionGesture<'static>,
    press: GhosttySelectionPressEvent<'static>,
    drag: GhosttySelectionDragEvent<'static>,
    release: GhosttySelectionReleaseEvent<'static>,
    anchor: Option<(u16, u16)>,
}

#[cfg(any(feature = "terminal", quickgui_terminal_extension))]
impl TerminalSelectionState {
    fn new() -> Result<Self, libghostty_vt::Error> {
        Ok(Self {
            gesture: GhosttySelectionGesture::new()?,
            press: GhosttySelectionPressEvent::new()?,
            drag: GhosttySelectionDragEvent::new()?,
            release: GhosttySelectionReleaseEvent::new()?,
            anchor: None,
        })
    }

    fn clear(&mut self, terminal: &GhosttyTerminal<'_, '_>) {
        self.gesture.reset(terminal);
        self.anchor = None;
        let _ = terminal.set_selection(None);
    }

    fn apply(
        &mut self,
        terminal: &GhosttyTerminal<'_, '_>,
        input: TerminalSelectionInput,
    ) -> Result<(), libghostty_vt::Error> {
        let grid_ref = || {
            terminal.grid_ref(GhosttyPoint::Viewport(PointCoordinate {
                x: input.column,
                y: u32::from(input.row),
            }))
        };
        match input.phase {
            TerminalSelectionPhase::Press => {
                self.press
                    .set_position(input.surface_x, input.surface_y)?
                    .set_repeat_distance(input.repeat_distance)?
                    .set_repeat_interval(TERMINAL_MULTI_CLICK_INTERVAL)?
                    .set_time(input.time)?;
                let selection = self.press.apply(&mut self.gesture, terminal, grid_ref()?)?;
                terminal.set_selection(selection.as_ref())?;
                self.anchor = Some((input.column, input.row));
            }
            TerminalSelectionPhase::Drag => {
                self.drag_to(terminal, input)?;
            }
            TerminalSelectionPhase::Release => {
                if let Some(anchor) = self.anchor
                    && (input.column, input.row) != anchor
                {
                    self.drag_to(terminal, input)?;
                }
                self.release
                    .apply(&mut self.gesture, terminal, Some(grid_ref()?))?;
                self.anchor = None;
            }
            TerminalSelectionPhase::Cancel => {
                self.release.apply(&mut self.gesture, terminal, None)?;
                self.anchor = None;
            }
        }
        Ok(())
    }

    fn drag_to(
        &mut self,
        terminal: &GhosttyTerminal<'_, '_>,
        input: TerminalSelectionInput,
    ) -> Result<(), libghostty_vt::Error> {
        self.drag
            .set_position(input.surface_x, input.surface_y)?
            .set_rectangle(input.rectangle)?;
        let grid_ref = terminal.grid_ref(GhosttyPoint::Viewport(PointCoordinate {
            x: input.column,
            y: u32::from(input.row),
        }))?;
        let selection = self
            .drag
            .apply(&mut self.gesture, terminal, grid_ref, input.geometry)?;
        terminal.set_selection(selection.as_ref())?;
        Ok(())
    }
}

#[derive(Clone)]
struct TerminalKeyInput {
    key: Key,
    key_char: Option<Key>,
    text: Option<String>,
    modifiers: Modifiers,
    action: GhosttyKeyAction,
}

#[cfg(not(quickgui_terminal_extension))]
impl TerminalKeyInput {
    fn down(event: &KeyDownEvent) -> Self {
        Self {
            key: event.key.clone(),
            key_char: event.key_char.clone(),
            text: event.text.clone(),
            modifiers: event.modifiers,
            action: if event.repeat {
                GhosttyKeyAction::Repeat
            } else {
                GhosttyKeyAction::Press
            },
        }
    }

    fn up(event: &KeyUpEvent) -> Self {
        Self {
            key: event.key.clone(),
            key_char: event.key_char.clone(),
            text: None,
            modifiers: event.modifiers,
            action: GhosttyKeyAction::Release,
        }
    }
}

#[cfg(all(test, any(feature = "terminal", quickgui_terminal_extension)))]
#[path = "terminal/tests.rs"]
mod tests;

#[cfg(not(any(feature = "terminal", quickgui_terminal_extension)))]
fn valid_os_string(value: &OsStr) -> bool {
    let value = value.to_string_lossy();
    value.len() <= MAX_TERMINAL_STRING_BYTES && !value.contains('\0')
}

#[cfg(not(any(feature = "terminal", quickgui_terminal_extension)))]
fn finite_clamp(value: f32, min: f32, max: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(min, max)
    } else {
        fallback
    }
}

#[cfg(not(any(feature = "terminal", quickgui_terminal_extension)))]
fn pack_size(size: TerminalSize) -> u64 {
    u64::from(size.cols)
        | (u64::from(size.rows) << 16)
        | (u64::from(size.cell_width_px.min(u16::MAX as u32)) << 32)
        | (u64::from(size.cell_height_px.min(u16::MAX as u32)) << 48)
}

fn key_character(key: &Key) -> Option<&str> {
    match key {
        Key::Character(value) => Some(value),
        _ => None,
    }
}
