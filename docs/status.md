# Status and roadmap

[Documentation index](README.md)

Implemented now:

- painted-bounds reporting: `LayoutBoundsHandle` with `Element::report_bounds` publishes an
  element's laid-out window bounds from the paint already being performed and requests one
  correcting frame when they change; the hosted scroll area and splitter bindings size themselves
  from it instead of from declared extents;
- the remaining Base UI components: `Separator` (orientation-only dividers), `Avatar` with an
  `AvatarState` whose Idle/Loading/Loaded/Error status arms one exact fallback deadline,
  `CheckboxGroup` with a 256-value bounded set and a parent checkbox derived from its children,
  `PreviewCard` with 600 ms hover-open and 300 ms pointer-leave close deadlines over the in-window
  `Popover`, `ScrollArea` with caller-styled viewport/scrollbar/thumb/corner parts and a
  `ScrollAreaStyleState` render snapshot beside the untouched built-in overlay scrollbars,
  `OtpField` with up to 12 one-character slots composed from `text_input()` plus auto-advance,
  paste distribution, and `submit_form` auto-submission, `Drawer` with bounded snap points,
  velocity-aware swipe dismissal, and `Dialog`'s focus containment and restoration, and
  `NavigationMenu` with a Navigation landmark, one roving trigger stop, exact 50 ms hover
  deadlines, and per-item popover panels. All eight are unstyled decorators with bounded state and
  no idle source;
- Base UI-shaped menu, select, and combobox parts and props: `MenuState` wraps the in-window
  `Popover` with Base UI's `Menu.Root` props — controlled `open`/`on_open_change`, `modal`,
  `orientation` (a horizontal menubar row installs `POPOVER_MENU_HORIZONTAL_KEY_CONTEXT` so Left
  and Right move the highlight and Down opens a submenu), `loop_focus`, `close_parent_on_esc`
  carried by the `MenuCloseParent` typed action, `disabled`, and Trigger `open_on_hover` with
  `delay`/`close_delay` on exact one-shot deadlines — and publishes `MenuPartState` from the
  resolved anchor placement. `PopoverMenu` gained link items dispatching `OpenMenuLink`,
  `radio_value`/`set_radio_value` keeping exactly one checked value per group, kind-checked
  CheckboxItem/RadioItem/LinkItem/SubmenuTrigger/Separator/GroupLabel decorators,
  accessibility-hidden indicator parts, RadioGroup parts, and `MenuItemPartState`. `SelectState`
  gained the full Base UI part set, `multiple` with a `MAX_SELECT_VALUES`-bounded value set and
  joined value text, `required`/`read_only`/`modal`/`align_item_with_trigger`, the `from_labels`
  items map form, `SelectPartState`, and scroll arrows that advance the option window one row every
  `SELECT_SCROLL_ARROW_INTERVAL` while hovered. `ComboboxState` gained the full Base UI part set,
  `multiple` with `MAX_COMBOBOX_VALUES`-bounded chips and chip removal, `auto_highlight`,
  `open_on_input_click`, `highlight_item_on_hover`, `loop_focus`, `read_only`, `required`, a
  `Contains`-by-default filter policy with `StartsWith`, `None`, and custom `PickerFilter`
  predicates shared with `AutocompleteState`, a polite `Status` live region, an `Empty` mounting
  rule, and `ComboboxPartState`/`ComboboxItemPartState`;
- Base UI-shaped tabs, toolbar, selection-control, dialog, and field props: tab activation direction
  with an anchored indicator and a published active-tab geometry snapshot; toolbar Button/Link/Input/
  Group/Separator parts with keyboard-reachable disabled items; `read_only` checkboxes, radios,
  radio groups, and switches that refuse their transition instead of only styling it, plus a
  registry-free parent-checkbox derivation; a dialog viewport part and a `DialogState` that holds a
  dialog mounted for its own exit transition on an exact deadline; and field item and validity parts
  with a `validation_mode` and bounded revalidation debounce;
- Base UI-shaped toast provider, parts, and manager: an inherited `timeout`, a visible `limit` that
  only flags presentation, an expanded stack with per-toast index and offset, `add`/`update`/
  `close`/`close_all` and `promise`/`resolve` helpers, captured swipe-to-dismiss with the movement
  exposed, and portal, positioner, and content parts beside the existing viewport, root, title,
  description, action, and close decorators;
- Base UI-shaped number-field parts and props: a captured scrub area that converts pointer travel
  into whole steps with a retained remainder and a caller-owned cursor, Shift and Alt large and
  small steps shared by the keyboard, wheel, and scrub paths, `snap_on_step`, opt-out
  `allow_wheel_scrub`, and `read_only`/`required` state projected through the new
  `Element::accessibility_read_only`;
- Base UI-shaped slider parts and props: caller-owned label, value, control, and indicator parts join
  the root, track, and thumb decorators; `min_steps_between_values`, `thumb_alignment` with
  `SliderThumb::offset`, an `onValueCommitted` commit boundary and dragging flag on
  `apply_pointer_change`, a `ValueFormat` hook for the value and per-thumb value text, and a
  copyable `SliderThumbState` carrying the thumb index and dragging state;
- Base UI-shaped progress and meter parts: `track_part`, `label_part`, and `value_part` join the
  root and indicator decorators, a declared identity relates the root to its mounted label and value,
  `ProgressStatus` and `ProgressPartState` expose the three Base UI states, and the bounded
  `ValueFormat` hook supplies the visible value text while `value_text` keeps precedence for
  assistive technology;
- Base UI-shaped compound tooltips: `TooltipProvider` and `TooltipState` decorate caller-owned
  trigger, portal/positioner, popup, and arrow parts with exact `delay`/`close_delay` deadlines,
  shared group `timeout` warmth so an adjacent trigger opens instantly, `hoverable`,
  `close_on_click`, `disabled`, and `track_cursor_axis` pointer following, over the same resolved
  anchor-placement report the popover arrow uses. The existing `Element::tooltip(...)` overlay keeps
  working unchanged;
- Base UI-shaped popover parts and props: `Popover` adds `side`, `align`, `side_offset`,
  `align_offset`, `collision_padding`, `sticky`, `anchor_element`/`anchor_point`, and `modal`
  alongside the existing names, the `portal_part`, `arrow_part`, and `viewport_part` decorators, and
  a copyable `PopoverPartState` snapshot. The arrow follows the placement QuickGUI actually
  resolved: `Element::report_anchor_placement` publishes it into an application-owned
  `AnchorPlacementHandle` during the paint already being performed and requests exactly one
  correcting frame when it changes, so a flipped popover is exact rather than guessed and an
  unflipped one adds no redraw source. `PopoverHoverState` supplies Base UI's `openOnHover` with
  `delay` and `close_delay` as exact one-shot deadlines and a hoverable popup;
- Compositing layers: `Element::transform` / `rotate_degrees` / `scale` / `skew_degrees` with
  `transform_origin` and state-style overrides, `Filter::Blur` and `Filter::DropShadow` over a whole
  subtree, `backdrop_blur` / `backdrop_filter`, and `blend_mode` with eleven exact separable CSS
  blend modes. A qualifying element renders its subtree — Glyphon text included — into a retained
  offscreen texture and composites it back; pointer input is inverse-mapped through the accumulated
  transform. Bounded by `MAX_LAYERS_PER_FRAME`, `MAX_LAYER_DEPTH`, `MAX_LAYER_TEXTURE_BYTES`, and
  `MAX_BLUR_RADIUS`, with over-budget elements painting without their effect and reporting it in
  `RenderStats::skipped_layer_effects`; a scene with no layer effects keeps its former cost exactly.
- per-instance component state accessors: every unstyled component that retains interaction state
  now exposes a `*_with` entry point taking a `StateAccessor<V, State>` — one reference-counted
  closure that may capture which instance it addresses — alongside the original
  `fn(&mut V) -> &mut State` method, which stays a thin wrapper so existing applications, examples,
  and tests are unchanged. `Select`, `Autocomplete`, `Combobox`, `Picker`, `Table`, `Tree`,
  `Slider`, `Splitter`, `Toolbar`, `ToggleGroup`, `Toast`, `DateField`, `TimeField`, `Calendar`,
  `Menubar`, and `ContextMenu` all accept it, so one hosted view can render many declared instances
  of the same component without giving up bounds or adding idle work;
- element geometry on captured pointer events: `PointerEvent::size` carries the captured element's
  own laid-out size, so slider, splitter, and custom drag arithmetic uses the extent layout already
  decided instead of re-deriving it;
