# macOS integration

[Documentation index](README.md)

## macOS platform services

Event callbacks can start window-owned AppKit prompts and file panels, then await the result on the
existing main-thread foreground executor:

```rust
let response = cx.prompt(
    PromptLevel::Warning,
    "Save changes?",
    Some("Unsaved edits will be lost."),
    &[
        PromptButton::ok("Save"),
        PromptButton::new("Don't Save"),
        PromptButton::cancel("Cancel"),
    ],
).expect("valid native prompt");

cx.spawn(|async_cx: AsyncViewContext<MyView>| async move {
    let answer = response.await.ok();
    async_cx.update(move |view, cx| {
        view.answer = answer;
        cx.invalidate();
    }).await
}).expect("foreground capacity").detach();
```

System notifications are application-wide and use stable tags for replacement and dismissal:

```rust
let notification = SystemNotification::new(
    "background-build",
    "Build finished",
    "All checks passed.",
)
.subtitle("QuickGUI")
.action(SystemNotificationAction::new("open", "Open"))
.action(SystemNotificationAction::new("reply", "Reply").text_input("Message"))
.sound(quickgui::SystemNotificationSound::Default);

cx.show_system_notification(notification)?;
// cx.dismiss_system_notification("background-build")?;
```

Native application events are configured once on `Application`, outside any particular window:

```rust
Application::new()
    .on_open_urls(|urls, cx| update_workspace(urls, cx))
    .on_reopen(|had_visible_windows, cx| reopen(had_visible_windows, cx))
    .on_system_wake(|cx| reconnect(cx))
    .on_system_notification_response(|response, cx| activate(response, cx))
    .on_window_closed(|window, cx| record_closed_window(window, cx))
    .run(|cx| {
        cx.open_window(WindowOptions::default(), view);
    })?;
```

Those callbacks receive an application-wide `EventContext` with no current window. They can
update globals or entities and open new root windows; window-owned prompts and foreground tasks
still require a window callback. Native observers are installed only for callbacks the
application registered.

The default `QuitMode` keeps a macOS application resident after its final window closes, matching
GPUI and standard Dock behavior. An `on_reopen` callback can create a new root window from its
windowless application context. Choose `QuitMode::LastWindowClosed` for utilities and examples
that should terminate as soon as their final window closes.

`prompt_for_paths(PathPromptOptions)` and `prompt_for_new_path(SavePathOptions)` return the same
single-use future shape; cancellation is `Ok(None)`. `open_url`, `open_path`, and `reveal_path`
project through `NSWorkspace`. A dropped response cancels a sheet that has already started, window
teardown cancels both the response task and native panel, related parent/child windows cannot stack
conflicting sheets, and unrelated windows remain independent. Requests, active dialogs, selected
path counts, and returned path bytes all have public hard limits. Unbound Cmd-W now goes through
AppKit's ordinary close path and still delivers `Event::CloseRequested`.

On macOS, notifications require a real `.app` bundle with a `CFBundleIdentifier`; an unbundled
`cargo run` process cannot use `UNUserNotificationCenter`. The model supports subtitles, buttons,
inline replies, sound, icons/attachments, and scheduled delivery. Permission status and explicit
permission requests are separate single-use futures. The first post can also make one contextual
authorization request, waits for its result, and coalesces replacement tags in a 64-entry bounded
queue. Dismissal also removes a matching authorization-pending post. Action category retention is
capped at 64 distinct action sets; later notifications still post without actions instead of
growing native state. Open callbacks retain at most 256 URLs and 1 MiB total. None of these
services installs a polling timer or idle frame. See
`cargo run --release --example platform_services`.

Dock badges, icon replacement, and a typed native Dock menu are application-wide Rust-core
services. Recent documents, standard About panels, and native file-icon lookup use AppKit/Workspace
services through the same bounded platform queue. See
[Desktop integrations](desktop-integrations.md) for the shared API and target matrix.

