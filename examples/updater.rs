//! Automatic updates in a Rust application. `cargo run --example updater --features updater`
//! reports `disabled`, as every development build does; `quickgui build` embeds `[updates]`.

use std::rc::Rc;

use quickgui::updater::{Event, UpdateStatus, Updater};
use quickgui::{
    Application, AsyncContextError, AsyncViewContext, Color, EventContext, Task, View, ViewContext,
    WindowOptions, button, div, text,
};

fn main() -> Result<(), quickgui::AppError> {
    Application::new().run(|cx| {
        cx.open_window(
            WindowOptions::new("QuickGUI — Updater")
                .size(560.0, 320.0)
                .background(Color::rgb8(16, 18, 23)),
            UpdaterDemo::default(),
        );
    })
}

#[derive(Default)]
struct UpdaterDemo {
    // The updater belongs to the application. A real app keeps it in a global, not in a window.
    updater: Option<Rc<Updater>>,
    events: Option<Task<Result<(), AsyncContextError>>>,
    state: Event,
    message: String,
}

impl UpdaterDemo {
    fn start(&mut self, cx: &mut EventContext) {
        if self.events.is_some() {
            return;
        }
        let task = cx.spawn(|task_cx: AsyncViewContext<Self>| async move {
            let (updater, mut events) = match Updater::start(quickgui::updater_options!()).await {
                Ok(started) => started,
                Err(error) => {
                    return task_cx
                        .update(move |this, cx| {
                            this.message = error.to_string();
                            cx.invalidate();
                        })
                        .await;
                }
            };
            let updater = Rc::new(updater);
            task_cx
                .update(move |this, _| this.updater = Some(updater))
                .await?;
            while let Some(event) = events.next().await {
                task_cx
                    .update(move |this, cx| {
                        if event.quit_required {
                            // The verified installer waits for the ordinary quit lifecycle.
                            cx.exit();
                        }
                        this.state = event;
                        cx.invalidate();
                    })
                    .await?;
            }
            Ok(())
        });
        match task {
            Ok(task) => self.events = Some(task),
            Err(error) => self.message = format!("Could not start: {error}"),
        }
    }

    fn command(&mut self, cx: &mut EventContext, install: bool) {
        let Some(updater) = self.updater.clone() else {
            return;
        };
        let task = cx.spawn(|task_cx: AsyncViewContext<Self>| async move {
            let result = if install {
                updater.install().await
            } else {
                updater.check().await
            };
            task_cx
                .update(move |this, cx| {
                    this.message = result.err().map(|e| e.to_string()).unwrap_or_default();
                    cx.invalidate();
                })
                .await
        });
        if let Ok(task) = task {
            task.detach();
        }
    }
}

impl View for UpdaterDemo {
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl quickgui::IntoElement {
        let start = cx.listener("start", |this, cx| this.start(cx));
        let check = cx.listener("check", |this, cx| this.command(cx, false));
        let install = cx.listener("install", |this, cx| this.command(cx, true));
        let status = match self.state.status {
            UpdateStatus::Available => format!("Version {} is available", self.state.version),
            UpdateStatus::Downloading => format!(
                "Downloading {} of {} bytes",
                self.state.downloaded_bytes, self.state.total_bytes
            ),
            status => format!("{status:?}"),
        };
        let detail = if self.state.error.is_empty() {
            self.message.clone()
        } else {
            self.state.error.clone()
        };
        let control = |label: &'static str| {
            button()
                .px_4()
                .py_2()
                .rounded_lg()
                .bg(Color::rgb8(45, 55, 72))
                .child(text(label).font_medium())
        };
        div()
            .size_full()
            .p_6()
            .flex_col()
            .gap_4()
            .text_color(Color::rgb8(235, 238, 244))
            .child(text("Automatic updates").text_2xl().font_bold())
            .child(text(status))
            .child(
                text(detail)
                    .wrap()
                    .text_sm()
                    .text_color(Color::rgb8(151, 160, 178)),
            )
            .child(
                div()
                    .flex_row()
                    .gap_2()
                    .child(control("Start").on_click(start))
                    .child(control("Check for updates").on_click(check))
                    .child(control("Install").on_click(install)),
            )
    }
}