- nested interaction-state styling in `@quickgui/ui`, at protocol v30: `style.hover`,
  `active`, `focus`, `disabled`, `invalid`, `dragging`, `dragOver`, `groupHover`, `groupActive`,
  and `focusWithin` objects carrying every property the core's `ElementStateStyle` can swap, group
  states listing one entry per group they follow, a boolean or named `group` prop marking the group
  over the core's new `Element::group`, `group_hover`, `group_active`, and `focus_within` (and the
  `_named` forms for `group/name` targeting), one bounded JSON
  declaration per state, and the flat `hover*`/`active*`/`focus*` names kept as deprecated aliases;
- JavaScript bindings for the Base UI-aligned menu, select, and combobox parts and props, at
  protocol v28: a compound `Menu` (`Root`/`Trigger`/`Portal`/`Backdrop`/`Positioner`/`Popup`/
  `Arrow`/`Item`/`LinkItem`/`SubmenuRoot`/`SubmenuTrigger`/`Group`/`GroupLabel`/`RadioGroup`/
  `RadioItem`/`RadioItemIndicator`/`CheckboxItem`/`CheckboxItemIndicator`/`Separator`) whose rows
  are ordinary application-styled child nodes over the core's `MenuState` and `PopoverMenu` model,
  with `modal`, `orientation`, `loopFocus`, `closeParentOnEsc`, `disabled`, Trigger `openOnHover`/
  `delay`/`closeDelay`, `href` through the core's own open-URL path, and `useMenuState()` /
  `useMenuItemState()` reporting the core's `MenuPartState` and `MenuItemPartState`;
  `ContextMenu.Root` accepting exactly the same row components as a bounded model its cursor-point
  surface paints; `Select.Label`/`Value`/`Icon`/`Backdrop`/`Portal`/`Positioner`/`Popup`/`Arrow`/
  `List`/`Item`/`ItemText`/`ItemIndicator`/`Group`/`GroupLabel`/`Separator`/`ScrollUpArrow`/
  `ScrollDownArrow` with `multiple`, `required`, `readOnly`, `modal`, `alignItemWithTrigger`, the
  `items` map form, and `useSelectState()`; and `Combobox`/`Autocomplete` `Label`/`Value`/`Icon`/
  `Input`/`InputGroup`/`Clear`/`Trigger`/`Chips`/`Chip`/`ChipRemove`/`Status`/`Empty` and the
  declaration-only popup parts, with `multiple` chips, the `filter` policy (`contains`,
  `startsWith`, `fuzzy`, `none`), `autoHighlight`, `openOnInputClick`, `highlightItemOnHover`,
  `loopFocus`, `readOnly`, `required`, and `useComboboxState()` / `useComboboxChips()`;
- JavaScript bindings for the Base UI-aligned compound parts and props, at protocol v25: the
  `Popover` compound (`Portal`/`Backdrop`/`Positioner`/`Popup`/`Arrow`/`Viewport`/`Title`/
  `Description`/`Close`) with `side`, `align`, `sideOffset`, `alignOffset`, `collisionPadding`,
  `sticky`, `anchor` (a node or one `{ x, y }` point), `modal`, and Trigger `openOnHover`/`delay`/
  `closeDelay`; a compound `Tooltip` (`Provider`/`Root`/`Trigger`/`Portal`/`Positioner`/`Popup`/
  `Arrow`) with `timeout`, `hoverable`, `trackCursorAxis`, and `closeOnClick`, beside the unchanged
  framework-owned `tooltip` prop; `Slider.Label`/`Value`/`Control`/`Indicator` with
  `minStepsBetweenValues`, `thumbAlignment`, `format`, `onValueCommitted`, and the core's own
  `dragging` flag; `NumberField.Group`/`ScrubArea`/`ScrubAreaCursor` with `smallStep`, `largeStep`,
  `snapOnStep`, `allowWheelScrub`, `readOnly`, `required`, and a `scrubbing` state;
  `Progress`/`Meter` `Track`/`Label`/`Value` with `format` and a reported `status`; a `Toast`
  provider, portal, positioner, and content with `timeout`, `limit`, `expanded`, `swipeDirection`,
  per-toast `index`/`offset`/`type`, and a `useToastManager()` API over the declared list; tab
  `activationDirection` and indicator geometry; `Toolbar.Button`/`Link`/`Input`/`Group`/`Separator`
  with `focusableWhenDisabled`; `Field.Item`/`Validity` with `validationMode` and
  `validationDebounceTime`; `readOnly` on every selection control and a registry-free parent
  checkbox; and `Dialog.Viewport` with `onOpenChangeComplete`. A declared side or alignment is only
  a preference, so the resolved placement the retained tree published during paint is reported back
  and exposed as a signal-friendly accessor (`usePopoverPlacement`, `useTooltipPlacement`,
  `useSliderState`, `useNumberFieldState`, `useGaugeState`, `useTabsState`), which is how an
  application styles from the real placement the way Base UI styles from `data-side`;
- JavaScript bindings for the Base UI parity components, at protocol v31: QuickGUI UI `Separator`,
  `Avatar` (`Root`/`Image`/`Fallback`), `CheckboxGroup` with `Checkbox.Root value`/`parent`
  members, `PreviewCard`, `ScrollArea`, `OtpField`, and `NavigationMenu`, each using Base
  UI's own compound and prop names. Every root allocates one bounded scope internally, so the parts
  of an instance resolve to the core's derived identities with no registry; the core keeps the
  avatar load status, the checked value set, the hover and close deadlines, the clamped scroll
  offsets and derived overflow flags, every OTP slot and its completion edge, and the navigation menu's open item, roving Tab stop, and activation
  direction, and reports each of them as one asynchronous `componentchange` event. A part whose
  activation, editing, gesture, or dismissal the core owns ignores a declared listener for that same
  edge rather than registering it twice, and a malformed declaration declares nothing instead of
  reaching a core constructor that would panic on it;
- JavaScript bindings for the extended styling surface, at protocol v23: extended text styling
  (`letterSpacing`, `wordSpacing`, `textTransform`, `textShadow`, the `textDecoration` family,
  `wordBreak`, `overflowWrap`, `hyphens`, `textDirection`, and logical `textAlign: "start"|"end"`),
  direction-relative layout (`direction` plus `paddingStart`/`End`, `marginStart`/`End`, and
  `borderStartWidth`/`EndWidth`), `position: "sticky"` insets, `overflowX: "scroll"`, scroll
  snapping (`scrollSnapType`, `scrollSnapAlign`, `scrollSnapStop`), CSS and object-form gradients,
  per-corner `borderRadius`, `borderStyle`, the `outline` ring, raster `backgroundImage` with its
  size, repeat, and position, CSS `filter` and `backdropFilter` chains, `transform` with
  `transformOrigin`, `mixBlendMode`, and hover/active/focus gradient, outline, and transform
  variants. Every declaration is parsed once in Rust into the core's own bounded value type, and a
  declaration the grammar does not cover declares nothing rather than reaching the core;
- JavaScript bindings for the declared option sources: QuickGUI UI `Select`, `Combobox`, and
  `Autocomplete` compound parts declaring a bounded option source — one `items` array or child
  `Option` nodes — the controlled value, the controlled input text, the filter mode, and one
  bounded `appearance` block the Rust core renders every popover row from in its own native window,
  so filtering, highlight movement, typeahead, surface placement, dismissal, and commit policy stay
  in the core and travel back as asynchronous `componentchange` and `commit` payloads;
- JavaScript bindings for the virtual collections: QuickGUI UI `Table` (`Root`/`Header`/`Row`/`Cell`) and
  `Tree` (`Root`/`Row`) declaring columns with sort state and widths, `rowCount`, selection mode
  and controlled selection ranges, the inline-edit position, a tree node source with lazy `pending`
  branches, controlled expansion and selection, and an atomically validated `setChildren` splice,
  with the core owning the virtual window it reports through `onVisibleRangeChange`, column
  resize and reorder, keyboard navigation, and the inline-edit lifetime;
- JavaScript bindings for the remaining stateful field components: QuickGUI UI `NumberField`,
  `DateField`, `TimeField`, `Calendar`, `Menubar`, and a declared `Toast` queue, with the core
  owning numeric parsing and clamping, the bounded stepper repeat, civil-value segment arithmetic,
  month and year movement, menubar roving focus, and the exact toast auto-dismiss deadline the
  binding sleeps on with one `request_repaint_at` instead of a timer of its own;
- JavaScript bindings for the declared range, ordering, and roving-focus components: QuickGUI UI
  `Slider` (single-thumb and range), `Splitter`, `Toolbar`, and `ToggleGroup` compound parts
  declared ahead of time as bounded protocol properties, with the Rust core owning clamping, step
  snapping, thumb ordering, captured pointer arithmetic, pane-size conservation, wrapping arrow
  navigation, disabled-item skipping, and the single roving Tab stop, and every result travelling
  back as one asynchronous `componentchange` payload;
