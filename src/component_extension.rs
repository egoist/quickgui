//! Generic component interpreter. Package identities, component props, and algorithms belong to
//! loaded extensions; this module only understands ordinary renderer primitives and lifecycle.
use crate::extension_api::{self as abi, ComponentApi};
use crate::{
    Color, Element, ElementId, EventContext, FontFamily, FontWeight, HighlightStyle, IntoElement,
    LayoutBoundsHandle, ListState, StyledText, ViewContext, WindowInvalidator, div,
};
use quickgui_extension_sdk::schema as wire;
use serde_json::{Value, json};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    ffi::c_void,
    rc::Rc,
};

type Output = Rc<dyn Fn(&str, &Value)>;

struct Instance {
    api: ComponentApi,
    handle: *mut c_void,
    props: Vec<u8>,
    lists: HashMap<u64, (ListState, u64, u64, f32)>,
    nested: HashMap<u64, (wire::ComponentRef, ExtensionComponent)>,
    bounds: LayoutBoundsHandle,
}
impl Drop for Instance {
    fn drop(&mut self) {
        unsafe { (self.api.destroy)(self.handle) };
    }
}

/// A retained instance of any registered native component package.
#[derive(Clone)]
pub struct ExtensionComponent {
    instance: Rc<RefCell<Instance>>,
    id: ElementId,
    wake: WindowInvalidator,
    output: Output,
}

struct WakeContext {
    wake: WindowInvalidator,
    id: ElementId,
}
unsafe extern "C" fn wake(context: *mut c_void) {
    let context = unsafe { &*context.cast::<WakeContext>() };
    context.wake.invalidate_element(context.id);
}
unsafe extern "C" fn release(context: *mut c_void) {
    drop(unsafe { Box::from_raw(context.cast::<WakeContext>()) });
}

#[derive(Default)]
struct Reply {
    bytes: Vec<u8>,
    received: bool,
    invalid: bool,
}
unsafe extern "C" fn receive(context: *mut c_void, bytes: abi::Bytes) {
    let target = unsafe { &mut *context.cast::<Reply>() };
    if target.received
        || bytes.len > wire::MAX_COMPONENT_BYTES
        || (bytes.len > 0 && bytes.data.is_null())
    {
        target.invalid = true;
        return;
    }
    target.received = true;
    if bytes.len > 0 {
        target
            .bytes
            .extend_from_slice(unsafe { std::slice::from_raw_parts(bytes.data, bytes.len) });
    }
}

impl ExtensionComponent {
    pub fn new(
        package: &str,
        name: &str,
        props: Value,
        id: ElementId,
        invalidator: WindowInvalidator,
        output: impl Fn(&str, &Value) + 'static,
    ) -> Result<Self, String> {
        let api = crate::extensions::component(package)
            .ok_or_else(|| format!("component extension {package} is not loaded"))?;
        let props = serde_json::to_vec(&props).map_err(|error| error.to_string())?;
        if props.len() > wire::MAX_COMPONENT_BYTES || name.len() > 256 {
            return Err("component properties are too large".into());
        }
        let wake_context = Box::into_raw(Box::new(WakeContext {
            wake: invalidator.clone(),
            id,
        }))
        .cast();
        let mut reply = Reply::default();
        let handle = unsafe {
            (api.create)(
                abi::Bytes::new(name.as_bytes()),
                abi::Bytes::new(&props),
                abi::Wake {
                    context: wake_context,
                    wake,
                    release,
                },
                (&mut reply as *mut Reply).cast(),
                receive,
            )
        };
        if handle.is_null() {
            return Err(if reply.bytes.is_empty() {
                "component creation failed".into()
            } else {
                String::from_utf8_lossy(&reply.bytes).into_owned()
            });
        }
        Ok(Self {
            instance: Rc::new(RefCell::new(Instance {
                api,
                handle,
                props,
                lists: HashMap::new(),
                nested: HashMap::new(),
                bounds: LayoutBoundsHandle::new(),
            })),
            id,
            wake: invalidator,
            output: Rc::new(output),
        })
    }

