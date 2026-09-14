# Layout and rendering architecture

[Architecture index](README.md) · [Documentation](../README.md)

## Invalidation and subtree reuse

The useful comparison is phase-specific invalidation: a state change should restart only the work
that depends on it. Compose documents separate composition, measurement/placement, and drawing
phases, with state reads determining which phases restart. SwiftUI tracks view dependencies and
uses them to decide which views need updating. Neither model implies that every state change can
skip layout, or that only a damaged screen rectangle is drawn.
Sources: [Compose phases](https://developer.android.com/develop/ui/compose/phases) and
[SwiftUI performance](https://developer.apple.com/documentation/Xcode/understanding-and-improving-swiftui-performance).

QuickGUI now retains its Taffy nodes across view declarations, matching them by runtime element
identity. Previously, every declaration cleared the complete layout arena and discarded all of
Taffy's subtree caches. Reconciliation changes only differing layout styles, intrinsic measurement
inputs, and child edges. Taffy propagates dirtiness to ancestors and reuses unchanged branches
under unchanged constraints. A clean layout root with the same viewport and scale skips the layout
call entirely, including the final pixel-rounding traversal. Text foreground, shadows, and
decorations do not invalidate intrinsic measurement. Font metrics, content, inherited direction,
constraints, and display scale still do.

The Go host also translates ordinary text, background-color, text-color, and opacity mutations into
`AppRunner::update_elements` batches. These locate mounted elements through retained parent/index
metadata and change their properties without invoking `View::render` or replacing listeners.
Inherited text color visits only the affected descendants and stops at explicit color overrides.
Text changes update the same measurement, selection, and accessibility state used by ordinary
declarations. A batch validates every target before applying any changes. Callers update their
source declaration first, so later rebuilds preserve the new values.

| Change | Declaration | Layout | Paint |
| --- | --- | --- | --- |
| Equivalent native mutation batch | Skipped; revision unchanged | Skipped | No redraw requested |
| Ordinary mounted text mutation | Skipped | Changed leaf and affected ancestors; clean branches cached | Geometry refreshed; commands recorded |
| Ordinary mounted color or opacity mutation | Skipped | Skipped | Dirty branches recorded; clean subtree commands shared |
| Ordinary structural, layout, or listener mutation | Affected scopes only | Only replaced declarations reconciled; clean measurements cached | Geometry and mount indexes refreshed |
| Compound part, callback-owned subtree, or unsupported target | Full declaration fallback | Reconciled; clean measurements cached | Rebuilt |
| Rust component entity/global dependency | Affected component scopes only | Reconciled if the component returns a new subtree | Updated subtree and affected geometry |
| Rust value binding | Binding callback only | Text: intrinsic layout; colors/opacity/transform: skipped | Affected drawing and, for transforms, placement |
| Hover, selection, retained scrolling, style transition | Usually skipped; virtualization or application callbacks can request a declaration | Retained boxes | Clean static subtrees reused when their paint and geometry inputs match |
| Viewport resize | Skipped unless the view observes viewport geometry | Recomputed for changed constraints | Rebuilt |

Removed nodes and their measurement contexts are released after container-query expansion
converges. Child edges are detached before reparenting to avoid transient cycles. Unsupported
targeted updates, callback-owned subtrees, and component parts fall back to the declaration path;
they cannot silently freeze derived state. Forwarded events wake the host queue without requesting
a speculative declaration before Go handles them. Native redraw requests remain coalesced once
per mutation batch, while core-owned interaction paint still runs when needed.

Natural bounds and scroll-snap geometry now have a lifetime separate from painting. Color-only
changes reuse them and the retained hit, dismiss, and selection regions. The hover refresh after
layout shares its natural-bounds pass with painting. Editable controls and state transforms refresh
interaction geometry when their state can move a caret viewport or change hit coordinates.

Substantial static subtrees retain immutable drawing-command chunks. A changed element invalidates
its ancestor path, while inherited style or group changes also invalidate affected descendants.
An unchanged panel shares its command chunks with the next scene instead of copying its primitives.
Callbacks, native views, animation owners, and compositing groups stay dynamic; surrounding static
branches can still be cached. A conservative 32 MiB command budget and 256 subtree entries bound
retention. Removing a subtree releases its cached references. Chunk ordering preserves stacking
order, though separate chunks can require more draw batches than a newly combined display list.

Rust views can opt into `ViewContext::component(id, callback)`. Entity and global reads inside a
component subscribe that scope, with reverse dependency indexes routing changes directly to its
observers. Nested scopes record their own reads. A dirty parent subsumes dirty descendants in the
same batch; removed scopes release callbacks, detached subscriptions, and dependency edges. Mouse,
key, and action listener keys remain stable across unrelated updates and retired keys never alias
new callbacks. Root-level observations, explicit view invalidation, and environment/focus changes
retain the full-declaration behavior.

`ViewContext::bind(element, callback)` attaches a direct property binding to an explicitly identified
element. Its callback returns `ElementUpdate::Text`, `BackgroundColor`, `TextColor`, `Opacity`, or
`Transform`. Only the binding reruns when its dependencies change. Text updates invalidate intrinsic
measurement and accessible text; colors and opacity require paint; transforms refresh hit geometry,
placement, and accessibility geometry without layout. A binding cannot declare listeners or child
components. `component` callbacks return an unkeyed root, or a root with the component's own ID;
nested components belong in its children.

Embedding renderers use `with_scope`, `View::render_scope`, and `AppRunner::invalidate_elements`.
The Go native host keeps direct text/color/opacity updates and uses scoped declaration for ordinary
layout, property removal, listener, insertion, removal, and keyed-move batches. Both parents of a
move are invalidated, and all old scope listeners are retired before any new scope is declared.
Root child-list changes, compound parts with derived identities or hoisted portals, and callback-
owned declarations use the established full-declaration path. Asynchronous image replacement also
uses a complete image-resource frame so removed loaders do not remain active indefinitely.

`ElementUpdate::Replace` reconciles only the replacement subtree with the retained layout arena.
Sibling declarations and measurement caches survive. Structural batches still validate window-wide
identity/limit constraints and refresh mount indexes (focus, dispatch, input, selection, scroll,
animations, and accessibility) against the final tree. Those scans are not proportional only to the
changed nodes. Batches with overlapping replacement roots are rejected for the source renderer to
coalesce or rebuild; disjoint replacements may move a keyed child between parents.

See the [optimization coverage](rendering-optimization-plan.md) for validation and remaining
performance boundaries. There is no GPU damage-region renderer or assumption that swapchain pixels
survive presentation.

Regression tests compare retained layout with a fresh layout after updates, reordering,
reparenting, resizing, query expansion, and unmounting. Work counters assert that updating one
label measures none of the 100 text leaves in an unchanged sibling panel, and that identical or
paint-only declarations perform no additional layout pass. Core and native-host tests also assert
that targeted updates preserve click delivery and accessible names without incrementing the view
render count.

`FrameMetrics.pipeline` exposes elapsed phase timings and work counters, including mutation work
received before redraw. `RenderStats` records tracked primitive buffer uploads and shadow storage,
and group passes skipped by pixel reuse. A test-only allocation probe verified that hovering a
toolbar button visits two paint nodes, resolves no natural geometry, and allocates 3,040 bytes in
nine allocations with either 100 or 10,000 unrelated mounted nodes. This is a UI-tree measurement;
renderer preparation and GPU submission have their own costs.

## Rust dependency bindings

A text binding can update a counter without calling the containing view's `render` method:

```rust
use quickgui::{button, div, text, ElementUpdate, Entity, IntoElement, View, ViewContext};

struct Counter {
    count: Entity<usize>,
}

impl View for Counter {
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        let count = self.count.clone();
        let label = cx.bind(text("").id("count"), move |cx| ElementUpdate::Text {
            id: "count".into(),
            content: cx.observe(&count, |value| value.to_string().into()),
        });
        let increment = cx.listener("increment", |view, cx| {
            view.count.update(cx, |value, _| *value += 1);
        });
        div().flex_col().child(label).child(
            button().on_click(increment).child("Increment"),
        )
    }
}
```

Use `cx.component("details", move |cx| { ... })` when the dependency changes the child structure.
Capture an `Entity` handle in the callback and read its current value with `cx.observe`; capturing
only a copied value would keep the callback tied to that old snapshot. The existing Go signal API
already emits native property/structural mutations and needs no application syntax changes.

## Layout

Taffy implements Flexbox, CSS Grid, and intrinsic measurement. Grid containers accept the
GPUI-compatible equal-track helpers plus explicit fixed, percentage, fractional, intrinsic,
fit-content, and `minmax(px, fr)` tracks. Equal columns or rows remain one compact `repeat()` style
component; explicit templates, spans, and line positions are clamped to 1,024 tracks per axis
before Taffy sees them. This bounds layout work from dynamic application data well below Taffy's
own 10,000-track safety limit without adding a second layout representation.

Container queries are Taffy leaves in the parent's layout graph. Their callback subtree uses nodes
from the same bounded arena but remains deliberately disconnected, then receives a separate root
layout with the query leaf's definite width and height. This matches web/GPUI intrinsic-size
isolation without maintaining a second geometry tree. Nested callbacks resolve one level at a time
with a 16-level convergence cap; 1,024 mounted queries bound callback and node work per window.
Unchanged assigned sizes retain their subtree, while a changed size remounts only its declaration
and preserves keyed motion state. Paint, hit testing, focus, accessibility, clipping, and scrolling
continue to walk the ordinary element tree, so they need no query-specific parallel indexes.

Text leaf measurement calls the renderer's shared Cosmic Text cache, so ordinary layout and paint
do not shape the same string twice. Standard ellipsis uses Cosmic Text's own Unicode-aware line
layout against the original buffer. Only a custom replacement affix needs a grapheme-safe visible
projection; later layout, paint, hit testing, selection, and decoration geometry share that result. Typography is inherited during tree construction; hover and active
variants are deliberately paint-only so a pointer move cannot trigger relayout. Opted-in style
transitions retain their current and target paint values by runtime element identity; every active
frame reuses the same boxes and interpolates fixed-size color, border, radius, opacity, text-color,
and at-most-eight-shadow state. Grid participates in the same retained layout boundary: resize or
changed layout inputs recompute it, while scrolling, selection, hover, and other paint-only changes reuse
its boxes and schedule no idle frames.

Scrollable nodes retain offsets outside the declaration tree. Their vertical overlay scrollbar has
a 12-point invisible hit track, a 4-point revealed thumb, and an 8-point hover/capture thumb. The
topmost eligible track captures both thumb and track presses, keeps capture outside its bounds, and
cannot click through to content. Scroll motion reveals it; leaving or releasing schedules one hide
deadline. Pointer motion that does not cross a hover boundary schedules no redraw.
Overflow clips descendants to the padding box, inside the border. Painting, geometry-only hit
testing, and inspector traversal share that boundary, so scrolled content cannot overwrite a pane's
divider or receive pointer events through it. Sticky positioning uses the same padding-box viewport.
Rounded parent backgrounds do not implicitly mask descendants. Edge-reaching gutter backgrounds
use matching inset radii; virtual row backgrounds anchor those radii to the stationary scroll
viewport. Scrolling code clips separately from its frozen gutter. Explicit rounded text masks
are carried in glyph instances and evaluated after rasterization, without reshaping or allocating
offscreen surfaces. Solid highlight and selection fragments reuse rounded quads with rectangular
slice clips. This keeps the corners intact without adding a blank content inset.

Fixed-height `VirtualList` is an O(1) range calculator for very large data sets; it never allocates
in `visible_rows()` and mounts only viewport rows plus configured overscan. A viewport bound with
`Element::virtual_scroll` shares its offset through one lock-free handle and enters the same
retained scrollbar path as an ordinary overflow container. Wheel and drag motion mark the view
dirty only when the offset changes; hover expansion and autohide remain paint-only.

Differently sized content uses `ListState`. Unmeasured items contribute one uniform height
estimate; real Taffy heights are retained in sparse 64-item blocks with a compact Fenwick prefix
over blocks. This keeps range lookup logarithmic, anchor correction narrow, and an unmeasured
million-item list's metric index near 64 KiB instead of allocating one record per item. Fully
measured state and mounted items have explicit count caps. Visible items remain a normal-flow
Flexbox column, avoiding overlap while estimates converge. A measurement revision requests one
correcting view rebuild and then sleeps. Width changes and targeted remeasurement preserve the
logical top item and inset. During a captured scrollbar drag, the estimated content extent is
frozen so measuring overscan cannot move the thumb under the pointer.

Hosted tables supply rows asynchronously. Their `ListState` tracks the supplied contiguous range
separately from the requested scroll destination, prefetches a bounded viewport of rows on either
side, and requests a refill when roughly half a viewport of buffer remains. If input outruns supplied
content, painting, hit testing, and accessibility keep the viewport within the mounted rows until the next
frontend mutation batch arrives. Input and scrollbar position continue tracking the requested
destination. Only the bounded window is mounted, and waiting for rows creates no polling timer.

## Rendering

Paint traversal carries one scalar opacity and restores it after each subtree. A child multiplies
its local value into the inherited value once; scene insertion then applies that effective alpha to
every primitive. Images and custom shaders carry it in their existing instance upload, native
AppKit hosts receive it as `alphaValue`, and the bounded Glyphon support fork adds it to glyph
vertices after rich-run color resolution. Opacity changes therefore do not affect layout keys,
shaping, raster identities, tessellation, batching, or idle scheduling, and require no offscreen
subtree texture.

Compositing layers extend that paint traversal with the one thing scoped opacity deliberately
avoids: an offscreen subtree texture. An element that declares a transform beyond a pure
translation, a `Filter::Blur` or `Filter::DropShadow`, a backdrop effect, or a blend mode other than
`Normal` becomes a *compositing group*. `Scene::begin_group` records the group's box, the clip that
applies to its composited result, and its effect parameters, then hands back the layer key its
subtree paints into. That key is `(plane, z_index, group)`: because `group` is the least
significant term, a group still sorts against its siblings by its own `z_index` instead of floating
above them, while every descendant layer — including a descendant's own `z_index` layer — stays
distinguishable from the parent's, which is what keeps a CSS stacking context's contents inside it.
The composite itself is one more primitive in the parent layer's cross-primitive paint order, so it
interleaves with siblings exactly as a quad would. Ancestor opacity is captured by the group and
applied once to the composited result rather than to each primitive.

`src/renderer/compositor.rs` owns the pass sequencing, and both `gpu.rs` and the headless
`offscreen.rs` call the same `Compositor::render_scene`, so a screenshot test exercises the
production path. It plans a frame in three phases. First it claims textures for each group,
deepest first, from a bounded least-recently-used pool; a group that cannot be served drops its
effect. Then it builds every composite and blur draw and flattens each target's layers into an
ordered step list. Finally it records passes: each group's own pass (clearing to transparent),
then its two separable Gaussian passes, and last the target's pass, which is split wherever a step
needs the destination copied first.

Group textures are allocated at the window's full physical resolution. Text is prepared by Glyphon
at absolute window coordinates and its vertex buffers are built before any pass begins; the shape,
image, SVG, path, and application-shader renderers likewise bake one window-sized projection into a
shared uniform. A group-sized texture would require every one of them to learn a per-layer origin,
so the compositor pays in memory instead and keeps the entire per-frame preparation path unchanged
— which is also why text rotates, blurs, and blends with its parent for free. `MAX_LAYER_TEXTURE_BYTES`
(128 MiB per window) bounds the cost, `MAX_LAYERS_PER_FRAME` (8) and `MAX_LAYER_DEPTH` (4) bound the
count, and `MAX_BLUR_RADIUS` (64 logical pixels) bounds the convolution, whose support is three
standard deviations evaluated in at most 48 strided taps per axis. Exceeding any bound paints the
subtree directly into its parent without the effect and reports it in
`RenderStats::skipped_layer_effects`, alongside `compositing_layers`, `layer_passes`, `blur_passes`,
and `layer_texture_bytes`.

A scene with no groups records exactly the passes and draws it always did: `render_scene` takes a
zero-group fast path that compiles no pipeline, allocates no texture, and adds no pass.

Group textures also retain their pixels. Immutable command snapshots distinguish raw content from
the group's own transform, opacity, and filter parameters. Rotating or fading stable content reuses
the raw texture; an unchanged blur reuses its blurred texture too. Content, nested effects, scale,
texture repurposing, and effect fallback invalidate the relevant result. Snapshots have a separate
32 MiB command-storage bound and hold only weak texture references, leaving GPU ownership and
eviction with the existing pool. This skips group draw/blur passes; primitive preparation can still
visit the scene. Headless tests compare these paths pixel-for-pixel with fresh rendering.

Backdrop filters and destination-reading blend modes need the target back. The window surface is
configured with `COPY_SRC` when the adapter advertises it; the whole target is then copied into a
shared scratch texture between passes. `Normal` and `Screen` reach their exact result through
fixed-function blend state and never copy; every other blend mode evaluates the separable
Porter-Duff form in premultiplied colour from the copy and writes the final result with a replacing
blend state, so it is exact over transparent destinations too.

Hit testing follows paint. `LayoutFrame` carries the accumulated window-space transform of the
enclosing groups, each `HitRegion` records it, and a pointer position is inverse-mapped through it
before the region's untransformed bounds and clip are tested. Layout itself is never transformed,
so Taffy, measurement, anchoring, and reported element bounds are unchanged; a pure translation is
folded into the painted box instead of opening a group at all.

The current renderer has six specialized primitive renderers. Windows using the same performance
profile share a compatible WGPU instance, adapter, device, and queue; a surface-incompatible window
falls back to its own context. Compatible windows also share the immutable format-matched shape
pipeline and Glyphon's device cache for its shader, layouts, sampler, and format-specific text
pipelines. Surface configuration, uniforms, upload buffers, glyph atlases, text-layout state, and
bounded asset caches remain per-window, so one window's working set or resize cannot invalidate
another. Path, image, SVG, and application-shader renderers are created only when a scene first
uses that primitive family; an ordinary shape-and-text popover never compiles or allocates those four
unused paths:

1. Shapes use six shader-generated vertices and one declaration-ordered instance stream. Quads carry logical bounds, fill, inside border, four corner radii, an optional gradient index, a border-style code, and clip. Multi-stop linear, radial, and conic gradients live in one read-only storage buffer indexed per instance and are resolved against the primitive's own rectangle on the CPU, so a gradient adds no texture, ramp cache, or draw call; at most 4,096 gradients are uploaded per frame in scene order and a shape past that bound falls back to its solid fill. Dashed and dotted borders measure arc length along the rounded outline analytically -- straight edges exactly and corners as exact quarter arcs -- and scale their declared period so a whole number of repeats closes the outline. Outlines are one additional instance whose rectangle and radii are the border box grown by the outline offset and width, so they never touch layout. Drop and inset shadows carry the element box, translated/spread subject box, color, blur radius, and clip. Rounded-corner antialiasing and Gaussian-CDF shadow falloff are analytic in WGSL, so shadows allocate no blur textures or retained cache entries. Outer shadows, the element quad, and inset shadows preserve CSS paint order while every non-empty stacking layer remains one draw call.
2. Images use six shader-generated vertices and one shared instance upload. Element background images reuse this primitive: tiles are generated only for the visible intersection of the element and its clip, masked by the element's rounded rectangle, and capped at 256 tiles per element, so a raster background creates no pipeline, pass, or cache of its own. Straight-alpha RGBA8 pixels are sampled from sRGB textures, converted to premultiplied sRGB output in the shader, and masked by the same logical clip and rounded box used for layout. A bounded chain of at most eight CSS color filters collapses on the CPU into one 4x5 matrix carried in the 160-byte instance, so filter count never affects GPU work and no offscreen group texture is allocated; matching CSS, the matrix is applied to encoded sRGB. Consecutive primitives with the same image identity share one draw call; texture switches preserve image source order.
3. SVGs retain a parsed `resvg` tree and rasterize only an identity-and-physical-size cache miss.
   The GPU stores `R8Unorm` alpha masks, while inherited tint, rounded clipping, translation, and
   rotation remain per-instance shader data. Scale participates in the raster key to preserve edge
   quality. Consecutive primitives with the same mask share a draw without making color part of the
   cache identity.
4. Paths retain CPU-tessellated Lyon triangles. Each uploaded vertex carries barycentric coordinates,
   a true-boundary mask, and a paint index; WGSL derives edge coverage only for outline edges, so
   internal triangulation stays opaque without a permanent multisample framebuffer. One 240-byte storage
   paint record supplies scale/translation, clip, a solid color or the same bounded eight-stop
   linear/radial/conic gradient used by quads, and linear-sRGB/sRGB/Oklab interpolation. All paths at one overlap order share one draw.
5. Application shaders retain validated WGSL fragment functions behind a framework-owned vertex
   stage and fragment wrapper. Rect, physical clip, and four `vec4<f32>` parameter slots are
   per-instance data; shaders sharing one retained identity batch at an overlap order. The wrapper
   enforces clipping and converts linear straight alpha to premultiplied sRGB output.
6. Text uses Cosmic Text/HarfRust for shared Unicode shaping and fallback, platform rasterizers for glyphs, and Glyphon for retained atlas rendering. One main-thread font database, including validated application fonts, is shared across windows; layout buffers, glyph atlases, and eviction caches remain window-local. Stable `TextId`s prevent reshaping unless content, relevant width, metrics, family, canonical OpenType features, ordered custom fallbacks, weight, alignment, wrap/overflow/clamp mode, or display scale changes. The selected primary family and up to eight precomputed custom families are tried in declaration order before platform/script fallback without allocating a fallback vector per shaped word. Left-aligned no-wrap text deliberately omits paint width from its key unless overflow replacement needs it; center, right, justified, wrapped, and truncated text retain width so painting, hit testing, caret placement, and selection use identical line origins. Standard end/start/middle ellipsis and line clamps operate directly on the complete shaped source, preserve Unicode offsets, and reflow in place during resize. Custom affixes use a grapheme-safe projection with explicit source mapping; a uniquely owned superseded-width projection is dropped immediately rather than accumulating through a resize burst.

Public colors and gradient interpolation remain in linear-light space. Eight-bit constructors decode sRGB on the CPU. Primitive shaders encode straight RGB before premultiplication, and the native surface and retained group textures use BGRA/RGBA Unorm. All UI source-over, text coverage, opacity, and blend modes therefore composite in encoded sRGB, matching native UI toolkits. Color images keep their sRGB source textures; application shaders continue returning straight linear colors. Transparent windows already contain the compositor-correct premultiplied encoding and need no conversion pass. Text wraps at word boundaries by default; intrinsic measurements reserve one physical pixel before a max-content width is fed back as a wrap constraint, preventing rounding-only reflow between layout and paint.

The default macOS sans-serif resolves to the system UI family, even if Open Sans is installed.
Text uses automatic optical sizing, as in the web's `font-optical-sizing: auto`: the logical
font size selects the variable font's `opsz` outlines and font-provided tracking. Display scale
only changes raster resolution. Shaping, basic glyph advances, decoration metrics, and rasterization use the same variation coordinates. Optical size participates in shaped
run and glyph atlas keys, so a 14-point label at 2× cannot reuse a 28-point display glyph at 1×.
Font instances for size-dependent faces are shared across windows and capped at 128 entries;
ordinary static faces reuse their existing font instance. CoreText comparison tests cover the
default family's widths at six sizes, three weights, and both 1× and 2× scale.

Glyph rasterization follows [GPUI on macOS](https://github.com/zed-industries/zed/blob/main/crates/gpui_macos/src/text_system.rs), [Windows](https://github.com/zed-industries/zed/blob/main/crates/gpui_windows/src/direct_write.rs), and [Linux](https://github.com/zed-industries/zed/blob/main/crates/gpui_wgpu/src/cosmic_text_system.rs):

- macOS uses CoreText with fractional positioning and alpha-only grayscale masks. Five foreground-luminance smoothing variants are included in the atlas key, honoring an explicit disabled AppleFontSmoothing preference. Glyph opacity stays independent of smoothing. Color bitmap emoji use CoreText RGBA; mixed COLR fonts retain Swash's per-glyph color handling.
- Windows uses DirectWrite's recommended rendering/grid-fit modes and grayscale glyph-run analysis. In-memory font files are reused across variable-font instances and capped at 32 MiB with 128 face instances. Unsupported DirectWrite versions and color fonts fall back to Swash.
- Linux uses Swash with hinting. Windows/Linux grayscale masks use GPUI's DirectWrite-style gamma 1.8 and contrast 1.0 treatment. macOS uses its native masks directly.

Native rasterization happens only on atlas misses. CoreText caches at most 128 logical font instances and reuses bounded scratch contexts; changing display scale reuses its logical font. The shared shaping, wrapping, selection, hit-testing, and retained-buffer paths remain unchanged. Regressions cover logical optical size, real native mask selection, context/cache reuse, grayscale/color glyphs, arbitrary opacity, and compositing of every primitive family. No heavier default font weight, guessed background, or extra text render pass is used.

Shape instance uploads rotate across three GPU buffers. Capacity grows geometrically. A shape
instance is 128 bytes, the gradient storage buffer rotates across the same three frames and holds
192-byte records of at most eight stops, and shadow overdraw is clipped to the viewport and limited
to three Gaussian standard deviations. Text layouts are age-evicted every frame and have a hard cap of 256 retained
areas; Glyphon's atlas is trimmed after presentation. The deliberately small layout cache keeps
long, disjoint scrolling from retaining whole off-screen text buffers while still covering several
nearby viewports.

Image instances also rotate across three geometrically growing buffers. Decoded CPU images are
immutable `Arc` allocations, so cloning an `Image` preserves cache identity without copying pixels.
Only viewport-visible image identities are admitted for upload. The per-renderer GPU cache is capped
at both 256 textures and 128 MiB of RGBA texels; an upload evicts the least-recently-used identity
that is not visible in the current frame. If one visible working set itself exceeds either cap,
admission is deterministic in scene order and later images remain unsubmitted rather than allowing
unbounded residency. Cache bytes, image count, uploads, and total draws are exposed in
`RenderStats`.

SVG parsing accepts at most 4 MiB of source or bounded SVGZ output and loads the system-font
database only if the source may contain text. SVG `<image>` references are disabled, keeping the
asset path vector-only. A raster target is derived from the fitted source UVs, display scale, and
render scale, then constrained to 4096 px per axis and 16 million pixels. Only visible unique keys
are admitted in scene order. The per-renderer cache retains at most 512 one-channel textures and 32
MiB, evicting the least-recently-used key outside the visible working set. First-use rasterization
is synchronous; cache hits perform no SVG parsing or CPU raster work, and recoloring never uploads a
new texture.

`PathBuilder` validates coordinates and styles before tessellating fills or strokes. A retained path
is immutable and shared by identity; it owns de-indexed triangles plus one three-bit boundary mask
per triangle. Commands are capped at 65,536, retained vertices at 196,605, dash expansion at 262,144
segments, and retained geometry at 4 MiB. The renderer rotates both vertex and paint uploads across
three geometrically growing buffers. Frame admission follows source order and is capped at 262,144
vertices and 16,384 paints; later paths are omitted rather than growing transient memory without a
bound. Declarative path elements use intrinsic Flexbox measurement and all five `ObjectFit` modes.
Canvas callbacks receive local bounds and a scoped painter whose commands are translated and clipped
to the element before entering the same retained scene.

`CustomShader::new` composes an application `quickgui_fragment` function with QuickGUI's fixed
interface, then runs Naga parsing and validation before the asset enters retained state. Bind-group
resources, pipeline overrides, and application entry points are rejected. The source is capped at
64 KiB and stores one shared immutable allocation. Pipeline compilation is lazy under a WGPU
validation error scope; each window retains at most 32 pipelines and evicts the least-recently-used
identity outside the visible working set. Admission keeps at most 4,096 visible rectangles per
frame in source order. Rectangles use one 96-byte instance record, one upload, and three rotating
geometrically-grown buffers, with no per-effect bind groups or intermediate textures. Custom
primitives enter the same `(plane, z_index)` layers and overlap-order stream as every other paint.
No time uniform or redraw source is implicit, so static effects add no idle CPU work. WGSL remains
trusted application code because validation cannot prevent a deliberately non-terminating GPU
program from resetting the device.

Asynchronous image resources are resolved before Taffy layout. Filesystem paths share a stable path
key, application assets share their source identity plus normalized path, and custom loaders use a
retained handle identity. The first resource in the application lazily creates
two sleeping decode workers backed by one shared 64-job synchronous channel. Every job and
completion carries its owning `WindowHandle`; completed decodes return through the event-loop
proxy, so only that clean window wakes once rather than polling worker state. Loading and fallback closures
produce ordinary child elements and therefore participate in the same flexbox and text-wrapping
rules as the rest of the tree. The per-window CPU cache independently caps all resource states at
256 entries and decoded residency at 128 MiB, evicting least-recently-used entries outside the
current tree. Each file is capped at 64 MiB encoded, and every decoded image retains the existing
4096 px per-axis and 64 MiB allocation limits. Worker panics become failed resources, and teardown
detaches instead of waiting indefinitely on application-provided blocking loaders.

`ViewContext::spawn_background` uses a separate application-wide two-thread pool with a 64-job
synchronous queue. Work and its type-erased completion are tagged with the owning `WindowHandle`;
one user event wakes that window and downcasts the callback to its concrete view type. A full queue
fails the spawn synchronously, worker panics become typed completion errors, and idle workers
sleep. Neither image loading nor blocking application work consumes event-loop frames while
pending. `spawn_blocking` is an explicit alias.

Animated GIF/WebP resources decode their composited RGBA frames on the same bounded workers.
Direct `AnimatedImage` values and loaded animations share one immutable representation capped at
256 frames and 64 MiB of unique decoded pixels; repeated `Image` identities are counted once. Frame
delays are clamped to 16.667 ms so malformed assets cannot force more than 60 presentations per
second. Playback state is keyed by the element's stable runtime ID rather than the asset, allowing
two uses of one animation to retain independent phases across view rebuilds. Only elements that
intersect the current clip remain active. Occlusion, explicit motion reduction, and macOS's system
Reduce Motion preference pause deadlines while preserving the current frame. Finite loop metadata
stops on the final frame. A delayed event advances elapsed time arithmetically and performs at most
one repaint, rather than iterating through every missed frame.

The scene groups primitives by `(plane, z_index)` and retains at most 16 inactive layer buffers.
Every accepted primitive also enters one global paint stream. An allocation-reusing R-tree assigns
each primitive one greater than the maximum order of intersecting earlier bounds; primitives sharing
an order are provably disjoint and may batch by pipeline or texture without changing pixels. The
renderer walks orders globally across shapes, paths, images, SVGs, and text, preserving web sibling
overlap while avoiding one draw per primitive. Effective bounds include clips, masks, and render
transforms. Every overlay layer remains after every base layer. Glyphon renderers are shared by
text-bearing order batches and capped at 32 retained instances.
An ordinary window still uses one WGPU surface. A macOS window with native children creates its
transparent overlay surface only when overlay content or input first becomes active, then retains
that swapchain for reuse. Recreating it on every close/open cycle is intentionally avoided because
Core Animation may keep old IOSurface drawables alive beyond the Rust `Surface` lifetime.

On macOS, QuickGUI creates and retains each `CAMetalLayer` itself, then gives WGPU the documented
`CoreAnimationLayer` surface target. AppKit live-resize callbacks only record the newest physical
size; WGPU reconfiguration is deferred to the next coalesced redraw immediately before drawable
acquisition. During AppKit's live-resize transaction, the owned base and overlay layers enable
`presentsWithTransaction`; ordinary frames disable it again. This keeps resize presentation in the
window-server transaction without reaching through WGPU's backend internals or configuring a
surface repeatedly inside `frameDidChange:`.

Opaque, transparent, and blurred window backgrounds share those retained layers and render
resources. On macOS, a runtime opacity transition updates `CAMetalLayer.opaque` in place instead of
reconfiguring or recreating the WGPU surface; the matching `SurfaceConfiguration` value is retained
for the next size-driven configuration. Native window opacity and blur are changed independently,
so moving between transparent and blurred modes does not disturb the active drawable chain. Blur
is supplied behind the alpha-capable layer by the native compositor and allocates no QuickGUI
texture, pipeline, timer, or frame loop.

On macOS, a single opaque AppKit launch shield uses the configured background color and stays above
all three planes until the base renderer completes `Presented`. Surface retries and occlusion keep
the shield mounted. Before ordering the window onscreen, QuickGUI runs view declaration, Flexbox,
text shaping, scene construction, native reconciliation, and GPU buffer preparation. A hidden Metal
surface is occluded before a render pass can execute, and WGPU intentionally refuses to acquire its
drawable. For the first surface submission only, QuickGUI retains and detaches the Winit content
view from the still-hidden `NSWindow`. Its CAMetalLayer then has no hosting window, so WGPU can
acquire, render, present, and await the actual surface drawable without exposing any AppKit window.
RAII reattaches the same content view to the same window at its unchanged frame after completion;
only then is the window ordered onscreen. This avoids partial native content, black launch covers,
and cross-monitor movement without polling. Failure to produce that hidden first surface frame is a
startup error rather than permission to reveal a partial window. Later occlusion sleeps normally.
If a view requests another animation frame while its surface is occluded, QuickGUI retains the
view damage but does not queue a redraw; the next platform-driven exposed frame resumes the view.
This also preserves frame requests across the detached first-present pass without creating an
occluded-window polling loop.
Winit APIs that resolve the window's current content view, such as IME cursor placement, are
skipped only during that detached preparation pass and resume after RAII reattaches the same view.
The declarative `WindowState` snapshot uses the retained initial mode for that one pass instead of
querying a temporarily absent AppKit content view.
