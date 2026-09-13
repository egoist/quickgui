# Changelog

All notable user-facing changes to QuickGUI are recorded here.

## Unreleased

### CLI

- `resources` and staged `fonts` are now copied beside the executable on Linux and Windows
  (AppDir, `.deb`, NSIS, and development builds), matching macOS `Contents/Resources`. Directory
  payloads are copied recursively instead of being treated as a single file.
- `linux.icon` is used as the Linux desktop PNG source when the top-level `icon` is omitted.
  It must be a file, matching the top-level `icon` check.
- Generated packaging names (`*.AppDir`, `*-setup.exe`, `quickgui.json`, and similar) are reserved
  so a `resources` entry cannot overwrite them. Update manifests select AppImage and NSIS artifacts
  from packaging outputs only. Debian `md5sums` omit directory members.

### Website

- Added App Icon and Bundled Resources guides for Go, TypeScript, and Rust.

## 0.1.4-next.4 - 2026-09-13

Same user-facing cut as 0.1.4-next.3. This version is the complete `latest` npm
set; `0.1.4-next.3` could not move `@quickgui/native` onto `latest` after that
package was already published.

## 0.1.4-next.3 - 2026-09-13

### CLI

- Shipped project templates are now `.tmpl` files. After `bun install`, Go, bun, and Cargo
  no longer treat `node_modules/@quickgui/cli/templates` as packages or modules.
- Native resource bundles now size the gzip envelope separately from decoded content. A
  framework at the 128 MiB content limit packs and unpacks, and an oversized envelope is
  rejected before any output is written.
- Published npm packages now ship Linux and Windows host libraries beside the macOS
  images, so `quickgui` installs on those platforms instead of being rejected as
  darwin-only. The Windows terminal image links Ghostty's static archive instead of
  the DLL import library, so `quickgui_terminal.dll` can link on MSVC.

### JavaScript

- `useParams`, `useLocation`, and `useSearchParams` now return store-like objects, so
  `params.id` and `location.pathname` work as they do in Solid Router instead of requiring
  accessor calls.
- Destroying the native application now rejects pending app-service requests. Those
  promises previously stayed pending after cleanup.

## @quickgui/cli 0.1.4-next.2 - 2026-09-13

### CLI

- `quickgui dev` now streams Cargo progress, warnings, and compiler diagnostics while building
  Rust applications, without exposing the machine-readable output used to locate the executable.

## 0.1.4-next.1 - 2026-09-13

### CLI

- The TypeScript runtime, universal renderer, and JSX compiler now use Solid `2.0.0-rc.8`
  together. Fresh installs could pair the rc.7 runtime with rc.8 Signals and crash `quickgui dev`
  while Solid's development diagnostics initialized.

## 0.1.4 - 2026-09-13

### CLI

- `@quickgui/cli` now builds native Rust applications with `cargo`. Rust apps link the `quickgui` crate and do not bundle `libquickgui_host`. `quickgui init --language rust` scaffolds a crate; `fmt`, `check`, and `test` dispatch to Cargo. Packaged identity and fonts are written to `quickgui.json` beside the executable, or in `Contents/Resources` on macOS.
- The application language option is `language` (`--language`, `language = "go"|"rust"|"typescript"`). The previous `frontend` name remains accepted as an alias.

### Framework

- Added `Element::selected_style`, a paint-only state that follows the element's `selected` flag
  the way `invalid_style` follows `invalid`. It sits above the pointer states and beneath
  `disabled_style`, so a selected list row keeps its selection colour while hovered or pressed,
  as a native list does.
- Added `TableState::element_with_rows`, which hands the caller a `TableRowState` for every
  mounted row and lays the row's cells out inside the container the caller returns, so a table
  row can declare its own background, divider, hover state, and `selected_style`.
- A `TableLayout` header height of zero now mounts no header row at all, for a list with nothing
  to label; it used to be clamped up to a blank 20-pixel band.
- Text on macOS is rasterized unhinted, as CoreText does, so stems keep their designed weight
  instead of snapping to the pixel grid; hinted glyphs read as heavier, "shadowed" text next to
  native controls. Other platforms are unchanged.
- A declared native `services` menu item no longer aborts the process. AppKit lets one menu hang
  from one item only, so when Winit's own application menu already holds the services menu, the
  declared item registers a fresh menu as the application's services menu instead of re-parenting
  the existing one, which raised an Objective-C exception the Rust runtime could not catch.
- Text and other anti-aliased edges over a translucent window no longer carry a light fringe. The
  renderer blends in linear light and wrote premultiplied linear coverage into the sRGB surface,
  but the compositor multiplies alpha in the encoded space, so every partially covered pixel came
  out brighter than its neighbours. A transparent window now renders into an intermediate and a
  final pass re-encodes each pixel as the compositor expects; opaque windows are unchanged and
  pay nothing.
- A window command queued between `open_window` returning a handle and the platform window's
  creation on the next event-loop turn now waits for that window instead of failing with "this
  context is not attached to a native window". A hosted app that sets the title or represented
  file of a window it just opened no longer has to know about that gap.
- A SwiftUI host now measures and lays out at its controls' native size. The host used to pad
  every subtree by 24 points on each side as headroom for Liquid Glass effects, and that padding
  was charged to layout, so a hosted text field or button sat inside a ring of empty space and
  pushed its neighbours apart. The headroom is now a few points for ordinary controls, enough for
  a bezel and its focus ring, 24 points only when the content contains a glass control, and it is
  expressed as a native-view *outset*: `native_view_with_outset` and `MacNativeView::with_outset`
  grow the AppKit frame past the element's layout box without moving siblings, and points in that
  ring that hit no hosted content fall through to the framework.
  A host laid out under a transparent titlebar also no longer has its controls pushed below the
  window's safe area, which grew the host on every build until its content sat at the bottom of
  a toolbar-height box.
- A hosted AppKit view that takes keyboard focus now blurs the framework's focused element. The
  element kept its focus ring and caret, and its listeners never heard about the change, while a
  SwiftUI text field received every key; the runtime now watches the window's first responder and
  drops its own focus, announcing it to the view, whenever AppKit owns it, whether the move came
  from a click, Tab, or the control's own focus state.
- A native popup menu given a position now opens there. The runtime measured the requested
  top-left point from the bottom of the view, but Winit's content view is flipped, so a menu
  asked to drop down from a control near the top of the window appeared near the bottom.
- A custom traffic-light position now survives the window's first appearance. AppKit lays the
  titlebar out again in the display pass after a window comes onscreen, and again when it becomes
  key or leaves full screen, each of which reset the controls to the default inset until the next
  resize; the runtime now re-checks the layout on those transitions and on the window-update
  notification that follows every event cycle, touching AppKit's views only when they have moved.

### JavaScript

- Native modules: a `modules/<name>/main.zig` next to the application is compiled by
  `quickgui dev`, `quickgui build`, and the new `quickgui modules` command into a Node-API addon
  that Bun embeds in the executable, and `modules/<name>/index.ts` is generated so every `pub fn`
  is an ordinary typed import, available both as a synchronous call and as an `…Async` variant
  that runs on the host's native thread pool. Scalars and strings cross directly, structs, slices,
  optionals, enums, and tagged unions travel as JSON with named TypeScript types generated from
  the Zig declarations, an `Allocator` parameter is an injected per-call arena, and a Zig error
  becomes a thrown `NativeModuleError`. The runtime lives in `@quickgui/native/zig` and
  `@quickgui/native/modules`; a `modules` config key sets the directory and Zig optimization mode.
  Requires Zig 0.16.
- `style.selected` is a new nested interaction state on every host component, and `selected` is
  a new boolean prop that sets the native selected flag it follows. A `Table.Row`'s style now
  reaches the core row its cells are laid out in, and the core marks that row selected, so a
  selected row paints on the frame the selection changed. Protocol version 31.
- `Host matchContents` sizes a SwiftUI host to its controls' own size, so native buttons, fields,
  toggles, and pickers align with QuickGUI elements at native density; the Liquid Glass headroom
  no longer widens or heightens the host.
- Choosing an item in a `Menu.popup` menu now runs its `click` callback. The binding drained a
  finished request ahead of the events queued in the same turn, so the popup's completion reached
  JavaScript before the chosen item's action and the callbacks had already been released. Queued
  events now drain first.

## 0.1.2 - 2026-09-04

### Framework

- Added `Element::group`, `Element::group_hover`, and `Element::group_active`, Tailwind's `group`,
  `group-hover`, and `group-active`: a descendant's group variants paint while its nearest group is
  hovered or holds a press, which the core decides from the retained hit regions and the pressed
  element's ancestry, so a row can reveal its actions without a listener or a rebuild. Group
  variants layer above the base style and beneath the element's own hover, active, focus,
  validation, and drag variants, so a revealed action the pointer reaches keeps every group value
  its own hover does not override. They can be declared several times — for different groups, or
  for hover and active — and every entry whose group is in its state paints, later declarations
  winning where they overlap, up to `MAX_GROUP_STYLES_PER_ELEMENT`. `Element::group_named` with
  `group_hover_named` and `group_active_named` are `group/name` and `group-hover/name`: a member
  follows the nearest ancestor group carrying that name past any nearer group, and a named group
  is still the nearest group for members that name none. `Element::focus_within` paints while the
  element or a descendant owns keyboard focus, like CSS `:focus-within`, following the focus itself
  rather than its visibility. `ElementStateStyle` gained `border_width`, `outline_dashed`, and
  `outline_dotted`.
- Added a renderer-neutral Rust `Router` with bounded pattern compilation, normalized internal
  locations, decoded parameters and query pairs, active matching, and memory-history traversal;
  `@quickgui/native` re-exposes it and `@quickgui/solid` adds `Router`, `Route`, `Link`, `Outlet`,
  and reactive hooks. The new `routing-solid` example covers nested layouts, dynamic and wildcard
  routes, query navigation, and back/forward controls.
- A tooltip is pointer-passive by default: `TooltipState::hoverable` now starts `false`, so the
  popup never takes the pointer and reaching it closes the tooltip, as an AppKit help tag does.
  Base UI defaults the other way; call `hoverable(true)` for a popup worth hovering.
- A menu row lit by the pointer goes dark when the pointer leaves it (`PopoverMenu::unhighlight`),
  in the in-window `Menu` surfaces and the context menu alike; a row anchoring an open submenu and
  a highlight the keyboard moved elsewhere are left alone. The last hovered row previously stayed
  highlighted after the pointer left the menu.
- A cursor-tracking tooltip (`track_cursor_axis`) now records the pointer while it rests on the
  trigger before the tooltip opens and pins its other axis to the trigger's painted bounds, so the
  popup appears at the pointer instead of opening on the trigger's centre and jumping on the next
  move.
- The text caret is painted in the input's own text colour and blinks on AppKit's cadence
  (`CARET_BLINK_HALF_PERIOD`, 530 ms): solid after focus, an edit, or a caret move, then
  alternating. The runtime wakes exactly once per toggle while an input is focused and never
  otherwise; the caret was previously a fixed light grey that vanished on light backgrounds and
  never blinked.
- Fixed a virtual table snapping back to its active cell after every scroll.
  `TableState::element_with` revealed the active cell on each build, and a build follows every
  wheel or scrollbar scroll that moves the mounted slice, so the overlay scrollbar thumb could
  not be dragged and the list rewound to row 0 one frame after any scroll. Normalizing the
  active cell on a build now only clamps it into the grid; keyboard and pointer moves still
  reveal the cell they select.
- Added `SplitterState::is_dragging` and `SplitterHandle::is_dragging`, maintained by
  `apply_pointer` from the press that starts a captured drag to the release or cancel that ends
  it, so an owner that applies sizes asynchronously can tell a size the pointer is still moving
  from one the gesture settled on.
- Added `TestAppContext::simulate_scrollbar_press`, `simulate_scrollbar_drag_to`,
  `simulate_scrollbar_release`, and `scrollbar_drag_active`, which drive the built-in overlay
  scrollbar through the exact production press, drag, and release path, painting between events
  as a native window does.
- Fixed the accessibility tree listing a child whose node the update did not carry: a descendant
  the last frame never painted, such as a virtual list's row column inside a body with no room
  yet, now leaves its parent's child list too, so an assistive client no longer aborts on an
  unresolved id. A hosted table cell with text children took the process down this way whenever a
  screen reader was attached.
