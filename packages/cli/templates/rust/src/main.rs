use quickgui::{
    App, AppInfo, Application, Color, EventContext, IntoElement, View, ViewContext, WindowOptions,
    div, text,
};

fn main() -> Result<(), quickgui::AppError> {
    Application::new()
        .app_info(AppInfo::new({{APP_NAME}}, "0.1.0", {{IDENTIFIER}}).expect("valid application identity"))
        .on_reopen(|has_visible_windows, cx| {
            if !has_visible_windows {
                open_window(cx);
            }
        })
        .run(open_window)
}

fn open_window(cx: &mut App) {
    cx.open_window(
        WindowOptions::new({{APP_NAME}}).size(760.0, 520.0),
        Counter { count: 0 },
    );
}

struct Counter {
    count: usize,
}

impl View for Counter {
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        let increment = cx.listener("increment", |this, cx: &mut EventContext| {
            this.count += 1;
            cx.invalidate();
        });

        div()
            .size_full()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_3()
            .bg(Color::rgb8(9, 13, 22))
            .text_color(Color::rgb8(226, 232, 240))
            .child(text("Fine-grained native UI").text_2xl().font_bold())
            .child(text(format!("Count: {}", self.count)))
            .child(
                div()
                    .on_click(increment)
                    .px_4()
                    .py_2()
                    .rounded_lg()
                    .bg(Color::rgb8(37, 99, 235))
                    .hover(|style| style.bg(Color::rgb8(59, 130, 246)))
                    .child("Increment"),
            )
    }
}