    pub fn set_props(&self, props: &Value) -> Result<bool, String> {
        let bytes = serde_json::to_vec(props).map_err(|error| error.to_string())?;
        if bytes.len() > wire::MAX_COMPONENT_BYTES {
            return Err("component properties are too large".into());
        }
        let mut instance = self.instance.borrow_mut();
        if instance.props == bytes {
            return Ok(false);
        }
        let status = unsafe { (instance.api.update)(instance.handle, abi::Bytes::new(&bytes)) };
        if status < 0 {
            return Err("component rejected its properties".into());
        }
        instance.props = bytes;
        Ok(status > 0)
    }

    fn request<T: serde::de::DeserializeOwned>(
        &self,
        value: &impl serde::Serialize,
        event: bool,
    ) -> Result<T, String> {
        let bytes = serde_json::to_vec(value).map_err(|error| error.to_string())?;
        if bytes.len() > wire::MAX_COMPONENT_BYTES {
            return Err("component request is too large".into());
        }
        let mut reply = Reply::default();
        let instance = self.instance.borrow();
        let callback = if event {
            instance.api.event
        } else {
            instance.api.render
        };
        let status = unsafe {
            callback(
                instance.handle,
                abi::Bytes::new(&bytes),
                (&mut reply as *mut Reply).cast(),
                receive,
            )
        };
        if status != 0 || !reply.received || reply.invalid {
            return Err("component returned an invalid reply".into());
        }
        serde_json::from_slice(&reply.bytes).map_err(|error| error.to_string())
    }