- Focus styles now follow focus visibility, like CSS `:focus-visible`: a pointer press that lands
  focus — a click on a button, an accordion trigger, or a tab, including a `cx.focus` a listener
  performs during the press — paints no `focus` styles (`focusOutline`, `focusBackgroundColor`,
  `focusColor`, `focusBackground`, `focusTransform`), while focus a key lands — Tab, a roving
  arrow, or a listener focusing in response to a key — paints them, and Tab or an arrow that moves
  nothing reveals the focus already there. A programmatic focus keeps the current visibility. Text
  inputs and text areas still paint their focus styles whenever focused, as a native text field
  always shows its ring. Keyboard focus, accessibility focus, and focus traps are unchanged.
  `TestAppContext::focus_visible` reports the flag, and every simulated input drives it exactly as
  production input does; a simulated left press that no listener prevents now also applies the
  production press default and focuses its target.
- Added `Element::w_fraction` and `Element::h_fraction`, which size an element as a fraction of
  its parent the way `w_full` and `h_full` size it to the whole, and `Element::is_viewport_portal`,
  which tells a host composing trees from declarations that an overlay's box is meant to be laid
  out against the window rather than its declaring parent.
- `TestAppContext::accessibility_update` is available under the `test-support` feature, so a host
  can assert on the exact tree a screen reader would receive.
- Added `LayoutBoundsHandle` and `Element::report_bounds`: an application-owned receiver for the
  window-relative bounds QuickGUI painted for one element, written during the paint already being
  performed and corrected in exactly one frame when the bounds change, so behavior can match real
  layout without an observer, timer, or idle source.
- Added `ScrollArea::positioned_thumb_with`, which places a scrollbar thumb along its track from
  the state's offset and thumb length, and fixed thumb drags to measure travel in window
  coordinates from the capture origin; a thumb that re-lays out under the pointer previously stalled.
- Aligned the menu family with Base UI's parts and props. `MenuState` is the surface around the
  `PopoverMenu` row model, composing the in-window `Popover` for anchoring, dismissal, focus
  restoration, and modal containment: `with_open`, `modal`, `orientation`, `loop_focus`,
  `close_parent_on_esc`, and `disabled` are Base UI's `Menu.Root` props, `open_on_hover` with
  `delay` (100 ms by default) and `close_delay` is Trigger `openOnHover` on exact one-shot
  deadlines, and root/trigger/portal/positioner/backdrop/popup/arrow/submenu-root/submenu-trigger
  parts decorate caller-owned elements. Escape reaches exactly one dismiss region, so
  `close_parent_on_esc` dispatches the new `MenuCloseParent` typed action up the focus path and
  stops at the first level already closed. `MenuPartState` carries `open`, `side`, `align`, and
  `anchor_hidden` from the resolved anchor placement, and `MenuItemPartState` carries
  `highlighted`, `disabled`, `checked`, and submenu `open`.
- Added Base UI-named menu item parts over the existing `PopoverMenu` model:
  `PopoverMenuItem::link` creates a `Menu.LinkItem` whose activation dispatches the new
  `OpenMenuLink` typed action through the ordinary owner path (bounded by
  `MAX_POPOVER_MENU_LINK_BYTES`); `radio_value` and `set_radio_value` are `Menu.RadioGroup`'s
  `value` and `onValueChange`, keeping exactly one checked value per group; `radio_group_with` and
  `labeled_radio_group_with` project the radio-group role; `checkbox_item_indicator_with` and
  `radio_item_indicator_with` are accessibility-hidden decoration; and `checkbox_item_with`,
  `radio_item_with`, `link_item_with`, `submenu_trigger_with`, `separator_with`, and
  `group_label_with` are kind-checked aliases of `item_with` that refuse the wrong row. A menu can
  now declare `MenuOrientation::Horizontal`, which installs
  `POPOVER_MENU_HORIZONTAL_KEY_CONTEXT` and `popover_menu_horizontal_key_bindings()` so Left and
  Right move the highlight and Down opens the highlighted submenu.
- `ContextMenuState` gained the Base UI-named `trigger_with` alias of `target_with`, an
  accessibility-hidden owner-window `backdrop_with`, and `item_state`.
- Aligned `Select` with Base UI's parts and props. `SelectState` gained
  root/label/trigger/value/icon/backdrop/portal/positioner/popup/arrow/list/item/item-text/
  item-indicator/group/group-label/separator decorators — the popup and item decorators being the
  ones the native option surface applies internally — plus `multiple` with a `MAX_SELECT_VALUES`
  (256) bounded value set, `toggle_source`, joined `value_text`, `required`, `read_only` (refused on
  the state, not only projected), `modal`, `align_item_with_trigger` over the declared
  `SelectPopoverLayout::trigger_height`, the `from_labels` items map form, and a copyable
  `SelectPartState` carrying `popup_open`, `popup_side`, `pressed`, `placeholder`, `valid`,
  `invalid`, `dirty`, `touched`, `filled`, `focused`, `read_only`, and `required`.
  `element_with_trigger` hands the surface renderer a `SelectPopupParts` whose
  `scroll_up_arrow_with` and `scroll_down_arrow_with` advance the option window one row every
  `SELECT_SCROLL_ARROW_INTERVAL` while hovered, each step an exact one-shot deadline armed by the
  previous one. Revealing the active row now runs only when the mounted window changes size, so a
  pointer scroll is no longer undone by the next frame.
- Aligned `Combobox` with Base UI's parts and props. `ComboboxState` gained
  root/label/value/icon/input/input-group/clear/trigger/chips/chip/chip-remove/portal/backdrop/
  positioner/popup/arrow/status/empty/list/row/item/item-indicator/group/group-label/collection/
  separator decorators, `multiple` with `MAX_COMBOBOX_VALUES` (64) bounded chips plus
  `add_chip_source`/`remove_chip`/`clear_chips`, `auto_highlight`, `open_on_input_click`,
  `highlight_item_on_hover`, `loop_focus`, `read_only`, `required`, `status_text` for the polite
  live region, `is_empty_result` for the Empty part, and the copyable `ComboboxPartState` and
  `ComboboxItemPartState` snapshots.
- Added Base UI's combobox `filter` policy. `PickerFilterMode` gained `Contains` (the combobox
  default) and `StartsWith`, and the new `PickerFilter` wraps an application-supplied
  `Fn(&str, &str) -> bool` predicate installed with `PickerState::set_filter`,
  `AutocompleteState::with_filter`/`set_filter`, or `ComboboxState::with_filter`/`set_filter`. The
  predicate replaces the mode entirely; `Fuzzy` stays the ranked palette matcher and the only mode
  that reports label highlight ranges.
- Added `Popover::open`, Base UI's Root `open` prop, so a component that retains the open flag
  itself can stamp the current value onto a descriptor built once.
