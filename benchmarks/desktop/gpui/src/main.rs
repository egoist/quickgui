use gpui::{
    App, Application, Bounds, Context, FontWeight, KeyBinding, KeyDownEvent, TitlebarOptions,
    Window, WindowBounds, WindowOptions, actions, div, prelude::*, px, rgb, size, white,
};
use serde::Deserialize;

const PAGE_SIZE: usize = 100;
const READY_TITLE: &str = "Issue tracker — ready";

actions!(benchmark_gpui, [Quit]);

fn ink() -> gpui::Rgba {
    rgb(0x20242c)
}

fn muted() -> gpui::Rgba {
    rgb(0x737c8c)
}

fn line() -> gpui::Rgba {
    rgb(0xe2e5eb)
}

fn accent() -> gpui::Rgba {
    rgb(0x285bd4)
}

#[derive(Clone, Deserialize)]
struct IssueData {
    id: String,
    title: String,
    project: String,
    owner: String,
    priority: String,
    status: String,
    description: String,
    notes: String,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum IssueFilter {
    All,
    Open,
    Completed,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Field {
    None,
    Search,
    Notes,
}

impl IssueFilter {
    const ALL: [Self; 3] = [Self::All, Self::Open, Self::Completed];

    const fn label(self) -> &'static str {
        match self {
            Self::All => "All issues",
            Self::Open => "Open",
            Self::Completed => "Completed",
        }
    }

    const fn id(self) -> &'static str {
        match self {
            Self::All => "filter-all",
            Self::Open => "filter-open",
            Self::Completed => "filter-completed",
        }
    }

    fn includes(self, issue: &IssueData) -> bool {
        match self {
            Self::All => true,
            Self::Open => issue.status != "Done",
            Self::Completed => issue.status == "Done",
        }
    }
}

struct IssueTracker {
    issues: Vec<IssueData>,
    query: String,
    filter: IssueFilter,
    page: usize,
    selected: usize,
    field: Field,
}

impl IssueTracker {
    fn new() -> Self {
        let issues: Vec<IssueData> = serde_json::from_str(include_str!("../issues.generated.json"))
            .expect("the generated benchmark dataset is valid JSON");
        assert_eq!(issues.len(), 1_000, "expected 1,000 benchmark issues");
        Self {
            issues,
            query: String::new(),
            filter: IssueFilter::All,
            page: 0,
            selected: 0,
            field: Field::None,
        }
    }

    fn matching_indices(&self) -> Vec<usize> {
        let needle = self.query.trim().to_lowercase();
        self.issues
            .iter()
            .enumerate()
            .filter(|(_, issue)| self.filter.includes(issue))
            .filter(|(_, issue)| {
                needle.is_empty()
                    || issue.id.to_lowercase().contains(&needle)
                    || issue.title.to_lowercase().contains(&needle)
                    || issue.project.to_lowercase().contains(&needle)
                    || issue.owner.to_lowercase().contains(&needle)
            })
            .map(|(index, _)| index)
            .collect()
    }

    fn page_count(matches: usize) -> usize {
        matches.div_ceil(PAGE_SIZE).max(1)
    }

    fn caption(value: impl Into<String>) -> gpui::Div {
        div()
            .text_size(px(12.))
            .line_height(px(18.))
            .text_color(muted())
            .child(value.into())
    }

    fn property(label: &'static str, value: impl Into<String>) -> impl IntoElement {
        div()
            .flex()
            .flex_row()
            .items_center()
            .w_full()
            .justify_between()
            .child(Self::caption(label))
            .child(
                div()
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .child(value.into()),
            )
    }
}

impl Render for IssueTracker {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let matching = self.matching_indices();
        let pages = Self::page_count(matching.len());
        self.page = self.page.min(pages - 1);
        let visible = matching
            .iter()
            .copied()
            .skip(self.page * PAGE_SIZE)
            .take(PAGE_SIZE)
            .collect::<Vec<_>>();
        let completed = self
            .issues
            .iter()
            .filter(|issue| issue.status == "Done")
            .count();