### Message boxes, Quick Look, and system panels

`cx.message_box(MessageBoxOptions)` presents an `NSAlert` that also carries a suppression checkbox
(`setShowsSuppressionButton:`), a custom icon (`setIcon:`), separate message and informative text,
and explicit default/cancel key equivalents. AppKit gives the first added button the Return
equivalent, so QuickGUI reassigns every button's key equivalent from the declared
`default_button`/`cancel_button` indices. The response reports the chosen index and the final
checkbox state together.

`cx.preview_file(path, display_name)` and `cx.close_file_preview()` drive `QLPreviewPanel` from the
Quartz framework. QuickGUI owns the `QLPreviewItem` and `QLPreviewPanelDataSource` objects: the
already-validated path and bounded display name are copied into Foundation objects up front, and the
data source is retained for exactly as long as the panel is open. Closing the preview clears the
panel's data source and releases the core-owned object.

`cx.show_color_panel(color, mode)` and `cx.show_font_panel(font)` drive `NSColorPanel` and
`NSFontManager` through one core-owned responder. `ColorPanelMode::Continuous` installs a
target/action pair with `setContinuous:YES`; `ColorPanelMode::OnClose` installs no action and
observes `NSWindowWillCloseNotification` on the panel instead, so a panel that is merely open costs
nothing. The font panel routes AppKit's `changeFont:` through the same responder, converting the
retained seed font with `convertFont:` and reporting the resulting family, weight, and slant.
Colors cross the boundary through the sRGB color space.

`cx.share_items(items, anchor)` builds an `NSSharingServicePicker` from `NSString`, `NSURL`, and
`NSImage` values and shows it relative to a rectangle inside the current window's content view. The
anchor is supplied in QuickGUI's top-left logical content coordinates and flipped into AppKit's
bottom-left view coordinates.

`cx.authenticate_with_biometrics(reason)` uses `LAContext` with
`LAPolicyDeviceOwnerAuthenticationWithBiometrics`. `canEvaluatePolicy:` is checked first, so a
machine without usable biometrics reports `PlatformError::Unsupported` before any system prompt.
The evaluation reply arrives on a background queue, so QuickGUI forwards it through the event loop
and completes the `PlatformResponse<bool>` on the AppKit main thread.

### Native images

`Image::named_system(name)` resolves an `NSImage` name first and then an SF Symbol through
`imageWithSystemSymbolName:accessibilityDescription:`, applying an `NSImageSymbolConfiguration`
point size so the rasterization is large enough, and rasterizes through the same bounded decoder
every other QuickGUI image uses. `Image::template(true)` reaches `NSImage setTemplate:`, and
`Image::with_representations` adds each variant's representations to the resulting `NSImage`, so
Dock, About-panel, message-box, and menu icons all pick the right backing scale. Tray artwork
carries the same flag through `TrayIconImage::template(bool)`, and `AppRunner::tray_icon_bounds(id)`
reports a status item's screen rectangle in global logical desktop coordinates.

## Clipboard and Find pasteboard

Every event callback can synchronously read or replace the general pasteboard with a bounded
`ClipboardItem`. The direct AppKit backend supports UTF-8 text with hash-bound application metadata,
encoded image formats without eager decoding, and native filename property lists with a text
fallback. Native byte lengths are rejected before Rust copies them.

`read_from_find_pasteboard` and `write_to_find_pasteboard` use macOS's shared search pasteboard and
the same item model. Both pasteboard handles are lazy, and QuickGUI installs no change-count observer
or polling timer. See [Clipboard](clipboard.md) and run:

```console
cargo run --release --example clipboard
```

## Native appearance

macOS windows follow AppKit's effective Aqua or Dark Aqua appearance by default. QuickGUI maps
those platform names to `WindowAppearance::Light` and `WindowAppearance::Dark`, applies an explicit
preference through the native `NSWindow` when requested, and routes AppKit effective-appearance
changes through Winit's existing event source. Application content opts into palette updates with
`cx.appearance()`; native titlebar controls update as part of the same window appearance.