- JavaScript bindings for the unstyled selection, tab, disclosure, and field descriptors: QuickGUI UI
  `Checkbox`, `Radio`, `RadioGroup`, `Switch`, `Tabs`, `Collapsible`, `Accordion`, `Field`, and
  `Fieldset` compound parts declared ahead of time as bounded protocol properties, so the Rust core
  keeps ownership of part identity, roles, roving/arrow keyboard behavior, label and description
  relationships, and inactive-panel mount policy; plus controlled QuickGUI UI `Dialog`/`AlertDialog`
  in-window modal parts whose overlay portal the core mounts only while open and whose
  Escape/backdrop dismissal policy is declared ahead of time, and a delayed native `tooltip`
  property with placement, delay, gap, and viewport-margin controls;
- element background gradients: bounded eight-stop linear, radial (circle/ellipse, three ending-shape
  extents, any center), and conic gradients in linear-sRGB, sRGB, or Oklab, evaluated analytically in
  the existing instanced shape draw, respecting rounded corners, borders, clipping, subtree opacity,
  and damage, swappable in hover/active/focus states, and shared with retained path and canvas fills;
- per-corner radii (`rounded_tl`/`tr`/`br`/`bl`, `rounded_t`/`b`/`l`/`r`, `corner_radii`,
  `rounded_full`) with CSS uniform-scale reduction, analytic dashed and dotted borders distributed
  evenly around the whole rounded outline, and `outline`/`outline_offset` rings painted outside the
  border box without affecting layout;
- bounded CSS-shaped color filters (`brightness`, `contrast`, `saturate`, `grayscale`, `invert`,
  `sepia`, `hue-rotate`, `opacity`) collapsed into one color matrix and applied to an element's own
  image and background-image pixels without an offscreen group texture;
- raster element backgrounds with `Auto`/`Cover`/`Contain`/`Fixed` sizing, four repeat modes, and
  fractional background positions, painted through the existing bounded image primitive and masked
  by the element's rounded corners;
- a cgo-free Go frontend (`packages/go`) that loads the same C ABI host as a `dynamic-host`
  shared library at runtime, so `go build` does not compile Rust or invoke a C compiler, with
  Solid-style signals, a retained native tree, `Show`/`For`/`KeyedFor`, unstyled View/Text/Button
  primitives, and the same Base UI-shaped compound NativePart families as `@quickgui/ui`;
- macOS TypeScript/TSX compilation through scriptc and a static Rust C ABI host with a native main-thread application loop,
  a core-backed singleton application readiness lifecycle, dynamically created `Window` instances,
  independent transactional bounded retained trees,
  window-routed bounded click/hover/input/submit/dismiss delivery, and an unstyled QuickGUI UI renderer
  exposing `View`, `Text`, `Button`, `Input`, `TextArea`, retained core `Markdown`, variable-height
  `VirtualList`, and controlled core-backed in-window `Popover`, plus a project CLI for
  safe initialization, stable real-`.app` development hosts, candidate-first source restart, and
  self-contained signed production packaging; the compiled application pipeline currently targets macOS;
- declared JavaScript popover and context menus over the Rust `PopoverMenu` model and cursor-point
  `ContextMenuState`, with one bounded JSON item model, core-owned validation, typeahead, toggle
  policy, submenu surfaces, and stable-id `onSelect` events; plus JavaScript CSS Grid tracks and
  placement, complete paint-transition declarations, retained image and application-shader nodes,
  progress/meter/toggle parts, and declared key, mouse, double-click, wheel, context-menu, gesture,
  focus, accelerator-keymap, and drag-and-drop listeners with bounded asynchronous payloads;
- macOS/Windows/Linux backend selection through Winit 0.30 and WGPU 30;
- inherited right-to-left layout direction with post-layout mirroring of paint, hit testing, and
  accessibility geometry inside each parent content box, logical `ps`/`pe`/`ms`/`me`/`border_s`/
  `border_e` edges, right-origin horizontal scrolling with inverted wheels, logical
  `text_start()`/`text_end()` alignment, and a forced bidirectional base paragraph direction, with
  document-order focus and logical caret movement preserved;
- CSS-style sticky positioning relative to the nearest scroll container, clamped to the parent box
  so a pinned header releases at its section end, affecting paint and hit testing only with no
  relayout, no per-frame allocation, and a hard `MAX_STICKY_ELEMENTS_PER_WINDOW` bound;
- per-axis scroll snapping with mandatory/proximity strictness, start/center/end child alignment,
  `snap_stop_always()` gesture capture, and resolution at momentum end, at a bounded settle
  deadline for phaseless wheels, at scrollbar release, or programmatically, animating through the
  existing motion machinery on exact deadlines and leaving no idle source behind;
- extended text styling with offset-exact text shadows and a bounded blur approximation, letter and
  word spacing, non-editable case mapping that keeps selection and copy mapped to the original
  string, overline decorations, `word_break`/`overflow_wrap` break control, and soft-hyphen
  handling, all folded into the canonical retained shaping key;
- validated immutable Rust-core application identity, app-scoped standard path resolution with
  explicit overrides, and bounded startup snapshots for OS/version/architecture/hostname and
  preferred locale/languages, shared directly by runner, view, event, and deterministic test
  contexts without render-time polling;
- one-shot Rust-core relaunch requests that preserve or explicitly replace the native process
  command, spawn only after structured application and service teardown, and remain inspectable
  without spawning in deterministic tests; plus HTTPS/Minisign updater download progress,
  mandatory install-time re-verification, confined bounded archive extraction, rollback-capable
  macOS bundle/Linux executable replacement, and Windows installer launch policies;
- bounded Rust-core crash reporting with a panic hook, an async-signal-safe native fatal-fault
  writer using a pre-opened descriptor and a pre-rendered report template, retention/parsing/
  deletion of stored reports, HTTPS upload of pending reports, and an opt-in main-thread hang
  watchdog that is disabled by default; plus explicit process metrics (CPU time, resident and
  macOS physical-footprint memory, virtual size, thread count, uptime), a stateful CPU-usage
  sampler, and whole-system memory readings, all exposed to JavaScript through Promise-backed
  `CrashReporter` and `Metrics` bindings;
- `@quickgui/cli` production packaging beyond the macOS DMG: declarative `documentTypes` file
  associations rendered into `Info.plist`, Linux desktop entries and `shared-mime-info`, and NSIS
  registry entries; pure-TypeScript `.icns`/`.ico`/`hicolor` icon generation from one square PNG;
  Linux AppDir/AppImage and a pure-TypeScript `.deb` writer; an NSIS installer with protocol and
  file-association registration plus an Authenticode signing hook; `quickgui build --mas` for Mac
  App Store `.pkg` submission; and `quickgui keygen` plus `quickgui build --update-manifest`
  producing the exact per-platform artifact the Rust updater installs, signed with Minisign and
  described by a `latest.json` both sides parse from one shared fixture;
- renderer-independent Rust-core RAII power assertions, bounded battery/thermal/low-power/CPU-limit
  snapshots, explicit idle and login-session queries, and event-driven suspend, lock, shutdown,
  source, thermal, and energy-mode transitions with platform-native resource teardown;
- bounded renderer-independent system preference snapshots with appearance/accessibility flags and
  semantic native colors, selective declarative invalidation, plus explicit camera, microphone,
  screen-recording, and accessibility permission status/request services;
- sleeping heterogeneous multi-window runtime with stable handles, close interception, targeted focus/close/invalidation, resize, DPI, pointer, wheel, keyboard-layout-aware command identity, full IME preedit/commit routing, and focus events;
- parent-owned normal, dialog-sheet, floating, and transient popover roles with retained restore
  bounds, initial/runtime minimum and maximum inner sizes, independent close/minimize/maximize/
  resize/move policies, decorations, shadow, capture protection, explicit/role-derived stacking,
  focusability, taskbar/workspace visibility, opacity/icons, retained cursor visibility/grab/
  position/hit testing, declarative native-state snapshots, a bounded core-owned application-window
  registry, hidden first-frame creation, child-first teardown, and bounded targeted runtime window
  commands;
- native window lifecycle events (`Minimized`, `Maximized`, `FullscreenChanged`, `FirstPresented`
  ready-to-show, `OcclusionChanged`, `WindowLevelChanged`) derived event-driven from the AppKit and
  Winit events that accompany each transition, plus `WillResize`/`WillMove` constrain hooks whose
  single bounded corrective resize or move can never loop, and application-wide
  `on_did_become_active`/`on_did_resign_active` callbacks reusing the existing macOS observer;