- Added the remaining Base UI-derived unstyled components, each a caller-owned `*_part` decorator
  set with bounded state, module tests, a `TestAppContext` interaction/accessibility test, and no
  idle source: `Separator` / `separator()` (Separator role with a horizontal-by-default
  orientation); `Avatar` / `AvatarState` (Image role and accessible name on the root,
  accessibility-hidden image and fallback parts, Idle/Loading/Loaded/Error status, an exact
  `MAX_AVATAR_FALLBACK_DELAY`-bounded fallback deadline, and `Avatar::apply_loading_status` as the
  `on_loading_status_change` counterpart); `CheckboxGroup` / `CheckboxGroupState`
  (`MAX_CHECKBOX_GROUP_VALUES`-bounded declared and checked value sets, group-disabled propagation,
  per-value parts bound to the existing `Checkbox`, and a parent part whose on/mixed/off state is
  derived from the children); `PreviewCard` / `PreviewCardState` (Link-role trigger, 600 ms
  hover-open and 300 ms pointer-leave close on exact one-shot deadlines, immediate focus opening,
  and portal/positioner/popup/arrow/backdrop parts over the in-window `Popover`); `ScrollArea` /
  `ScrollAreaState` / `ScrollAreaStyleState` (ScrollView and ScrollBar roles, clamped offsets,
  per-edge overflow flags on a bounded `overflow_edge_threshold`, thumb extents with a 24 px floor,
  captured thumb-drag and track-press arithmetic mirroring the built-in overlay scrollbar, and
  `keep_mounted`); `OtpField` / `OtpFieldState` (up to `MAX_OTP_LENGTH` one-character slots composed
  from the existing `text_input()`, Numeric/Alpha/Alphanumeric/None validation, masking, auto
  advance, replace-on-retype, paste distribution, contextual Backspace/Delete/arrow/Home/End
  actions, and `auto_submit` through `EventContext::submit_form`); `Drawer` / `DrawerState`
  (root/trigger/portal/backdrop/viewport/popup/content/title/description/close/swipe-area parts,
  `Modal`/`TrapFocus`/`NonModal` modality, `MAX_DRAWER_SNAP_POINTS` bounded snap points,
  velocity-aware swipe dismissal reported as a `DrawerGesture`, `MAX_NESTED_DRAWERS` bounded
  declared nesting, and `Dialog`'s focus containment and restoration); and `NavigationMenu` /
  `NavigationMenuState` (Navigation landmark, List-role list, item/trigger/icon/content/link parts,
  per-item popover panels, one roving Tab stop with bounded arrow/Home/End navigation, exact 50 ms
  hover open and close deadlines, Escape closing inside the bar, and an exposed
  `activation_direction`).
- Added the `AccessibilityRole::Navigation`, `AccessibilityRole::ScrollView`, and
  `AccessibilityRole::ScrollBar` projections used by the new components.
- Added the `examples/base_ui_components.rs` caller-styled gallery covering all eight components and
  the [`docs/base-ui-components.md`](docs/base-ui-components.md) guide.
- Aligned the remaining Base UI form and navigation components. `Tabs` gained
  `TabsState::select_at` activation-direction tracking, `Tabs::activation_direction`, and
  `Tab::anchored_indicator_with` / `tracked_indicator_with`, which keep a caller-owned indicator on
  the tab that is really active and publish that tab's laid-out box as `TabsIndicatorGeometry`.
  `Toolbar` gained Button, Link, Input, Group, and Separator parts and `focusable_when_disabled`
  items. `Checkbox`, `Radio`, `RadioGroup`, and `Switch` gained `read_only` with the matching
  `next_state` / `accepts_selection` / `next_checked` refusal contracts, plus `Checkbox::parent` for
  a registry-free indeterminate derivation. `Dialog` gained `viewport_with` and `DialogState`, which
  holds a dialog mounted for the application's own exit transition on an exact deadline and reports
  Base UI's `onOpenChangeComplete`. `Field` gained `item_with`, `validity_with`, `validation_mode`,
  and a bounded `validation_debounce` answered through `should_validate` and `validation_delay`.
- Added `Element::focusable_when_disabled`, Base UI's `focusableWhenDisabled`. A disabled control
  normally leaves the Tab sequence; a composite widget — a toolbar above all — can opt one item back
  in so keyboard users can still discover that the command exists. The element still reports as
  disabled and still refuses pointer focus.
- Aligned `Toast` with Base UI's provider, parts, and manager API. `ToastManager` became the
  provider: `timeout` sets the inherited auto-dismiss duration, `limit` (three by default) flags
  older toasts `ToastEntry::is_limited` without silencing them, and `set_expanded` carries the
  expanded stack. `add`, `update`, `close`, and `close_all` join `push`, `dismiss`, and `clear`, and
  `promise`/`resolve` queue a persistent `ToastKind::Loading` toast and turn it into its result
  while keeping the same identity and stack position — QuickGUI owns no future, so the application
  drives both halves from the task it already spawned. `apply_swipe` implements swipe-to-dismiss
  from captured pointer events with a bounded `swipe_threshold`, a declared `swipe_direction`, and
  the movement exposed for the application to translate the toast by. `portal_with`,
  `positioner_with`, and `content_with` join the viewport, root, title, description, action, and
  close decorators, and `ToastViewport::toasts` hands each toast its stack `index`, limited flag,
  expanded flag, and an `offset(pitch)` helper. `ToastEntry` and `ToastManager` no longer derive
  `Eq` because swipe movement is measured in logical pixels; they still derive `PartialEq`.
- Aligned `NumberField` with Base UI's compound parts and props. `group_with`, `scrub_area_with`,
  and `scrub_area_cursor_with` join the root, input, and stepper decorators.
  `NumberFieldState::apply_scrub` turns a captured pointer drag into whole steps at a bounded
  `scrub_sensitivity`, retaining the unconverted remainder for the gesture so a slow drag moves one
  step at a time, with `scrub_direction`, `is_scrubbing`, and `scrub_position` for a caller-owned
  cursor. `small_step` (Alt) and `large_step` (Shift) default to a tenth and ten times the step and
  reach the keyboard, wheel, and scrub through `NumberFieldStepSize` and `step_with_modifiers`;
  `snap_on_step` lands a stepped value on the step grid; `allow_wheel_scrub(false)` opts out of
  focused wheel stepping; and `read_only` refuses every change while staying focusable, unlike
  `disabled`. `required` and a copyable `NumberFieldPartState` complete the snapshot. `step_by`,
  `wheel`, and every other existing method keep their original behavior.
- Added `Element::accessibility_read_only`, projected as AccessKit's read-only state. This is the
  web's `readonly` rather than `disabled`: the control keeps its place in the Tab sequence and its
  value in the accessible name while refusing changes.
- Aligned `Slider` with Base UI's compound parts and props. `label_with`, `value_with`,
  `control_with`, and `indicator_with` (a Base UI-named alias of `range_with`) join the root, track,
  and thumb decorators, and the root now points its accessible name and description at the mounted
  label and value. `SliderState::min_steps_between_values` holds a whole-step gap open between
  adjacent thumbs, `thumb_alignment` plus `SliderThumb::offset` implement Base UI's `thumbAlignment`
  without moving the numeric contract, and `apply_pointer_change` exposes the `onValueCommitted`
  boundary and the `data-dragging` flag while `apply_pointer` keeps its original return exactly.
  `Slider::display_value` and `SliderThumb::value_text` apply the shared `ValueFormat`, and
  `SliderThumb::state()` returns a copyable `SliderThumbState` carrying `data-index`, dragging, and
  disabled state.
- Aligned `Progress` and `Meter` with Base UI's compound parts. `track_with`, `label_with`, and
  `value_with` join the existing root and indicator decorators; declaring `.id(...)` derives their
  identities and points the root's accessible name and description at the mounted label and value.
  `Progress::status()` reports `ProgressStatus::{Progressing, Complete, Indeterminate}` and
  `Progress::state()` returns a copyable `ProgressPartState`. `ValueFormat` is the bounded `format`
  hook — `percent()`, `fraction()`, or any closure — with `display_value()` for a caller-owned value
  part and `accessible_value()` for assistive technology, where an explicit `value_text` keeps Base
  UI's `getAriaValueText` precedence. `Meter` gained the same `id`, `format`, `value_text`, and
  parts. Descriptors without an identity behave exactly as before.
- Added `TooltipProvider` and `TooltipState`, the Base UI-shaped compound tooltip. Provider, Root,
  Trigger, Portal/Positioner, Popup, and Arrow parts sit on caller-owned elements in the ordinary
  view tree, with `delay`, `close_delay`, `hoverable`, `close_on_click`, `disabled`,
  `track_cursor_axis` (`None`/`X`/`Y`/`Both`), bounded `side`/`align`/`side_offset`/
  `collision_padding`, a resolved-placement arrow, framework-owned Escape dismissal, and a copyable
  `TooltipPartState` snapshot. A shared provider makes an adjacent trigger open instantly while the
  group stays warm; that warm window is itself one exact deadline, so a settled group owns no task
  or timer. Bounded by `MAX_TOOLTIP_DELAY`, `MAX_TOOLTIP_GROUP_TIMEOUT`, `MAX_TOOLTIP_SIDE_OFFSET`,
  and `MAX_TOOLTIP_COLLISION_PADDING`. The existing framework-owned `Element::tooltip(...)` overlay
  is unchanged and remains the shortest path to a native-style hint.
- Aligned `Popover` with Base UI's compound-part API. `side`, `align`, `side_offset`,
  `align_offset`, `collision_padding`, `sticky`, `anchor_element`, `anchor_point`,
  `anchor_trigger`, and `modal` join the existing `placement`, `anchor_gap`, and `viewport_margin`
  props, which keep working and now have Base UI-named aliases writing the same bounded value.
  `portal_with`, `arrow_with`, and `viewport_with` join the trigger, positioner, popup, backdrop,
  title, description, and close parts, and `Popover::state()` returns a copyable `PopoverPartState`
  carrying `open`, `modal`, `side`, `align`, `anchor_hidden`, and the measured anchor and available
  sizes an application needs to size a popup. Bounded by the new `MAX_POPOVER_SIDE_OFFSET`,
  `MAX_POPOVER_ALIGN_OFFSET`, `MAX_POPOVER_COLLISION_PADDING`, and `MAX_POPOVER_ARROW_SIZE`.
- Added resolved anchor placement reporting. A declared `AnchorPlacement` is only a preference: the
  retained tree flips the side and re-aligns the cross axis whenever it does not fit.
  `Element::report_anchor_placement(handle)` publishes the placement, anchor rectangle, placed
  rectangle, remaining room, and anchor-hidden state into an application-owned
  `AnchorPlacementHandle` during the paint QuickGUI was already performing, and exactly one
  correcting frame is requested when that value changes — an unchanged placement requests none, so
  a settled window stays settled. `Popover::track_placement` feeds it back so `arrow_with` pins the
  caller-owned arrow to the popup edge that really faces the anchor instead of guessing from the
  preferred side. `AnchorSide`, `AnchorAlign`, `anchor_placement`, and `ResolvedAnchorPlacement` are
  public.
- Added `Element::anchor_align_offset` and `Element::anchor_sticky`: a cross-axis shift applied
  before collision handling, so it can slide an anchored surface along its anchor but never off
  screen, and an opt-out from QuickGUI's default clamping for a surface that should leave the
  viewport with a scrolling anchor.
- Added `PopoverHoverState`, Base UI's Trigger `openOnHover`. A hovered trigger opens after `delay`
  (300 ms by default) and closes after `close_delay` once neither the trigger nor a hoverable popup
  is hovered, so the pointer can cross the side-offset gap. Both are exact one-shot deadlines
  bounded by `MAX_POPOVER_HOVER_DELAY`; entering or leaving cancels the outstanding task rather
  than polling, a zero delay applies in the same controlled update with no task at all, and
  `open_now`/`close_now`/`toggle` cancel any deadline so a click and a hover cannot fight. Both
  `fn`-pointer and `StateAccessor` entry points are supplied.
- Added compositing layers: an element that declares a transform beyond a pure translation, a
  subtree blur or drop shadow, a backdrop effect, or a non-normal blend mode now renders its whole
  subtree — Glyphon text included — into a bounded offscreen texture and composites it back through
  the declared effect. `Scene` records the group, keeps its descendants' `z_index` layers inside it,
  and places the composite in its parent's cross-primitive paint order; the new
  `src/renderer/compositor.rs` owns the pass sequencing and both the window renderer and the
  headless visual-test renderer drive it, so screenshot tests cover the production path.
- Added `Transform2D` (`translate`, `scale`, `scale_uniform`, `rotate_degrees`, `rotate_radians`,
  `skew_degrees`, `then`, `compose`, `inverse`, `apply`, `transform_rect`, `around`, `lerp`) with
  `Element::transform`, `transform_origin`, `translate`, `rotate_degrees`, `scale`, `scale_uniform`,
  and `skew_degrees`, mirrored on `ElementStateStyle` so hover, active, focus, disabled, invalid,
  dragging, and drag-over states can move a subtree with no relayout. Transforms are paint-only:
  layout, measurement, and reported element bounds are unchanged, and pointer positions are
  inverse-mapped through the accumulated group transform so clicks, hover, drag, and cursor
  declarations follow the painted pixels. A pure translation stays a paint offset and allocates no
  layer.
- Added `Filter::Blur` and `Filter::DropShadow` with `Element::blur` and `Element::drop_shadow`: a
  separable Gaussian over the composited subtree bounded by the new `MAX_BLUR_RADIUS` (64 logical
  pixels), and a shadow that follows the subtree's real painted alpha rather than its element box.
  An element's existing colour-filter chain now applies to the whole subtree once anything else has
  opened a group, and stays the cheap per-primitive colour matrix when nothing has.
- Added `Element::backdrop_blur` and `Element::backdrop_filter`: the region already painted behind
  the element is copied, filtered, and drawn clipped to its rounded rectangle through the element's
  own transform. The window surface is configured with `COPY_SRC` when the adapter advertises it;
  without it the element paints without its backdrop and the frame reports it.
- Added `Element::blend_mode` and `BlendMode` (`Normal`, `Multiply`, `Screen`, `Darken`, `Lighten`,
  `Overlay`, `Difference`, `Exclusion`, `HardLight`, `ColorDodge`, `ColorBurn`). Every mode is
  exact: `Normal` and `Screen` through fixed-function blend state, the rest by evaluating the
  separable Porter-Duff form in premultiplied colour from a bounded destination copy.
- Added the compositing bounds `MAX_LAYERS_PER_FRAME` (8), `MAX_LAYER_DEPTH` (4), and
  `MAX_LAYER_TEXTURE_BYTES` (128 MiB per window, least-recently-used eviction). Exceeding any of
  them paints the subtree directly into its parent without the effect and counts it in the new
  `RenderStats::skipped_layer_effects`, alongside `compositing_layers`, `layer_passes`,
  `blur_passes`, and `layer_texture_bytes`. A scene that declares no layer effect keeps its former
  cost exactly: no pipeline compiled, no texture allocated, no extra pass recorded.
- Added `StateAccessor<V, State>`, a cloneable per-instance accessor from a view to one component's
  retained state, and a `*_with` entry point that takes it on every unstyled component that
  registers listeners: `SelectState::element_with`, `AutocompleteState::element_with`,
  `ComboboxState::element_with`, `PickerState::element_with`, `TableState::element_with`,
  `TreeState::element_with`, `ContextMenuState::element_with`, and `key_with_accessor` on `Slider`,
  `SliderThumb`, `SplitterHandle`, `ToolbarEntry`, `ToggleGroupEntry`, `ToastParts`,
  `DateFieldSegment`, `TimeFieldSegment`, `Calendar`, and `MenubarItem`. The existing
  `fn(&mut V) -> &mut State` methods stay, now as thin wrappers, so applications, examples, and
  tests are unchanged. The accessor is one erased reference-counted closure rather than a generic
  parameter, so a view can declare many instances of one component without multiplying the
  component code or the listener set each mounted instance registers, and without adding an idle
  source.
- Added `PointerEvent::size`, the captured element's own laid-out size in logical pixels, so slider,
  splitter, and custom drag arithmetic uses the extent the core's layout already decided instead of
  re-deriving it. It is `Size::ZERO` before an event is localized to an element.
- Added a bounded `CrashReporter` behind the default `crash-reporter` feature: a panic hook that
  writes a size-limited JSON report atomically, an async-signal-safe native fatal-fault path
  (`SIGSEGV`/`SIGBUS`/`SIGILL`/`SIGFPE`/`SIGABRT` through `sigaction`, `SetUnhandledExceptionFilter`
  on Windows) built on a pre-opened descriptor and a pre-rendered report template, bounded
  retention, `last_crash_report`/`pending_reports`/`delete_report`, mutable extra parameters, and
  `upload_pending` over HTTPS. The `Watchdog` main-thread hang detector is opt-in and documented
  with its idle cost.
- Added `ProcessMetrics`, `SystemMemory`, and `CpuUsageSampler`: explicit, allocation-light process
  and system readings using `proc_pidinfo`/`proc_pid_rusage`/`host_statistics64` on macOS,
  `/proc` on Linux, and `GetProcessTimes`/`GlobalMemoryStatusEx` on Windows, reporting `None`
  rather than a guess where a platform does not expose a value.
- Added `Accelerator::parse`, an Electron-compatible accelerator grammar that resolves to
  QuickGUI's `Keystroke`, bounded by the new `MAX_ACCELERATOR_BYTES`. `MenuItem::accelerator`,
  `MenuItem::try_accelerator`, and `MenuItem::keystroke` declare an explicit key equivalent that
  outranks keymap derivation, and `MenuItem::hidden` keeps a declared item out of the presented
  menu without changing its action id or collection order.
- Added the `paste-and-match-style`, `delete`, `start-speaking`, `stop-speaking`, `select-next-tab`,
  `select-previous-tab`, `merge-all-windows`, `move-tab-to-new-window`, `toggle-tab-bar`, and
  `toggle-tab-overview` menu roles. Each follows the platform responder chain first and falls back
  to the retained plain-text paste, selection delete, or window-tab command.
- Added `SystemMenuType::RecentDocuments`, an operating-system-populated recent-documents submenu.
- Added `AppRunner::request_quit()` for a preventable quit that runs the before-quit and will-quit
  phases, alongside the existing `AppRunner::exit()` forced shutdown.
- Added `AppRunner` character-palette, tabbing-identifier, tab selection, merge, detach, tab-bar,
  and tab-overview commands plus macOS Find-pasteboard read/write, so externally pumped hosts reach
  the same document-window and search integrations as `EventContext`.
- Added window lifecycle events `Event::Minimized`, `Event::Maximized`,
  `Event::FullscreenChanged`, `Event::FirstPresented`, `Event::OcclusionChanged`, and
  `Event::WindowLevelChanged`. `FirstPresented` is the flicker-free ready-to-show moment for a
  window created with `WindowOptions::show(false)` and is delivered exactly once; the state events
  are equality-suppressed and derived event-driven, with no observer, timer, or idle sampling.
- Added the `Event::WillResize`/`Event::WillMove` constrain hooks with
  `EventContext::constrain_resize` and `constrain_move`. Each proposal issues at most one
  corrective native resize or move, and the corrective value is remembered so a hook can never
  loop. Both validate through the existing window-bounds range.
- Added `Application::on_did_become_active` and `on_did_resign_active`.
- Extended `WindowLevel` with `Floating`, `ModalPanel`, `MainMenu`, `Status`, `PopUpMenu`, and
  `ScreenSaver`, plus `WindowLevel::macos_level()` and `is_above_normal()`. Non-macOS backends
  collapse every above-normal level to the topmost hint while retaining the exact requested level.
- Added `move_window_top`, `move_window_above`, `set_ignore_mouse_events(ignore, forward)`,
  `set_window_enabled`, `set_aspect_ratio`/`clear_aspect_ratio`, and
  `set_window_button_visibility`, each with a `_handle` variant, matching `WindowOptions` builders,
  and new `WindowState` fields. Aspect ratios are validated against the new
  `MAX_WINDOW_ASPECT_RATIO` bound.
- Added the serde-serializable `WindowRestoreState`, `WindowState::restore_state`,
  `WindowRestoreState::resolve`, `ResolvedWindowRestoreState`, and `WindowOptions::restore`, which
  validate persisted geometry against the connected displays, clamp into the remembered work area,
  and fall back to centered placement rather than placing a window off every display.
- Added application-shell services on `EventContext`: `set_activation_policy`,
  `activate_application`, `hide_application`, `unhide_application`, `request_dock_attention`,
  `cancel_dock_attention`, `set_dock_visible`, `set_secure_keyboard_entry`, `beep`,
  `applications_folder_support`, `move_to_applications_folder`, `is_application_packaged`, and
  `exit_with_code`, plus the free `quickgui::is_application_packaged()`.
- Added deterministic coverage through `TestAppContext::simulate_minimize`, `simulate_maximize`,
  `simulate_fullscreen_change`, `simulate_occlusion_change`, `simulate_first_presented`,
  `simulate_window_resize`, `simulate_window_move`, `window_restore_state`, `exit_code`, and the
  `TestApplicationShell` snapshot.
- Added controlled unstyled `Slider` with bounded `MAX_SLIDER_THUMBS` values, step snapping, thumb
  ordering, horizontal or vertical captured pointer arithmetic, typed arrow/page/Home/End actions,
  and Slider accessibility with numeric value, minimum, maximum, step, and orientation.
- Added `Progress` and `Meter` descriptors with determinate and indeterminate semantics, optional
  low/high/optimum meter markers, and no framework-owned animation, so an indeterminate indicator
  never keeps a settled window awake.
- Added `NumberFieldState` and `NumberField` composing the existing `text_input()` with
  caller-supplied decimal and grouping separators, sign and exponent policy, fixed precision,
  commit-time clamping and formatting, arrow/wheel/stepper stepping, press-and-hold repeat on exact
  one-shot deadlines that leave no idle source once released, and SpinButton accessibility with
  numeric value, bounds, step, and invalid state.
- Added `SplitterState` and `Splitter` for resizable panes with size-conserving drags from captured
  pointer deltas, per-pane minimum sizes, collapse and restore, proportional `set_total` rescaling,
  typed keyboard resizing, and focusable Splitter handles carrying numeric value, bounds, axis, and
  a controls relationship to the pane they resize.
- Added `Toolbar` with the Toolbar role, one roving Tab stop over the caller's ordered items,
  per-item arrow/Home/End navigation, disabled-item skipping, and optional looping.
- Added `Toggle` and `ToggleGroup` with pressed-state button semantics distinct from checkbox state,
  single or multiple selection over inline bounded pressed values, and the same roving focus
  contract as the toolbar.
- Added `ToastManager` and `ToastViewport` with a bounded `MAX_TOASTS` queue, exact one-shot
  auto-dismiss deadlines reported through `next_deadline`, pause on hover or focus, polite or
  assertive live regions chosen by toast kind, focused Escape dismissal, and caller-owned
  viewport/root/title/description/action/close parts.
- Added `AccessibilityRole::Slider`, `SpinButton`, `ProgressIndicator`, `Meter`, `SplitterHandle`,
  `Toolbar`, `ToggleButton`, `MenuBar`, `Alert`, and `Status`, plus public
  `AccessibilityOrientation`, `AccessibilityLive`, and `AccessibilityValueRange` with
  `Element::accessibility_orientation`, `accessibility_live`, and `accessibility_value_range`.

- Added per-input text checking with `.spellcheck(..)`, `.grammar_check(..)`, `.autocorrect(..)`,
  `.smart_quotes(..)`, `.smart_dashes(..)`, `.text_replacement(..)`, and
  `.lookup_on_force_click(..)`, layered over an application-wide `set_default_text_checking(..)`
  policy. Checking runs once on a single 300 ms settle deadline over at most 16 KiB around the
  caret, retains at most 512 flagged ranges, projects them as wavy underlines merged over the
  controlled run table without rewriting it, and skips the word the caret is inside. A settled input
  holds no timer.
- Added the `SpellCheckProvider` trait plus `set_spell_check_provider`/`clear_spell_check_provider`,
  bounded `guesses`/`learn`/`ignore` with per-input document tags, boundary autocorrect and
  replacement-dictionary substitutions applied as one undoable edit with a "Change back" record, and
  insertion-time smart quotes and dashes that never rewrite paste or IME commits. Portable targets
  fall back to `NoSpellCheckProvider` and report `TextServiceError::Unsupported`.
- Added `spelling_menu_items(..)` and the typed `ReplaceWord`, `LearnWord`, `IgnoreWord`, and
  `LookUpSelection` actions so applications can build the standard spelling context menu and bind
  dictionary lookup, plus `show_definition_for(..)` and `word_range_at(..)`.
- Added bounded literal find and replace: `FindState` with case-sensitive and whole-word options,
  4,096 retained matches, wrap-around `find_next`/`find_previous`, distinct all-match and
  current-match highlight runs, a `2 of 17` count label, and `replace_current`/`replace_all` results
  an application applies as one controlled edit. Added the unstyled `FindBar` component over
  caller-owned root, query, replace, count, next, previous, replace, replace-all, and close parts
  with `find_bar_key_bindings()` for Return, Shift+Return, and Escape.
- Added an application-wide `UndoManager` with 256 bounded named entries, `begin_group`/`end_group`
  collection, a value-based `UndoableChange<T>` shortcut, `undo_action_name`/`redo_action_name`
  menu titles, `Undo`/`Redo` actions with `undo_key_bindings()`, and a documented policy that a
  focused text input's own history claims Undo and Redo before the manager sees them.
- Added `examples/text_services.rs` and new `docs/text-and-forms.md` sections covering spelling,
  dictionary lookup, find and replace, and application undo.

- Added inherited right-to-left layout with `direction(Direction::Rtl)`, `rtl()`, and `ltr()`.
  In an RTL subtree in-flow positions, physical horizontal insets, and margins mirror inside the
  parent's content box for paint, hit testing, and accessibility together; horizontal scrolling
  starts at the right edge and inverts physical wheel deltas; and shaping uses a forced RTL base
  paragraph direction. Added the logical edge helpers `ps`, `pe`, `ms`, `me`, `border_s`, and
  `border_e`, plus the `TextAlign::Start`/`TextAlign::End` alignments behind `text_start()` and
  `text_end()`. Focus order still follows document order and caret movement stays logical.
- Added CSS-style sticky positioning with `sticky()`, `sticky_top`, `sticky_bottom`, `sticky_left`,
  and `sticky_right`. A sticky element pins inside the nearest scroll container and releases at its
  parent's end, changing painted geometry and hit regions only, and never triggering a relayout.
  Bounded by the new `MAX_STICKY_ELEMENTS_PER_WINDOW`.
- Added scroll snapping with `scroll_snap_x`, `scroll_snap_y`, `snap_align`, and
  `snap_stop_always`. Snapping resolves at a native momentum end phase, at a bounded settle
  deadline for wheels without phases, at scrollbar release, or programmatically, then animates to
  the target on exact deadlines and leaves the window fully settled. Bounded by the new
  `MAX_SCROLL_SNAP_CONTAINERS_PER_WINDOW` and `MAX_SCROLL_SNAP_POINTS_PER_WINDOW`.
- Added extended text styling: `text_shadow` (exact offset and color, bounded blur approximation),
  `letter_spacing`, `word_spacing`, `text_transform` with `uppercase`/`lowercase`/`capitalize` for
  non-editable text, `overline`, `word_break`, `overflow_wrap`, `hyphens`, and `text_direction`.
  Every new property is part of the canonical retained shaping key; case mapping and soft-hyphen
  removal keep selection, copy, and accessibility mapped to the original string, and editable
  inputs are never transformed.
- Added `overflow_x_scroll()` and `overflow_scroll()`.

- Added element background gradients: `bg_linear_gradient`, `bg_radial_gradient`,
  `bg_radial_gradient_at`, `bg_conic_gradient`, and the general `bg_gradient`, backed by a new
  `Gradient`/`ColorStops`/`GradientKind` API with `MAX_GRADIENT_STOPS` (8) stops, CSS angles or
  `GradientDirection`, `RadialGradientShape`/`RadialGradientExtent`/`GradientCenter`, and
  linear-sRGB, sRGB, or Oklab interpolation. Gradients are evaluated analytically in the existing
  instanced shape draw, honor rounded corners, borders, clipping, subtree opacity, and damage, and
  are swappable in hover, active, focus, validation, and drag states through
  `ElementStateStyle::bg_gradient`. `MAX_GRADIENTS_PER_FRAME` (4,096) bounds the per-frame upload.
- Extended `Background` so retained paths and canvas fills accept the same multi-stop linear,
  radial, and conic gradients instead of only two-stop linear gradients.
- Added per-corner radii: `rounded_tl`, `rounded_tr`, `rounded_br`, `rounded_bl`, `rounded_t`,
  `rounded_b`, `rounded_l`, `rounded_r`, `corner_radii(Corners)`, and `rounded_full()`. Radii are
  capped by `MAX_CORNER_RADIUS` and reduced by the CSS uniform-scale rule; element box shadows
  follow the same per-corner geometry.
- Added `border_solid()`, `border_dashed()`, and `border_dotted()`. Dash geometry is analytic arc
  length along the rounded outline, with the period scaled so a whole number of repeats closes the
  outline.
- Added `outline(width, color)`, `outline_offset(px)`, `outline_style`, `outline_dashed`,
  `outline_dotted`, and `outline_none`, plus `ElementStateStyle::outline`/`outline_offset` for
  focus rings. Outlines are painted outside the border box and never affect layout, bounded by
  `MAX_OUTLINE_WIDTH` and `MAX_OUTLINE_OFFSET`.
- Added raster element backgrounds: `bg_image(image, BackgroundSize, BackgroundRepeat,
BackgroundPosition)` with `bg_image_cover`, `bg_image_contain`, `bg_image_tiled`, and
  `bg_image_none`. Tiles reuse the existing bounded image primitive and GPU texture cache, are
  masked by the element's rounded corners, and are capped by `MAX_BACKGROUND_IMAGE_TILES` (256).
- Added bounded CSS-shaped color filters: `Filter`, `Filters`, `ColorMatrix`,
  `MAX_FILTERS_PER_ELEMENT`, `Element::filters`/`filter`, and the `brightness`, `contrast`,
  `saturate`, `invert`, `sepia`, and `hue_rotate` shorthands. `grayscale(bool)` now routes through
  the same chain, and `ImagePrimitive::grayscale` is expressed as a `ColorMatrix`. Filters apply to
  an element's own image and background-image pixels; subtree filters, blur, and drop-shadow are
  not implemented because they require an offscreen group texture.
- Added the `effects` example.

- Added display rotation, built-in-panel, and color-depth metadata to `Display`, a deterministic
  bounded `Displays::diff`, the granular `DisplayEvent::{Added, Removed, MetricsChanged}` value, and
  `Application::on_display_event`. The coarse displays-changed observation is unchanged and no timer
  or polling source was added.
- Added `EventContext::message_box` with `MessageBoxOptions`: severity, separate `message`/`detail`
  text, explicit `default_button`/`cancel_button` indices, a suppression `checkbox`, and a custom
  `icon`. The bounded `MessageBoxResponse` carries the chosen button index and the checkbox state.
- Added open-panel `can_create_directories`, `resolves_aliases`,
  `treats_file_packages_as_directories`, and `message` options, and save-panel `name_field_label`
  and `shows_tag_field` options, each validated before the request is retained.
- Added `EventContext::preview_file`/`close_file_preview`, `show_color_panel`/`close_color_panel`,
  `show_font_panel`, `share_items`, and `authenticate_with_biometrics`, plus
  `Application::on_color_panel_change` and `on_font_panel_change`. Non-macOS targets return
  `PlatformError::Unsupported` and the new `DesktopIntegrationSupport` flags report it.
- Added `Image::from_data_url`, `Image::named_system`, `Image::named_system_sized`, `Image::resize`,
  `Image::crop`, `Image::to_png`, `Image::to_jpeg`, `Image::template`, and
  `Image::with_representations`, all bounded by existing and new `MAX_*` image constants.
- Added `AppRunner::tray_icon_bounds` and `TrayIconImage::template`.
- Added `AppRunner`-level window stacking, input-policy, and application-shell commands, so an
  externally pumped host reaches `move_window_to_top`, `move_window_above`,
  `set_window_ignore_mouse_events`, `set_window_enabled`, `set_window_aspect_ratio`,
  `set_window_button_visibility`, `set_window_always_on_top`, `set_activation_policy`,
  `activate_application`, `hide_application`/`unhide_application`, `request_dock_attention`/
  `cancel_dock_attention`, `set_dock_visible`, `set_secure_keyboard_entry`, `beep`,
  `applications_folder_support`, `move_to_applications_folder`, `is_application_packaged`, and
  `exit_with_code` under the identical queue bounds as the `EventContext` forms.
- Added `AppRunner::set_window_menus`, `AppRunner::use_application_menus_for_window`, and
  `AppRunner::show_window_popup_menu`. Per-window menus and native popup menus need the runtime's
  window-scoped `EventContext`, so each is declared as a deferred request that the runtime resolves
  inside its own effect cycle. Both queues carry the public `MAX_PENDING_NATIVE_POPUP_MENUS` bound,
  the popup response completes when the menu closes, and a popup declared for a window that closes
  first completes with `PlatformError::Unavailable`.

- Added unstyled segmented `DateField` and `TimeField` editors over plain `CivilDate`/`CivilTime`
  values with no date dependency: proleptic Gregorian validation including leap years, configurable
  year/month/day order, 12- or 24-hour presentation over one retained 24-hour value, typed-digit
  entry that advances as soon as no further digit fits, wrapping arrow steps, Home/End segment
  bounds, Backspace clearing, bounded per-segment placeholders, `min`/`max` validity reported without
  rewriting typed text, caller-owned root and segment parts, `date_field_key_bindings()` and
  `time_field_key_bindings()`, and one native spin button per segment inside a group root.
- Added `CalendarState`, a bounded month grid with roving day focus: at most `MAX_CALENDAR_WEEKS`
  mounted week rows whatever the month, a declared week-start weekday, arrow/Home/End/Page/Shift-Page
  navigation that follows focus across month and year boundaries, selection bounds that refuse the
  selection rather than the movement, caller-owned grid/week/day parts, `calendar_key_bindings()`,
  and exact grid, row, and grid-cell accessibility.
- Added bounded multiple row selection to `TableState`: `TableSelection` retains merged inclusive
  ranges rather than one entry per row, Shift extends from an anchor, the platform modifier toggles
  one row, Space and Command-A work from the keyboard, rows project `selected` state, a multiple
  selection grid projects `multiselectable`, and a real change dispatches the typed
  `TableSelectionChanged` action.
- Added caller-placed column-resize handles, keyboard column reordering, and an inline-edit hook to
  `TableState`. The header renderer receives a behavior-only handle carrying captured pointer drag,
  Left/Right resizing in its own key context, declared minimum widths, and splitter semantics;
  Alt-Left and Alt-Right move the active column through a retained display order that keeps declared
  cell positions stable; `begin_edit` gives one cell its own key context and reports Return and
  Escape as the typed `TableEditEnded` action.
- Added lazy tree children: `TreeNode::pending(true)` mounts exactly one bounded loading placeholder
  row on first expansion, dispatches the typed `TreeLoadChildren` action once, and
  `TreeState::set_children` validates depth, node budget, label bytes, and duplicate IDs before
  splicing atomically, preserving selection, expansion, and the logical scroll anchor or leaving the
  tree untouched.
- Added an unstyled in-window `Menubar` over `PopoverMenu` surfaces: caller-owned root and trigger
  parts, one roving Tab stop over at most `MAX_MENUBAR_MENUS` menus, wrapping Left/Right navigation
  that switches menus rather than closing while one is open, Home/End, Down/Return/Space opening,
  Escape closing without leaving the bar, hover switching only while the bar is open, and exact
  menubar/menu-item accessibility. Native `Menu` remains the platform application menu.
- Added `Element::accessibility_multiselectable`, projected as the native multiselectable state, so
  a grid can announce that it accepts more than one selected descendant.
- Added `AutocompleteState::with_value`, the constructor-time equivalent of `set_value` for a state
  that has never been mounted and therefore owns no popover to synchronize. It applies the same
  `MAX_AUTOCOMPLETE_VALUE_BYTES` bound and seeds the suggestion query, so a host that builds its
  state during a render pass — where no `EventContext` exists — can still declare an initial value.

### macOS

- Expanded the Rust-owned SwiftUI host with controlled `Slider`, `Toggle`, `ProgressView`,
  `Stepper`, `TextField`, `SecureField`, `Picker`, segmented picker, `DatePicker`, and `ColorPicker`
  support, plus native `Gauge`. Native value and submit events are queued back through the existing
  asynchronous host boundary, and `@quickgui/solid/swift-ui` exposes typed reactive adapters for
  the same core descriptors under mutation protocol v28. Pickers also expose macOS 27's native
  segmented-tabs role for the neutral Xcode-style navigation treatment.
- Native menu items now honor an explicit declaration accelerator ahead of both the keymap binding
  and the AppKit standard binding for a role, and hidden items set `NSMenuItem.hidden` and no
  longer claim the Command-W close fallback.
- An "Open Recent" system submenu is built with a `clearRecentDocuments:` "Clear Menu" item so
  `NSDocumentController` populates it.
- Dock menu items now render their declared accelerators.

- Window levels now apply the exact `NSWindowLevel` after Winit's three-level hint;
  `move_window_top`/`move_window_above` use `orderFront:` and `orderWindow:relativeTo:` so a window
  restacks without activating the application or becoming key.
- Click-through uses `ignoresMouseEvents` with `acceptsMouseMovedEvents`, so a forwarding window
  keeps AppKit's existing event-driven `mouseMoved:` stream without a tracking area or timer.
- `set_window_enabled` expresses "visible but inert" with `ignoresMouseEvents` plus refusing and
  resigning key-window status; `set_aspect_ratio` uses `contentAspectRatio`; and
  `set_window_button_visibility` hides the standard traffic-light buttons while keeping the
  titlebar.
- Application-shell services use `NSApplicationActivationPolicy`, `NSApp.activate` /
  `activateIgnoringOtherApps:`, `hide:`/`unhide:`, `requestUserAttention:` /
  `cancelUserAttentionRequest:`, Carbon `EnableSecureEventInput`/`DisableSecureEventInput` (guarded
  by `IsSecureEventInputEnabled` so the process-global counter stays balanced), and `NSBeep`.
- `on_did_become_active`/`on_did_resign_active` reuse the existing application observer, adding no
  new native observer.

- Added an `NSSpellChecker`-backed default text checking provider covering spelling, guesses,
  corrections, the user replacement dictionary, learned and ignored words, and per-input
  `uniqueSpellDocumentTag` sessions released on unmount, with explicit UTF-8/UTF-16 offset
  conversion at the AppKit boundary.
- Added the macOS dictionary popover through `NSView showDefinitionForAttributedString:atPoint:` on
  the key window's content view, positioned at a window-local logical point.

- Message boxes use `NSAlert` suppression buttons, custom icons, and reassigned key equivalents so
  an explicit default or cancel index wins over AppKit's first-button default.
- File previews use `QLPreviewPanel` with a core-owned `QLPreviewItem`/data-source pair retained
  only while the panel is open.
- The system color and font panels are driven through one core-owned responder;
  `ColorPanelMode::OnClose` observes the panel's close notification instead of installing an action,
  so a closed panel retains no observation.
- Share sheets use `NSSharingServicePicker` anchored to the current window's content view, and
  biometric authentication uses `LAContext` with `LAPolicyDeviceOwnerAuthenticationWithBiometrics`,
  forwarding its background-queue reply through the event loop.
- `Display` now reports `CGDisplayRotation`, `CGDisplayIsBuiltin`, and
  `NSBitsPerPixelFromDepth(NSScreen.depth)`.
- `NSImage` conversion honors QuickGUI template metadata and additional backing-scale
  representations for Dock, About-panel, message-box, and menu icons, and `Image::named_system`
  resolves `NSImage` names and SF Symbols.

### JavaScript tooling

- Interaction states in `@quickgui/solid` are nested `style` objects — `hover`, `active`, `focus`,
  `disabled`, `invalid`, `dragging`, `dragOver`, `groupHover`, `groupActive`, and `focusWithin` —
  each accepting every property
  the core's `ElementStateStyle` can swap: `background`/`backgroundColor`, `color`, `borderColor`,
  `borderWidth`, `borderRadius`, `outline`, `boxShadow`, `opacity`, `cursor`, `transform`, and
  `transformOrigin`, with `outline: "none"` and `boxShadow: "none"` as explicit removals. A `group`
  prop marks the hover group a descendant's `groupHover` follows: `true` opens an unnamed group and
  a string names one that `groupHover: { group: "name" }` follows past nearer groups. `Select.Item`,
  `Combobox.Option`, and the other option parts keep their `group` label prop and route it
  themselves. `groupHover` and `groupActive` also accept a list of entries, one per group they
  follow, and in a style array entries for the same group merge while entries for different groups
  accumulate. Each state crosses N-API
  as one bounded JSON declaration (`MAX_STATE_STYLE_JSON_BYTES`), a layout property inside a state
  throws a `TypeError`, and the `invalid` prop now sets the core's invalid state on any element.
  The flat `hoverBackgroundColor`-style names still work, overlay the nested form, and are
  deprecated. The native protocol is now version 30, so `@quickgui/native` must be rebuilt.
- `style` in `@quickgui/solid` accepts an array — objects and `false`/`null`/`undefined` entries,
  nested to any depth — merged left to right with later values winning, so a conditional style is
  one expression. Nested interaction states merge one level deep. `flattenStyle` exposes the same
  merge, and `Link` now layers `activeStyle` over `style` through it instead of spreading.
- `Tooltip.Root`'s `hoverable` defaults to `false`; declare `hoverable` to keep a tooltip open while
  the pointer rests on its popup.
- `DateField.Segment` and `TimeField.Segment` declared without children now show the core's own
  digits and placeholder for that segment, as the docs always described; they previously mounted as
  empty boxes unless the application rendered the text itself.
- A `ContextMenu.Trigger` whose declared item list is empty now opens the core's surface from its
  `Menu.Item` child parts; the binding's always-present appearance declaration used to shadow the
  parts and open nothing.
- Added `Menu.popup` to the components gallery's Context Menu tab, next to the two in-window shapes.
- Fixed a controlled `Splitter` handle falling behind the pointer and rewinding after the
  release. Every `onSizesChange` comes back as the next `value` declaration one or more frames
  late, and the binding reseeded its retained sizes from each such echo, so the next incremental
  pointer delta built on a stale value. A captured drag now owns the sizes until it ends, sizes
  the core itself reported are recognized when they come back (up to 32 outstanding reports)
  instead of reseeding, and a changed `panes` minimum, `collapsible` flag, or `step` is applied
  to the retained state rather than rebuilding it, which also keeps the size a collapsed pane
  restores to.
- Dragging a hosted `Table` body's overlay scrollbar now scrolls the table as the wheel does,
  and the reported `visibleRange` follows the drag.
- Percentage `width` and `height` values now size against the parent: `width: "62%"` on a
  `Meter.Indicator`, `Progress.Indicator`, or `Slider.Indicator` fills that share of its track,
  where previously only the literal `"100%"` resolved.
- Viewport portals mount under the window root. A `Dialog.Portal`, `AlertDialog.Portal`,
  `Drawer.Portal`, `Toast.Portal`, or any `overlay` view declared inside a panel now covers the
  window, as a DOM portal renders into the document body, instead of laying out inside the panel
  that declared it; anchored popups stay where they are declared.
- Scroll areas now measure their viewport, content, and scrollbar extents from painted bounds, so
  `viewportSize` and `contentSize` are optional overrides, the thumb is positioned by the core, and
  thumb drags track the pointer. Splitters rescale their retained sizes to the extent flex layout
  gave their panes, so a handle follows the pointer one logical pixel per pixel.
- Bound the Base UI-aligned menu, select, and combobox parts and props to JavaScript at protocol
  version 26. `Menu` is the compound form of the core's `MenuState` plus `PopoverMenu` model, and
  its rows are ordinary application-styled child nodes rather than a JSON model the core paints:
  `Menu.Root` is a logical coordinator carrying `open`/`defaultOpen`/`onOpenChange`, `modal`,
  `orientation`, `loopFocus`, `closeParentOnEsc`, and `disabled`; `Menu.Trigger` adds `openOnHover`
  with `delay` and `closeDelay`; `Menu.Portal`, `Backdrop`, `Positioner`, `Popup`, and `Arrow`
  decorate caller-owned surface elements; and `Menu.Item`, `LinkItem`, `SubmenuRoot`,
  `SubmenuTrigger`, `Group`, `GroupLabel`, `RadioGroup`, `RadioItem`, `RadioItemIndicator`,
  `CheckboxItem`, `CheckboxItemIndicator`, and `Separator` declare the rows. The binding gathers
  the declared rows of one level in one depth-first pass and hands them to the core, so a row's
  derived identity, `menuitem` semantics, roving highlight, typeahead, toggle policy, radio
  exclusivity, activation, and closing policy are all the core's and none of them is reimplemented
  in JavaScript.
- Reported everything a menu decides as one asynchronous `componentchange`: `useMenuState()`
  carries the core's `MenuPartState` (`open`, the resolved `side` and `align`, `anchorHidden`) and
  `useMenuItemState()` carries `MenuItemPartState` (`highlighted`, `disabled`, `checked`, submenu
  `open`). An activated row reports `activated`, a checkbox row reports the `checked` value the
  core toggled to through `onCheckedChange`, a radio row reports its group's new `value` through
  `Menu.RadioGroup`'s `onValueChange`, and a `Menu.LinkItem` hands its bounded `href` to the core's
  own open-URL path and reports it through `onNavigate`. A submenu trigger is one element that is
  both a row of its parent level and the trigger of its own, so it anchors to the identity the
  parent model derived instead of a second standalone trigger.
- Kept the existing JSON-`items` `PopoverMenu` and `ContextMenu` bindings working unchanged, and
  taught `ContextMenu.Root` to accept exactly the same row components. A cursor-point menu is
  painted by the core in its own window, so those rows contribute a bounded model rather than
  owner-window elements: they mount nothing at all, and their activation still reports through
  `onClick` and `onSelect`.
- Bound Base UI's `Select` parts and props. `Select.Label`, `Value`, `Icon`, and `Backdrop` are
  owner-window elements the core decorates; `Portal`, `Positioner`, `Popup`, `Arrow`, `List`,
  `Item`, `ItemText`, `ItemIndicator`, `Group`, `GroupLabel`, `Separator`, `ScrollUpArrow`, and
  `ScrollDownArrow` are declarations, because the option list lives in a separate native child
  window the core paints from `appearance`. `Select.Item` is the child option declaration alongside
  the `items` prop and takes its label from `Select.ItemText`; a `Select.Group` label becomes the
  searchable group name of the options inside it; and the scroll arrows switch the binding onto the
  core's own `element_with_trigger`, whose hovered decorators advance the option window one row every
  50 ms on exact one-shot deadlines. `multiple` with a bounded value array, `required`, `readOnly`,
  `modal`, `alignItemWithTrigger`, and the `items` map form join the existing props, and
  `useSelectState()` reports the core's whole `SelectPartState`.
- Bound Base UI's `Combobox` and `Autocomplete` parts and props: `Label`, `Value`, `Icon`, `Input`,
  `InputGroup`, `Clear`, `Trigger`, `Chips`, `Chip`, `ChipRemove`, `Backdrop`, `Status`, and
  `Empty` as owner-window elements, and `Portal`, `Positioner`, `Popup`, `Arrow`, `List`, `Row`,
  `Item`, `ItemIndicator`, `Group`, `GroupLabel`, `Collection`, and `Separator` as declarations.
  `multiple` turns the declared value set into the core's bounded chips, `useComboboxChips()`
  reports the set it retained with the label it took from each option, `Combobox.Empty` mounts only
  while the core's own query really matched nothing, and `useComboboxState()` reports
  `ComboboxPartState` plus the derived `Status` text and result count. `filterMode` became Base
  UI's `filter` policy — `contains` (the combobox default), `startsWith`, `fuzzy`, and `none` — and
  `autoHighlight`, `openOnInputClick`, `highlightItemOnHover`, `loopFocus`, `readOnly`, and
  `required` are declared ahead of every decision the core makes with them.
- Made a text input's compound siblings rather than children: `Combobox.Root` and
  `Autocomplete.Root` are still the declaration-carrying input, but their children now mount beside
  it instead of inside it, because a text field paints its own content. Declared option nodes are
  gathered from the whole compound in declaration order, so a `Combobox.Option` keeps working in
  either position, and every part resolves its instance from the scope the root supplies.
- Declared ahead of the core everything it must decide synchronously: seven new property codes
  (`closeParentOnEsc`, `href`, `multiple`, `alignItemWithTrigger`, `autoHighlight`,
  `openOnInputClick`, `highlightItemOnHover`) beside the existing scope, value, orientation,
  `readOnly`, `required`, `modal`, `loopFocus`, `openOnHover`, `delay`, and `closeDelay`
  declarations. A menu row without a stable value, an interactive row with no label, an oversized
  `href`, a malformed `items` map, and a duplicate option value all decline or clamp instead of
  reaching a core constructor that would panic, and a declared `onClick` on a row whose activation
  the core owns is dropped rather than registered twice.
- Added `SelectState::reset_touched` and `ComboboxState::reset_touched` to the core, the
  counterpart of the existing `reset_dirty`: a value applied from a declaration is not an
  interaction, so a hosted control reports Base UI's `data-touched` only for what the user did.
- Added the two compounds to `examples/components-solid`, binding tables and worked examples to
  `docs/solid.md`, Solid sections to `docs/popovers.md`, `docs/context-menus.md`, and
  `docs/select-and-autocomplete.md`, and six Rust binding tests through `TestAppContext` plus four
  Solid tests and two protocol tests covering identity, mount policy, reported payloads, bounds,
  and malformed declarations.
- Bound the Base UI-aligned compound parts and props to JavaScript at protocol version 25. The
  `Popover` compound gained `Portal`, `Backdrop`, `Positioner`, `Popup`, `Arrow`, `Viewport`,
  `Title`, `Description`, and `Close` parts beside the existing one-element `Content`, with
  Base UI's `side`, `align`, `sideOffset`, `alignOffset`, `collisionPadding`, `sticky`, `anchor`
  (another node or one `{ x, y }` logical point), `modal`, and Trigger `openOnHover`/`delay`/
  `closeDelay`. A declared side is only a preference, so the placement the retained tree really
  resolved to — side, alignment, anchor-hidden state, and the measured anchor and available sizes —
  is published during the paint QuickGUI was already performing and reported back through
  `onPlacementChange` and the signal-friendly `usePopoverPlacement()`, which is how an application
  styles from the real placement the way Base UI styles from `data-side` and `data-align`. The
  trigger is the one part the core keeps mounted whether the surface is open or closed, so it
  carries the whole declaration and every other part only repeats the compound scope.
- Added the compound `Tooltip` (`Provider`, `Root`, `Trigger`, `Portal`, `Positioner`, `Popup`,
  `Arrow`) with `delay`, `closeDelay`, `timeout`, `disabled`, `hoverable`, `trackCursorAxis`,
  `closeOnClick`, and `useTooltipPlacement()`. One shared provider makes an adjacent trigger open
  instantly while the group stays warm. The framework-owned `tooltip` prop every native node
  accepts is unchanged and remains the shortest path to a native-style hint.
- Bound the aligned range and feedback parts. `Slider` gained `Label`, `Value`, `Control`, and
  `Indicator` parts, `minStepsBetweenValues`, `thumbAlignment`, a bounded `format`, the core's own
  `onValueCommitted` pointer boundary, and `useSliderState()` for the `dragging` flag and formatted
  value. `NumberField` gained `Group`, `ScrubArea`, and `ScrubAreaCursor` parts, `smallStep`,
  `largeStep`, `snapOnStep`, `allowWheelScrub`, `readOnly`, `required`, `scrubDirection`,
  `scrubSensitivity`, `onValueCommitted`, and `useNumberFieldState()` for the `scrubbing` flag.
  `Progress` and `Meter` gained `Track`, `Label`, and `Value` parts, the same bounded `format`, and
  `useGaugeState()` reporting the core's derived status, formatted value, and completion.
- Bound the aligned `Toast` provider, parts, and manager. `Toast.Provider` owns the declared queue
  and Base UI's `timeout`, `limit`, `expanded`, `swipeDirection`, and stack `pitch`; `Portal`,
  `Positioner`, and `Content` join the viewport, root, title, description, action, and close parts;
  and `useToastManager()` supplies `add`, `update`, `close`, `closeAll`, and `promise` over that
  declaration. Every toast's stack index, `limited` and `expanded` flags, `type`, `offset`, and live
  swipe displacement come back from the core.
- Bound the remaining aligned parts and props: tab `activationDirection` and indicator geometry
  through `useTabsState()` (with `index` on a tab and `placement` on the indicator);
  `Toolbar.Button`, `Link`, `Input`, `Group`, and `Separator` with `focusableWhenDisabled` items;
  `Field.Item` and `Field.Validity` with `validationMode` and `validationDebounceTime` answered by
  the core; `readOnly` on `Checkbox`, `Radio`, `RadioGroup`, and `Switch` plus a registry-free
  parent checkbox from declared `childrenChecked`; and `Dialog.Viewport` with `enterDuration`,
  `exitDuration`, and `onOpenChangeComplete`, where the core holds a closing dialog mounted for
  exactly the declared exit transition.
- Bound the eight Base UI parity components to `@quickgui/native` and `@quickgui/solid`, bumping
  the mutation protocol to v24. Solid gained `Separator`, `Avatar` (`Root`/`Image`/`Fallback`),
  `CheckboxGroup` whose members are ordinary `Checkbox.Root` nodes with `value` or `parent`,
  `PreviewCard` (`Root`/`Trigger`/`Portal`/`Backdrop`/`Positioner`/`Popup`/`Arrow`), `ScrollArea`
  (`Root`/`Viewport`/`Content`/`Scrollbar`/`Thumb`/`Corner`), `OtpField` (`Root`/`Input`/
  `Separator`), `Drawer` (`Root`/`Trigger`/`Portal`/`Backdrop`/`Viewport`/`Popup`/`Content`/
  `Title`/`Description`/`Close`/`SwipeArea`), and `NavigationMenu` (`Root`/`List`/`Item`/`Trigger`/
  `Icon`/`Content`/`Link`/`Portal`/`Positioner`/`Popup`/`Viewport`/`Arrow`/`Backdrop`), using Base
  UI's own prop names throughout — `delay` on `Avatar.Fallback`, `delay`/`closeDelay` on
  `PreviewCard.Trigger`, `orientation`/`keepMounted` on `ScrollArea.Scrollbar`, `length`,
  `validationType`, `mask`, `readOnly`, `autoSubmit` on `OtpField.Root`, `modal`, `swipeDirection`,
  `snapPoints`, `snapPoint`, `disablePointerDismissal` on `Drawer.Root`, and `value`,
  `orientation`, `delay`, `closeDelay` on `NavigationMenu.Root`.
- Kept every one of them on the declared-part scheme: each root allocates one bounded scope, the
  Rust binding rebuilds the matching core descriptor and applies its exact identity, semantics,
  keyboard behavior, and mount policy, and JavaScript reimplements none of it. A closed preview
  card, drawer, or navigation panel contributes no element at all; a scrollbar for an axis that
  cannot scroll is unmounted unless `keepMounted` is declared; an avatar mounts exactly one of its
  image and fallback.
- Reported everything the core decides as one asynchronous `componentchange` event — the avatar
  loading status, the checked value set, a preview card's open value, a scroll area's clamped
  offset and its eight derived style flags, the OTP code plus its separate completion edge, the
  drawer's open value, snap point, and live swipe, and the navigation menu's open item, roving Tab
  stop, and activation direction — surfaced through `onLoadingStatusChange`, `onValueChange`,
  `onOpenChange`, `onScrollStateChange`, `onComplete`, `onSnapPointChange`, `onSwipeChange`, and
  `onActivationDirectionChange`, plus `useScrollAreaState()` and `useDrawerSwipe()` for reading the
  same state anywhere inside a subtree.
- Declared ahead of the core everything it must decide synchronously: eleven new property codes
  (`delay`, `closeDelay`, `length`, `mask`, `readOnly`, `autoSubmit`, `swipeDirection`,
  `viewportSize`, `contentSize`, `overflowEdgeThreshold`, `disablePointerDismissal`) and the
  existing scope, item, value, and orientation declarations. Because the hosted boundary has no
  layout observer, a scroll area declares the `viewportSize` and `contentSize` extents it laid out
  and the core owns every offset, overflow flag, and thumb measurement derived from them.
- Made a duplicate listener impossible rather than fatal: a part whose activation, editing,
  gesture, or dismissal the core owns — a grouped checkbox, a navigation-menu trigger, an OTP slot,
  a scroll area's scrollbar, thumb, and wheel, a drawer's swipe area, and every popup's dismissal —
  now drops the declared listener for that same edge instead of registering a second one for the
  identity. Malformed `items`, `values`, `viewportSize`, or `contentSize` declarations, a duplicate
  navigation-menu value, an empty or oversized snap-point list, and an out-of-range OTP length all
  decline or clamp instead of reaching a core constructor that would panic.
- Added the eight components to `examples/components-solid`, binding tables and worked examples to
  `docs/solid.md`, a Solid section to `docs/base-ui-components.md`, and nine Rust binding tests
  through `TestAppContext` plus seven Solid tests and two protocol tests covering identity, mount
  policy, reported payloads, bounds, and malformed declarations.
- Added the extended styling surface to `@quickgui/native` and `@quickgui/solid`, bumping the
  mutation protocol to v23. Text-bearing nodes gained `letterSpacing`, `wordSpacing`,
  `textTransform`, `textShadow`, `textDecoration`/`textDecorationColor`/`textDecorationStyle`/
  `textDecorationThickness`, `wordBreak`, `overflowWrap`, `hyphens`, and `textDirection`, and
  `textAlign: "start"` and `"end"` now reach the core's own direction-relative `TextAlign::Start`
  and `TextAlign::End` instead of collapsing into the physical edges. Layout gained the inherited
  `direction` with the logical `paddingStart`/`paddingEnd`, `marginStart`/`marginEnd`, and
  `borderStartWidth`/`borderEndWidth` edges, `position: "sticky"` whose `top`/`right`/`bottom`/
  `left` become the core's sticky offsets, `overflowX: "scroll"`, and `scrollSnapType`/
  `scrollSnapAlign`/`scrollSnapStop`. Boxes gained gradient backgrounds, per-corner
  `borderRadius`, `borderStyle`, the `outline` ring with its width, color, offset, and style,
  raster `backgroundImage` with `backgroundSize`/`backgroundRepeat`/`backgroundPosition`, CSS
  `filter` and `backdropFilter` chains, `transform` with `transformOrigin`, `mixBlendMode`, and
  hover, active, and focus variants for exactly the gradient, outline, and transform the core's
  `ElementStateStyle` can swap.
- Parsed every one of those declarations in Rust rather than JavaScript: a new
  `packages/native/src/styles.rs` turns the CSS `linear-gradient()`, `radial-gradient()`, and
  `conic-gradient()` grammars — angles, `to <side>` directions, radial shape, extent and center,
  conic `from` angle, positioned stops, and an `in srgb`/`in oklab` interpolation space — plus CSS
  filter-function lists, transform-function lists, the `matrix()` object form, text shadows,
  one-to-four value corner radii, outline shorthands, background size, repeat, and position, and
  blend-mode keywords into the core's own `Gradient`, `Filter`, `Transform2D`, `TextShadow`,
  `Corners`, `Outline`, `BackgroundSize`, `BackgroundRepeat`, `BackgroundPosition`, and `BlendMode`
  values. Colors inside those strings use the same bounded CSS grammar the renderer packs
  everywhere else. A background image is decoded exactly once per declaration and retained until
  the source changes.
- Bounded the new surface at both ends: a gradient, filter, transform, outline, or text-shadow
  declaration past `MAX_STYLE_DECLARATION_BYTES` (4 096) throws a `TypeError` in JavaScript before
  it can cross N-API, `borderRadius` refuses more than four radii, and anything the Rust grammar
  does not cover — an unknown filter function, an unsupported color, an unparsable angle, a
  malformed object form — declares nothing at all rather than reaching a core constructor. Stops
  past `MAX_GRADIENT_STOPS` (8) and filters past `MAX_FILTERS_PER_ELEMENT` (8) are dropped in
  source order by the core's own bounded types.
- Added the `styling-solid` and `components-solid` examples. The first declares text alignment, the
  new text styles, gradients, per-corner radii, dashed and dotted borders, outlines, filters, a
  backdrop material, transforms with a hover variant, blend modes, right-to-left layout, sticky
  headers, and mandatory scroll snapping; the second puts `Select`, `Combobox`, `Autocomplete`,
  `Table`, `Tree`, `Slider`, `NumberField`, `Splitter`, `Toolbar`, `ToggleGroup`, `Toast`,
  `DateField`, `TimeField`, `Calendar`, `Menubar`, `PopoverMenu`, `ContextMenu`, `Dialog`, and
  `Tabs` in one window.
- Added declared `Select`, `Combobox`, and `Autocomplete` compound components to `@quickgui/solid`,
  bumping the mutation protocol to v22 with the new `options`, `inputValue`, `filterMode`,
  `appearance`, and `commit`-listener properties. The option source is one bounded `items` array or
  a set of child `Option` nodes; the controlled value, the controlled input text, and the filter
  mode are declared ahead of the core's decision. The core opens its own native popover window and
  renders every row from the declared `appearance` block, exactly the way a declared `PopoverMenu`
  renders its rows, so filtering, highlight movement, typeahead, surface placement, dismissal, and
  commit policy all stay in Rust and reach JavaScript only as an asynchronous `componentchange` or
  `commit` payload decoded by `onValueChange`, `onInputValueChange`, `onOpenChange`, `onCommit`, or
  `commitFromEvent`. Because every core mutator that replaces a source, layout, or selection closes
  a live native popover — something a render pass has no `EventContext` for — the binding rebuilds a
  picker's retained state only when the declaration changes and the surface is closed, and adopts
  the deferred declaration on the frame the close already schedules.
- Added declared `Table` (`Root`/`Header`/`Row`/`Cell`) and `Tree` (`Root`/`Row`) compound
  components to `@quickgui/solid`, with the new `columns`, `rowCount`, `rowHeight`, `headerHeight`,
  `selectionMode`, `selection`, `sortColumn`, `sortDirection`, `editing`, `rowIndex`, `columnIndex`,
  `nodes`, `expanded`, `selectedValue`, `setChildren`, `loadingLabel`, and `disclosure` properties.
  Both are on-demand: the core owns the virtual window and reports the range it mounted through
  `onVisibleRangeChange`, so JavaScript declares only the rows on screen for a million-row table.
  Column resizing and reordering, selection policy, keyboard navigation, expansion, the once-only
  lazy-children request, and the inline-edit lifetime stay in Rust and travel back as
  `onSelectionChange`, `onSortChange`, `onActiveCellChange`, `onColumnResize`, `onColumnReorder`,
  `onEditEnd`, `onExpandedChange`, `onLoadChildren`, and `onActivate` payloads keyed by the caller's
  own declared identifiers. A declared header, row, or cell carries content only, because an element
  holds exactly one stable id and the core assigns the grid, tree-item, and active-descendant
  identities itself.
- Added declared `NumberField`, `DateField`, `TimeField`, `Calendar`, `Menubar`, and `Toast`
  compound components to `@quickgui/solid`, with the new `precision`, `civilValue`, `civilMinimum`,
  `civilMaximum`, `segmentOrder`, `segment`, `firstWeekday`, `menuCount`, and `toasts` properties.
  Civil values cross the boundary as ISO `YYYY-MM-DD` and `HH:MM[:SS]` strings with no time zone,
  and a string the core would not accept declares no value at all. The core owns numeric parsing,
  clamping, formatting, and the bounded press-and-hold stepper repeat — which the binding sleeps on
  with one `request_repaint_at` rather than a timer of its own — plus segment arithmetic, digit
  entry, month and year movement, the menubar's roving Tab stop, live-region politeness, and the
  exact toast auto-dismiss deadline. Pushing a toast is adding an entry to the declared `toasts`
  list, and every dismissal the core decided, including the timed ones, reaches JavaScript through
  `onDismiss`.
- Bounded every new declaration at the JavaScript boundary exactly as the Rust binding bounds it:
  `MAX_OPTIONS_JSON_BYTES` (512 KiB), `MAX_COLLECTION_JSON_BYTES` (2 MiB), `MAX_DECLARED_OPTIONS`
  (4,096), `MAX_TABLE_COLUMNS` (512), `MAX_TABLE_ROWS` (1,000,000), `MAX_DECLARED_TREE_NODES`
  (65,536), `MAX_TOASTS` (8), and `MAX_MENUBAR_MENUS` (64). Malformed JSON declares nothing at all,
  duplicate option, column, and node identifiers keep their first occurrence, and the binding
  installs the core's picker, select, combobox, table, tree, date-field, time-field, calendar, and
  menubar key bindings once so declared components adopt the core's typed actions instead of a
  JavaScript keyboard implementation.
- Added declared `Slider`, `Splitter`, `Toolbar`, and `ToggleGroup` compound components to
  `@quickgui/solid`, bumping the mutation protocol to v21 with the new `values`, `items`, `step`,
  `largeStep`, and `componentchange`-listener properties. Bounds, values, pane constraints, and the
  ordered navigation model travel as one bounded declaration, and the hosted view reaches each
  declared instance's retained state through a per-instance `StateAccessor`, so many controls in one
  window stay independent. The Rust core keeps ownership of clamping, step snapping, thumb ordering,
  captured pointer arithmetic, pane-size conservation, wrapping arrow navigation, disabled-item
  skipping, and the single roving Tab stop; results reach JavaScript only as an asynchronous
  `componentchange` payload decoded by `onValueChange`, `onSizesChange`, `onActiveChange`, or
  `componentChangeFromEvent`. The binding installs the core's slider, splitter, toolbar, and
  toggle-group key bindings once, and bounds every declaration: unparsable JSON declares nothing,
  duplicate item values keep the first occurrence, and lists past `MAX_COMPONENT_VALUES` (64) or
  `MAX_COMPONENT_ITEMS` (256) are truncated rather than reaching a core that would panic.
- Added declared `PopoverMenu` and `ContextMenu` compound components to `@quickgui/solid`. Rows are
  one bounded JSON model rather than JSX children, so the Rust core keeps ownership of validation,
  highlighting, typeahead, checkbox/radio policy, submenu models, work-area placement, and the
  cursor-point native surface. `onSelect` reports the application's own stable item id, and the
  binding installs `popover_menu_key_bindings()` once so declared menus adopt the core's contextual
  navigation.
- Added CSS Grid props (`gridTemplateColumns`/`gridTemplateRows`, `gridAutoFlow`, and the
  `gridColumn`/`gridRow` shorthands with their start/end/span forms) and the complete paint
  transition declaration (`transitionProperty`, `transitionDuration`, `transitionTimingFunction`,
  `transitionMaxFps`, plus object and CSS-shorthand forms) mapped onto the core's own
  `TransitionProperties` flags and easing curves.
- Added retained `Image` and `Shader` nodes: a filesystem path stays a lazy core `ImageResource`, a
  base64 `data:` URL is decoded once, `fit` selects the core's `ObjectFit`, and `shaderParameters`
  fills the core's four fixed vectors behind its WGSL validation.
- Added `Progress`, `Meter`, and `Toggle` compound parts over the core's value-range and
  toggle-button descriptors.
- Added declared input listeners — `onKeyDown`/`onKeyUp`, `onMouseDown`/`onMouseUp`/`onMouseMove`,
  `onDoubleClick` with the exact native click count, `onWheel`, `onContextMenu`, `onPinch`,
  `onRotate`, `onSmartMagnify`, `onPressure`, and `onFocus`/`onBlur` — with bounded JSON payloads
  and `keyEventFromEvent`/`mouseEventFromEvent`/`wheelEventFromEvent`/`gestureEventFromEvent`
  decoders. A declared `tabIndex` now makes an ordinary container focusable, matching the web.
- Added a bounded `keymap` prop whose Electron-shaped accelerators are parsed by the core's own
  `Accelerator::parse` and dispatched to JavaScript as one `onAction` event carrying the binding id.
- Added declared drag and drop: `draggable` carries an application-local id plus optional text, URL,
  or file payloads promoted to other applications, `dropKinds` declares the accepted payload kinds
  ahead of the native drag, and `onDragStart`/`onDragEnd`/`onDrop`/`onFilesDropped` report the
  core's typed outcome.
- Bumped the hosted mutation protocol to version 20 for the new component parts, node tags, and
  declared listener properties.

- Added `CrashReporter` and `Metrics` to `@quickgui/native`, both Promise-backed by `AsyncTask`,
  and an `onProgress` option for `Updater.downloadAndStage` delivered through a napi threadsafe
  function from the download worker thread.
- Added `quickgui keygen` and `quickgui build --update-manifest [--update-base-url <url>]`, which
  produce the exact artifact the Rust updater installs for each target, sign it with `minisign` or
  `rsign`, and write a `latest.json` whose platform keys match `default_update_target()`.
- Added `documentTypes` file associations that reach macOS `Info.plist`, the Linux desktop entry
  and `shared-mime-info` package, and the Windows NSIS registry; `icon` PNG-to-`.icns`/`.ico`/
  `hicolor` generation written in pure TypeScript; Linux AppDir/AppImage output and a pure
  TypeScript `.deb` writer; an NSIS installer with shortcuts, uninstall registration, protocol
  handlers, and an Authenticode `signtool` hook; and `quickgui build --mas` for Mac App Store
  `.pkg` submission.
- Added window lifecycle events to `@quickgui/native`: `window.on("minimize" | "restore" |
"maximize" | "unmaximize" | "enterFullScreen" | "leaveFullScreen" | "readyToShow" |
"occlusionChange" | "levelChange" | "willResize" | "willMove" | "resize" | "move" | "focus" |
"blur" | "appearanceChange")`, plus `app.on("activate" | "deactivate")`. `willResize` and
  `willMove` are notifications: the core answers the window manager synchronously, so the narrowing
  is declared ahead with `window.setResizePolicy({ aspectRatio, minimum, maximum, snap })` and
  `window.setMovePolicy({ keepOnScreen })` instead of blocking on a JavaScript callback.
- Added `window.setAlwaysOnTop(flag, level?)` accepting the Electron level names over the extended
  core `WindowLevel`, `window.moveTop()`, `window.moveAbove(other)`,
  `window.setIgnoreMouseEvents(ignore, { forward })`, `window.setEnabled`,
  `window.setAspectRatio(ratio | null)`, `window.setWindowButtonVisibility`, `window.setHasShadow`,
  and `window.getRestoreState()` round-tripping into `WindowOptions.restoreState`.
- Added the application shell to `app`: `setActivationPolicy`, `focus({ steal })`, `hide()`,
  `show()`, `dock.bounce(type)`/`dock.cancelBounce(id)`/`dock.hide()`/`dock.show()`/
  `dock.isVisible()` alongside the dock badge, icon, and menu, `setSecureKeyboardEntryEnabled`,
  `isInApplicationsFolder()`, `moveToApplicationsFolder()`, `isPackaged`, and `Shell.beep()`.
  `app.exit(code)` now maps onto the core's `exit_with_code`. Mutations the operating system applies
  at once stay fire-and-forget; operations with a native answer resolve a Promise from an
  asynchronous event.
- Added `Menu.popup(items, { window, x, y })`, which resolves after the popup closes and releases
  its item callbacks, and `window.setMenu(definitions | null)`. Both reuse `serializeNativeMenu`, so
  popup and per-window items keep the same roles, marks, icons, accelerators, and bounds as the
  application menu.
- Added application-level `SpellChecker.learnWord(word)` and `SpellChecker.ignoreWord(word)` over
  the installed core spell-check provider, bounded to 256 UTF-8 bytes.

- Added `window.onCloseRequested(listener)`, `window.destroy()`, and `window.on("closed" |
"closeRequested", …)`. While at least one listener is registered the window declares close
  interception to the core, which prevents the native close and emits `closeRequested` instead;
  `window.close()` or `window.destroy()` completes the held request, and withdrawing the last
  listener restores ordinary native closing.
- Added `app.on("beforeQuit" | "willQuit", …)`. A `beforeQuit` listener declares quit interception,
  so the core's `on_before_quit` hook prevents the quit and reports `{ reason }` to JavaScript.
  `app.quit()` now asks for a preventable quit, `app.quit({ force: true })` completes a held one,
  and the new `app.exit(code)` force-quits and reports `code` from `app.run()` and the `quit` event.
  `quitMode` semantics are unchanged.
- Added `accelerator` and `hidden` to `MenuActionItem`/`MenuRoleItem`, the new `MenuRole` names, and
  `{ type: "system-menu", menu: "recent-documents" }`. Invalid or oversized accelerators are
  rejected by the core rather than silently dropped.
- Added `window.setTabbingIdentifier`, `selectNextTab`, `selectPreviousTab`, `selectTab`,
  `mergeAllWindows`, `moveTabToNewWindow`, `toggleTabBar`, `toggleTabOverview`, and
  `showCharacterPalette`.
- Added `Clipboard.availableFormats()`, `Clipboard.has(format)`, `Clipboard.readBuffer(format)`,
  `Clipboard.writeBuffer(format, data)`, `Clipboard.readFindText()`, and
  `Clipboard.writeFindText(text)` over the core's typed clipboard entries.

- Added Base-UI-shaped Solid `Checkbox`, `Radio`, `RadioGroup`, `Switch`, `Tabs`, `Collapsible`,
  `Accordion`, `Field`, and `Fieldset` compound parts. Each part is one native node that declares
  which Rust core part descriptor to rebuild, so the core keeps ownership of part identity, roles,
  toggle and selected state, roving Tab and arrow navigation, label/description/error
  relationships, and whether an inactive tab or disclosure panel is mounted at all. Controlled
  values, scope keys, and item values are declared ahead of time as bounded protocol properties;
  nothing about a component is answered by a synchronous JavaScript callback.
- Added Solid `Dialog` and `AlertDialog` compound parts for the Rust core's caller-styled in-window
  modal surface, separate from the operating-system panels in `@quickgui/native`'s `Dialog`
  namespace. The overlay portal is mounted by the core only while the dialog is open, and the
  Escape/backdrop dismissal policy is declared ahead of time instead of answered by a callback.
- Added a `tooltip` property with `tooltipPlacement`, `tooltipDelay`, `tooltipGap`, and
  `tooltipViewportMargin` to every Solid host component, projecting the Rust core's delayed,
  pointer-passive tooltip and its native accessibility description.
- Raised the mutation protocol to version 19 for the new component-part and tooltip properties.

## 0.1.1 - 2026-08-31

### JavaScript tooling

- Changed Solid window mounting to `new Window({ renderer: createRenderer(() => <App />) })`.
  Applications now import native lifecycle APIs from `@quickgui/native` directly; the Solid
  package no longer re-exports them. The Electron-style singleton `app` exposes `whenReady()`, the
  CLI owns its application loop, and creating a window before readiness now throws explicitly.
  Readiness comes from the new windowless Rust-core `Application`/`AppRunner` lifecycle, while
  `Window.getCurrentWindow()` projects the core-routed render or event window.
- Added `@quickgui/cli` project initialization, target-aware production builds, and a stable native
  development host. On macOS, development runs a signed `.app`, loads project TS/TSX on demand,
  restarts without repackaging on source edits, and keeps the prior app alive when a candidate
  cannot start.
- Production macOS builds now retain the signed `.app`, create a versioned DMG with `create-dmg`,
  and can submit, staple, and validate the DMG with an Apple Notary Keychain profile.
- Published the initial `@quickgui/native`, `@quickgui/solid`, and `@quickgui/cli` packages for
  macOS arm64 and x64.
- Added controlled `Input`/`TextArea` and retained core `Markdown` bindings to
  `@quickgui/native`/`@quickgui/solid`, plus a packaged Solid AI chat example using Vercel AI SDK
  streaming and the DeepSeek provider.
- Added Base-UI-shaped `Root`/`Trigger`/`Popup` parts for Solid `Popover` and `SystemPopover`.
  Triggers now register their retained native anchor internally, while only the system popup subtree
  crosses into its parent-owned native child window renderer.
- Fixed native controlled inputs resetting each keystroke before Solid could commit the queued
  value update.
- Fixed the Solid compiler plugin to preserve and then correctly erase TypeScript syntax after JSX
  lowering, allowing typed `.tsx` applications to build for production.

### Framework

- Added a bounded retained native `Markdown` document with CommonMark tables, task lists, streamed
  tail mending, append-only settled-prefix parsing, and cached `StyledText` flattening.

## 0.1.0 - 2026-08-27

Initial macOS-first framework release.

### Framework

- Damage-driven WGPU rendering with clean-window sleep, bounded renderer caches, batched shapes,
  retained text, images, SVGs, paths, custom shaders, shadows, and declarative motion.
- GPUI-shaped retained views with Flexbox, CSS Grid, container queries, inherited typography,
  Tailwind-style helpers, typed actions, entities, globals, async tasks, and multi-window ownership.
- Uniform and measured variable-height virtualization with native-style captured overlay scrollbars.

### macOS

- Native window roles, hidden-inset title bars, traffic-light positioning, display-aware placement,
  menus, dialogs, clipboard, notifications, document state, drag and drop, and platform services.
- Embedded `NSView` children composed between retained WGPU base and overlay planes without a black
  first frame or cross-display startup movement.
- Overflow-capable nonactivating child panels for popovers, menus, select, autocomplete, combobox,
  and context menus.

### Interaction and components

- Keyboard, mouse, IME, editable and selectable text, forms, accessibility, gestures, native
  cursors, tooltips, typed drag and drop, and deterministic input simulation.
- Unstyled selection controls, tabs, fields, disclosures, dialogs, popover menus, pickers, virtual
  tables, and virtual trees with application-owned presentation.

### Validation and resource ownership

- A 561-test library suite, downstream test-support coverage, all-example and benchmark compilation,
  warning-free Clippy/rustdoc gates, and Rust 1.89 downstream package verification.
- Live 100,000-row scrolling and AppKit composition gates enforce frame-time, CPU, memory, cache,
  lifecycle, resize, focus, native-view, popover, and idle-frame budgets.

### Platform scope

- macOS is the accepted 0.1 platform. Windows and Linux compile through Winit/WGPU but do not yet
  carry equivalent native runtime, visual, accessibility, or resource acceptance evidence.
