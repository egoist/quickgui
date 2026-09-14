# View API and layout

[Documentation index](README.md)

## View API

Compound controls use short constructors such as `popover.root()`, `popover.trigger()`, and `popover.popup()`. Each returns a fluent element accepting direct strings: `popover.trigger().child("Help")`. See the [component API guide](component-api.md) for the matching Go instance API.

Views use regular Rust with JSX-like composition, composable Tailwind-style spacing, inherited typography, Flexbox and CSS Grid, stable identities, and view-local listeners:

```rust
use quickgui::{
    Application, Color, EventContext, IntoElement, View, ViewContext, WindowOptions, div, text,
};

struct Counter {
    count: usize,
}

impl View for Counter {
    fn render(
        &mut self,
        cx: &mut ViewContext<'_, Self>,
    ) -> impl IntoElement {
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
            .bg(Color::rgb8(18, 18, 20))
            .text_color(Color::rgb8(240, 241, 244))
            .child(text(format!("Count: {}", self.count)).text_2xl().font_semibold())
            .child(
                div()
                    .on_click(increment)
                    .px_4()
                    .py_2()
                    .rounded_lg()
                    .bg(Color::rgb8(45, 105, 180))
                    .hover(|style| style.bg(Color::rgb8(56, 122, 204)))
                    .active(|style| style.bg(Color::rgb8(37, 87, 151)))
                    .child("Increment"),
            )
    }
}

fn main() -> Result<(), quickgui::AppError> {
    Application::new().run(|cx| {
        cx.open_window(
            WindowOptions::new("Counter").size(480.0, 320.0),
            Counter { count: 0 },
        );
    })
}
```

`Scene`, `Quad`, `Shadow`, `PathPrimitive`, `ImagePrimitive`, `SvgPrimitive`, and
`CustomShaderPrimitive` remain public as the lower-level escape hatch for specialized widgets.
Ordinary application views should use `Element` builders or the scoped `canvas(...)` painter.

Typography cascades through ordinary containers. `text_left`, `text_center`, and `text_right` match
GPUI's alignment helpers; `text_justify` adds the web paragraph case. Alignment affects shaping only
when the assigned width matters, so left-aligned no-wrap labels keep the existing width-independent
cache fast path. `whitespace_normal`/`whitespace_nowrap`, all three ellipsis placements,
`truncate`, and `line_clamp` provide the corresponding web/GPUI overflow vocabulary. Font slant,
complete `Font` values, independently inherited OpenType features and ordered fallback families,
solid/wavy single or double underline, 0/1/2/4/8-point thickness, underline color, strikethrough,
and explicit decoration reset inherit through the same retained text style; see
[Text, editing, and forms](text-and-forms.md) for composition and selection semantics.

`hidden()` is GPUI/CSS `display: none`: its complete subtree leaves layout, paint, hit testing,
focus, accessibility, image resolution, container-query callbacks, and animation scheduling.
`invisible()` keeps the layout box and controlled input state warm but suppresses paint and runtime
interaction for the subtree; `visible()` restores it without changing `block`, `flex`, or `grid`.

`opacity(value)` clamps finite values to `0.0..=1.0` and applies to the complete subtree. Nested
values multiply, including rich text, images, SVGs, paths, canvas/custom-shader primitives, and
macOS native child views. Like CSS and GPUI, opacity does not remove layout, hit testing, focus, or
accessibility; use `hidden()` or `invisible()` when those semantics are wanted. Opacity is carried
as paint data, so changing it does not reshape text, rerasterize SVG masks, or allocate an
offscreen group texture.

Backgrounds accept more than a solid color. `bg_linear_gradient(angle, stops)`,
`bg_radial_gradient(stops)`, `bg_radial_gradient_at(shape, center, stops)`,
`bg_conic_gradient(from_angle, stops)`, and the general `bg_gradient(background)` paint a bounded
multi-stop gradient; `bg_image(image, size, repeat, position)` and its `bg_image_cover`,
`bg_image_contain`, and `bg_image_tiled` shorthands paint a raster background behind children.
Corners round independently with `rounded_tl`/`rounded_tr`/`rounded_br`/`rounded_bl`,
`rounded_t`/`rounded_b`/`rounded_l`/`rounded_r`, `corner_radii(Corners)`, and `rounded_full()`.
Borders gain `border_solid()`, `border_dashed()`, and `border_dotted()` on top of the existing
per-side widths, and `outline(width, color)` with `outline_offset(px)`, `outline_dashed()`,
`outline_dotted()`, and `outline_none()` draws a ring outside the border box without affecting
layout. `filters([...])`, `filter(...)`, and the `brightness`, `contrast`, `saturate`, `invert`, `sepia`,
`hue_rotate`, and `grayscale` shorthands apply a bounded CSS-shaped color-filter chain to an
element's own raster content. All of these are paint-only: they change no Taffy style and schedule
no frame of their own. See [Graphics and media](graphics.md) for the exact bounds.