- AppKit-exact window stacking levels (`Floating`, `ModalPanel`, `MainMenu`, `Status`, `PopUpMenu`,
  `ScreenSaver` with topmost fallbacks elsewhere), `move_window_top`/`move_window_above` restacking
  without activation, click-through with optional pointer-motion forwarding, visible-but-inert
  window input policy, validated content aspect ratios with a portable resize clamp, macOS
  traffic-light visibility, and runtime shadow mutation;
- serde-serializable `WindowRestoreState` captured from `WindowState::restore_state` and applied
  with `WindowOptions::restore`, matching the remembered display by stable UUID then process
  identifier, keeping intersecting bounds exactly, clamping into the remembered work area, and
  falling back to centered placement for a disconnected display or an invalid persisted value;
- application-shell services: activation policy, activation, hide/unhide, Dock attention with
  cancellation, Dock visibility, secure keyboard entry, system alert sound, Applications-folder
  support query and move, packaged-process detection, and `exit_with_code` applied after ordinary
  structured teardown;
- renderer-independent desktop capability queries plus Windows taskbar progress/overlays and Jump
  List user tasks; macOS Dock badges/icons/typed menus; macOS/Windows recent documents, native About
  panels, and file-icon lookup; unified bounded initial and second-instance deep-link callbacks;
- true nonactivating macOS `NSPanel` popovers with retained-element anchors, work-area flip/slide/resize constraints, cross-parent-edge rendering, independent grab/key-window eligibility, nested grab dismissal, interactive never-key children, and passive no-monitor operation;
- window-owned macOS `NSAlert`, `NSOpenPanel`, and `NSSavePanel` futures with cancellation-safe lifecycle and hard request/result bounds, plus `NSWorkspace` URL/open/reveal actions and default Cmd-W close routing;
- lazy bounded application clipboard items with text and hash-bound metadata, arbitrary MIME/HTML/
  RTF data, URL bookmarks, encoded images, native file lists, direct macOS general/Find pasteboards,
  cross-platform fallback projection, deterministic in-memory tests, and one shared editor-shortcut
  path;
- bounded macOS/Windows/Linux system notifications with tag replacement, buttons and replies,
  permission status/request futures, application-wide response callbacks, and target-specific icon,
  attachment, sound, and scheduling support; plus opt-in URL-open, Dock-reopen, and system-wake
  lifecycle callbacks;
- GPUI-shaped `QuitMode` semantics, preventable before/will-quit phases, structured child-first
  application teardown, post-destruction window callbacks, and a zero-work windowless macOS
  Dock-reopen state, plus an `AppRunner::request_quit()` preventable quit for embedding runtimes;
- explicit Electron-syntax menu accelerators (`Accelerator::parse`, `MenuItem::accelerator`/
  `keystroke`/`try_accelerator`) that outrank keymap derivation in AppKit key equivalents and
  register matching Win32 accelerator-table bindings, bounded by `MAX_ACCELERATOR_BYTES`, plus
  `MenuItem::hidden`, an operating-system-populated `SystemMenuType::RecentDocuments` submenu, and
  paste-and-match-style, delete, speech, and window-tab menu roles with retained fallbacks;
- `AppRunner`-level character-palette, tabbing-identifier, tab selection/merge/detach/bar/overview
  commands and macOS Find-pasteboard access, so an externally pumped host reaches the same document
  window and search integrations as `EventContext`;
- declared JavaScript lifecycle vetoes: `window.onCloseRequested`/`window.destroy()` and
  `app.on("beforeQuit")`/`app.on("willQuit")` with `app.quit({ force })`/`app.exit(code)`, where
  interception is declared to the core ahead of the native decision and completed later by an
  explicit host command, never by a synchronous JavaScript veto;
- JavaScript menu `accelerator`/`hidden` declarations, the new menu roles, a
  `{ type: "system-menu", menu: "recent-documents" }` submenu, imperative window-tab and
  character-palette commands, and Electron-shaped clipboard `availableFormats`/`has`/`readBuffer`/
  `writeBuffer`/`readFindText`/`writeFindText` helpers over the core's typed entries;
- JavaScript window lifecycle events — `window.on("minimize"|"restore"|"maximize"|"unmaximize"|
  "enterFullScreen"|"leaveFullScreen"|"readyToShow"|"occlusionChange"|"levelChange"|"willResize"|
  "willMove"|"resize"|"move"|"focus"|"blur"|"appearanceChange"|"closed")` and
  `app.on("activate"|"deactivate")` — where `willResize`/`willMove` are notifications and the
  narrowing itself is declared ahead as `window.setResizePolicy({ aspectRatio, minimum, maximum,
  snap })` / `window.setMovePolicy({ keepOnScreen })`, answered synchronously by the core through
  `constrain_resize`/`constrain_move`;
- JavaScript window stacking, input, and state commands: `setAlwaysOnTop(flag, level)` over the
  extended `WindowLevel` with Electron level names, `moveTop`, `moveAbove`, `setIgnoreMouseEvents`,
  `setEnabled`, `setAspectRatio`, `setWindowButtonVisibility`, `setHasShadow`, and
  `getRestoreState()` round-tripping into `WindowOptions.restoreState`;
- JavaScript application shell: `app.setActivationPolicy`, `app.focus({ steal })`, `app.hide()`,
  `app.show()`, `app.dock.bounce`/`cancelBounce`/`hide`/`show`/`isVisible` alongside the dock badge,
  icon, and menu, `app.setSecureKeyboardEntryEnabled`, `Shell.beep()`,
  `app.isInApplicationsFolder()`/`app.moveToApplicationsFolder()`, `app.isPackaged`, and
  `app.exit(code)` mapped onto the core's `exit_with_code`, with fire-and-forget mutations kept
  distinct from the operations whose native outcome resolves a Promise;
- `AppRunner`-level stacking, input-policy, restore-state, and application-shell commands, plus the
  deferred per-window-menu and native popup-menu requests an externally pumped host needs; both
  menu queues resolve inside the runtime's own window-scoped effect cycle and carry the public
  `MAX_PENDING_NATIVE_POPUP_MENUS` bound;
- JavaScript `Menu.popup(items, { window, x, y })` resolving after the popup closes and
  `window.setMenu(definitions | null)`, both reusing the application-menu item grammar, plus
  application-level `SpellChecker.learnWord`/`ignoreWord` over the installed provider;
- bounded immutable active-display snapshots with global logical work areas, scale/refresh metadata, stable macOS UUIDs, declarative change observation, current-display window state, display-targeted centered placement/fullscreen, disconnect fallback, and deterministic no-polling tests;
- display rotation, built-in-panel, and color-depth metadata read from Core Graphics and `NSScreen`, plus bounded deterministic `Displays::diff` snapshots projected as granular `DisplayEvent::{Added, Removed, MetricsChanged}` through `Application::on_display_event` at the existing screen-parameters boundary, with the coarse snapshot observation unchanged and no added polling;
- native message boxes with a suppression checkbox, custom icon, separate message/detail text, and explicit default/cancel button indices, whose bounded response carries both the chosen button and the checkbox state, plus open-panel create-directory/alias/file-package/message options and save-panel name-field-label and tag-field options with an honest per-OS support table;
- macOS Quick Look file previews driven by a core-owned `QLPreviewPanel` data source, with bounded paths and display names and explicit dismissal;
- system color and font panels observed through one core-owned AppKit responder and `Application::on_color_panel_change`/`on_font_panel_change`, with continuous and on-close reporting modes and no observation while a panel is closed;
- `NSSharingServicePicker` share sheets anchored inside the current window's content view, and `LAContext` biometric authentication whose background-queue reply is completed on the main thread through the bounded platform request/response queue;
- native image model additions on `Image`: base64 `data:` URL decoding, macOS `NSImage`/SF Symbol lookup rasterized at a requested point size and scale, bilinear `resize`, whole-pixel `crop`, PNG/JPEG encoding, template metadata, and multi-scale representations honored when building an `NSImage` for Dock, About, message-box, and menu icons;
- tray-icon screen bounds in global logical desktop coordinates and per-artwork macOS template rendering through `TrayIconImage::template`;
- main-thread `Entity<T>`/`WeakEntity<T>` shared state with retained per-window observation, coalesced bounded notification fan-out, and automatic conditional unsubscribe, plus typed `EventEmitter` delivery with RAII `Subscription` lifetimes and bounded deterministic queues;
- main-thread application globals with exact typed access, conditional per-window observation, RAII change subscriptions, deterministic deferred delivery, and bounded notification fan-out;
- cancellable main-thread foreground futures with `Task` drop/detach semantics, fallible typed async view updates, exact event-loop timers, structured window-close cancellation, and hard task/poll/update/timer bounds;
- bounded deterministic `TestAppContext` coverage for retained view updates, semantic click/focus/input/form interaction, two-phase typed actions and focused raw keys, contextual keymaps, globals, foreground futures/timers, application lifecycle hooks, and heterogeneous window ownership without creating native or GPU resources;
- lazy deterministic `VisualTestContext` geometry assertions and bounded physical-pixel screenshots through the production Taffy, Cosmic Text, scene-ordering, and WGPU pipelines, with explicit comparison tolerances, exact injected time, and no native window or presentation loop;
- compile-time opt-in per-window retained-tree inspection with topmost pointer picking, occluded-layer wheel traversal, frozen selection, an independent WGPU overlay tree, and bounded hierarchy/layout/clip/paint/hit/focus/accessibility/frame/cache snapshots; disabled builds contain no inspector state or hot-path branches and an open inspector owns no idle scheduler source;
- compatible windows share WGPU device/queue ownership, one format-matched immutable shape
  pipeline, Glyphon's device pipeline cache, and one lazy bounded image worker pool while retaining
  independent surfaces, uniforms, upload buffers, glyph atlases, schedulers, input state, and
  bounded render caches; path, image, SVG, and application-shader renderers initialize only after
  their first scene primitive;