There is no distributed-notification observer, preference query timer, or animation loop. A system
change invalidates only views that observed native window state, while `Event::AppearanceChanged`
remains available for stateful application reactions. Run the example with:

```console
cargo run --release --example appearance
```

## Window backgrounds

`WindowBackgroundAppearance` controls composition independently from the window's light/dark
palette. `Opaque` uses Metal's opaque fast path. `Transparent` keeps scene alpha through the
`CAMetalLayer` and clears the `NSWindow` background. `Blurred` uses that same alpha-capable path
and enables the native window-server blur behind it; QuickGUI does not blur application pixels or
render a fullscreen blur pass.

Runtime switches retain the existing WGPU surface, pipelines, caches, and drawable chain. QuickGUI
changes `CAMetalLayer.opaque` directly on macOS and invokes AppKit opacity or blur operations only
when that exact component changes. This avoids both surface allocation and redundant native
transparency calls that can strand a previously presented Metal drawable. The transition requests
one damage-driven frame; a stationary transparent or blurred window then returns to
`ControlFlow::Wait`.

Use translucent scene colors to expose the effect and run the native sample with:

```console
cargo run --release --example window_background
```

## Vibrancy materials

`MacOsVibrancy` is the platform-specific material layer above the portable background policy. It
supports the complete Electron-compatible set: `AppearanceBased`, `Titlebar`, `Selection`, `Menu`,
`Popover`, `Sidebar`, `Header`, `Sheet`, `Window`, `Hud`, `FullscreenUi`, `Tooltip`, `Content`,
`UnderWindow`, and `UnderPage`. `MacOsVisualEffectState` selects `FollowWindow`, `Active`, or
`Inactive` activity behavior.

```rust
use quickgui::{MacOsVibrancy, MacOsVisualEffectState, WindowOptions};

let options = WindowOptions::new("Workspace")
    .background(quickgui::Color::TRANSPARENT)
    .macos_vibrancy(MacOsVibrancy::Sidebar)
    .macos_visual_effect_state(MacOsVisualEffectState::FollowWindow);
```

The JavaScript API follows Electron's names directly:

```ts
const window = new Window({
  background: "transparent",
  vibrancy: "sidebar",
  visualEffectState: "followWindow",
  renderer,
});

window.setVibrancy("under-window");
window.setVisualEffectState("active");
window.setVibrancy(); // remove the material
```

Enabling vibrancy makes the existing Metal surface alpha-capable and places its stable Winit view
above one `NSVisualEffectView` sibling using behind-window blending. Changing the material or
activity state mutates that view in place; disabling vibrancy restores the original content view
and the portable `WindowBackgroundAppearance` policy. These transitions add no polling or idle
frame source. Scene pixels still need alpha—normally a transparent window background and a
translucent sidebar—to reveal the material.

A translucent surface also changes how the frame is presented. The renderer blends in linear
light, while CoreAnimation composites the layer's premultiplied pixels in the encoded sRGB space,
which would leave a light fringe on every anti-aliased edge over the material. A window with an
alpha-capable surface therefore renders into an intermediate texture and a final full-screen pass
re-encodes each pixel for the compositor. Opaque windows keep presenting directly.

The interactive QuickGUI UI example exposes every material and effect state:

```console
cd examples/sidebar-vibrancy
bun run dev
```

## macOS window chrome

Represented file URLs, native edited-state indication, the character palette, and AppKit system
tabs are documented separately in [Native document windows](document-windows.md). That layer
reuses the same retained window command path, reapplies custom traffic-light placement after
AppKit titlebar mutations, and adds no polling or idle redraw source.

`HiddenInset` extends GPU content through a transparent titlebar while retaining native traffic
lights. Its implicit AppKit drag area is disabled; layout boxes explicitly opt into web-style
`drag` and `no-drag` regions:

```rust
Application::new().run(|cx| {
    cx.open_window(
        WindowOptions::default()
            .title_bar_style(TitleBarStyle::HiddenInset)
            .traffic_light_position(16.0, 13.0),
        view,
    );
})?;

div()
    .h(64.0)
    .app_region_drag()
    .child(button().app_region_no_drag().child("Refresh"))
```

The topmost declared app region wins, so interactive descendants use `.app_region_no_drag()`.
Drag regions do not receive element clicks or captured pointer gestures. Built-in overlay
scrollbars take input precedence over an ancestor drag region.

On macOS, any retained AppKit `NSView` subclass can participate in the same layout:

```rust
let field: Retained<NSTextField> = /* construct on the main thread */;

native_view(field.as_ref())
    .id("native-field")
    .w_full()
    .h(44.0)
    .rounded_lg()
```

QuickGUI switches that window to three-plane composition: WGPU base content, native child views,
then a transparent WGPU overlay. The second full-window swapchain is created only when the first
overlay opens, then retained for low-latency reuse. Native views are retained by Objective-C
identity, clipped and resized in logical points, reordered by `z_index`, and removed when their
elements disappear. AppKit and QuickGUI first-responder focus are synchronized without an idle
polling loop. A background-colored launch shield stays above every plane until WGPU successfully
presents and completes the first frame, so an AppKit child cannot flash over an otherwise empty
window. Layout, text shaping, native reconciliation, and buffer uploads run while the window is
still hidden. QuickGUI briefly detaches the retained content view from that hidden `NSWindow`, which
allows WGPU to present the actual Metal layer without ordering any window onscreen. It reattaches
the same view at the unchanged frame only after GPU completion, preventing both black intermediate
frames and cross-monitor movement in unoptimized builds.

The native Liquid Glass example embeds an AppKit `NSButton` with the macOS 26
`NSBezelStyleGlass` bezel and falls back to a standard native push button on older systems:

```console
cargo run --release --example liquid_glass_button
```

With the `swift-ui` feature enabled, the Rust core exposes descriptors for native buttons, sliders,
toggles, progress views, steppers, text fields, pickers, date pickers, color pickers, and gauges.
`SwiftUiSegmentedControl` and `SwiftUiSegmentedTabs` are discoverable aliases for `SwiftUiPicker`.
`SwiftUiPicker::segmented` selects the value-selection treatment, while `SwiftUiPicker::tabs`
selects the neutral segmented-tabs role used for Xcode-style navigation on macOS 27.
`MacSwiftUiHost::new_with_control_events` reports button presses, controlled value changes,
text-field submissions, and popover presentation without blocking the AppKit main thread.

QuickGUI UI applications adapt those same descriptors into typed reactive components:

```tsx
import { Host, Slider } from "@quickgui/ui/swift-ui";

<Host matchContents>
  <Slider
    label="Volume"
    value={volume()}
    min={0}
    max={1}
    step={0.05}
    onValueChange={setVolume}
  />
</Host>;
```

The QuickGUI UI subpath also exports controlled `Toggle`, `ProgressView`, `Stepper`, `TextField`,
`SecureField`, `Picker`, `SegmentedControl`, `DatePicker`, and `ColorPicker` components, plus
`Gauge`, alongside the existing `Button`, `Popover`, and `QuickGUIHostView`.

`matchContents` sizes the host to the controls' own fitting size, so a hosted control lines up
with QuickGUI elements at native density. The host keeps a few points of headroom around its
content for bezels and focus rings, and more around content that draws well past its bounds, a
`glass` or `glassProminent` button on macOS 26; that headroom is an outset of the AppKit frame
rather than part of the layout box, so it neither moves siblings nor clips the effect, and clicks
in the ring that hit no control fall through to the framework. Keyboard focus is shared: when a
hosted control becomes the window's first responder, the framework blurs its own focused element
and announces the change, and the next framework focus change makes the Winit view first responder
again, so exactly one control shows focus at a time. Rust
callers get the same behaviour from `native_view_with_outset` paired with
`MacSwiftUiHost::fitting_size` and `MacSwiftUiHost::effect_inset`.