Hover, active, focus, validation, and drag-state variants are paint-only. Add
`.transition(Duration::from_millis(140))` to interpolate their colors, border, radius, inherited
text color, opacity, and bounded shadow list without rebuilding the view or rerunning Taffy. Layout
values use `AnimationExt::with_animation`; see [Declarative motion](animations.md).

`group()` marks an element as the group whose hover and presses its descendants follow, and a
descendant's `group_hover(|style| …)` and `group_active(|style| …)` variants paint while that nearest
group is hovered or holds a press — Tailwind's `group`, `group-hover`, and `group-active`. The group
counts as hovered wherever the pointer rests inside it, over its own padding or over any descendant,
unless a surface above it consumes the pointer, and as active while a press on it or on any
clickable descendant is held. Group variants layer above the base style and beneath the element's
own hover, active, focus, validation, and drag variants, so a revealed action button the pointer
reaches keeps every group value its own `hover` does not override. A cursor declared in them is
ignored, because the pointer is over some other element.

Group variants can be declared several times. Every entry whose group is in its state paints, later
declarations winning where they overlap, as matching CSS rules of equal specificity do — declare
`group_active` after `group_hover` so the press wins — and one element follows at most
`MAX_GROUP_STYLES_PER_ELEMENT` (8) group states. `group_named("sidebar")` with
`group_hover_named("sidebar", |style| …)` or `group_active_named` are Tailwind's `group/sidebar` and
`group-hover/sidebar`: the member follows the nearest ancestor group carrying that name, past any
nearer group, and paints nothing when no ancestor carries it. A named group is still the nearest
group for members that name none. Names are bounded by `MAX_HOVER_GROUP_NAME_BYTES`.

`focus_within(|style| …)` paints while the element or any descendant owns keyboard focus, like CSS
`:focus-within`. Unlike `focus`, it follows the focus itself rather than focus visibility, so a field
container highlights whenever its input is focused however that focus arrived; it layers beneath
the element's own `focus` and above its group variants.

```rust
div().group().flex_row().children([
    text("Quarterly report"),
    button()
        .opacity(0.0)
        .transition(Duration::from_millis(120))
        .group_hover(|style| style.opacity(1.0))
        .group_active(|style| style.opacity(0.8))
        .child(text("Rename")),
])
```

The `focus` variant paints only while focus is *visible*, like CSS `:focus-visible`. Focus that a
pointer press lands — including a `cx.focus(...)` a listener performs while a press is being
dispatched — paints no focus styles; focus that a key lands — Tab, a roving arrow, or a listener
focusing in response to a key — paints them, and Tab or an arrow that moves nothing reveals the
focus already there. A `cx.focus(...)` outside any input dispatch keeps the current visibility. Text
inputs and text areas always paint their focus styles, as a native text field always shows its
ring. Keyboard focus, accessibility focus, and focus traps are unaffected; only the styles are
gated.

## Flexbox and spacing

The fluent surface includes GPUI's everyday flex vocabulary rather than requiring direct
Taffy mutation:

```rust
div()
    .flex_row()
    .items_center()
    .justify_between()
    .gap_x_3()
    .gap_y_2()
    .child(sidebar.w(192.0).flex_none())
    .child(content.min_w(0.0).flex_1())
    .child(inspector.flex_initial().self_stretch())
```

`flex_row_reverse`/`flex_col_reverse`, `flex_auto`, `flex_initial`, `flex_none`, an explicit
`flex_basis`, sanitized grow/shrink factors, baseline and per-item alignment, wrapped-content
alignment, `justify_around`/`justify_evenly`, and `aspect_ratio`/`aspect_square` all map directly to
the retained Taffy style. These declarations allocate no runtime object and schedule no frame.