- immutable application assets with normalized paths, zero-copy static bytes, deterministic bounded listings, stable worker-decoded image identities, direct view/event access, and validated custom fonts in one application-wide main-thread font database;
- declarative elements, retained Taffy Flexbox with GPUI-shaped reverse flow, sizing, alignment, aspect-ratio, axis-gap, negative-margin, and auto-margin helpers, CSS Grid, layout-isolated parent-size container queries, absolute positioning, clipping, semantically complete `display: none` and layout-preserving visibility, inherited text alignment, complete fonts, bounded canonical OpenType features, ordered custom fallback families, slant, GPU-instanced solid/wavy decorations and fixed thickness helpers, and Tailwind-like helpers;
- multiplicative GPUI/web-style subtree opacity across all GPU primitives and macOS `NSView`
  children, with zero-opacity interaction/accessibility retention, paint-only transitions, and no
  rich-text reshaping or offscreen group allocation;
- native-style overlay scrollbars with 12-point hit tracks, captured thumb/track dragging, hover expansion, and one-shot auto-hide deadlines;
- GPUI-compatible typed native cursors with Tailwind-style helpers, topmost/default overrides, automatic text and drag cursors, disabled-state semantics, stationary-pointer refresh, and no cursor-owned scheduling;
- event-driven Force Touch, pinch, rotation, and smart-magnify input with bounded allocation-free payloads, topmost ancestor targeting, overlay blocking, and deterministic headless simulation;
- captured raw direct multi-contact touch with stable contact-lifetime IDs, child-first bubbling, focus-loss cancellation, a 32-contact hard bound, deterministic simulation, and macOS `NSTouch` delivery through the existing content view; indirect trackpad contacts stay on their native gesture and scroll paths because they have no window position;
- GPUI-shaped element scroll-wheel events with precise pixel/line deltas, native gesture phases,
  child-first bubbling, independent web-style propagation/default control, bounded payloads, and
  an unchanged coalesced fast path for ordinary retained scrolling;
- bounded desktop mouse down/up/move/exit dispatch with outside capture, root-to-target capture,
  target-to-root bubble, independent propagation/default prevention, exact AppKit multi-click
  counts, stationary-layout hover transitions, reusable scratch storage, and deterministic
  headless/visual simulation, while direct pointer capture remains available for scrollbars and
  custom drags;
- event-driven native light/dark window appearance with system following, explicit per-window overrides, declarative observation, and deterministic no-polling simulation;
- runtime opaque, transparent, and native-blurred window backgrounds with retained macOS Metal surfaces, component-diffed compositor mutation, and no idle work;
- macOS hidden-inset titlebars with native traffic-light positioning and explicit topmost `drag`/`no-drag` app regions;
- bounded native document-window integration with filesystem-preserving represented URLs, edited-state indication, character palette commands, opt-in AppKit system tabs, constant-size observable tab snapshots, deterministic simulation, and restoration of the process-wide automatic-tabbing policy without polling;
- ordered `z_index` stacking layers plus portal-style overlays with anchor flip/shift, pointer blocking, outside/Escape dismissal, and focus restoration;
- controlled window-bounded popovers with caller-owned unstyled trigger, combined portal/positioner,
  popover, backdrop, title, description, and close parts; paired and derived stable IDs, exact
  expanded/controls/has-popup and visible-label relationships, optional same-turn initial focus,
  independent topmost Escape/outside dismissal, merged-surface shorthand, and no component-owned
  store or scheduling, plus an unstyled `SystemPopover` child-window host for
  cross-parent-edge content;
- controlled unstyled in-window dialogs and alert dialogs with caller-owned portal/backdrop/popover/title/description/close parts, nested z-ordered focus containment, independent Escape/backdrop policy, exact focus restoration, modal AccessKit roles and visible relationships, overlay-plane native-view occlusion, and no component-owned scheduling;
- arbitrary delayed GPU tooltips with keyboard accessibility and exact one-shot scheduling, plus
  an unstyled `ContextMenuState` adapter with bubbling secondary-click triggers, caller-owned
  popover/row parts, native cursor-point overflow surfaces, nested `PopoverMenu` models, exact close
  synchronization, exact group-label/separator accessibility, typed owner actions, and zero
  closed-state work;
- macOS `NSView` children composed between the base and transparent overlay WGPU surfaces, with atomic first-frame reveal, keyed lifetime, clipping, sizing, first-responder handoff, and merged AccessKit/AppKit accessibility routing;
- keyed hover/active/focus/click state, captured pointer gestures, Tab traversal, keyboard button activation, and type-safe `ViewContext` listeners;
- typed actions with root-to-focus capture and default-consuming focus-to-root bubble, two-phase focused raw key-down/up listeners with independent propagation/default prevention, non-focusable focus scopes, contextual keymaps, programmatic command dispatch, and replay-safe multi-stroke bindings;
- bounded macOS keyboard-layout snapshots with event-driven observation, fixed command/Option/Shift translation tables, non-Latin and Dvorak-QWERTY command handling, alternate printable-character matching, opt-in Apple-localized key equivalents shared with native menus, and deterministic no-polling simulation;
- reusable unstyled command-palette pickers with caller-owned input/empty/row/root presentation,
  bounded fuzzy matching, UTF-8-safe highlight ranges, contextual keyboard navigation,
  visible-only rows, focus restoration, and heterogeneous typed-action dispatch;
- standalone unstyled `SelectState` with caller-owned trigger/listbox/option presentation on the
  cross-parent-edge native popover host, exact child-close synchronization, stable identities,
  preview/commit/cancel, bounded typeahead, disabled options, atomic source replacement, form
  validation, active-descendant accessibility, and visible-only rows; standalone unstyled
  `AutocompleteState` with caller-owned input/popover/option parts, arbitrary bounded text,
  completion or action-only commits, fuzzy or externally filtered sources, in-place shared child
  snapshots, a never-key cross-edge suggestion panel, owner-IME focus, owner-tree accessibility
  proxies, and visible-only rows; and standalone unstyled `ComboboxState` with a distinct declared
  value/edit-query contract, caller-owned parts, exact committed-label restoration, stable async
  source rebinding, the same never-key overflow host, and ordinary text-editing/IME ownership;
- controlled unstyled virtual data collections with one composite focus target: an
  application-rendered sortable million-row table with caller-owned headers/cells, visible-only
  rendering, and exact grid/header/cell semantics, plus a bounded million-node tree with
  caller-composed rows/disclosures, a compact preorder arena, stable-ID selection/expansion,
  atomic source replacement, visible-index rebuilding, disabled-node navigation, and exact
  hierarchy/set semantics; both retain only bounded layout geometry, reuse `ListState` scrolling,
  and add no idle scheduler source;
- range-merged bounded table row selection with Shift ranges, platform-modified toggles, Select All,
  `multiselectable`/`selected` projection, and a selection-change typed action; caller-placed
  behavior-only column-resize handles with captured pointer drag, declared minimum widths, keyboard
  resizing, and splitter semantics; keyboard column reordering over a retained display order that
  keeps declared cell positions stable; and an inline-edit hook whose editor cell owns a deeper key
  context with Return/Escape commit and cancel actions;
- lazy tree children: a pending branch mounts exactly one bounded loading placeholder row, asks the
  application once through a typed load action, and accepts a validated atomic child splice that
  preserves selection, expansion, and the logical scroll anchor or leaves the tree untouched;
