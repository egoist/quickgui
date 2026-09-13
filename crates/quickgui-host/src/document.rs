use super::*;
use quickgui::{
    Element, ElementId,
    document::{CodeDocument, DiffDocument, theme::Theme},
};
use serde::Deserialize;
use std::collections::HashSet;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Declaration {
    kind: String,
    #[serde(default)]
    source: String,
    language: Option<String>,
    path: Option<String>,
    #[serde(default)]
    show_line_numbers: bool,
    #[serde(default)]
    word_diff: bool,
    #[serde(default)]
    scroll: bool,
    max_lines: Option<usize>,
    #[serde(default)]
    collapsed_files: HashSet<String>,
    theme: Option<serde_json::Value>,
    #[serde(default)]
    events: HashSet<String>,
}
pub(super) fn validate(raw: &str) -> bool {
    raw.len() <= 4 * 1024 * 1024
        && serde_json::from_str::<Declaration>(raw).is_ok_and(|v| {
            matches!(v.kind.as_str(), "code" | "diff")
                && v.source.lines().count() <= 32768
                && v.collapsed_files.len() <= 4096
                && v.theme.as_ref().is_none_or(validate_theme_value)
        })
}
#[derive(Default)]
pub(super) struct NativeDocument {
    raw: String,
    declaration: Option<Declaration>,
    code: CodeDocument,
    diff: DiffDocument,
    theme: Theme,
}
impl NativeDocument {
    pub(super) fn element(
        &mut self,
        node: &NativeNode,
        id: ElementId,
        window: u32,
        target: u32,
        events: &EventQueue,
        cx: &mut ViewContext<NativeView>,
    ) -> Element {
        let raw = node.string(property::RICH_DOCUMENT).unwrap_or_default();
        if self.raw != raw {
            self.raw = raw.to_owned();
            self.declaration = serde_json::from_str(raw).ok();
            if let Some(d) = &self.declaration {
                self.theme = Theme::from_prop(d.theme.as_ref());
                if d.kind == "code" {
                    self.code
                        .set_source(&d.source, d.language.as_deref(), d.path.as_deref());
                } else {
                    self.diff.set_source(&d.source);
                }
            }
        }
        let d = self
            .declaration
            .as_ref()
            .expect("validated native document");
        if d.kind == "code" {
            let size = node
                .number(property::FONT_SIZE)
                .unwrap_or(self.theme.metrics.code_text_size);
            let line_height = node.number(property::LINE_HEIGHT).unwrap_or(
                self.theme.metrics.code_line_height * size
                    / self.theme.metrics.code_text_size.max(1.),
            );
            self.code
                .element(id, &self.theme, d.show_line_numbers, size, line_height)
        } else {
            self.diff.set_scale_factor(cx.scale_factor());
            self.diff.element(id,&self.theme,&d.collapsed_files,d.max_lines,d.word_diff,d.scroll, |row_id,element,action| {
            use quickgui::document::DiffAction;
            let (name,data) = match action {
                DiffAction::ShowMore(remaining) => ("showMore",serde_json::json!({"gpuixEvent":"showMore","value":remaining.to_string()})),
                DiffAction::ToggleFile(path) => ("toggleFile",serde_json::json!({"gpuixEvent":"toggleFile","value":path})),
                DiffAction::LineClick{text,old_line,new_line} => ("lineClick",serde_json::json!({"gpuixEvent":"lineClick","value":text,"oldLine":old_line,"newLine":new_line})),
            };
            if !d.events.contains(name) { return element; }
            let events = Rc::clone(events);
            let value: Arc<str> = Arc::from(data.to_string());
            element.on_click(cx.listener(row_id,move |_view,_cx| {
                enqueue_event(&events,QueuedEvent{kind:"click",window,target,value:Some(Arc::clone(&value))});
            }))
        })
        }
    }
}

pub(super) fn validate_theme(raw: &str) -> bool {
    raw.len() <= 16384
        && serde_json::from_str::<serde_json::Value>(raw)
            .is_ok_and(|value| validate_theme_value(&value))
}
fn validate_theme_value(value: &serde_json::Value) -> bool {
    use serde_json::Value;
    if serde_json::from_value::<quickgui::document::theme::ThemeOverride>(value.clone()).is_err() {
        return false;
    }
    for name in ["fontSans", "fontMono"] {
        if value
            .get(name)
            .and_then(Value::as_str)
            .is_some_and(|s| s.len() > 1024)
        {
            return false;
        }
    }
    let Some(metrics) = value.get("metrics").and_then(Value::as_object) else {
        return true;
    };
    metrics.iter().all(|(name, value)| {
        let positive = name.ends_with("Height")
            || name.ends_with("Size")
            || name == "diffBodyBottomPad"
            || name.starts_with("mdHeading");
        let valid = |value: &Value| {
            value.is_null()
                || value.as_f64().is_some_and(|v| {
                    v.is_finite() && v <= 4096. && if positive { v > 0. } else { v >= 0. }
                })
        };
        match value {
            Value::Array(values) => values.len() <= 4 && values.iter().all(valid),
            _ => valid(value),
        }
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn document_metrics_cannot_create_invalid_virtual_rows() {
        for height in ["0", "-1", "1e100"] {
            assert!(!validate(&format!(
                r#"{{"kind":"diff","theme":{{"metrics":{{"diffLineHeight":{height}}}}}}}"#
            )));
        }
        assert!(validate(
            r#"{"kind":"diff","theme":{"metrics":{"diffLineHeight":22,"diffRowPaddingX":0}}}"#
        ));
        assert!(!validate_theme(r#"{"metrics":{"mdHeadingSizes":[12,-1]}}"#));
    }
}