    pub fn element<V: 'static>(&self, cx: &mut ViewContext<'_, V>) -> Result<Element, String> {
        let bounds = self.instance.borrow().bounds.clone();
        let size = bounds
            .bounds()
            .map(|bounds| crate::Size::new(bounds.width, bounds.height))
            .unwrap_or_default();
        let focus = cx.focus_handle(self.id);
        let request = wire::RenderRequest {
            width: size.width,
            height: size.height,
            scale: cx.scale_factor(),
            focused: cx.is_focused(focus) && cx.window_state().focused,
            ..Default::default()
        };
        let frame: wire::Frame = self.request(&request, false)?;
        frame.validate().map_err(str::to_owned)?;
        if let Some(delay) = frame.repaint_after_ms {
            cx.request_repaint_at(
                web_time::Instant::now() + web_time::Duration::from_millis(delay.clamp(1, 60_000)),
            );
        }
        for event in &frame.events {
            (self.output)(&event.kind, &event.value);
        }
        if frame.nodes.len() != 1 {
            return Err("a component must render exactly one root".into());
        }
        let mut live = HashSet::new();
        let mut nested = HashSet::new();
        let element = self.build(&frame.nodes[0], cx, &mut live, &mut nested, 0)?;
        let mut instance = self.instance.borrow_mut();
        instance.lists.retain(|key, _| live.contains(key));
        instance.nested.retain(|key, _| nested.contains(key));
        Ok(element.id(self.id).report_bounds(bounds))
    }

    /// Resolve an extension-local key for focus, geometry queries, and integration tests.
    pub fn element_id(&self, key: u64) -> ElementId {
        if key == 0 {
            return self.id;
        }
        ElementId::new(
            (self.id.as_u64().wrapping_mul(0x9e3779b97f4a7c15)
                ^ key.rotate_left(23)
                ^ 0x455854434f4d50)
                | (1 << 63),
        )
    }

    fn build<V: 'static>(
        &self,
        node: &wire::Node,
        cx: &mut ViewContext<'_, V>,
        live: &mut HashSet<u64>,
        nested: &mut HashSet<u64>,
        depth: usize,
    ) -> Result<Element, String> {
        if depth > wire::MAX_COMPONENT_DEPTH {
            return Err("component nesting is too deep".into());
        }
        let id = self.element_id(node.key);
        let styled = || {
            StyledText::new(node.text.clone()).with_highlights(node.spans.iter().map(|span| {
                let mut style = HighlightStyle::default();
                if let Some(value) = span.foreground {
                    style = style.color(color(value));
                }
                if let Some(value) = span.background {
                    style = style.background(color(value));
                }
                if let Some(value) = &span.font_family {
                    style = style.font_family(if value == "monospace" {
                        FontFamily::Monospace
                    } else {
                        FontFamily::from(value.clone())
                    });
                }
                if let Some(value) = span.underline_color {
                    style = style.underline_color(color(value));
                }
                if let Some(value) = span.weight {
                    style = style.font_weight(FontWeight(value));
                }
                if span.italic {
                    style = style.italic();
                }
                if span.underline {
                    style = style.underline();
                }
                if span.strikethrough {
                    style = style.strikethrough();
                }
                (span.start..span.end, style)
            }))
        };
        let mut element = match node.kind {
            wire::Primitive::View => div(),
            wire::Primitive::Button => crate::button(),
            wire::Primitive::Text => styled().into_element(),
            wire::Primitive::Input => {
                let options = node.input.clone().unwrap_or_default();
                let mut input = if options.multiline {
                    crate::styled_text_area(styled())
                } else {
                    crate::styled_text_input(styled())
                };
                #[cfg(feature = "text-input-decorations")]
                if let crate::element::ElementKind::TextInput(state) = &mut input.kind {
                    if options.tab_size.is_some() || options.read_only {
                        state.constraints.editor = Some(
                            crate::TextInputIndentation {
                                tab_size: options.tab_size.unwrap_or(4),
                                insert_spaces: options.insert_spaces,
                                auto_indent: options.auto_indent,
                                read_only: options.read_only,
                            }
                            .sanitized(),
                        );
                    }
                    state.editor = options.gutter.as_ref().map(|gutter| {
                        crate::TextInputGutter {
                            line_numbers: gutter.line_numbers,
                            minimum_line_number_digits: gutter.minimum_digits,
                            gutter_background: color(gutter.background),
                            gutter_foreground: gutter.foreground.map(color),
                            gutter_active_foreground: gutter.active_foreground.map(color),
                            gutter_border: color(gutter.border),
                            active_line_background: color(gutter.active_line_background),
                            content_padding_left: gutter.content_padding[3],
                            content_padding_right: gutter.content_padding[1],
                            content_padding_y: gutter.content_padding[0],
                            gutter_padding_left: gutter.padding_left,
                            gutter_padding_right: gutter.padding_right,
                        }
                        .sanitized()
                    });
                }
                apply_input_text_checking(input, &options.text_checking)
                    .focus(|_| crate::ElementStateStyle::default())
                    .accessibility_read_only(options.read_only)
            }
            wire::Primitive::Svg => {
                let svg = crate::Svg::from_svg(&node.text).map_err(|error| error.to_string())?;
                crate::svg(svg)
            }
            wire::Primitive::Canvas => {
                let rectangles = node.rectangles.clone();
                crate::canvas(move |_, canvas| {
                    for rectangle in &rectangles {
                        let b = rectangle.bounds;
                        canvas.fill_rect(
                            crate::Rect::new(b[0], b[1], b[2], b[3]),
                            color(rectangle.color),
                        );
                    }
                })
            }
            wire::Primitive::Component => {
                let reference = node
                    .component
                    .as_ref()
                    .ok_or("missing component reference")?;
                nested.insert(node.key);
                let mut instance = self.instance.borrow_mut();
                let replace = instance.nested.get(&node.key).is_none_or(|(previous, _)| {
                    previous.package != reference.package || previous.name != reference.name
                });
                if replace {
                    let output = self.output.clone();
                    let component = Self::new(
                        &reference.package,
                        &reference.name,
                        reference.props.clone(),
                        id,
                        self.wake.clone(),
                        move |kind, value| output(kind, value),
                    )?;
                    instance
                        .nested
                        .insert(node.key, (reference.clone(), component));
                }
                let component = instance.nested.get(&node.key).unwrap().1.clone();
                drop(instance);
                component.set_props(&reference.props)?;
                component.element(cx)?
            }
        }
        .id(id);
        for style in &node.styles {
            element = apply_style(element, style);
        }
        if !node.focus_styles.is_empty() {
            let styles = node.focus_styles.clone();
            element = element.focus(move |_| {
                let mut state = crate::ElementStateStyle::default();
                for style in &styles {
                    match style {
                        wire::Style::Border(widths, paint) => {
                            state = state.border(widths[0], color(*paint))
                        }
                        wire::Style::Background(paint) => state = state.bg(color(*paint)),
                        wire::Style::Foreground(paint) => state = state.text_color(color(*paint)),
                        _ => {}
                    }
                }
                state
            });
        }
        if let Some(spec) = &node.list {
            live.insert(spec.key);
            let list = {
                let mut instance = self.instance.borrow_mut();
                let (list, generation, measured, estimate) =
                    instance.lists.entry(spec.key).or_insert_with(|| {
                        (
                            ListState::new(spec.count, spec.estimate).with_overscan(spec.overscan),
                            spec.generation,
                            spec.measurement_generation,
                            spec.estimate,
                        )
                    });
                if *generation != spec.generation {
                    list.reset(spec.count);
                    *generation = spec.generation;
                } else {
                    list.set_item_count(spec.count);
                }
                if *estimate != spec.estimate {
                    list.set_estimated_item_height(spec.estimate);
                    *estimate = spec.estimate;
                }
                if *measured != spec.measurement_generation {
                    list.remeasure();
                    *measured = spec.measurement_generation;
                }
                list.clone()
            };
            let range = list.visible_rows().range;
            let frame: wire::Frame = self.request(
                &wire::RenderRequest {
                    renderer: Some(spec.renderer),
                    start: range.start,
                    end: range.end,
                    ..Default::default()
                },
                false,
            )?;
            frame.validate().map_err(str::to_owned)?;
            if frame.nodes.len() != range.len() {
                return Err("component returned the wrong number of virtual rows".into());
            }
            let mut rows = Vec::with_capacity(frame.nodes.len());
            for row in &frame.nodes {
                rows.push(self.build(row, cx, live, nested, depth + 1)?);
            }
            let mut rows = rows.into_iter();
            let mut column = if spec.mirror {
                list.render_mirrored_rows(spec.renderer, |_| rows.next().unwrap())
            } else {
                list.render_rows(range, |_| rows.next().unwrap())
            };
            for style in &spec.row_styles {
                column = apply_style(column, style);
            }
            element = element.child(column);
            element = if spec.mirror {
                element.variable_virtual_scroll_mirror(&list)
            } else {
                element.variable_virtual_scroll(&list)
            };
        }
        for child in &node.children {
            element = element.child(self.build(child, cx, live, nested, depth + 1)?);
        }
        for kind in &node.events {
            let component = self.clone();
            let target = node.key;
            match kind.as_str() {
                "input" => element = element.on_input(cx.input_listener(id, move |_, value, cx| component.dispatch(target, "input", json!(value), cx))),
                "click" => element = element.on_click(cx.listener(id, move |_, cx| component.dispatch(target, "click", Value::Null, cx))),
                "key-down" => element = element.on_key_down(cx.key_down_listener(id, move |_, event, cx| component.dispatch(target, "key-down", json!({"key": key(&event.key), "text": event.text, "modifiers": event.modifiers.bits(), "repeat": event.repeat}), cx))),
                "key-up" => element = element.on_key_up(cx.key_up_listener(id, move |_, event, cx| component.dispatch(target, "key-up", json!({"key": key(&event.key), "modifiers": event.modifiers.bits()}), cx))),
                "pointer" => element = element.on_pointer(cx.pointer_listener(id, move |_, event, cx| component.dispatch(target, "pointer", json!({"phase": format!("{:?}", event.phase).to_lowercase(), "x": event.local_position.x, "y": event.local_position.y, "width": event.size.width, "height": event.size.height, "button": format!("{:?}", event.button).to_lowercase(), "modifiers": event.modifiers.bits()}), cx))),
                "wheel" => element = element.on_scroll_wheel(cx.scroll_wheel_listener(id, move |_, event, cx| {
                    let delta = event.delta.pixel_delta(20.0);
                    component.dispatch(target, "wheel", json!({"x":delta.x,"y":delta.y,"phase":format!("{:?}",event.phase).to_lowercase(),"modifiers":event.modifiers.bits()}), cx)
                })),
                _ => return Err(format!("unknown component event {kind}")),
            }
        }
        Ok(element)
    }

    fn dispatch(&self, target: u64, kind: &str, value: Value, cx: &mut EventContext) {
        let result: Result<wire::EventResult, _> = self.request(
            &wire::InputEvent {
                target,
                kind: kind.to_owned(),
                value,
            },
            true,
        );
        let Ok(result) = result else {
            return;
        };
        if result.prevent_default {
            cx.prevent_default();
        }
        if result.stop_propagation {
            cx.stop_propagation();
        }
        if let Some(value) = result.clipboard
            && let Ok(item) = crate::ClipboardItem::new_string(value)
        {
            let _ = cx.write_to_clipboard(item);
        }
        if result.read_clipboard
            && let Ok(Some(item)) = cx.read_from_clipboard()
            && let Some(text) = item.text()
        {
            self.dispatch(target, "paste", json!(text), cx);
        }
        for event in result.events {
            (self.output)(&event.kind, &event.value);
        }
        if result.changed && !self.wake.invalidate_element(self.id) {
            cx.invalidate();
        }
    }
}