Margins use web names: `m`, `mx`, `my`, `mt`, `mr`, `mb`, and `ml`. Finite negative values are
accepted, the matching `_auto` helpers participate in CSS auto-margin distribution, and common
four-point scale suffixes run from `_0` through `_32`. Axis-specific gaps compose independently.
For example, `.max_w(700.0).mx_auto()` centers a bounded content column without an extra wrapper.
Run the focused gallery with:

```console
cargo run --release --example flex_layout
```

## CSS Grid

The GPUI-shaped equal-track and placement helpers work directly on ordinary elements:

```rust
div()
    .size_full()
    .grid()
    .grid_cols(5)
    .grid_rows(5)
    .gap_2()
    .child(header.col_span_full().row_span(1))
    .child(sidebar.col_span(1).row_span(3))
    .child(content.col_span(3).row_span(3))
    .child(inspector.col_span(1).row_span(3))
    .child(footer.col_span_full().row_span(1))
```

For app-shell and dashboard layouts, explicit web-style tracks support fixed pixels, percentages,
fractions, intrinsic sizing, fit-content, and the common `minmax(px, fr)` form:

```rust
use quickgui::GridTrack;

div()
    .grid()
    .grid_template_columns([
        GridTrack::px(180.0),
        GridTrack::minmax_px_fr(240.0, 1.0),
        GridTrack::fit_content_px(160.0),
    ])
    .grid_template_rows([GridTrack::px(64.0), GridTrack::fr(1.0)])
```

`grid_cols[_min_content|_max_content]`, matching row helpers, explicit line start/end, full and
numeric spans, and sparse/dense row/column auto-flow are available. Track lists and spans are
hard-capped at 1,024 per axis; equal tracks remain one compact Taffy `repeat()` component. Grid is
retained CPU layout only: paint-only interaction and scrolling reuse it, and it creates no GPU
resources or idle frames. Run `cargo run --release --example grid_layout` and resize across 700 pt
to compare the grid with its stacked Flexbox fallback.

## Container queries

Use `container_query` when a reusable view must respond to the box its parent actually assigned,
instead of the whole window size:

```rust
use quickgui::{container_query, div, text};

container_query(|size| {
    if size.width < 480.0 {
        div().flex_col().child(text("Compact layout"))
    } else {
        div().grid().grid_cols(3).child(text("Wide layout"))
    }
})
```

The query fills its parent by default; ordinary sizing helpers such as `.w(320.0)`, `.flex_1()`,
and `.max_w(...)` can refine that box. The box participates in its parent's Taffy layout as a
leaf. Only after its size is known does QuickGUI invoke the callback and lay the returned element
out as an independent root inside that fixed size. Callback contents therefore cannot change the
query's intrinsic size or create a layout feedback loop.

Within one retained declaration, an unchanged assigned size reuses the existing callback subtree.
A retained relayout invokes only affected callbacks, preserves stable declarative-animation IDs,
and creates no timer, polling source, GPU resource, or idle frame. Queries work inside normal
views, tooltips, and drag previews; detached surfaces keep their existing pointer-passive contract.
One window is hard-limited to 1,024 mounted queries and 16 nested query levels.

The current callback deliberately receives `Size` only. Capture cloneable entities, globals, and
listeners from the surrounding `View::render` declaration when responsive contents need
application state. Query contents come exclusively from the callback, so `.child(...)` and
`.children(...)` on the query itself are rejected.

Run the live responsive example and resize through its compact, two-column, and three-column
breakpoints:

```console
cargo run --release --example container_queries
```

## Variable-height lists

`VirtualList` remains the allocation-free O(1) choice for uniform rows. Wrapped comments, chat
messages, logs, and other differently sized items use intrusive `ListState` instead:

```rust
use quickgui::{ListState, div, text};

// Store this on the view. The estimate sizes unmeasured offscreen content.
let comments = ListState::new(comment_count, 96.0).with_overscan(3);

// In render, seed the known viewport and mount only its bounded range.
comments.set_viewport_size(cx.size().width, cx.size().height);
let visible = comments.visible_rows();
let rows = comments.render_rows(visible.range, |index| {
    div()
        .w_full()
        .px_4()
        .py_3()
        .child(text(comment_text(index)).w_full().wrap())
});

div()
    .relative()
    .size_full()
    .overflow_hidden()
    .variable_virtual_scroll(&comments)
    .child(rows)
```