- controlled unstyled segmented date and time fields over plain `CivilDate`/`CivilTime` values with
  leap-year validation, configurable YMD/DMY/MDY order, 12- or 24-hour presentation over one
  retained 24-hour value, typed-digit entry with automatic advance, wrapping arrow steps, per-segment
  placeholders and bounds, min/max validity, and one spin button per segment inside a group root,
  plus a bounded six-week month grid with roving day focus, month/year navigation that follows focus
  across boundaries, a declared week-start weekday, selection bounds that refuse selection rather
  than movement, and exact grid/row/cell semantics;
- an unstyled in-window menubar with one roving Tab stop over at most 64 caller-declared menus,
  wrapping arrow navigation that switches rather than closes while a menu is open, open-on-click,
  hover switching only while the bar is open, Escape closing without leaving the bar, and exact
  menubar/menu-item semantics over ordinary `PopoverMenu` surfaces;
- typed drag/drop with GPU previews and paint-only source/target states, arbitrary process-local values crossing macOS windows without serialization, plus bounded inbound/outbound file, text, and URL formats;
- bounded native macOS/Windows application, per-window, and popup menus with standard roles,
  nested menus, macOS system-owned submenus, contextual key equivalents, focused command
  validation, dynamic replacement/removal/query, controlled check/radio marks, item icons, and
  native responder actions;
- controlled plain or attributed single-line and wrapped multiline text editing with grapheme/word/line navigation and deletion, visual-line caret movement, mouse caret and drag selection, two-axis scrolling, copy/cut/paste, IME composition, and bounded text-plus-style undo/redo history;
- per-input and application-wide text checking: settle-deadline spelling/grammar underlining over a
  bounded 16 KiB caret window with no polling, wavy-underline projection merged over controlled runs
  without rewriting them, caret-word skipping, boundary autocorrect and replacement-dictionary
  substitutions as one undoable edit with "Change back", insertion-time smart quotes and dashes,
  bounded guesses/learn/ignore with per-input `NSSpellChecker` document tags, typed
  `ReplaceWord`/`LearnWord`/`IgnoreWord` menu items, and a pluggable `SpellCheckProvider` with a
  portable `Unsupported` fallback;
- macOS dictionary lookup for a selection or the caret word through
  `NSView showDefinitionForAttributedString:atPoint:`, exposed as a bounded `show_definition_for`
  request plus a typed `LookUpSelection` action and opt-in force-click behavior;
- bounded literal find and replace with case-sensitive/whole-word options, 4,096 retained matches,
  wrap-around navigation, distinct all-match and current-match highlight runs, single-edit
  replace-all, and an unstyled `FindBar` over caller-owned query/replace/count/navigation/dismissal
  parts with Return/Shift+Return/Escape behavior and an accessible match count;
- application-wide `UndoManager` with 256 bounded named entries, reverse-order grouping, a
  value-based `UndoableChange<T>` shortcut, menu-ready action names, and a documented policy that a
  focused text input's own history claims Undo and Redo first;
- semantic browser-style forms with nearest-form Return and submit-button routing, shared controlled field data, bounded document-order validation reports, deterministic first-invalid focus, and one-shot AccessKit live announcements;
- controlled unstyled `Checkbox`, `Radio`, `RadioGroup`, and `Switch` descriptors with
  caller-owned roots, indicators, thumbs, layout, paint, and motion; exact AccessKit toggle roles,
  desktop arrow cursors, hidden-inset drag exclusion, roving Tab/arrow radio behavior,
  accessibility-hidden decorative parts, and no component-owned allocation or scheduling;
- controlled unstyled in-window `Tabs`/`Tab` parts with caller-owned root/list/tab/indicator/panel
  presentation, manual or automatic horizontal/vertical navigation, a single roving Tab stop,
  disabled-item skipping, optional looping and retained panels, exact AccessKit tab relationships,
  deterministic idle coverage, and no component-owned registry or scheduling;
- controlled unstyled `Slider`, `Splitter`, `NumberField`, `Progress`, `Meter`, `Toolbar`,
  `Toggle`/`ToggleGroup`, and `ToastManager`/`ToastViewport` components with caller-owned parts:
  bounded slider bounds/step/thumb ordering with captured pointer arithmetic in either orientation
  and typed arrow/page/Home/End actions, size-conserving splitter panes with minimum sizes,
  collapse, and captured-delta drags, caller-supplied number parsing/formatting separators with
  commit-time clamping and press-and-hold stepping on exact one-shot deadlines, determinate and
  indeterminate progress plus meters with no framework-owned animation, one roving Tab stop with
  disabled-item skipping across toolbars and toggle groups, and a bounded toast queue with polite or
  assertive live regions, pause on hover or focus, focused Escape dismissal, and no idle source once
  drained;
- AccessKit Slider, SpinButton, ProgressIndicator, Meter, Splitter, Toolbar, pressed-button, Alert,
  and Status projections with numeric value/minimum/maximum/step, orientation, and live-region
  politeness;
- retained `StyledText` with bounded Unicode-safe byte ranges, per-run family/features/fallbacks/weight/slant/foreground/background/decorations, inherited left/center/right/justified alignment, GPUI-compatible whitespace/end/start/middle ellipsis/truncate/line-clamp helpers, cached Unicode-grapheme-safe overflow projection back to original selection offsets, wrapped BiDi shaping, visible-only decoration geometry, and document-order selection shared with ordinary text;
- bounded retained native `Markdown` with CommonMark tables, task lists, quotes, code, semantic
  application-owned styling, append-only settled-prefix parsing, streamed-tail marker mending,
  cached `StyledText` flattening, UTF-8-safe source limits, and no webview, network ownership, or
  idle work;
- AccessKit trees with semantic roles, labels, disabled/selected state, native focus/click actions,
  editable or immutable text-selection actions, distinct select/editable combobox roles and
  autocomplete state, active-descendant relationships, exact tab/list/panel orientation and
  relationships, exact option/table/tree positions, table row/column counts and indices, tree
  levels/set positions, expanded state, and sort direction;
- linear-light colors, premultiplied blending, analytic rounded rectangles, borders, CSS-ordered drop/inset shadows, and HiDPI rendering;
- static PNG/JPEG/TIFF/WebP/GIF/RGBA, asynchronous, and animated GIF/WebP images with intrinsic
  layout, all web `object-fit` modes, rounded clipping, GPU grayscale, delayed loading/error
  fallbacks, per-element playback, Reduce Motion, stable identity, and hard-bounded CPU/GPU caches;
- retained SVG/SVGZ icons and tessellated fill/stroke paths with intrinsic layout, web `object-fit`, transforms, dashes, arcs, two-stop gradients, analytic boundary antialiasing, and scoped custom canvas painting;
- retained validated WGSL rectangle effects with framework-owned clipping and blending, bounded per-window pipeline caching, four per-instance parameter vectors, and one triple-buffered instanced upload;
- bounded declarative one-shot, chained, local-repeat, and application-synchronized duration animations with finite easing, optional exact max-FPS deadlines, retained terminal values, occlusion-aware pause/resume, Reduce Motion static resolution, and independently owned tooltip/drag-preview motion that never rebuilds the application view;
- retargetable declaration-time springs with retained position and velocity, analytic frame-rate-independent stepping across damping regimes, explicit playback states, finite-input recovery, and no delayed-frame catch-up loop;
- web-style paint-only transitions for interaction and application state across background, border, radius, subtree opacity, inherited text color, and fixed-cap shadow lists, with continuous retargeting, optional exact throttling, occlusion pause, Reduce Motion resolution, and no view or layout rebuild per frame;
- Cosmic Text/Glyphon shaping, real OpenType feature application, primary/custom/platform ordered fallback, rasterization, atlas reuse, and bounded canonical text-layout retention;
- fixed-height O(1) virtualization plus sparse measured variable-height lists with stable logical
  anchors, bottom/tail following, targeted remeasurement, clamped scrolling, shared native-style
  scrollbar capture, hard metric/mount bounds, true Unix application-thread CPU telemetry, a
  self-terminating macOS scroll/idle/process-CPU/peak-RSS gate, deterministic no-rebuild
  retained-scroll coverage, and a separate live
  resize/wrapped-text/native-child/lifecycle/first-responder/document-chrome gate with
  context-menu interaction plus a 128-cycle current-memory plateau, presentation, CPU, cache,
  draw-call, idle, and peak-RSS budgets.

## Roadmap

QuickGUI is macOS-first. Roadmap priority follows the needs of an editor-class desktop app:
correct input, text, windows, accessibility, predictable resource ownership, and measured runtime
behavior. “GPUI parity” here means those reusable framework contracts, not every Zed-specific
service or every API exposed by AppKit.

Roadmap state has a strict meaning:

- **Implemented** means source, deterministic tests, a public example or guide, and explicit
  ownership/resource bounds exist in the current working tree.