fn key(key: &crate::Key) -> String {
    match key {
        crate::Key::Character(value) => value.clone(),
        _ => format!("{key:?}"),
    }
}
fn color(value: wire::Paint) -> Color {
    Color::linear(value[0], value[1], value[2], value[3])
}

fn apply_input_text_checking(mut input: Element, policy: &wire::TextChecking) -> Element {
    if let Some(value) = policy.spellcheck {
        input = input.spellcheck(value);
    }
    if let Some(value) = policy.grammar_check {
        input = input.grammar_check(value);
    }
    if let Some(value) = policy.autocorrect {
        input = input.autocorrect(value);
    }
    if let Some(value) = policy.smart_quotes {
        input = input.smart_quotes(value);
    }
    if let Some(value) = policy.smart_dashes {
        input = input.smart_dashes(value);
    }
    if let Some(value) = policy.text_replacement {
        input = input.text_replacement(value);
    }
    input
}

fn apply_style(element: Element, style: &wire::Style) -> Element {
    use wire::Style as S;
    match style {
        S::Width(v) => element.w(*v),
        S::Height(v) => element.h(*v),
        S::MinWidth(v) => element.min_w(*v),
        S::MinHeight(v) => element.min_h(*v),
        S::MaxWidth(v) => element.max_w(*v),
        S::MaxHeight(v) => element.max_h(*v),
        S::WidthFraction(v) => element.w_fraction(*v),
        S::HeightFraction(v) => element.h_fraction(*v),
        S::FlexGrow(v) => element.flex_grow(*v),
        S::FlexShrink(v) => element.flex_shrink(*v),
        S::FlexBasis(v) => element.flex_basis(*v),
        S::FlexDirection(v) => {
            if v == "row" {
                element.flex_row()
            } else {
                element.flex_col()
            }
        }
        S::AlignItems(v) => match v.as_str() {
            "center" => element.items_center(),
            "end" => element.items_end(),
            "stretch" => element.items_stretch(),
            _ => element.items_start(),
        },
        S::AlignSelf(v) => match v.as_str() {
            "stretch" => element.self_stretch(),
            "center" => element.self_center(),
            _ => element.self_start(),
        },
        S::JustifyContent(v) => match v.as_str() {
            "center" => element.justify_center(),
            "end" => element.justify_end(),
            _ => element.justify_start(),
        },
        S::Gap(v) => element.gap(*v),
        S::Padding(v) => element.padding(v[0], v[1], v[2], v[3]),
        S::Background(v) => element.bg(color(*v)),
        S::Foreground(v) => element.text_color(color(*v)),
        S::Border(v, c) => element
            .border_top(v[0], color(*c))
            .border_right(v[1], color(*c))
            .border_bottom(v[2], color(*c))
            .border_left(v[3], color(*c)),
        S::Radius(v) => element.rounded(*v),
        S::CornerRadii(v) => element.corner_radii(crate::Corners::new(v[0], v[1], v[2], v[3])),
        S::ScrollBackgroundCorners(v) => element.scroll_background_corners(*v),
        S::ScrollClip(v, r) => element.scroll_clip(*v, *r),
        S::FontFamily(v) => element.font_family(if v == "monospace" {
            FontFamily::Monospace
        } else {
            FontFamily::from(v.clone())
        }),
        S::FontSize(v) => element.text_size(*v),
        S::LineHeight(v) => element.line_height(*v),
        S::FontWeight(v) => element.font_weight(FontWeight(*v)),
        S::MonospaceWidth(v) => element.monospace_width(*v),
        S::BasicTextShaping => element.text_shaping_basic(),
        S::FontThicken(v) => element.font_thicken(*v),
        S::Italic(v) => {
            if *v {
                element.italic()
            } else {
                element
            }
        }
        S::Wrap(v) => {
            if *v {
                element.wrap()
            } else {
                element.no_wrap()
            }
        }
        S::TextAlign(v) => match v.as_str() {
            "right" => element.text_right(),
            "center" => element.text_center(),
            _ => element.text_left(),
        },
        S::TextEllipsis => element.text_ellipsis(),
        S::Overflow(v) => {
            if v == "scroll" {
                element.overflow_scroll()
            } else {
                element.overflow_hidden()
            }
        }
        S::OverflowX(v) => {
            if v == "scroll" {
                element.overflow_x_scroll()
            } else {
                element.overflow_hidden()
            }
        }
        S::OverflowY(v) => {
            if v == "scroll" {
                element.overflow_y_scroll()
            } else {
                element.overflow_hidden()
            }
        }
        S::Position(v) => {
            if v == "absolute" {
                element.absolute()
            } else {
                element.relative()
            }
        }
        S::Top(v) => element.top(*v),
        S::Left(v) => element.left(*v),
        S::Right(v) => element.right(*v),
        S::Bottom(v) => element.bottom(*v),
        S::StickyLeft(v) => element.sticky_left(*v),
        S::ZIndex(v) => element.z_index(*v),
        S::UserSelect(v) => {
            if *v {
                element.user_select_text()
            } else {
                element.user_select_none()
            }
        }
        S::Cursor(v) => {
            if v == "text" {
                element.cursor_text()
            } else {
                element.cursor_default()
            }
        }
        S::Role(v) => element.accessibility_role(match v.as_str() {
            "grid" => crate::AccessibilityRole::Grid,
            "row" => crate::AccessibilityRole::Row,
            "row-header" => crate::AccessibilityRole::RowHeader,
            "grid-cell" => crate::AccessibilityRole::GridCell,
            "separator" => crate::AccessibilityRole::Separator,
            "region" => crate::AccessibilityRole::Region,
            _ => crate::AccessibilityRole::Group,
        }),
        S::RowCount(v) => element.accessibility_row_count(*v),
        S::RowIndex(v) => element.accessibility_row_index(*v),
        S::Label(v) => element.accessibility_label(v.clone()),
        S::Hidden => element.hidden(),
        S::GridColumns(v) => element.grid().grid_cols(*v),
        S::HorizontalScrollbarInset(v) => element.horizontal_scrollbar_left_inset(*v),
        S::HideVerticalScrollbar(v) => {
            if *v {
                element.hide_vertical_scrollbar()
            } else {
                element
            }
        }
        S::Focusable(v) => {
            if *v {
                element.focusable()
            } else {
                element
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quickgui_extension_sdk::{Component, ComponentFactory, ComponentRuntime, WakeHandle};
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn extension_input_policy_is_package_owned_and_can_inherit_or_override() {
        for value in [None, Some(false), Some(true)] {
            let policy = wire::TextChecking {
                spellcheck: value,
                grammar_check: value,
                autocorrect: value,
                smart_quotes: value,
                smart_dashes: value,
                text_replacement: value,
            };
            let input = apply_input_text_checking(crate::text_area("text"), &policy);
            let crate::element::ElementKind::TextInput(input) = input.kind else {
                panic!("expected input");
            };
            let actual = input.constraints.text_checking;
            assert_eq!(
                [
                    actual.spellcheck,
                    actual.grammar_check,
                    actual.autocorrect,
                    actual.smart_quotes,
                    actual.smart_dashes,
                    actual.text_replacement
                ],
                [value; 6]
            );
        }
    }

    static ROWS: AtomicUsize = AtomicUsize::new(0);
    static DROPPED: AtomicUsize = AtomicUsize::new(0);
    struct Provider {
        label: String,
    }
    impl Drop for Provider {
        fn drop(&mut self) {
            DROPPED.fetch_add(1, Ordering::SeqCst);
        }
    }
    struct Factory;
    impl ComponentFactory for Factory {
        fn create(name: &str, props: Value, _: WakeHandle) -> Result<Box<dyn Component>, String> {
            if name != "document" {
                return Err("unknown type".into());
            }
            Ok(Box::new(Provider {
                label: props["label"].as_str().unwrap_or_default().into(),
            }))
        }
    }
    impl Component for Provider {
        fn update(&mut self, props: Value) -> Result<bool, String> {
            let next = props["label"].as_str().unwrap_or_default();
            let changed = self.label != next;
            self.label = next.into();
            Ok(changed)
        }
        fn render(&mut self, request: wire::RenderRequest) -> Result<wire::Frame, String> {
            if request.renderer.is_some() {
                ROWS.fetch_add(request.end - request.start, Ordering::SeqCst);
                return Ok(wire::Frame {
                    nodes: (request.start..request.end)
                        .map(|index| wire::Node {
                            key: index as u64 + 100,
                            kind: wire::Primitive::Text,
                            text: format!("row {index}"),
                            styles: vec![wire::Style::Height(20.0)],
                            ..Default::default()
                        })
                        .collect(),
                    ..Default::default()
                });
            }
            Ok(wire::Frame {
                nodes: vec![wire::Node {
                    key: 0,
                    styles: vec![
                        wire::Style::Width(300.0),
                        wire::Style::Height(120.0),
                        wire::Style::FlexDirection("column".into()),
                    ],
                    children: vec![
                        wire::Node {
                            key: 1,
                            kind: wire::Primitive::Button,
                            events: vec!["click".into()],
                            styles: vec![wire::Style::Height(20.0)],
                            children: vec![wire::Node {
                                key: 2,
                                kind: wire::Primitive::Text,
                                text: self.label.clone(),
                                ..Default::default()
                            }],
                            ..Default::default()
                        },
                        wire::Node {
                            key: 3,
                            styles: vec![
                                wire::Style::Height(100.0),
                                wire::Style::WidthFraction(1.0),
                            ],
                            list: Some(wire::List {
                                key: 1,
                                renderer: 1,
                                count: 10_000,
                                estimate: 20.0,
                                overscan: 2,
                                generation: 0,
                                measurement_generation: 0,
                                mirror: false,
                                row_styles: vec![],
                            }),
                            ..Default::default()
                        },
                    ],
                    ..Default::default()
                }],
                ..Default::default()
            })
        }
        fn event(&mut self, event: wire::InputEvent) -> Result<wire::EventResult, String> {
            Ok(wire::EventResult {
                events: vec![wire::OutputEvent {
                    kind: "chosen".into(),
                    value: event.value,
                }],
                ..Default::default()
            })
        }
    }
    static API: ComponentApi = ComponentRuntime::<Factory>::API;
    static DESCRIPTOR: abi::Extension = abi::Extension {
        abi_version: abi::ABI_VERSION,
        descriptor_size: size_of::<abi::Extension>() as u32,
        kind: abi::COMPONENT_EXTENSION,
        api_size: size_of::<ComponentApi>() as u32,
        name: abi::Bytes::new(b"independent-document-test"),
        version: abi::Bytes::new(b"7.2.1"),
        api: std::ptr::addr_of!(API).cast(),
    };
    struct Demo {
        component: Option<ExtensionComponent>,
    }
    impl crate::View for Demo {
        fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
            let component = self.component.get_or_insert_with(|| {
                ExtensionComponent::new(
                    "independent-document-test",
                    "document",
                    json!({"label":"Choose"}),
                    ElementId::new(22),
                    cx.window_invalidator(),
                    |_, _| {},
                )
                .unwrap()
            });
            component.element(cx).unwrap()
        }
    }

    #[test]
    fn independent_component_mounts_updates_virtualizes_and_releases_through_public_abi() {
        unsafe {
            crate::extensions::register_extension_versioned(
                &DESCRIPTOR,
                b"independent-document-test",
                b"7.2.1",
            )
        }
        .unwrap();
        let (mut app, view) = crate::Application::new()
            .into_test_context(crate::WindowOptions::default(), Demo { component: None })
            .unwrap();
        let window = view.window_handle();
        app.run_until_idle().unwrap();
        assert!(ROWS.load(Ordering::SeqCst) > 0);
        assert!(
            ROWS.load(Ordering::SeqCst) < 100,
            "only the viewport should cross the ABI"
        );
        let before = ROWS.load(Ordering::SeqCst);
        app.run_until_idle().unwrap();
        assert_eq!(
            ROWS.load(Ordering::SeqCst),
            before,
            "a clean component must sleep"
        );
        assert_eq!(
            app.element_bounds(window, ElementId::new(22))
                .unwrap()
                .width,
            300.0
        );
        drop(app);
        assert_eq!(
            DROPPED.load(Ordering::SeqCst),
            1,
            "unmount releases the native instance"
        );
    }
}