Rows stay in one normal Flexbox column, so their real wrapped heights stack correctly in the first
layout that sees them. Taffy reports those heights into a sparse 64-item metric index; a changed
measurement requests one correcting declaration, while settled rows and scrollbar hover remain
paint-only. The logical top item and pixel inset survive measurement changes. Width changes
invalidate measurements, and `remeasure_items(range)` handles content changes without throwing
away unaffected blocks.

`with_overscan_pixels` adds a stable logical render-ahead distance on top of row-count overscan.
When row heights are already known, `set_item_heights` seeds all of them at once and preserves the
current logical scroll anchor. Go and TypeScript expose the same controls as `OverscanPixels` /
`ItemHeights` and `overscanPixels` / `itemHeights`.

`set_item_count` preserves unchanged prefix measurements; use `reset` when item identity changes.
Bottom-aligned transcripts can combine `ListAlignment::Bottom` and `FollowMode::Tail`. Cloned
`ListState` values intentionally share scroll and measurement state, so mount one shared state as
one list.


## Layout direction

`direction(Direction::Rtl)`, or the `rtl()` and `ltr()` shorthands, set the inline layout
direction for an element and every descendant that does not declare its own. Direction resolution
happens once, while the layout tree is built, so it costs nothing per frame.

```rust
div()
    .rtl()
    .flex_row()
    .ps(20.0) // inline start: the right edge here
    .pe(4.0)
    .child(text("مرحبا بالعالم"))
```

What an RTL subtree changes:

- **In-flow positions are mirrored inside the parent's content box.** Row flex order, grid column
  order, and wrapped line placement all read right to left. Mirroring is applied after layout to
  painted geometry, hit regions, and accessibility bounds together, so all three stay consistent
  and intrinsic sizing is untouched.
- **Physical `left`/`right` insets and `ml`/`mr` margins mirror with the content**, which makes
  them resolve as inline start and end. `ms`/`me` are the direction-independent spellings of the
  same two edges.
- **Padding and borders stay physical.** Use `ps`/`pe` and `border_s`/`border_e` for
  direction-relative edges; they are folded into physical Taffy edges during the layout build.
- **The horizontal scroll origin moves to the right edge.** A scroll offset of zero rests against
  the container's right edge, and the offset grows as content to the left is revealed. Physical
  wheel deltas are inverted for those containers so a trackpad still feels natural.
- **Text alignment and shaping follow.** `TextAlign::Start` (the default) resolves to `Right` and
  `TextAlign::End` to `Left`; `text_start()` and `text_end()` declare them explicitly.
  `text_left()`, `text_center()`, and `text_right()` stay physical. The paragraph's base
  bidirectional direction is forced to RTL for the subtree, so neutral characters and punctuation
  resolve against the declared direction instead of the first strong character in the content.

What it deliberately does not change:

- **Focus order still follows document order.** Tab moves through the declaration order of the
  tree, exactly as in a browser.
- **Caret movement in text inputs stays logical.** `Left`/`Right` in an RTL input move to the
  previous and next character in the string, not to the previous and next glyph on screen. Visual
  caret motion for mixed-direction runs is not implemented.
- **Vertical geometry, scrollbars, and `top`/`bottom` insets are unaffected.**

## Sticky positioning

`sticky_top`, `sticky_bottom`, `sticky_left`, and `sticky_right` pin an element inside the nearest
ancestor scroll container while that container scrolls, exactly like CSS `position: sticky`.
`sticky()` marks an element sticky without an offset.

```rust
div()
    .overflow_y_scroll()
    .flex_col()
    .child(
        div()
            .w_full()
            .flex_col()
            .child(div().h(28.0).flex_none().sticky_top(0.0).child(text("Inbox")))
            .children(rows),
    )
```

A sticky element keeps the space it occupies in flow: sticking shifts painted geometry, hit
regions, and accessibility bounds only, so scrolling a sticky header never triggers a relayout and
adds no per-frame allocation. The shift is clamped to the element's parent box, which is what
releases a pinned header when its own section scrolls past — the next section's header takes over
the pinned position. The pinning viewport is the nearest ancestor scroll container's padding box,
or the window viewport when there is no scroll container above it.

