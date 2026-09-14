//! Terminal-owned component state and declarations. Only generic primitives cross the host ABI.
use super::*;
use crate::Canvas;
use quickgui_extension_sdk::{
    Component, ComponentFactory, WakeHandle,
    schema::{self as w, Style as S},
};
use serde_json::{Value, json};

pub struct Factory;
struct Surface {
    terminal: Terminal,
    wake: WakeHandle,
    props: Value,
    last_size: Option<TerminalSize>,
    metrics: CellMetrics,
    epoch: Instant,
    blink_epoch: Instant,
    last_revision: u64,
    last_status: Value,
    width: f32,
    height: f32,
    wheel: f32,
    scroll_drag: f32,
    scrollbar_until: Option<Instant>,
}
impl ComponentFactory for Factory {
    fn create(name: &str, props: Value, wake: WakeHandle) -> Result<Box<dyn Component>, String> {
        if name != "terminal" {
            return Err(format!("unknown terminal component {name}"));
        }
        let options = options(&props)?;
        let terminal = Terminal::spawn(options, WindowInvalidator(wake.clone()))
            .map_err(|error| error.to_string())?;
        let mut result = Surface {
            terminal,
            wake,
            props: Value::Null,
            last_size: None,
            metrics: CellMetrics::new(13.0, 18.0, 0.6, 1.0),
            epoch: Instant::now(),
            blink_epoch: Instant::now(),
            last_revision: u64::MAX,
            last_status: Value::Null,
            width: 0.0,
            height: 0.0,
            wheel: 0.0,
            scroll_drag: 0.0,
            scrollbar_until: None,
        };
        result.update(props)?;
        Ok(Box::new(result))
    }
}
fn options(v: &Value) -> Result<TerminalOptions, String> {
    let strings = |key: &str| -> Result<Vec<OsString>, String> {
        v[key]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|v| {
                        v.as_str()
                            .map(Into::into)
                            .ok_or_else(|| "terminal arguments must be strings".into())
                    })
                    .collect()
            })
            .unwrap_or(Ok(vec![]))
    };
    Ok(TerminalOptions {
        program: v["program"]
            .as_str()
            .filter(|v| !v.is_empty())
            .map(Into::into),
        arguments: strings("arguments")?,
        working_directory: v["workingDirectory"]
            .as_str()
            .filter(|v| !v.is_empty())
            .map(Into::into),
        environment: v["environment"]
            .as_object()
            .map(|o| {
                o.iter()
                    .map(|(k, v)| {
                        Ok((
                            k.into(),
                            v.as_str()
                                .ok_or("environment values must be strings")?
                                .into(),
                        ))
                    })
                    .collect::<Result<Vec<_>, String>>()
            })
            .transpose()?
            .unwrap_or_default(),
        max_scrollback: v["scrollback"]
            .as_u64()
            .filter(|v| *v > 0)
            .unwrap_or(10_000)
            .min(MAX_TERMINAL_SCROLLBACK as u64) as usize,
        ..Default::default()
    })
}
fn metric(v: &Value, key: &str, default: f32) -> f32 {
    v[key]
        .as_f64()
        .map(|n| n as f32)
        .filter(|n| n.is_finite())
        .unwrap_or(default)
        .clamp(0.0, 512.0)
}
fn color(v: &Value) -> Option<Color> {
    if let Some(n) = v.as_u64() {
        return Some(Color::rgba8(
            n as u8,
            (n >> 8) as u8,
            (n >> 16) as u8,
            (n >> 24) as u8,
        ));
    }
    let s = v.as_str()?.strip_prefix('#')?;
    let n = u32::from_str_radix(s, 16).ok()?;
    match s.len() {
        6 => Some(Color::rgb8((n >> 16) as u8, (n >> 8) as u8, n as u8)),
        8 => Some(Color::rgba8(
            (n >> 24) as u8,
            (n >> 16) as u8,
            (n >> 8) as u8,
            n as u8,
        )),
        _ => None,
    }
}
fn status(snapshot: &TerminalSnapshot) -> Value {
    let mut value = json!({"status":snapshot.status.kind(),"title":snapshot.title,"workingDirectory":snapshot.working_directory});
    match &snapshot.status {
        TerminalStatus::Running { process_id } => value["processId"] = json!(process_id),
        TerminalStatus::Exited { exit_code, signal } => {
            value["exitCode"] = json!(exit_code);
            value["signal"] = json!(signal);
        }
        TerminalStatus::Failed { message } => value["message"] = json!(message),
        _ => {}
    }
    if let Some(agent) = &snapshot.agent {
        value["agent"] = json!(agent.kind);
        value["agentStatus"] = json!(agent.status.kind());
        value["agentProcessId"] = json!(agent.process_id);
    }
    value
}
fn positioned(key: u64, x: f32, y: f32, width: f32, height: f32, paint: Color) -> w::Node {
    w::Node {
        key,
        styles: vec![
            S::Position("absolute".into()),
            S::Left(x),
            S::Top(y),
            S::Width(width),
            S::Height(height),
            S::Background(paint.as_array()),
        ],
        ..Default::default()
    }
}
impl Component for Surface {
    fn update(&mut self, props: Value) -> Result<bool, String> {
        if self.props == props {
            return Ok(false);
        }
        if !self.props.is_null() && options(&self.props)? != options(&props)? {
            self.terminal = Terminal::spawn(options(&props)?, WindowInvalidator(self.wake.clone()))
                .map_err(|error| error.to_string())?;
            self.last_status = Value::Null;
            self.last_revision = u64::MAX;
            self.last_size = None;
        }
        if let Some(palette) = props["palette"].as_array().filter(|p| p.len() == 16) {
            let snapshot = self.terminal.snapshot();
            let mut theme = TerminalTheme::new(
                color(&props["style"]["color"]).unwrap_or(snapshot.foreground),
                color(&props["style"]["backgroundColor"]).unwrap_or(snapshot.background),
                std::array::from_fn(|index| color(&palette[index]).unwrap_or(snapshot.foreground)),
            );
            theme.cursor = color(&props["cursorColor"]).unwrap_or(theme.foreground);
            self.terminal
                .try_send(WorkerMessage::Theme(Some(Box::new(theme))));
        }
        self.props = props;
        Ok(true)
    }
    fn render(&mut self, request: w::RenderRequest) -> Result<w::Frame, String> {
        let style = &self.props["style"];
        let font = metric(style, "fontSize", 13.0).max(1.0);
        let line = metric(style, "lineHeight", 18.0).max(1.0);
        self.metrics = CellMetrics::new(font, line, 0.6, request.scale);
        let m = self.metrics;
        let pad = metric(style, "padding", 8.0);
        self.width = (request.width - pad * 2.0).max(1.0);
        self.height = (request.height - pad * 2.0).max(1.0);
        let size = TerminalSize {
            cols: (self.width / m.logical_width)
                .floor()
                .clamp(1.0, MAX_COLS as f32) as u16,
            rows: (self.height / m.logical_height)
                .floor()
                .clamp(1.0, MAX_ROWS as f32) as u16,
            cell_width_px: m.physical_width,
            cell_height_px: m.physical_height,
        };
        if request.width > 0.0 && request.height > 0.0 && self.last_size != Some(size) {
            self.terminal.try_send(WorkerMessage::Resize(size));
            self.last_size = Some(size);
        }
        let snapshot = self.terminal.snapshot();
        let now = Instant::now();
        if snapshot.revision != self.last_revision {
            self.last_revision = snapshot.revision;
            self.blink_epoch = now;
        }
        let background = color(&style["backgroundColor"]).unwrap_or(snapshot.background);
        let foreground = color(&style["color"]).unwrap_or(snapshot.foreground);
        let mut spans: Vec<w::Span> = snapshot
            .highlights
            .iter()
            .map(|(range, highlight)| w::Span {
                start: range.start,
                end: range.end,
                foreground: highlight.color.map(Color::as_array),
                background: highlight.background.map(Color::as_array),
                weight: (highlight.flags & 4 != 0).then_some(700),
                italic: highlight.flags & 8 != 0,
                underline: highlight.flags & (16 | 32 | 64) != 0,
                strikethrough: highlight.flags & 128 != 0,
                ..Default::default()
            })
            .collect();
        let mut cursor_shape = None;
        let mut repaint_after_ms = None;
        if let Some(cursor) = snapshot.cursor {
            let elapsed = now.duration_since(self.blink_epoch).as_millis();
            if request.focused {
                repaint_after_ms = Some(500 - (elapsed % 500) as u64);
                if (elapsed / 500) % 2 == 0 {
                    cursor_shape = Some(cursor.style)
                }
            } else {
                cursor_shape = Some(TerminalCursorStyle::BlockHollow)
            }
            if cursor_shape == Some(TerminalCursorStyle::Block) {
                if let Some(range) = &snapshot.cursor_range {
                    // Split existing runs at the cursor so highlighting stays non-overlapping.
                    let mut next = Vec::new();
                    for span in spans {
                        if span.end <= range.start || span.start >= range.end {
                            next.push(span)
                        } else {
                            if span.start < range.start {
                                next.push(w::Span {
                                    end: range.start,
                                    ..span.clone()
                                })
                            }
                            if span.end > range.end {
                                next.push(w::Span {
                                    start: range.end,
                                    ..span
                                })
                            }
                        }
                    }
                    next.push(w::Span {
                        start: range.start,
                        end: range.end,
                        foreground: Some(background.as_array()),
                        background: Some(cursor.color.as_array()),
                        ..Default::default()
                    });
                    next.sort_by_key(|s| s.start);
                    spans = next;
                }
            }
        }
        let content = w::Node {
            key: 1,
            kind: w::Primitive::Text,
            text: snapshot.content.to_string(),
            spans,
            styles: vec![
                S::FontFamily(style["fontFamily"].as_str().unwrap_or("monospace").into()),
                S::FontSize(font),
                S::LineHeight(m.logical_height),
                S::MonospaceWidth(m.logical_width),
                S::BasicTextShaping,
                S::FontThicken(self.props["fontThicken"].as_bool().unwrap_or(false)),
                S::Foreground(foreground.as_array()),
                S::Wrap(false),
                S::UserSelect(false),
            ],
            ..Default::default()
        };
        let mut grid = w::Node {
            key: 2,
            styles: vec![
                S::Position("absolute".into()),
                S::Left(pad),
                S::Top(pad),
                S::Right(pad),
                S::Bottom(pad),
                S::Overflow("hidden".into()),
                S::UserSelect(false),
            ],
            events: vec!["pointer".into()],
            children: vec![content],
            ..Default::default()
        };
        let mut rectangles = Vec::new();
        {
            let mut canvas = Canvas {
                bounds: crate::Rect::new(0.0, 0.0, self.width, self.height),
                rectangles: &mut rectangles,
            };
            for graphic in snapshot.graphics.iter() {
                let paint = if cursor_shape == Some(TerminalCursorStyle::Block)
                    && snapshot
                        .cursor
                        .is_some_and(|c| c.column == graphic.column && c.row == graphic.row)
                {
                    background
                } else {
                    graphic.foreground.unwrap_or(foreground)
                };
                paint_block(
                    &mut canvas,
                    graphic.column,
                    graphic.row,
                    graphic.character,
                    m,
                    paint,
                );
            }
        }
        grid.children.push(w::Node {
            key: 3,
            kind: w::Primitive::Canvas,
            rectangles,
            styles: vec![
                S::Position("absolute".into()),
                S::WidthFraction(1.0),
                S::HeightFraction(1.0),
            ],
            ..Default::default()
        });
        if let (Some(cursor), Some(shape)) = (snapshot.cursor, cursor_shape) {
            let x = cursor.column as f32 * m.logical_width;
            let y = cursor.row as f32 * m.logical_height;
            let node = match shape {
                TerminalCursorStyle::Bar => {
                    positioned(4, x, y, 2.0, m.logical_height, cursor.color)
                }
                TerminalCursorStyle::Underline => positioned(
                    4,
                    x,
                    y + m.logical_height - 2.0,
                    m.logical_width,
                    2.0,
                    cursor.color,
                ),
                TerminalCursorStyle::BlockHollow => {
                    let mut n = positioned(
                        4,
                        x,
                        y,
                        m.logical_width,
                        m.logical_height,
                        Color::TRANSPARENT,
                    );
                    n.styles.push(S::Border([1.0; 4], cursor.color.as_array()));
                    n
                }
                TerminalCursorStyle::Block => positioned(4, x, y, 0.0, 0.0, Color::TRANSPARENT),
            };
            grid.children.push(node);
        }
        let mut root = w::Node {
            key: 0,
            styles: vec![
                S::Position("relative".into()),
                S::WidthFraction(1.0),
                S::HeightFraction(1.0),
                S::MinWidth(0.0),
                S::MinHeight(0.0),
                S::Overflow("hidden".into()),
                S::Background(background.as_array()),
                S::Focusable(true),
                S::UserSelect(false),
            ],
            events: vec!["key-down".into(), "key-up".into(), "wheel".into()],
            children: vec![],
            ..Default::default()
        };
        if self.props["paddingColor"] == "extend" {
            let mut rectangles = Vec::new();
            let mut canvas = Canvas {
                bounds: crate::Rect::new(0.0, 0.0, request.width, request.height),
                rectangles: &mut rectangles,
            };
            paint_padding_extension(&mut canvas, &snapshot.edge_backgrounds, m, pad, pad);
            root.children.push(w::Node {
                key: 5,
                kind: w::Primitive::Canvas,
                rectangles,
                styles: vec![
                    S::Position("absolute".into()),
                    S::WidthFraction(1.0),
                    S::HeightFraction(1.0),
                ],
                ..Default::default()
            });
        }
        root.children.push(grid);
        if snapshot.scroll.max_offset_rows() > 0
            && self.scrollbar_until.is_some_and(|until| until > now)
        {
            let h = (self.height * snapshot.scroll.viewport_rows as f32
                / snapshot.scroll.total_rows as f32)
                .max(24.0)
                .min(self.height);
            let y = (self.height - h) * snapshot.scroll.offset_rows as f32
                / snapshot.scroll.max_offset_rows() as f32;
            let mut thumb = positioned(
                6,
                (request.width - 6.0).max(0.0),
                pad + y,
                4.0,
                h,
                foreground.with_alpha(0.45),
            );
            thumb.events.push("pointer".into());
            root.children.push(thumb);
            let hide = self
                .scrollbar_until
                .unwrap()
                .duration_since(now)
                .as_millis() as u64;
            repaint_after_ms = Some(repaint_after_ms.map_or(hide, |delay| delay.min(hide)));
        }
        let mut events = vec![];
        let next = status(&snapshot);
        if self.last_status != next {
            self.last_status = next.clone();
            events.push(w::OutputEvent {
                kind: "status".into(),
                value: next,
            });
        }
        Ok(w::Frame {
            nodes: vec![root],
            events,
            repaint_after_ms,
        })
    }
    fn event(&mut self, event: w::InputEvent) -> Result<w::EventResult, String> {
        let mut result = w::EventResult::default();
        let v = &event.value;
        match event.kind.as_str() {
            "key-down" | "key-up" => {
                let mods =
                    Modifiers::from_bits_truncate(v["modifiers"].as_u64().unwrap_or(0) as u32);
                let raw = v["key"].as_str().unwrap_or("");
                let shortcut = mods.contains(Modifiers::SUPER)
                    || (!cfg!(target_os = "macos")
                        && mods.contains(Modifiers::CONTROL | Modifiers::SHIFT));
                if shortcut && event.kind == "key-down" {
                    match raw.to_lowercase().as_str() {
                        "c" => {
                            result.clipboard = self
                                .terminal
                                .snapshot()
                                .selected_text
                                .as_ref()
                                .map(ToString::to_string)
                        }
                        "v" => result.read_clipboard = true,
                        "a" => {
                            self.terminal.try_send(WorkerMessage::SelectAll);
                        }
                        _ => return Ok(result),
                    }
                } else if !mods.contains(Modifiers::SUPER) {
                    let key = parse_key(raw);
                    self.terminal.try_send(WorkerMessage::Key(TerminalKeyInput {
                        key: key.clone(),
                        key_char: Some(key),
                        text: v["text"].as_str().map(Into::into),
                        modifiers: mods,
                        action: if event.kind == "key-up" {
                            GhosttyKeyAction::Release
                        } else if v["repeat"] == true {
                            GhosttyKeyAction::Repeat
                        } else {
                            GhosttyKeyAction::Press
                        },
                    }));
                }
                self.blink_epoch = Instant::now();
                result.prevent_default = true;
                result.stop_propagation = true;
                result.changed = true;
            }
            "paste" => {
                if let Some(text) = v.as_str() {
                    self.terminal.try_send(WorkerMessage::Paste(text.into()));
                }
            }
            "wheel" => {
                self.wheel -= v["y"].as_f64().unwrap_or(0.0) as f32 / self.metrics.logical_height;
                let rows = self.wheel.trunc() as isize;
                self.wheel -= rows as f32;
                if rows != 0 {
                    self.terminal.try_send(WorkerMessage::Scroll(rows));
                }
                self.scrollbar_until = Some(Instant::now() + TERMINAL_SCROLLBAR_HIDE_DELAY);
                result.changed = true;
                result.prevent_default = true;
                result.stop_propagation = true;
            }
            "pointer" => {
                let phase = v["phase"].as_str().unwrap_or("");
                if event.target == 6 {
                    let y = v["y"].as_f64().unwrap_or(0.0) as f32;
                    if phase == "down" {
                        self.scroll_drag = y;
                    } else if phase == "move" {
                        let s = self.terminal.snapshot().scroll;
                        let h = (self.height * s.viewport_rows as f32 / s.total_rows as f32)
                            .max(24.0)
                            .min(self.height);
                        let rows = ((y - self.scroll_drag) * s.max_offset_rows() as f32
                            / (self.height - h).max(1.0))
                        .round() as isize;
                        self.scroll_drag = y;
                        self.terminal.try_send(WorkerMessage::Scroll(rows));
                    }
                    self.scrollbar_until = Some(Instant::now() + TERMINAL_SCROLLBAR_HIDE_DELAY);
                } else if event.target == 2 && v["button"] == "left" {
                    let snapshot = self.terminal.snapshot();
                    let m = self.metrics;
                    let x = v["x"].as_f64().unwrap_or(0.0);
                    let y = v["y"].as_f64().unwrap_or(0.0);
                    self.terminal
                        .try_send(WorkerMessage::Selection(TerminalSelectionInput {
                            phase: match phase {
                                "down" => TerminalSelectionPhase::Press,
                                "move" => TerminalSelectionPhase::Drag,
                                "up" => TerminalSelectionPhase::Release,
                                _ => TerminalSelectionPhase::Cancel,
                            },
                            column: (x as f32 / m.logical_width)
                                .floor()
                                .clamp(0.0, snapshot.cols.saturating_sub(1) as f32)
                                as u16,
                            row: (y as f32 / m.logical_height)
                                .floor()
                                .clamp(0.0, snapshot.rows.saturating_sub(1) as f32)
                                as u16,
                            surface_x: x * m.scale_factor as f64,
                            surface_y: y * m.scale_factor as f64,
                            geometry: GhosttySelectionGeometry {
                                columns: u32::from(snapshot.cols),
                                cell_width: m.physical_width,
                                padding_left: 0,
                                screen_height: (self.height * m.scale_factor) as u32,
                            },
                            time: Instant::now().duration_since(self.epoch),
                            repeat_distance: TERMINAL_MULTI_CLICK_DISTANCE as f64
                                * m.scale_factor as f64,
                            rectangle: v["modifiers"].as_u64().unwrap_or(0) & 4 != 0,
                        }));
                }
                result.prevent_default = true;
                result.stop_propagation = true;
                result.changed = true;
            }
            _ => {}
        }
        Ok(result)
    }
}
fn parse_key(raw: &str) -> Key {
    match raw {
        "ArrowUp" => Key::ArrowUp,
        "ArrowDown" => Key::ArrowDown,
        "ArrowLeft" => Key::ArrowLeft,
        "ArrowRight" => Key::ArrowRight,
        "PageUp" => Key::PageUp,
        "PageDown" => Key::PageDown,
        "Home" => Key::Home,
        "End" => Key::End,
        "Enter" => Key::Enter,
        "Escape" => Key::Escape,
        "Space" => Key::Space,
        "Tab" => Key::Tab,
        "Backspace" => Key::Backspace,
        "Delete" => Key::Delete,
        "Insert" => Key::Insert,
        _ => Key::Character(raw.into()),
    }
}