        let filters = IssueFilter::ALL.map(|filter| {
            let selected = self.filter == filter;
            div()
                .id(filter.id())
                .flex()
                .flex_row()
                .items_center()
                .w_full()
                .h(px(42.))
                .flex_none()
                .px(px(12.))
                .justify_start()
                .rounded(px(7.))
                .font_weight(if selected {
                    FontWeight::SEMIBOLD
                } else {
                    FontWeight::NORMAL
                })
                .bg(if selected {
                    rgb(0xe4ebfb)
                } else {
                    rgb(0xf4f5f7)
                })
                .text_color(if selected { accent() } else { ink() })
                .cursor_pointer()
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.filter = filter;
                    this.page = 0;
                    this.field = Field::None;
                    cx.notify();
                }))
                .child(filter.label())
        });

        let sidebar = div()
            .flex()
            .flex_col()
            .min_h(px(0.))
            .min_w(px(0.))
            .w(px(176.))
            .h_full()
            .flex_none()
            .px(px(12.))
            .py(px(24.))
            .gap(px(8.))
            .bg(rgb(0xf4f5f7))
            .border_r_1()
            .border_color(line())
            .child(
                div()
                    .text_size(px(22.))
                    .font_weight(FontWeight::BOLD)
                    .px(px(12.))
                    .mb(px(6.))
                    .child("Orbit"),
            )
            .child(Self::caption("Product workspace").px(px(12.)).mb(px(24.)))
            .children(filters)
            .child(div().flex_1())
            .child(
                div()
                    .p(px(12.))
                    .flex()
                    .flex_col()
                    .child(Self::caption("September cycle"))
                    .child(Self::caption("4 projects · 5 teammates")),
            );

        let search_empty = self.query.is_empty();
        let header = div()
            .flex()
            .flex_row()
            .items_center()
            .min_w(px(0.))
            .h(px(94.))
            .flex_none()
            .px(px(24.))
            .justify_between()
            .border_b_1()
            .border_color(line())
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .child(
                        div()
                            .text_size(px(24.))
                            .font_weight(FontWeight::BOLD)
                            .child("Issue inbox"),
                    )
                    .child(Self::caption(format!(
                        "{} open · {completed} completed",
                        self.issues.len() - completed
                    ))),
            )
            .child(
                div()
                    .id("search")
                    .w(px(280.))
                    .h(px(38.))
                    .px(px(12.))
                    .flex()
                    .flex_row()
                    .items_center()
                    .bg(rgb(0xf8f9fb))
                    .border_1()
                    .border_color(rgb(0xdce0e7))
                    .rounded(px(7.))
                    .text_size(px(14.))
                    .text_color(if search_empty { muted() } else { ink() })
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.field = Field::Search;
                        cx.notify();
                    }))
                    .child(if search_empty {
                        "Search issues, projects, people".to_string()
                    } else {
                        self.query.clone()
                    }),
            );

        let mut rows = Vec::with_capacity(visible.len().max(1));
        if visible.is_empty() {
            rows.push(
                div()
                    .id("empty")
                    .p(px(20.))
                    .child(Self::caption("No matching issues"))
                    .into_any_element(),
            );
        } else {
            for index in visible {
                let issue = &self.issues[index];
                let title = issue.title.clone();
                let meta = format!(
                    "{}  ·  {}  ·  {}  ·  {}",
                    issue.id, issue.project, issue.status, issue.owner
                );
                let selected = self.selected == index;
                rows.push(
                    div()
                        .id(("issue", index))
                        .flex()
                        .flex_col()
                        .w_full()
                        .h(px(68.))
                        .flex_none()
                        .justify_center()
                        .gap(px(8.))
                        .px(px(20.))
                        .border_b_1()
                        .border_color(rgb(0xedf0f4))
                        .bg(if selected {
                            rgb(0xedf3ff)
                        } else {
                            rgb(0xffffff)
                        })
                        .cursor_pointer()
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.selected = index;
                            this.field = Field::None;
                            cx.notify();
                        }))
                        .child(
                            div()
                                .w_full()
                                .text_size(px(14.))
                                .font_weight(FontWeight::MEDIUM)
                                .whitespace_nowrap()
                                .overflow_hidden()
                                .text_ellipsis()
                                .child(title),
                        )
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(muted())
                                .whitespace_nowrap()
                                .child(meta),
                        )
                        .into_any_element(),
                );
            }
        }

        let inbox = div()
            .flex()
            .flex_col()
            .min_h(px(0.))
            .min_w(px(0.))
            .flex_1()
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .h(px(48.))
                    .flex_none()
                    .px(px(20.))
                    .justify_between()
                    .bg(rgb(0xfafbfc))
                    .border_b_1()
                    .border_color(line())
                    .child(Self::caption(format!("{} issues", matching.len())))
                    .child(Self::caption("Updated this week")),
            )
            .child(
                div()
                    .id("issues")
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h(px(0.))
                    .overflow_y_scroll()
                    .children(rows),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .h(px(58.))
                    .flex_none()
                    .px(px(20.))
                    .justify_between()
                    .border_t_1()
                    .border_color(line())
                    .child(Self::caption(format!("Page {} of {pages}", self.page + 1)))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(8.))
                            .child(page_control(
                                "previous-page",
                                "Previous",
                                self.page == 0,
                                cx.listener(|this, _, _, cx| {
                                    this.page = this.page.saturating_sub(1);
                                    this.field = Field::None;
                                    cx.notify();
                                }),
                            ))
                            .child(page_control(
                                "next-page",
                                "Next",
                                self.page + 1 >= pages,
                                cx.listener(|this, _, _, cx| {
                                    let pages = Self::page_count(this.matching_indices().len());
                                    this.page = (this.page + 1).min(pages - 1);
                                    this.field = Field::None;
                                    cx.notify();
                                }),
                            )),
                    ),
            );

        let current = &self.issues[self.selected];
        let complete_label = if current.status == "Done" {
            "Reopen issue"
        } else {
            "Mark complete"
        };
        let notes = current.notes.clone();
        let details = div()
            .id("details")
            .flex()
            .flex_col()
            .min_h(px(0.))
            .min_w(px(0.))
            .w(px(350.))
            .h_full()
            .flex_none()
            .overflow_y_scroll()
            .p(px(24.))
            .border_l_1()
            .border_color(line())
            .child(Self::caption(format!(
                "{} / {}",
                current.id, current.project
            )))
            .child(
                div()
                    .text_size(px(21.))
                    .line_height(px(28.))
                    .font_weight(FontWeight::BOLD)
                    .mt(px(14.))
                    .mb(px(22.))
                    .child(current.title.clone()),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(12.))
                    .mb(px(24.))
                    .child(Self::property("Status", current.status.clone()))
                    .child(Self::property("Assignee", current.owner.clone()))
                    .child(Self::property("Priority", current.priority.clone())),
            )
            .children(current.description.split("\n\n").map(|paragraph| {
                div()
                    .text_size(px(13.))
                    .line_height(px(20.))
                    .mb(px(12.))
                    .child(paragraph.replace('\n', " "))
            }))
            .child(
                div()
                    .text_size(px(12.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .mb(px(8.))
                    .mt(px(10.))
                    .child("Working notes"),
            )
            .child(
                div()
                    .id("working-notes")
                    .h(px(100.))
                    .flex_none()
                    .p(px(10.))
                    .text_size(px(13.))
                    .line_height(px(19.))
                    .bg(white())
                    .text_color(ink())
                    .border_1()
                    .border_color(rgb(0xdce0e7))
                    .rounded(px(7.))
                    .overflow_hidden()
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.field = Field::Notes;
                        cx.notify();
                    }))
                    .child(notes),
            )
            .child(
                div()
                    .id("toggle-complete")
                    .flex()
                    .flex_row()
                    .items_center()
                    .w_full()
                    .h(px(36.))
                    .flex_none()
                    .justify_center()
                    .rounded(px(7.))
                    .mt(px(16.))
                    .bg(accent())
                    .text_color(white())
                    .font_weight(FontWeight::MEDIUM)
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, _, cx| {
                        let status = &mut this.issues[this.selected].status;
                        *status = if status == "Done" {
                            "Open".to_owned()
                        } else {
                            "Done".to_owned()
                        };
                        this.field = Field::None;
                        cx.notify();
                    }))
                    .child(complete_label),
            )
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(muted())
                    .mt(px(10.))
                    .child("Changes are kept for this session."),
            );

        div()
            .id("orbit")
            .size_full()
            .flex()
            .flex_row()
            .bg(white())
            .text_color(ink())
            .text_size(px(14.))
            .on_key_down(cx.listener(Self::on_key_down))
            .child(sidebar)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .min_h(px(0.))
                    .min_w(px(0.))
                    .flex_1()
                    .h_full()
                    .child(header)
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .min_h(px(0.))
                            .min_w(px(0.))
                            .flex_1()
                            .child(inbox)
                            .child(details),
                    ),
            )
    }
}