One window may retain at most `MAX_STICKY_ELEMENTS_PER_WINDOW` (4096) sticky declarations; a view
that exceeds it is rejected before any layout work happens.

## Scroll snapping

A scroll container opts into snapping per axis with `scroll_snap_x` and `scroll_snap_y`, and its
children declare where they line up with `snap_align`:

```rust
div()
    .overflow_x_scroll()
    .scroll_snap_x(SnapStrictness::Mandatory)
    .flex_row()
    .child(page(0).snap_align(SnapAlign::Start))
    .child(page(1).snap_align(SnapAlign::Start))
    .child(page(2).snap_align(SnapAlign::Start).snap_stop_always())
```

- `SnapStrictness::Mandatory` always lands on a snap position.
  `SnapStrictness::Proximity` only snaps when the container settled within half a viewport (at
  most 200 logical pixels) of one.
- `SnapAlign::Start`, `Center`, and `End` are inline-relative: in an RTL container `Start` is the
  right edge.
- `snap_stop_always()` forbids a gesture from passing over that child: a fling that started before
  it and ended after it lands on it instead.

Snapping resolves **at the end of a scroll**, never during it:

- On platforms that report gesture phases, the native momentum end phase resolves immediately.
- A plain wheel without phases arms one bounded settle deadline (90 ms) that each further delta
  pushes back.
- Releasing a scrollbar thumb resolves immediately, through the same programmatic entry point a
  keyboard or scroll-into-view movement uses.

The container then travels to its target over 220 ms through the existing motion machinery, using
exact deadlines: one deadline for the settle, one for the end of the travel, and none afterwards.
A window that has finished snapping is fully settled and sleeps in `ControlFlow::Wait`. With
animations disabled or reduced motion on, the target is applied in one step instead.

Snap geometry is rebuilt in place by the geometry pass that already walks the tree, bounded by
`MAX_SCROLL_SNAP_CONTAINERS_PER_WINDOW` (256) and `MAX_SCROLL_SNAP_POINTS_PER_WINDOW` (4096). At
most one settle and one travel exist per window at a time, because a window has one pointer.

## Transforms and compositing layers

```rust
use quickgui::{BlendMode, Color, Filter, Transform2D, div, text};

// Paint-only: layout never moves.
div().rotate_degrees(-4.0).child(text("Tilted, text and all"));
div().scale_uniform(1.05).transform_origin(0.5, 1.0);
div().transform(Transform2D::skew_degrees(8.0, 0.0));

// State styles carry transforms too, so a hover lift costs no relayout.
div().hover(|style| style.scale_uniform(1.08));

// Subtree effects.
div().blur(8.0);
div().drop_shadow(0.0, 6.0, 12.0, Color::rgba8(0, 0, 0, 80));
div().rounded_xl().backdrop_blur(12.0).backdrop_filter([Filter::Saturate(1.5)]);
div().blend_mode(BlendMode::Multiply);
```

`transform`, `translate`, `rotate_degrees`, `scale`, `scale_uniform`, `skew_degrees`, and
`transform_origin` are paint-only, exactly like CSS `transform`: the element keeps its untransformed
layout box, and that is what `element_bounds` and anchoring report. Pointer input is inverse-mapped
through the accumulated transform, so clicks, hover, drag, drop, and cursor declarations follow the
painted pixels. The same builders exist on `ElementStateStyle` for `hover`, `active`, `focus`,
`disabled`, `invalid`, `selected`, `dragging`, `drag_over`, `focus_within`, `group_hover`, and
`group_active`. `selected_style` follows the element's `selected` flag and sits above the pointer
states, so a selected list row keeps its colour while hovered, as a native list does.

Anything but a pure translation renders the subtree into a bounded offscreen texture first, as do
`blur`, `drop_shadow`, `backdrop_blur`, `backdrop_filter`, and a non-`Normal` `blend_mode`. Text is
rasterized into that texture, so it transforms, blurs, and blends with the shapes around it. The
bounds — `MAX_LAYERS_PER_FRAME`, `MAX_LAYER_DEPTH`, `MAX_LAYER_TEXTURE_BYTES`, `MAX_BLUR_RADIUS` —
and how an element degrades once one is reached are documented in
[`graphics.md`](graphics.md#compositing-layers).
