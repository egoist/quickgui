//! Optional editor components. All rendering uses the generic component SDK.
pub use quickgui_extension_sdk::builder::*;
use quickgui_extension_sdk::{
    self as sdk, Component, ComponentFactory, ComponentRuntime, WakeHandle, schema,
};
use serde_json::Value;
use std::ptr;
mod languages;
mod props;

pub const MAX_TEXT_HIGHLIGHTS: usize = 4096;
pub const MAX_LIST_ITEMS: usize = 1_000_000;
#[path = "../../../src/syntax.rs"]
mod syntax;
pub use syntax::*;
#[path = "../../../src/text_input_decorations.rs"]
mod text_input_decorations;
pub use text_input_decorations::{TextInputGutter, TextInputIndentation};
#[path = "../../../src/code_block.rs"]
mod code_block;
#[path = "../../../src/diff_view.rs"]
mod diff_view;
#[path = "../../../src/editor.rs"]
mod editor;

trait InputDecorations {
    fn set_input_decorations(&mut self, gutter: TextInputGutter, behavior: TextInputIndentation);
}
impl InputDecorations for Element {
    fn set_input_decorations(&mut self, g: TextInputGutter, b: TextInputIndentation) {
        let text_checking = self
            .node
            .input
            .as_ref()
            .map(|input| input.text_checking.clone())
            .unwrap_or_default();
        self.node.input = Some(schema::InputOptions {
            text_checking,
            read_only: b.read_only,
            multiline: true,
            tab_size: Some(b.tab_size),
            insert_spaces: b.insert_spaces,
            auto_indent: b.auto_indent,
            gutter: Some(schema::Gutter {
                line_numbers: g.line_numbers,
                minimum_digits: g.minimum_line_number_digits,
                background: g.gutter_background.paint(),
                foreground: g.gutter_foreground.map(Color::paint),
                active_foreground: g.gutter_active_foreground.map(Color::paint),
                border: g.gutter_border.paint(),
                active_line_background: g.active_line_background.paint(),
                content_padding: [
                    g.content_padding_y,
                    g.content_padding_right,
                    g.content_padding_y,
                    g.content_padding_left,
                ],
                padding_left: g.gutter_padding_left,
                padding_right: g.gutter_padding_right,
            }),
        });
    }
}

enum Model {
    Editor(editor::Editor),
    Code(code_block::CodeBlock),
    Diff(diff_view::DiffView),
}
struct Surface {
    model: Model,
    props: Value,
    renderers: Renderers,
    generation: u64,
    _wake: std::sync::Arc<WakeHandle>,
}
pub struct Factory;
impl ComponentFactory for Factory {
    fn create(name: &str, props: Value, wake: WakeHandle) -> Result<Box<dyn Component>, String> {
        let model = match name {
            "editor" => Model::Editor(editor::Editor::default()),
            "code-block" => Model::Code(code_block::CodeBlock::default()),
            "diff-view" => {
                Model::Diff(diff_view::DiffView::new(diff_view::DiffDocument::default()))
            }
            _ => return Err(format!("unknown editor component {name}")),
        };
        let mut surface = Surface {
            model,
            props: Value::Null,
            renderers: Renderers::default(),
            generation: 0,
            _wake: languages::attach(wake)?,
        };
        surface.update(props)?;
        Ok(Box::new(surface))
    }
}
impl Component for Surface {
    fn update(&mut self, props: Value) -> Result<bool, String> {
        let generation = syntax::syntax_language_generation();
        if props == self.props && self.generation == generation {
            return Ok(false);
        }
        let source = props.get("value").and_then(Value::as_str).unwrap_or("");
        let language = props
            .get("language")
            .and_then(Value::as_str)
            .and_then(SyntaxLanguage::from_name)
            .unwrap_or_default();
        match &mut self.model {
            Model::Editor(editor) => {
                editor.set_text(source);
                editor.set_language(language);
                editor.set_style(props::editor_style(&props));
                editor.set_behavior(props::behavior(&props));
            }
            Model::Code(block) => {
                block.set_text(source);
                block.set_language(language);
                block.set_style(props::code_style(&props));
            }
            Model::Diff(diff) => {
                let source_changed = ["patch", "oldPath", "oldText", "newPath", "newText"]
                    .iter()
                    .any(|key| props[*key] != self.props[*key]);
                if source_changed {
                    let document = if let Some(patch) = props.get("patch").and_then(Value::as_str) {
                        diff_view::DiffDocument::from_patch(patch)
                    } else {
                        diff_view::DiffDocument::from_texts(
                            props
                                .get("oldPath")
                                .and_then(Value::as_str)
                                .unwrap_or("before"),
                            props.get("oldText").and_then(Value::as_str).unwrap_or(""),
                            props
                                .get("newPath")
                                .and_then(Value::as_str)
                                .unwrap_or("after"),
                            props.get("newText").and_then(Value::as_str).unwrap_or(""),
                        )
                    };
                    diff.set_document(document);
                }
                diff.set_style(props::diff_style(&props));
            }
        }
        match &mut self.model {
            Model::Editor(model) => {
                model.refresh_syntax_languages();
            }
            Model::Code(model) => {
                model.refresh_syntax_languages();
            }
            Model::Diff(model) => {
                model.refresh_syntax_languages();
            }
        }
        self.generation = generation;
        self.props = props;
        Ok(true)
    }
    fn render(&mut self, request: schema::RenderRequest) -> Result<schema::Frame, String> {
        if request.renderer.is_some() {
            return self.renderers.rows(&request);
        }
        if self.generation != syntax::syntax_language_generation() {
            self.update(self.props.clone())?;
        }
        Ok(self.renderers.root(|| match &self.model {
            Model::Editor(model) => {
                let mut node = model.element(ElementId::new(0));
                node.node.events.push("input".into());
                node
            }
            Model::Code(model) => model.element(ElementId::new(0)),
            Model::Diff(model) => model.element(ElementId::new(0)),
        }))
    }
    fn event(&mut self, event: schema::InputEvent) -> Result<schema::EventResult, String> {
        if event.kind == "input" {
            if let Model::Editor(model) = &mut self.model {
                model.set_text(event.value.as_str().unwrap_or(""));
            }
            return Ok(schema::EventResult {
                changed: true,
                events: vec![schema::OutputEvent {
                    kind: "input".into(),
                    value: event.value,
                }],
                ..Default::default()
            });
        }
        Ok(Default::default())
    }
}
static API: sdk::abi::PackageApi = sdk::abi::PackageApi {
    component: ComponentRuntime::<Factory>::API,
    service: languages::API,
};
static DESCRIPTOR: sdk::abi::Extension = sdk::abi::Extension {
    abi_version: sdk::abi::ABI_VERSION,
    descriptor_size: size_of::<sdk::abi::Extension>() as u32,
    kind: sdk::abi::PACKAGE_EXTENSION,
    api_size: size_of::<sdk::abi::PackageApi>() as u32,
    name: sdk::abi::Bytes::new(b"editor"),
    version: sdk::abi::Bytes::new(env!("CARGO_PKG_VERSION").as_bytes()),
    api: ptr::addr_of!(API).cast(),
};
#[unsafe(no_mangle)]
pub extern "C" fn quickgui_extension_v1() -> *const sdk::abi::Extension {
    ptr::addr_of!(DESCRIPTOR)
}