`QuickGUIHostView` provides the reverse direction, analogous to Expo UI's `RNHostView`. It creates
one child retained renderer in the Rust core, reparents that renderer's stable AppKit/WGPU view
into SwiftUI, and keeps QuickGUI pointer, wheel, keyboard, IME, terminal, focus, and accessibility
paths intact. The view can use a fixed `width`/`height` or max-content measurement on either axis.
Its hidden backing `NSWindow` is never presented and remains only as the stable Winit event
identity; closing the owner tears down every embedded child.

The SwiftUI `Popover` API uses controlled `isPresented` state plus compound `Trigger` and `Content`
parts. `Popover.Trigger` follows Base UI-style composition through
`render={<Button label="Open" />}`: the rendered SwiftUI component keeps its own props and handler,
then the trigger requests presentation unless that handler prevents the default action. Because
AppKit owns the popover window, it is natively above the owner QuickGUI scene; a `QuickGUIHostView`
inside `Popover.Content` makes ordinary QuickGUI components interactive there.

Run `cd examples/swift-ui && bun run dev` for a sidebar gallery with one live page per exposed
native SwiftUI control. Its Popover page opens a native popover containing QuickGUI text, input,
and button components. The Go frontend mirrors that API as `ui.SwiftUI`;
`cd examples/swift-ui-go && bun run dev` is the same gallery.

See also `cargo run --release --example native_view` and
`cargo run --release --example overlays`.
\n
## Window and application shell

QuickGUI keeps Winit's AppKit delegate intact and never swizzles it. The window lifecycle events in
[Windows and shared state](windows.md) are therefore derived from the native events Winit does
deliver — resize, move, occlusion — rather than from `windowDidMiniaturize:`,
`windowDidEnterFullScreen:`, or `windowWillResize:toSize:` directly. The derivation reads
`NSWindow.isMiniaturized`, the AppKit fullscreen mask, and the zoom geometry only at those events;
an idle window performs no sampling.

The macOS-specific window operations are:

- `orderFront:` and `orderWindow:relativeTo:` for `move_window_top` / `move_window_above`, which
  restack without activating the application or making the window key.
- `ignoresMouseEvents` plus `acceptsMouseMovedEvents` for
  `set_ignore_mouse_events(ignore, forward)`. Forwarding uses AppKit's existing event-driven
  `mouseMoved:` stream; QuickGUI installs no tracking area, timer, or polling pass.
- `setLevel:` with the exact `NSWindowLevel` named by `WindowLevel`, applied after Winit's own
  three-level hint.
- `contentAspectRatio` for `set_aspect_ratio`, and `contentResizeIncrements` reset to `1 x 1` when
  the ratio is cleared.
- `standardWindowButton(_:)` hiding for `set_window_button_visibility`, which keeps the titlebar.
- `setHasShadow:` for runtime shadow mutation.

Application-shell services use `NSApplication` (`setActivationPolicy:`, `activate`,
`activateIgnoringOtherApps:`, `hide:`, `unhide:`, `requestUserAttention:`,
`cancelUserAttentionRequest:`), Carbon's `EnableSecureEventInput`/`DisableSecureEventInput`, and
AppKit's `NSBeep`. `on_did_become_active` and `on_did_resign_active` reuse the
`QuickGuiApplicationObserver` that already watches
`NSApplicationDidBecomeActiveNotification`/`NSApplicationDidResignActiveNotification` for work-area
refresh and popover dismissal, so no additional observer is registered.

Everything above except the pure queries requires the AppKit main thread and fails with
`PlatformError::Unavailable` off it.