impl IssueTracker {
    fn on_key_down(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.field == Field::None {
            return;
        }
        let modifiers = event.keystroke.modifiers;
        if modifiers.control || modifiers.alt || modifiers.platform || modifiers.function {
            return;
        }
        let key = event.keystroke.key.as_str();
        if key == "backspace" {
            match self.field {
                Field::Search => {
                    self.query.pop();
                    self.page = 0;
                }
                Field::Notes => {
                    self.issues[self.selected].notes.pop();
                }
                Field::None => {}
            }
            cx.notify();
            return;
        }
        if key == "enter" {
            if self.field == Field::Notes {
                self.issues[self.selected].notes.push('\n');
                cx.notify();
            }
            return;
        }
        if let Some(text) = event.keystroke.key_char.as_deref() {
            if text.chars().any(|ch| ch.is_control()) {
                return;
            }
            match self.field {
                Field::Search => {
                    self.query.push_str(text);
                    self.page = 0;
                }
                Field::Notes => self.issues[self.selected].notes.push_str(text),
                Field::None => {}
            }
            cx.notify();
        }
    }
}

fn page_control(
    id: &'static str,
    label: &'static str,
    disabled: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .flex()
        .flex_row()
        .items_center()
        .justify_center()
        .h(px(32.))
        .px(px(12.))
        .border_1()
        .border_color(rgb(0xdce0e7))
        .rounded(px(6.))
        .bg(white())
        .text_size(px(12.))
        .opacity(if disabled { 0.4 } else { 1. })
        .cursor_pointer()
        .on_click(move |event, window, cx| {
            if !disabled {
                on_click(event, window, cx);
            }
        })
        .child(label)
}

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1_100.), px(720.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some(READY_TITLE.into()),
                    appears_transparent: false,
                    traffic_light_position: None,
                }),
                window_min_size: Some(size(px(1_100.), px(720.))),
                app_id: Some("dev.quickgui.benchmark.gpui".into()),
                ..Default::default()
            },
            |window, cx| {
                window.set_window_title(READY_TITLE);
                cx.new(|_| IssueTracker::new())
            },
        )
        .unwrap();
        cx.activate(true);
        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
    });
}