- **Live accepted** additionally means the named behavior was exercised in a real native window on
  every platform claimed by that milestone.
- **Release-ready** additionally requires the clean candidate, package, toolchain, and publication
  gates. Passing a unit test or compiling an example is never promoted to native visual proof.

QuickGUI 0.1.0 was published to crates.io on 2026-08-27 after the core macOS framework surface,
local release evidence, and registry package checks completed. Broad ecosystem parity is not
claimed. Further components, unrelated input devices, and application-owned services belong to
later releases; a 0.1 correction now requires a new patch version because registry releases are
immutable.
Milestones are evidence-gated rather than date-gated:

| Milestone | Product boundary | Exit criteria |
| --- | --- | --- |
| 0.1 | **Released 2026-08-27.** macOS-first framework foundation for editor-class applications. | P0.1 through P0.4 below, including a clean downstream Rust 1.89 check using only the published registry crates. |
| 0.2 | Productive macOS application development above the rendering/runtime foundation. | An unstyled component contract with live-accepted popover menu, select/autocomplete/combobox, dialog, document-window, inspector, table, and tree workflows plus broader assistive-technology coverage. |
| 0.3 | Credible Windows and Linux runtime parity. | Native visual, IME, accessibility, clipboard, drag/drop, window-role, and performance gates on both platforms; compilation alone does not qualify. |
| 1.0 | Stable cross-platform contract. | Supported-platform acceptance and resource budgets are green, public API compatibility/deprecation policy is documented, and no known release-blocking lifecycle, text, input, window, or accessibility defects remain. |

The parity lanes are deliberately uneven:

| Lane | Status | Next gap |
| --- | --- | --- |
| Retained rendering, layout, text, images, animation, and virtualization | 0.1 surface implemented and current resource gates green | Preserve the CPU, frame-time, memory, cache, and idle budgets as the framework grows. |
| Focus, keyboard, typed actions, mouse, gestures, selection, forms, drag/drop, and IME | 0.1 surface implemented | Broaden manual real-device, IME, and VoiceOver workflow coverage after 0.1 without blocking the frozen release. |
| macOS windows, menus, document state, platform services, native views, and accessibility projection | 0.1 source and focused display evidence complete; optional system tabs are deferred | Add mixed-scale and broader VoiceOver/native-child acceptance when the required hardware or workflow is available. |
| State, globals, entities, foreground work, deterministic tests, visual tests, and inspection | Inspector implementation complete in the working tree | Run its live macOS overlay/native-child smoke while keeping disabled builds and closed inspectors at zero added idle work. |
| High-level reusable application components and inspector tooling | Unstyled selection-control, tabs, popover, popover-menu, context-menu, standalone select, free-form autocomplete, constrained combobox, dialog, alert-dialog, field, fieldset, collapsible, accordion, picker, sortable virtual-table, and bounded virtual-tree foundations are source-complete. Context menus include bounded delayed hover and a right/left safe pointer corridor over actual child geometry. | Continue live accessibility, IME, pointer, multi-monitor, repeated-open, and large-data acceptance under 0.2, then follow the dependency-ordered [component ledger](component-roadmap.md). |
| Windows/Linux behavioral parity | Compile-time foundation only | 0.3 native acceptance and platform integration. |

The implemented list above describes current source. It does not make the repository release-ready
by itself. Version 0.1 readiness is tracked in this order; native probes are rerun when their
covered subsystem changes, not mechanically after unrelated documentation, packaging, example, or
component work:

| Priority | Work | Current state | Remaining exit criteria |
| --- | --- | --- | --- |
| P0.1 | Desktop mouse dispatch verification | **Complete in the current working tree.** Focused tests cover bounded ordering, stop/prevent independence, payloads, overlay blocking, callback invalidation, disabled nodes, hover, and idle sleep. The passing live WindowServer gate posts native AppKit events and verifies left/right down/up, outside capture, root-to-target capture, target-to-root bubble, independent stop/prevent behavior, exact buffered AppKit click counts, pressed-button drag motion, hover entry/exit, targeted window exit, and zero prevented default clicks. | No implementation or repeat run remains unless mouse dispatch changes. |
| P0.2 | Window-constraint verification | **Complete in the current working tree.** Initial/runtime minimum size is validated, observable through `WindowState`, mutable through current/target-window set and clear commands, and applied without a correction loop. The passing live gate starts below a larger runtime minimum, verifies one-time growth and exact `NSWindow.contentMinSize`, clears both retained/native state, then resizes below the former minimum while wrapped text and native composition remain stable. Explicit programmatic geometry remains application-authored. | No implementation or repeat run remains unless window constraints, resizing, text layout, or native composition changes. |
| P0.3 | macOS release-candidate acceptance | **Complete for the 2026-08-27 source candidate.** The current tree passes the full 561-test all-target suite, downstream test-support check, every example and benchmark target, current-stable and Rust 1.89 all-target checks, warning-free all-target Clippy, formatting/script/diff checks, rustdoc, and the five-archive Rust 1.89 downstream-consumer gate. The latest focused current-tree scroll run reached 95.0496 Hz at 9.9708% framework CPU and 14.650% whole-process CPU, with 1.3496 ms p95 frame CPU, 106.906 MiB peak RSS, 154.204 MiB peak footprint, bounded caches/draws, and zero extra idle frames. The current-tree schema-3 display gate passed first-render centering plus settled retained/native identity, 2x scale, native-frame origin, screen/work-area agreement, full native-frame containment, and no focus steal on both connected displays. The last complete full AppKit composition/mouse/popover/lifecycle soak passed before the display-startup correction; that correction is covered by the newer focused two-display gate and did not change those unrelated paths, so the full soak is not repeated. Both connected displays are 2x, so mixed-scale status is explicitly unavailable rather than passed. | No further local gate is required for 0.1 unless covered runtime code changes. Manual VoiceOver component wording, mixed-scale hardware, and broader product workflows continue under the 0.2 acceptance ledger; native system tabs remain out of scope. |
| P0.4 | Registry delivery | **Complete.** On 2026-08-27, `quickgui-winit = 0.30.13-quickgui.1`, `quickgui-accesskit-winit = 0.33.2-quickgui.1`, `quickgui-cosmic-text = 0.19.0-quickgui.1`, `quickgui-glyphon = 0.12.0-quickgui.1`, and `quickgui = 0.1.0` were published in dependency order and confirmed indexed. A fresh Rust 1.89 crate using only `quickgui = "=0.1.0"` from crates.io compiled successfully without path or patch dependencies. | No 0.1 registry work remains. GitHub commit, tag, workflow artifact, and source-release delivery are tracked separately from the completed crates.io publication. |

P0.1 through P0.4 are complete and QuickGUI 0.1.0 is an official crates.io release. Any subsequent
registry correction requires a new version rather than overwriting 0.1.0.

### 0.2 execution roadmap: macOS parity and developer experience

Version 0.2 starts only after the 0.1 evidence and delivery gates. Work is ordered by reusable
editor workflow and acceptance risk, not by novelty:

| Priority | Outcome | Current state | Exit criteria |
| --- | --- | --- | --- |
| P1.1 | Unstyled component composition contract | **Source-complete in the current working tree.** Caller-owned selection-control, tabs, popover, select, autocomplete, combobox, menu, context-menu, dialog, alert-dialog, field, fieldset, collapsible, accordion, picker, virtual-table, and virtual-tree parts separate behavior from presentation. The old selection-control presets, framework-painted marks, `PopoverStyle`, `PickerStyle`, `TableStyle`, and `TreeStyle` are removed. `PickerLayout`, `TableLayout`, and `TreeLayout` retain only finite virtualization geometry; callers supply input, empty-state, header, cell, row, and disclosure elements plus root, highlight, and interaction-state presentation. | Live-accept the complete parts contract. Add a flip-aware arrow or multi-trigger animated viewport only with exact resolved-placement behavior, never a preferred-side approximation. |
| P1.2 | Unstyled popover menu and context menu | **Popover-menu and context-menu foundations are implemented in the current working tree.** `PopoverMenu` is separate from native `Menu` and supplies bounded action/checkbox/radio/submenu items, exact non-interactive separator roles, mounted visible-label relationships for caller-composed groups, caller-rendered roots/rows, active-descendant keyboard navigation, pointer highlighting, bounded timer-free typeahead, derived submenu anchors, cross-popover typed actions to the non-popover owner, whole-chain command close, and an owner-controlled hover hook. Optional accessibility relationships now cost one nullable pointer on ordinary elements and one fixed allocation only on relationship-bearing controls. `ContextMenuState` adds caller-owned trigger/root/row parts, exact secondary-click point placement on the cross-edge WGPU child host, replacement/native-close synchronization, 150 ms hover intent, a 300 ms actual-child-geometry pointer corridor that works for right and left placement, root-owned grab/monitor semantics across attached native descendants, chain-level AppKit focus dismissal, and nested child ownership without adding closed-state work. Styled examples exercise both anchored-button and cursor-point contracts. The self-driving live gate proves eight repeated native root/submenu command cycles, four owner-press dismissals, four Escape dismissals, 128 paced full native root/submenu resource lifecycles with 1.109 MiB positive RSS growth and zero positive footprint growth after the cycle-32 baseline, and one cooperative application-deactivation dismissal with the submenu open. It requires no accidental command delivery, correct active/inactive owner focus policy, complete child teardown, at most two popover windows, bounded draw/cache state, and zero extra idle frames. | Record live VoiceOver, human-visible popover appearance, multi-monitor/mixed-scale placement, and nested submenu placement. |
| P1.3 | Standalone select, autocomplete, and combobox | **All three distinct unstyled foundations are implemented in the current working tree.** `SelectState` validates up to 65,536 options, preserves stable selection across atomic replacement, accepts caller-owned trigger/popover/row parts, opens an overflow-capable child, synchronizes every close path, supports preview/commit/cancel, disabled options, bounded timer-free typeahead, validation, active-descendant semantics, and visible-only mounting. `AutocompleteState` accepts caller-owned input/popover/row parts, retains arbitrary bounded text independently of suggestions, supports completion and dismiss-only commits, local fuzzy or externally filtered sources, in-place open source replacement, typed cross-window hover/commit, visible-only 20,000-result behavior, exact close synchronization, and zero added idle scheduling. `ComboboxState` reuses that native host while retaining a declared committed value independently from the bounded edit query, restores its label on Escape/Tab/outside/native closure, rejects unmatched or disabled commits, preserves stable selections across externally filtered absence, and exposes selected state independently from active preview. Both editable panels are permanently never-key, so owner text/IME focus remains authoritative; their visual children are accessibility-hidden while bounded ListBox/option proxies remain in the owner AccessKit tree. The old styled/select-only compatibility implementation is no longer exported or retained. | Live-accept Select, Autocomplete, and Combobox pointer/keyboard/scroll, IME composition, clipboard/undo, VoiceOver proxy wording/order and selected-versus-active state, edge and mixed-scale placement, owner focus loss, async result replacement, repeated-open CPU/memory, and idle behavior. Never serialize arbitrary `T` into forms implicitly. |
| P1.4 | Dialog and alert-dialog composition | **Source-complete in the current working tree.** `Dialog` and `Dialog::alert` provide caller-owned portal/backdrop/popover/title/description/close parts, stable derived IDs, nested render-ordered focus traps, wrapping Tab and programmatic-focus containment, configurable independent Escape/backdrop dismissal, initial focus and restoration, hidden-inset no-drag regions, overlay-plane native-view occlusion, distinct modal AccessKit roles, mounted label/description relationships, deterministic interaction/accessibility tests, and a styled nested gallery. Closed descriptors and settled open dialogs add no timer, observer, task, or idle source. | Record live pointer-backdrop, Escape, nested focus, VoiceOver wording/order, embedded-native-view occlusion, hidden-inset dragging, and repeated-open CPU/memory acceptance on the release candidate. |
| P1.5 | Remaining common component families | `Field`/`Fieldset`, `Collapsible`/`Accordion`, and in-window `Tabs` are source-complete in the current working tree. Tabs add controlled manual/automatic activation, horizontal/vertical roving focus, looping/non-looping edges, disabled skipping, exact panel relationships, caller-owned indicators, mounting policy, deterministic interaction/accessibility tests, and zero idle source. Disclosure parts retain their controlled single/multiple open state, current APG Tab/Enter/Space behavior, exact heading/region relationships, default unmounting or retained closed panels, and 4,096-open-value hard bound. The [component ledger](component-roadmap.md) records every remaining Base UI reference component as behavior-present, primitive-only, platform-only, or missing. | Live-accept Field/Fieldset, Collapsible/Accordion, and Tabs VoiceOver wording/order, pointer/keyboard behavior, group/disabled semantics, retained content, and application-styled states. Range and feedback components (`Slider`, `Splitter`, `NumberField`, `Progress`, `Meter`, `Toolbar`, `Toggle`/`ToggleGroup`, and the toast queue) are source-complete with deterministic interaction/accessibility tests, examples, and documented bounds. Live-accept their VoiceOver wording, pointer capture, and stepper repeat next; an in-window menubar, `DateField`/`TimeField`, and the table/tree collection upgrades remain unimplemented. |
| P1.6 | Product-driven collection workflows | Unstyled sortable virtual tables and expandable virtual trees are implemented, with bounded multi-selection, column resizing, keyboard column reordering, an inline-edit hook, and lazy tree children. Pointer-drag column reordering is intentionally absent because it needs header geometry the framework does not retain. | First live-accept VoiceOver and large application data. Then implement only workflows backed by an editor/product scenario, with stable ownership, bounded state, visible-only work, deterministic tests, and a live resource gate. |
| P1.7 | Inspector developer experience | The feature-gated read-only inspector is implemented with deterministic snapshots and zero disabled-build branches. | Pass the live overlay/native-child smoke, then evaluate source metadata and explicit live style editing without placing debug storage in normal elements or release builds. |
| P1.8 | Broader assistive-technology workflows | Deterministic AccessKit projection covers text, forms, controls, tabs, popovers, tables, trees, select, autocomplete proxies, and comboboxes; the complete live workflow matrix is not recorded. | Record VoiceOver scenarios for tabs, menus, popover preview/commit/cancel, dialogs, collection navigation/editing, drag/drop announcements, validation, document windows, and native-child focus while retaining zero polling. |

Every new 0.2 component must identify the application-owned value/source, retained framework state,
hard CPU/memory bounds, mounted work, invalidation events, accessibility model, deterministic tests,
live macOS acceptance, and sleeping behavior. A fluent API or gallery alone is not completion.

Known macOS gaps are therefore explicit: optional flip-aware popover arrow/multi-trigger viewport
parts,
pending tabs/popover-menu/select/autocomplete/combobox/dialog/field/disclosure live acceptance, arbitrary custom
form-field serialization, pointer-drag column reordering, and complete live VoiceOver
coverage. They are tracked here instead of being hidden behind a broad “GPUI parity” claim.

### After 0.1: other desktop platforms

1. Run native runtime, visual, IME, accessibility, clipboard, and performance acceptance on Windows
   and Linux rather than treating compilation as runtime proof.
2. Project application menus and platform window roles through native Windows/Linux facilities.
3. Promote typed drags across QuickGUI windows and native applications on those platforms with the
   same bounded payload and teardown rules as macOS.
4. Publish a portable benchmark matrix with equivalent scroll, resize, text, idle, and memory
   budgets.

### Explicit 0.1 non-blockers

- Keychain/credential storage belongs in an application or focused platform-service crate unless a
  concrete framework ownership requirement appears.
- `text_center` and inherited left/center/right/justified text alignment are already implemented
  across ordinary and styled text; text alignment is part of the 0.1 surface, not deferred work.
- Stylus/tablet axes are not a GPUI-parity requirement and are not on the macOS 0.1 roadmap.
  QuickGUI already supports the relevant desktop gestures and Force Touch; specialized drawing
  hardware can be considered only when a real application requires it.
- Full Windows/Linux behavioral parity, mobile/touch-first widgets, and a bundled high-level design
  system do not block the macOS-first release.
- Native system-tab grouping, navigation, and detaching are optional and deferred; the existing
  bounded API may remain experimental without blocking 0.1.

See [Releasing QuickGUI 0.1](releasing.md) for the exact commands, package order, and immutable
publication procedure.

## Why this renderer

Most desktop UI pixels are rectangles, glyphs, icons, images, and modest vector paths. Dedicated data-driven pipelines minimize CPU preparation and draw calls for that workload. QuickGUI uses retained Lyon tessellation plus a narrow WGPU path pipeline instead of making a general compute vector renderer part of every frame; the bounded application-shader path stays opt-in for specialized effects.

The design is informed by the same proven ideas documented by [Zed's GPU UI architecture](https://zed.dev/blog/videogame), while changing the scheduling and ownership choices that matter for low CPU: no permanent redraw loop, frame-boundary input coalescing, retained layout between view changes, and bounded text/list caches. Sublime Text 4 likewise documents GPU compositing as the route to fluid high-DPI UI with lower power use in its [GPU rendering notes](https://www.sublimetext.com/docs/gpu_rendering.html).

See the [architecture index](architecture/README.md) for the ownership and renderer model.
