//! An in-window menubar composed from `MenubarState` and `PopoverMenu`.
//!
//! QuickGUI owns the bar's roving focus, open state, hover switching, and menu semantics. This
//! application owns every color, size, separator, row, and the placement of each open surface.

use std::sync::Arc;

use quickgui::{
    Application, Color, Element, IntoElement, Menubar, MenubarState, PopoverMenu, PopoverMenuItem,
    PopoverMenuItemKind, PopoverMenuItemState, TitleBarStyle, View, ViewContext, div,
    menubar_key_bindings, popover_menu_key_bindings, text,
};

#[derive(Clone, Debug, PartialEq)]
enum Command {
    New,
    Open,
    Undo,
    Redo,
    ToggleSidebar(bool),
    Zoom,
}

fn main() -> Result<(), quickgui::AppError> {
    Application::new()
        .bind_keys(menubar_key_bindings())
        .bind_keys(popover_menu_key_bindings())
        .run(|cx| {
            cx.open_window(
                quickgui::WindowOptions::new("QuickGUI — In-window menubar")
                    .size(820.0, 520.0)
                    .title_bar_style(TitleBarStyle::HiddenInset)
                    .traffic_light_position(16.0, 13.0),
                MenubarGallery::new(),
            );
        })
}

#[derive(Clone, Copy)]
struct BarPalette {
    page: Color,
    bar: Color,
    surface: Color,
    foreground: Color,
    muted: Color,
    border: Color,
    highlight: Color,
    open: Color,
}

impl BarPalette {
    fn new(dark: bool) -> Self {
        if dark {
            Self {
                page: Color::rgb8(12, 15, 20),
                bar: Color::rgb8(18, 21, 27),
                surface: Color::rgb8(26, 30, 38),
                foreground: Color::rgb8(232, 234, 239),
                muted: Color::rgb8(143, 149, 162),
                border: Color::rgb8(61, 67, 79),
                highlight: Color::rgb8(31, 62, 98),
                open: Color::rgb8(38, 44, 55),
            }
        } else {
            Self {
                page: Color::rgb8(239, 241, 245),
                bar: Color::WHITE,
                surface: Color::WHITE,
                foreground: Color::rgb8(28, 31, 38),
                muted: Color::rgb8(102, 108, 120),
                border: Color::rgb8(211, 215, 222),
                highlight: Color::rgb8(220, 235, 252),
                open: Color::rgb8(233, 236, 241),
            }
        }
    }
}

const MENU_TITLES: [&str; 3] = ["File", "Edit", "View"];

struct MenubarGallery {
    menubar: MenubarState,
    file: PopoverMenu,
    edit: PopoverMenu,
    view: PopoverMenu,
    sidebar: bool,
    status: Arc<str>,
}

impl MenubarGallery {
    fn new() -> Self {
        Self {
            menubar: MenubarState::new(MENU_TITLES.len()),
            file: PopoverMenu::new([
                PopoverMenuItem::action("new", "New Window", Command::New).shortcut("⌘N"),
                PopoverMenuItem::action("open", "Open…", Command::Open).shortcut("⌘O"),
            ])
            .expect("the File menu is valid"),
            edit: PopoverMenu::new([
                PopoverMenuItem::action("undo", "Undo", Command::Undo).shortcut("⌘Z"),
                PopoverMenuItem::action("redo", "Redo", Command::Redo).shortcut("⇧⌘Z"),
            ])
            .expect("the Edit menu is valid"),
            view: PopoverMenu::new([
                PopoverMenuItem::checkbox_with(
                    "sidebar",
                    "Show Sidebar",
                    false,
                    Command::ToggleSidebar,
                ),
                PopoverMenuItem::separator(),
                PopoverMenuItem::action("zoom", "Actual Size", Command::Zoom),
            ])
            .expect("the View menu is valid"),
            sidebar: false,
            status: Arc::from("Tab focuses the bar; arrows move, Down opens, Escape closes"),
        }
    }

    fn menubar(view: &mut Self) -> &mut MenubarState {
        &mut view.menubar
    }
}

impl View for MenubarGallery {
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        let palette = BarPalette::new(cx.appearance().is_dark());

