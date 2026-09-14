//! Optional terminal backend. Shared backend sources deliberately do not depend on quickgui,
//! WGPU, Taffy, or the application runtime. Rendering stays in the host's retained view adapter.
#![allow(dead_code, unused_imports)]

#[path = "../../../src/color.rs"]
mod color;
#[path = "../../../src/extension_api.rs"]
mod extension_api;
#[path = "../../../src/terminal.rs"]
mod terminal;
#[path = "../../../src/terminal_process.rs"]
mod terminal_process;

use color::Color;
use quickgui_extension_sdk::{ComponentRuntime, WakeHandle};
use std::sync::Arc;

const MAX_TEXT_HIGHLIGHTS: usize = 4096;

// Screen-run data used by the backend; actual font/layout types remain in the renderer.
#[derive(Clone, Debug, Default, PartialEq)]
struct HighlightStyle {
    color: Option<Color>,
    background: Option<Color>,
    flags: u32,
}
impl HighlightStyle {
    fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
    fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }
    fn font_bold(mut self) -> Self {
        self.flags |= 4;
        self
    }
    fn italic(mut self) -> Self {
        self.flags |= 8;
        self
    }
    fn underline(mut self) -> Self {
        self.flags |= 16;
        self
    }
    fn double_underline(mut self) -> Self {
        self.flags |= 32;
        self
    }
    fn text_decoration_wavy(mut self) -> Self {
        self.flags |= 64;
        self
    }
    fn strikethrough(mut self) -> Self {
        self.flags |= 128;
        self
    }
}

#[derive(Clone)]
enum Key {
    Character(String),
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    PageUp,
    PageDown,
    Home,
    End,
    Enter,
    Escape,
    Space,
    Tab,
    Backspace,
    Delete,
    Insert,
    Function(u8),
    Other,
}
bitflags::bitflags! {
    #[derive(Clone, Copy)]
    struct Modifiers: u32 { const SHIFT = 1; const CONTROL = 2; const ALT = 4; const SUPER = 8; }
}

#[derive(Clone)]
struct WindowInvalidator(WakeHandle);
impl WindowInvalidator {
    fn invalidate(&self) -> bool {
        self.0.invalidate();
        true
    }
}

fn static_selection_color() -> Color {
    Color::rgba8(48, 120, 196, 105)
}

/// # Safety
/// The returned descriptor is immutable and valid for the lifetime of the loaded image.
#[unsafe(no_mangle)]
pub extern "C" fn quickgui_extension_v1() -> *const extension_api::Extension {
    &DESCRIPTOR
}

static API: extension_api::ComponentApi = ComponentRuntime::<terminal::component::Factory>::API;
static DESCRIPTOR: extension_api::Extension = extension_api::Extension {
    abi_version: extension_api::ABI_VERSION,
    descriptor_size: size_of::<extension_api::Extension>() as u32,
    kind: extension_api::COMPONENT_EXTENSION,
    api_size: size_of::<extension_api::ComponentApi>() as u32,
    name: extension_api::Bytes::new(b"terminal"),
    version: extension_api::Bytes::new(env!("CARGO_PKG_VERSION").as_bytes()),
    api: (&API as *const extension_api::ComponentApi).cast(),
};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Rect {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}
impl Rect {
    const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
    fn is_empty(self) -> bool {
        self.width <= 0.0 || self.height <= 0.0
    }
}
struct Canvas<'a> {
    bounds: Rect,
    rectangles: &'a mut Vec<quickgui_extension_sdk::schema::PaintRect>,
}
impl Canvas<'_> {
    fn bounds(&self) -> Rect {
        self.bounds
    }
    fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.rectangles
            .push(quickgui_extension_sdk::schema::PaintRect {
                bounds: [rect.x, rect.y, rect.width, rect.height],
                color: color.as_array(),
            });
    }
}
