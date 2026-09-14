//! Fluent construction of wire primitives, with retained callbacks for virtual rows.
use crate::schema::{self, Node, Primitive, Style};
use std::{
    cell::RefCell,
    collections::HashMap,
    ops::Range,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}
impl Color {
    pub const TRANSPARENT: Self = Self::linear(0.0, 0.0, 0.0, 0.0);
    pub const BLACK: Self = Self::linear(0.0, 0.0, 0.0, 1.0);
    pub const WHITE: Self = Self::linear(1.0, 1.0, 1.0, 1.0);
    pub const fn linear(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }
    pub fn rgb8(r: u8, g: u8, b: u8) -> Self {
        Self::rgba8(r, g, b, 255)
    }
    pub fn rgba8(r: u8, g: u8, b: u8, a: u8) -> Self {
        let linear = |v: u8| {
            let v = v as f32 / 255.0;
            if v <= 0.04045 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        Self::linear(linear(r), linear(g), linear(b), a as f32 / 255.0)
    }
    pub fn with_alpha(self, a: f32) -> Self {
        Self { a, ..self }
    }
    pub const fn paint(self) -> schema::Paint {
        [self.r, self.g, self.b, self.a]
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct HighlightStyle(schema::Span);
impl HighlightStyle {
    pub fn color(mut self, v: Color) -> Self {
        self.0.foreground = Some(v.paint());
        self
    }
    pub fn background(mut self, v: Color) -> Self {
        self.0.background = Some(v.paint());
        self
    }
    pub fn font_family(mut self, _: FontFamily) -> Self {
        self.0.font_family = Some("monospace".into());
        self
    }
    pub fn underline_color(mut self, v: Color) -> Self {
        self.0.underline_color = Some(v.paint());
        self
    }
    pub fn font_bold(mut self) -> Self {
        self.0.weight = Some(700);
        self
    }
    pub fn italic(mut self) -> Self {
        self.0.italic = true;
        self
    }
    pub fn underline(mut self) -> Self {
        self.0.underline = true;
        self
    }
    pub fn strikethrough(mut self) -> Self {
        self.0.strikethrough = true;
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StyledText {
    source: Arc<str>,
    spans: Vec<schema::Span>,
}
impl StyledText {
    pub fn new(source: impl Into<Arc<str>>) -> Self {
        Self {
            source: source.into(),
            spans: Vec::new(),
        }
    }
    pub fn content(&self) -> &str {
        &self.source
    }
    pub fn with_highlights(
        mut self,
        spans: impl IntoIterator<Item = (Range<usize>, HighlightStyle)>,
    ) -> Self {
        self.spans = spans
            .into_iter()
            .take(4096)
            .map(|(range, mut style)| {
                style.0.start = range.start;
                style.0.end = range.end;
                style.0
            })
            .collect();
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ElementId(u64);
impl ElementId {
    pub const fn new(id: u64) -> Self {
        Self(id)
    }
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}
impl From<u64> for ElementId {
    fn from(id: u64) -> Self {
        Self(id)
    }
}
#[derive(Clone, Copy, Debug)]
pub enum FontFamily {
    Monospace,
}
#[derive(Clone, Copy, Debug)]
pub struct FontWeight(pub u16);
impl FontWeight {
    pub const NORMAL: Self = Self(400);
    pub const BOLD: Self = Self(700);
    pub const SEMIBOLD: Self = Self(600);
}
#[derive(Clone, Copy, Debug)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}
#[derive(Clone, Copy, Debug)]
pub enum AccessibilityRole {
    Group,
    Region,
    Grid,
    Row,
    RowHeader,
    GridCell,
    Separator,
    Heading,
    List,
    ListItem,
    Table,
    Cell,
    ColumnHeader,
    Image,
    Link,
}

#[derive(Default)]
pub struct ElementStateStyle {
    pub styles: Vec<Style>,
}
impl ElementStateStyle {
    pub fn border(mut self, w: f32, c: Color) -> Self {
        self.styles.push(Style::Border([w; 4], c.paint()));
        self
    }
}

type RowRenderer = Rc<RefCell<dyn FnMut(usize) -> Element>>;
#[derive(Clone, Default)]
pub struct Element {
    pub node: Node,
    rows: Option<RowRenderer>,
    scroll: Option<u64>,
}
pub trait IntoElement {
    fn into_element(self) -> Element;
}
impl IntoElement for Element {
    fn into_element(self) -> Element {
        self
    }
}
impl IntoElement for StyledText {
    fn into_element(self) -> Element {
        Element {
            node: Node {
                kind: Primitive::Text,
                text: self.source.to_string(),
                spans: self.spans,
                ..Default::default()
            },
            rows: None,
            scroll: None,
        }
    }
}
impl IntoElement for &str {
    fn into_element(self) -> Element {
        text(self)
    }
}
impl IntoElement for String {
    fn into_element(self) -> Element {
        text(self)
    }
}
pub fn div() -> Element {
    Element::default()
}
pub fn text(value: impl Into<Arc<str>>) -> Element {
    StyledText::new(value).into_element()
}
pub fn styled_text_area(value: StyledText) -> Element {
    let mut e = value.into_element();
    e.node.kind = Primitive::Input;
    e.node.input = Some(schema::InputOptions {
        multiline: true,
        ..Default::default()
    });
    e
}

#[cfg(test)]
mod input_tests {
    use super::*;

    #[test]
    fn sibling_virtual_lists_do_not_alias_anonymous_row_keys() {
        let left = ListState::new(100, 20.0);
        let right = ListState::new(100, 20.0);
        let mut renderers = Renderers::default();
        let root = renderers.root(|| {
            div().children([
                div()
                    .child(left.render_rows(0..0, |n| div().child(text(format!("left {n}")))))
                    .variable_virtual_scroll(&left),
                div()
                    .child(right.render_rows(0..0, |n| div().child(text(format!("right {n}")))))
                    .variable_virtual_scroll(&right),
            ])
        });
        let mut keys = std::collections::HashSet::new();
        for list in &root.nodes[0].children {
            let request = schema::RenderRequest {
                renderer: Some(list.list.as_ref().unwrap().renderer),
                start: 3,
                end: 5,
                ..Default::default()
            };
            let first = renderers.rows(&request).unwrap();
            let repeated = renderers.rows(&request).unwrap();
            assert_eq!(first.nodes, repeated.nodes);
            for row in first.nodes {
                assert!(keys.insert(row.key));
                assert!(keys.insert(row.children[0].key));
            }
        }
    }

    #[test]
    fn rows_can_register_nested_virtual_renderers_and_reject_unbounded_requests() {
        let outer = ListState::new(1, 20.0);
        let inner = ListState::new(100, 20.0);
        let mut renderers = Renderers::default();
        let root = renderers.root(|| {
            div()
                .child(outer.render_rows(0..0, move |_| {
                    div()
                        .child(inner.render_rows(0..0, |n| text(format!("nested {n}"))))
                        .variable_virtual_scroll(&inner)
                }))
                .variable_virtual_scroll(&outer)
        });
        let mut request = schema::RenderRequest {
            renderer: Some(root.nodes[0].list.as_ref().unwrap().renderer),
            end: 1,
            ..Default::default()
        };
        let row = renderers.rows(&request).unwrap();
        request.renderer = Some(row.nodes[0].list.as_ref().unwrap().renderer);
        request.start = 10;
        request.end = 11;
        assert_eq!(renderers.rows(&request).unwrap().nodes[0].text, "nested 10");
        request.end = request.start + schema::MAX_COMPONENT_ROWS + 1;
        assert!(renderers.rows(&request).is_err());
        request.end = request.start - 1;
        assert!(renderers.rows(&request).is_err());
    }

    #[test]
    fn input_policy_survives_the_serialized_component_boundary() {
        for enabled in [false, true] {
            let input = styled_text_area(StyledText::new("text"))
                .spellcheck(enabled)
                .grammar_check(enabled)
                .autocorrect(enabled)
                .smart_quotes(enabled)
                .smart_dashes(enabled)
                .text_replacement(enabled);
            let encoded = serde_json::to_vec(&input.node).unwrap();
            let node: Node = serde_json::from_slice(&encoded).unwrap();
            let checking = node.input.unwrap().text_checking;
            assert_eq!(
                [
                    checking.spellcheck,
                    checking.grammar_check,
                    checking.autocorrect,
                    checking.smart_quotes,
                    checking.smart_dashes,
                    checking.text_replacement
                ],
                [Some(enabled); 6]
            );
        }
        assert_eq!(
            styled_text_area(StyledText::new(""))
                .node
                .input
                .unwrap()
                .text_checking,
            schema::TextChecking::default()
        );
    }
}

// The SDK retains row closures outside serialized nodes, indexed by package-local list identity.
thread_local! {static ROWS:RefCell<HashMap<u64,RowRenderer>>=RefCell::new(HashMap::new());}
pub struct Renderers(HashMap<u64, RowRenderer>);
impl Default for Renderers {
    fn default() -> Self {
        Self(HashMap::new())
    }
}
impl Renderers {
    pub fn root(&mut self, build: impl FnOnce() -> Element) -> schema::Frame {
        ROWS.with(|rows| rows.borrow_mut().clear());
        let mut root = build().node;
        root.key = 0;
        assign_keys(&mut root, 0);
        self.0 = ROWS.with(|rows| std::mem::take(&mut *rows.borrow_mut()));
        schema::Frame {
            nodes: vec![root],
            ..Default::default()
        }
    }
    pub fn rows(&mut self, request: &schema::RenderRequest) -> Result<schema::Frame, String> {
        if request.end < request.start || request.end - request.start > schema::MAX_COMPONENT_ROWS {
            return Err("invalid component row range".into());
        }
        let renderer_key = request.renderer.ok_or("missing row renderer")?;
        let renderer = self
            .0
            .get(&renderer_key)
            .cloned()
            .ok_or("unknown row renderer")?;
        let frame = schema::Frame {
            nodes: (request.start..request.end)
                .map(|index| {
                    let mut node = renderer.borrow_mut()(index).node;
                    // Anonymous rows must be stable within a list, but distinct
                    // from anonymous rows belonging to another list in this component.
                    let seed = renderer_key
                        .wrapping_mul(0x9e3779b97f4a7c15)
                        .wrapping_add(index as u64 + 1)
                        .max(1);
                    assign_keys(&mut node, seed);
                    node
                })
                .collect(),
            ..Default::default()
        };
        // Row callbacks may themselves declare a virtual list. Retain its callback
        // before the host asks for the nested visible range.
        self.0
            .extend(ROWS.with(|rows| std::mem::take(&mut *rows.borrow_mut())));
        Ok(frame)
    }
}

fn assign_keys(node: &mut Node, seed: u64) {
    if node.key == 0 && seed != 0 {
        node.key = seed;
    }
    let parent = node.key;
    for (index, child) in node.children.iter_mut().enumerate() {
        assign_keys(
            child,
            parent
                .wrapping_mul(0x9e3779b97f4a7c15)
                .wrapping_add(index as u64 + 1),
        );
    }
}

macro_rules! metric {($($method:ident=>$variant:ident),*$(,)?)=>{$(pub fn $method(self,value:f32)->Self{self.style(Style::$variant(value))})*};}
macro_rules! flag {($($method:ident=>$variant:ident($value:expr)),*$(,)?)=>{$(pub fn $method(self)->Self{self.style(Style::$variant($value))})*};}
impl Element {
    pub fn cursor_default(self) -> Self {
        self.style(Style::Cursor("default".into()))
    }
    pub fn focus(mut self, apply: impl FnOnce(ElementStateStyle) -> ElementStateStyle) -> Self {
        self.node.focus_styles = apply(ElementStateStyle::default()).styles;
        self
    }
    pub fn spellcheck(mut self, enabled: bool) -> Self {
        self.node
            .input
            .get_or_insert_with(Default::default)
            .text_checking
            .spellcheck = Some(enabled);
        self
    }
    pub fn grammar_check(mut self, enabled: bool) -> Self {
        self.node
            .input
            .get_or_insert_with(Default::default)
            .text_checking
            .grammar_check = Some(enabled);
        self
    }
    pub fn autocorrect(mut self, enabled: bool) -> Self {
        self.node
            .input
            .get_or_insert_with(Default::default)
            .text_checking
            .autocorrect = Some(enabled);
        self
    }
    pub fn smart_quotes(mut self, enabled: bool) -> Self {
        self.node
            .input
            .get_or_insert_with(Default::default)
            .text_checking
            .smart_quotes = Some(enabled);
        self
    }
    pub fn smart_dashes(mut self, enabled: bool) -> Self {
        self.node
            .input
            .get_or_insert_with(Default::default)
            .text_checking
            .smart_dashes = Some(enabled);
        self
    }
    pub fn text_replacement(mut self, enabled: bool) -> Self {
        self.node
            .input
            .get_or_insert_with(Default::default)
            .text_checking
            .text_replacement = Some(enabled);
        self
    }
    pub fn accessibility_read_only(mut self, v: bool) -> Self {
        self.node
            .input
            .get_or_insert_with(Default::default)
            .read_only = v;
        self
    }
    pub fn style(mut self, value: Style) -> Self {
        let key = std::mem::discriminant(&value);
        self.node
            .styles
            .retain(|v| std::mem::discriminant(v) != key);
        self.node.styles.push(value);
        self
    }
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.node.key = id.into().0;
        self
    }
    pub fn child(mut self, child: impl IntoElement) -> Self {
        let mut child = child.into_element();
        self.adopt_rows(&mut child);
        self.node.children.push(child.node);
        self.bind_virtual_rows();
        self
    }
    fn adopt_rows(&mut self, child: &mut Element) {
        if let (Some(rows), Some(list)) = (child.rows.take(), child.node.list.as_ref()) {
            ROWS.with(|all| all.borrow_mut().insert(list.renderer, rows));
        }
    }
    pub fn children(self, children: impl IntoIterator<Item = impl IntoElement>) -> Self {
        children
            .into_iter()
            .fold(self, |parent, child| parent.child(child))
    }
    pub fn when(self, condition: bool, f: impl FnOnce(Self) -> Self) -> Self {
        if condition { f(self) } else { self }
    }
    metric!(w=>Width,h=>Height,min_w=>MinWidth,min_h=>MinHeight,max_w=>MaxWidth,max_h=>MaxHeight,w_fraction=>WidthFraction,h_fraction=>HeightFraction,gap=>Gap,text_size=>FontSize,line_height=>LineHeight,rounded=>Radius,top=>Top,left=>Left,right=>Right,bottom=>Bottom,sticky_left=>StickyLeft,horizontal_scrollbar_left_inset=>HorizontalScrollbarInset);
    flag!(w_full=>WidthFraction(1.0),h_full=>HeightFraction(1.0),flex_row=>FlexDirection("row".into()),flex_col=>FlexDirection("column".into()),items_center=>AlignItems("center".into()),items_stretch=>AlignItems("stretch".into()),items_start=>AlignItems("start".into()),justify_end=>JustifyContent("end".into()),justify_center=>JustifyContent("center".into()),self_stretch=>AlignSelf("stretch".into()),user_select_none=>UserSelect(false),user_select_text=>UserSelect(true),overflow_hidden=>Overflow("hidden".into()),overflow_x_scroll=>OverflowX("scroll".into()),overflow_y_scroll=>OverflowY("scroll".into()),relative=>Position("relative".into()),absolute=>Position("absolute".into()),hide_vertical_scrollbar=>HideVerticalScrollbar(true),wrap=>Wrap(true),no_wrap=>Wrap(false),font_semibold=>FontWeight(600),font_bold=>FontWeight(700),italic=>Italic(true),text_right=>TextAlign("right".into()),rounded_sm=>Radius(2.0),rounded_lg=>Radius(8.0),gap_2=>Gap(8.0),gap_3=>Gap(12.0));
    pub fn size_full(self) -> Self {
        self.w_full().h_full()
    }
    pub fn size(self, w: f32, h: f32) -> Self {
        self.w(w).h(h)
    }
    pub fn flex_1(self) -> Self {
        self.style(Style::FlexGrow(1.0))
            .style(Style::FlexShrink(1.0))
            .style(Style::FlexBasis(0.0))
    }
    pub fn flex_none(self) -> Self {
        self.style(Style::FlexGrow(0.0))
            .style(Style::FlexShrink(0.0))
    }
    pub fn padding(self, t: f32, r: f32, b: f32, l: f32) -> Self {
        self.style(Style::Padding([t, r, b, l]))
    }
    pub fn p_2(self) -> Self {
        self.padding(8.0, 8.0, 8.0, 8.0)
    }
    pub fn p_3(self) -> Self {
        self.padding(12.0, 12.0, 12.0, 12.0)
    }
    pub fn px_2(self) -> Self {
        self.padding(0.0, 8.0, 0.0, 8.0)
    }
    pub fn px_3(self) -> Self {
        self.padding(0.0, 12.0, 0.0, 12.0)
    }
    pub fn bg(self, v: Color) -> Self {
        self.style(Style::Background(v.paint()))
    }
    pub fn text_color(self, v: Color) -> Self {
        self.style(Style::Foreground(v.paint()))
    }
    pub fn border(self, w: f32, c: Color) -> Self {
        self.style(Style::Border([w; 4], c.paint()))
    }
    pub fn rounded_l(self, radius: f32) -> Self {
        self.style(Style::CornerRadii([radius, 0.0, 0.0, radius]))
    }
    pub fn scroll_background_corners(self, radii: [f32; 4]) -> Self {
        self.style(Style::ScrollBackgroundCorners(radii))
    }
    pub fn scroll_clip(self, insets: [f32; 4], radii: [f32; 4]) -> Self {
        self.style(Style::ScrollClip(insets, radii))
    }
    pub fn rounded_t(self, radius: f32) -> Self {
        self.style(Style::CornerRadii([radius, radius, 0.0, 0.0]))
    }
    fn border_side(mut self, index: usize, w: f32, c: Color) -> Self {
        let mut sides = [0.0; 4];
        for style in &self.node.styles {
            if let Style::Border(widths, _) = style {
                sides = *widths;
            }
        }
        sides[index] = w;
        self = self.style(Style::Border(sides, c.paint()));
        self
    }
    pub fn border_top(self, w: f32, c: Color) -> Self {
        self.border_side(0, w, c)
    }
    pub fn border_right(self, w: f32, c: Color) -> Self {
        self.border_side(1, w, c)
    }
    pub fn border_bottom(self, w: f32, c: Color) -> Self {
        self.border_side(2, w, c)
    }
    pub fn border_left(self, w: f32, c: Color) -> Self {
        self.border_side(3, w, c)
    }
    pub fn font_family(self, _: FontFamily) -> Self {
        self.style(Style::FontFamily("monospace".into()))
    }
    pub fn font_weight(self, v: FontWeight) -> Self {
        self.style(Style::FontWeight(v.0))
    }
    pub fn text_align(self, v: TextAlign) -> Self {
        self.style(Style::TextAlign(format!("{v:?}").to_lowercase()))
    }
    pub fn text_ellipsis(self) -> Self {
        self.style(Style::TextEllipsis)
    }
    pub fn z_index(self, v: i16) -> Self {
        self.style(Style::ZIndex(v))
    }
    pub fn accessibility_role(self, v: AccessibilityRole) -> Self {
        let name = match v {
            AccessibilityRole::RowHeader => "row-header".into(),
            AccessibilityRole::GridCell => "grid-cell".into(),
            _ => format!("{v:?}").to_lowercase(),
        };
        self.style(Style::Role(name))
    }
    pub fn accessibility_label(self, v: impl Into<String>) -> Self {
        self.style(Style::Label(v.into()))
    }
    pub fn accessibility_row_count(self, v: usize) -> Self {
        self.style(Style::RowCount(v))
    }
    pub fn accessibility_row_index(self, v: usize) -> Self {
        self.style(Style::RowIndex(v))
    }
    pub fn grid(self) -> Self {
        self
    }
    pub fn grid_cols(self, n: u16) -> Self {
        self.style(Style::GridColumns(n))
    }
    pub fn variable_virtual_scroll(mut self, list: &ListState) -> Self {
        self.scroll = Some(list.0.borrow().key);
        self.bind_virtual_rows();
        self
    }

    fn bind_virtual_rows(&mut self) {
        let Some(key) = self.scroll else {
            return;
        };
        if let Some(index) = self
            .node
            .children
            .iter()
            .position(|child| child.list.as_ref().is_some_and(|list| list.key == key))
        {
            let child = self.node.children.remove(index);
            self.node.list = child.list;
            if let Some(list) = &mut self.node.list {
                list.row_styles = child.styles;
            }
        }
    }
    pub fn variable_virtual_scroll_mirror(self, list: &ListState) -> Self {
        self.variable_virtual_scroll(list)
    }
}

#[derive(Clone)]
pub struct ListState(Rc<RefCell<schema::List>>);
pub struct VisibleRows {
    pub range: Range<usize>,
}
impl ListState {
    pub fn new(count: usize, estimate: f32) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let key = NEXT.fetch_add(1, Ordering::Relaxed);
        Self(Rc::new(RefCell::new(schema::List {
            key,
            renderer: key,
            count,
            estimate,
            overscan: 6,
            generation: 0,
            measurement_generation: 0,
            mirror: false,
            row_styles: Vec::new(),
        })))
    }
    pub fn with_overscan(self, n: usize) -> Self {
        self.0.borrow_mut().overscan = n;
        self
    }
    pub fn item_count(&self) -> usize {
        self.0.borrow().count
    }
    pub fn visible_rows(&self) -> VisibleRows {
        VisibleRows { range: 0..0 }
    }
    pub fn reset(&self, count: usize) {
        let mut state = self.0.borrow_mut();
        state.count = count;
        state.generation = state.generation.wrapping_add(1);
    }
    pub fn set_item_count(&self, count: usize) {
        self.0.borrow_mut().count = count;
    }
    pub fn remeasure(&self) {
        self.0.borrow_mut().measurement_generation += 1;
    }
    pub fn render_rows<E: IntoElement>(
        &self,
        _: Range<usize>,
        mut render: impl FnMut(usize) -> E + 'static,
    ) -> Element {
        Element {
            node: Node {
                list: Some(self.0.borrow().clone()),
                ..Default::default()
            },
            rows: Some(Rc::new(RefCell::new(move |index| {
                render(index).into_element()
            }))),
            scroll: None,
        }
    }
    pub fn render_mirrored_rows<E: IntoElement>(
        &self,
        salt: u64,
        mut render: impl FnMut(usize) -> E + 'static,
    ) -> Element {
        let mut list = self.0.borrow().clone();
        list.renderer ^= salt;
        list.mirror = true;
        Element {
            node: Node {
                list: Some(list),
                ..Default::default()
            },
            rows: Some(Rc::new(RefCell::new(move |index| {
                render(index).into_element()
            }))),
            scroll: None,
        }
    }
}