        let command = cx.action_listener(
            "menubar-gallery",
            |view: &mut Self, action: &Command, cx| {
                view.status = Arc::from(match action {
                    Command::New => "New Window",
                    Command::Open => "Open…",
                    Command::Undo => "Undo",
                    Command::Redo => "Redo",
                    Command::ToggleSidebar(_) => "Toggled the sidebar",
                    Command::Zoom => "Actual Size",
                });
                if let Command::ToggleSidebar(next) = action {
                    view.sidebar = *next;
                }
                view.menubar.close();
                cx.invalidate();
            },
        );

        let bar = Menubar::new("menubar");
        let mut bar_row = bar
            .root_with(self.menubar, div())
            .accessibility_label("Application menus")
            .flex_row()
            .items_center()
            .gap_1()
            .h(30.0)
            .px(8.0)
            .bg(palette.bar)
            .border(1.0, palette.border);
        for (index, title) in MENU_TITLES
            .iter()
            .enumerate()
            .take(self.menubar.menu_count())
        {
            let item = bar.item(self.menubar, index).expect("a declared menu");
            let open = item.is_open();
            let element = item.item_with(
                div()
                    .px(10.0)
                    .h(24.0)
                    .flex_row()
                    .items_center()
                    .rounded(5.0)
                    .bg(if open {
                        palette.open
                    } else {
                        Color::TRANSPARENT
                    })
                    .hover(move |hover| hover.bg(palette.open))
                    .child(text(*title).text_sm()),
            );
            bar_row = bar_row.child(item.key_with(cx, element, Self::menubar));
        }

        let surface = self.menubar.open_menu().map(|index| {
            let root = div()
                .absolute()
                .top(30.0)
                .left(8.0 + index as f32 * 78.0)
                .w(220.0)
                .flex_col()
                .p(4.0)
                .rounded(8.0)
                .border(1.0, palette.border)
                .bg(palette.surface)
                .shadow_lg();
            let render_item =
                move |item: &PopoverMenuItem, state: PopoverMenuItemState| -> Element {
                    if item.kind() == PopoverMenuItemKind::Separator {
                        return div().h(1.0).my(4.0).bg(palette.border);
                    }
                    div()
                        .h(26.0)
                        .px(8.0)
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .rounded(5.0)
                        .bg(if state.highlighted {
                            palette.highlight
                        } else {
                            Color::TRANSPARENT
                        })
                        .child(text(item.label().clone()).text_sm())
                        .child(
                            text(
                                item.shortcut_text()
                                    .cloned()
                                    .unwrap_or_else(|| Arc::from("")),
                            )
                            .text_xs()
                            .text_color(palette.muted),
                        )
                };
            let dismiss = |view: &mut Self, cx: &mut quickgui::EventContext| {
                view.menubar.close();
                cx.invalidate();
            };
            match index {
                0 => self.file.element(
                    cx,
                    "file-menu",
                    |view| &mut view.file,
                    root,
                    render_item,
                    dismiss,
                ),
                1 => self.edit.element(
                    cx,
                    "edit-menu",
                    |view| &mut view.edit,
                    root,
                    render_item,
                    dismiss,
                ),
                _ => self.view.element(
                    cx,
                    "view-menu",
                    |view| &mut view.view,
                    root,
                    render_item,
                    dismiss,
                ),
            }
        });

        let mut page = div()
            .id("menubar-gallery")
            .on_action(command)
            .size_full()
            .relative()
            .flex_col()
            .bg(palette.page)
            .text_color(palette.foreground)
            .child(bar_row)
            .child(
                div()
                    .flex_1()
                    .min_h(0.0)
                    .flex_col()
                    .gap_2()
                    .p(20.0)
                    .child(text("In-window menubar").text_lg().font_semibold())
                    .child(
                        text("The bar owns one Tab stop. Arrows move between menus and keep switching while one is open, hovering another title switches menus, and Escape closes without leaving the bar.")
                            .wrap()
                            .text_sm()
                            .text_color(palette.muted),
                    )
                    .child(
                        text(format!(
                            "Sidebar is {}",
                            if self.sidebar { "visible" } else { "hidden" }
                        ))
                        .text_sm(),
                    )
                    .child(text(self.status.clone()).text_xs().text_color(palette.muted)),
            );
        if let Some(surface) = surface {
            page = page.child(surface);
        }
        page
    }
}

/// Focused presentation of the same native example for browser documentation.
#[cfg(target_arch = "wasm32")]
pub fn docs_demo(component: String) -> impl quickgui::View {
    let _ = component;
    MenubarGallery::new()
}
