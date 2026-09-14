use super::*;

#[cfg(any(target_os = "macos", target_os = "windows"))]
use std::sync::Mutex;
use std::{collections::HashSet, path::Path};

/// Maximum tray icons retained by one QuickGUI application.
pub const MAX_TRAY_ICONS: usize = 32;
/// Maximum action items retained by one tray menu.
pub const MAX_TRAY_MENU_ITEMS: usize = 256;
/// Maximum nested tray-menu depth.
pub const MAX_TRAY_MENU_DEPTH: usize = 8;
/// Maximum UTF-8 bytes accepted for tray titles, tooltips, and menu labels.
pub const MAX_TRAY_TEXT_BYTES: usize = 4_096;
/// Maximum width or height accepted for an in-memory tray icon.
pub const MAX_TRAY_ICON_DIMENSION: u32 = 1_024;
/// Maximum encoded image bytes decoded by [`TrayIconImage::from_encoded`].
pub const MAX_TRAY_ENCODED_ICON_BYTES: usize = 16 * 1024 * 1024;

/// A validated RGBA8 image used by a native tray icon.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrayIconImage {
    rgba: Arc<[u8]>,
    width: u32,
    height: u32,
    template: bool,
}

impl TrayIconImage {
    pub fn from_rgba(
        rgba: impl Into<Arc<[u8]>>,
        width: u32,
        height: u32,
    ) -> Result<Self, PlatformError> {
        let rgba = rgba.into();
        let expected = width
            .checked_mul(height)
            .and_then(|pixels| pixels.checked_mul(4))
            .and_then(|bytes| usize::try_from(bytes).ok());
        if width == 0
            || height == 0
            || width > MAX_TRAY_ICON_DIMENSION
            || height > MAX_TRAY_ICON_DIMENSION
            || expected != Some(rgba.len())
        {
            return Err(tray_error(
                "tray icon RGBA data must exactly match nonzero dimensions no larger than 1024x1024",
            ));
        }
        Ok(Self {
            rgba,
            width,
            height,
            template: false,
        })
    }

    /// Decode PNG, JPEG, GIF, WebP, or another image format enabled by QuickGUI.
    pub fn from_encoded(bytes: &[u8]) -> Result<Self, PlatformError> {
        if bytes.is_empty() || bytes.len() > MAX_TRAY_ENCODED_ICON_BYTES {
            return Err(tray_error(
                "encoded tray icons must be nonempty and no larger than 16 MiB",
            ));
        }
        let image = image_codecs::load_from_memory(bytes)
            .map_err(|error| tray_error(format!("could not decode tray icon: {error}")))?
            .into_rgba8();
        Self::from_rgba(
            Arc::<[u8]>::from(image.as_raw().as_slice()),
            image.width(),
            image.height(),
        )
    }

    /// Read and decode an image file before queuing main-thread tray work.
    ///
    /// Paths whose stem ends in `Template` (optionally `@2x`) are marked for macOS template
    /// rendering. Call [`Self::template`] to override that convention.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, PlatformError> {
        let path = path.as_ref();
        let bytes = std::fs::read(path).map_err(|error| {
            tray_error(format!(
                "could not read tray icon {}: {error}",
                path.display()
            ))
        })?;
        Ok(Self::from_encoded(&bytes)?.template(crate::is_template_image_path(path)))
    }

    /// Mark the artwork as a macOS template image.
    ///
    /// AppKit then recolors the icon for the menu bar's light, dark, and highlighted appearances
    /// using only the alpha channel. Other platforms retain the flag and ignore it.
    pub const fn template(mut self, template: bool) -> Self {
        self.template = template;
        self
    }

    /// Whether this artwork is marked for macOS template rendering.
    pub const fn is_template(&self) -> bool {
        self.template
    }

    pub fn rgba(&self) -> &[u8] {
        &self.rgba
    }

    pub const fn width(&self) -> u32 {
        self.width
    }

    pub const fn height(&self) -> u32 {
        self.height
    }
}

/// One item in a tray icon's native context menu.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TrayMenuItem {
    Action {
        id: u32,
        label: Arc<str>,
        enabled: bool,
        checked: bool,
    },
    Separator,
    Submenu {
        label: Arc<str>,
        enabled: bool,
        items: Vec<TrayMenuItem>,
    },
}

impl TrayMenuItem {
    pub fn action(id: u32, label: impl Into<Arc<str>>) -> Self {
        Self::Action {
            id,
            label: label.into(),
            enabled: true,
            checked: false,
        }
    }

    pub const fn separator() -> Self {
        Self::Separator
    }

    pub fn submenu(
        label: impl Into<Arc<str>>,
        items: impl IntoIterator<Item = TrayMenuItem>,
    ) -> Self {
        Self::Submenu {
            label: label.into(),
            enabled: true,
            items: items.into_iter().collect(),
        }
    }

    pub fn enabled(mut self, value: bool) -> Self {
        match &mut self {
            Self::Action { enabled, .. } | Self::Submenu { enabled, .. } => *enabled = value,
            Self::Separator => {}
        }
        self
    }

    pub fn checked(mut self, value: bool) -> Self {
        if let Self::Action { checked, .. } = &mut self {
            *checked = value;
        }
        self
    }
}

/// Complete state for one native tray icon. Sending it again atomically replaces prior state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrayIconOptions {
    pub id: u32,
    pub icon: TrayIconImage,
    pub tooltip: Option<Arc<str>>,
    pub title: Option<Arc<str>>,
    pub icon_is_template: bool,
    pub menu_on_left_click: bool,
    pub visible: bool,
    pub menu: Vec<TrayMenuItem>,
}

impl TrayIconOptions {
    pub fn new(id: u32, icon: TrayIconImage) -> Self {
        Self {
            id,
            icon,
            tooltip: None,
            title: None,
            icon_is_template: false,
            menu_on_left_click: true,
            visible: true,
            menu: Vec::new(),
        }
    }

    pub fn tooltip(mut self, tooltip: impl Into<Arc<str>>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    pub fn title(mut self, title: impl Into<Arc<str>>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn menu(mut self, menu: impl IntoIterator<Item = TrayMenuItem>) -> Self {
        self.menu = menu.into_iter().collect();
        self
    }

    /// Treat the artwork as a macOS template image.
    pub fn icon_is_template(mut self, template: bool) -> Self {
        self.icon_is_template = template;
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TrayMouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TrayEventKind {
    Click,
    DoubleClick,
    Enter,
    Move,
    Leave,
    MenuItem,
    Scroll,
}

/// An interaction delivered by a native tray host.
#[derive(Clone, Debug, PartialEq)]
pub struct TrayEvent {
    pub tray_id: u32,
    pub kind: TrayEventKind,
    pub menu_item_id: Option<u32>,
    pub button: Option<TrayMouseButton>,
    pub position: Option<(f64, f64)>,
    pub pressed: Option<bool>,
    pub scroll_delta: Option<i32>,
    pub horizontal: Option<bool>,
}

impl TrayEvent {
    fn simple(tray_id: u32, kind: TrayEventKind) -> Self {
        Self {
            tray_id,
            kind,
            menu_item_id: None,
            button: None,
            position: None,
            pressed: None,
            scroll_delta: None,
            horizontal: None,
        }
    }

    fn menu_item(tray_id: u32, menu_item_id: u32) -> Self {
        Self {
            menu_item_id: Some(menu_item_id),
            ..Self::simple(tray_id, TrayEventKind::MenuItem)
        }
    }
}

pub(super) enum TrayCommand {
    Set {
        options: TrayIconOptions,
        responder: crate::platform::PlatformResponder<()>,
    },
    Remove {
        id: u32,
        responder: crate::platform::PlatformResponder<()>,
    },
    ShowMenu {
        id: u32,
        responder: crate::platform::PlatformResponder<()>,
    },
}

#[cfg(not(target_arch = "wasm32"))]
impl AppRunner {
    pub const fn tray_icons_supported(&self) -> bool {
        DesktopIntegrationSupport::current().tray_icons
    }

    pub fn is_tray_icon_registered(&self, id: u32) -> bool {
        self.runtime.tray_icons.contains_key(&id)
    }

    /// Sorted ids of tray icons currently owned by this application.
    pub fn tray_icon_ids(&self) -> Vec<u32> {
        let mut ids = self.runtime.tray_icons.keys().copied().collect::<Vec<_>>();
        ids.sort_unstable();
        ids
    }

    /// Screen rectangle of one tray icon in global logical desktop coordinates.
    ///
    /// `Ok(None)` means the platform host has not laid the item out yet, or the id is unknown.
    /// Linux StatusNotifierItem hosts own item placement and never report a rectangle.
    pub fn tray_icon_bounds(&self, id: u32) -> Result<Option<Rect>, PlatformError> {
        if id == 0 {
            return Err(tray_error("a tray icon id must be nonzero"));
        }
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        {
            let Some(tray) = self.runtime.tray_icons.get(&id) else {
                return Ok(None);
            };
            Ok(tray.rect().and_then(|rect| {
                crate::display::logical_rect_from_physical(
                    &self.runtime.displays,
                    (rect.position.x, rect.position.y),
                    (rect.size.width, rect.size.height),
                )
            }))
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            let _ = id;
            Err(PlatformError::Unsupported)
        }
    }

    /// Create or atomically replace a native tray icon.
    pub fn set_tray_icon(
        &mut self,
        options: TrayIconOptions,
    ) -> Result<PlatformResponse<()>, PlatformError> {
        validate_tray_options(&options)?;
        self.queue_tray_command(|responder| TrayCommand::Set { options, responder })
    }

    /// Remove one native tray icon. Removing an unknown id succeeds.
    pub fn remove_tray_icon(&mut self, id: u32) -> Result<PlatformResponse<()>, PlatformError> {
        if id == 0 {
            return Err(tray_error("a tray icon id must be nonzero"));
        }
        self.queue_tray_command(|responder| TrayCommand::Remove { id, responder })
    }

    /// Open a tray icon's context menu at the current cursor position.
    ///
    /// Linux StatusNotifierItem hosts own menu presentation and do not expose this operation.
    pub fn show_tray_menu(&mut self, id: u32) -> Result<PlatformResponse<()>, PlatformError> {
        if id == 0 {
            return Err(tray_error("a tray icon id must be nonzero"));
        }
        self.queue_tray_command(|responder| TrayCommand::ShowMenu { id, responder })
    }

    fn queue_tray_command(
        &mut self,
        command: impl FnOnce(crate::platform::PlatformResponder<()>) -> TrayCommand,
    ) -> Result<PlatformResponse<()>, PlatformError> {
        if !matches!(self.status, AppRunStatus::Continue) {
            return Err(PlatformError::Unavailable);
        }
        if self.runtime.pending_tray_commands.len() >= MAX_TRAY_ICONS * 4 {
            return Err(PlatformError::PendingQueueFull);
        }
        let (responder, response) = crate::platform::response_channel();
        self.runtime
            .event_proxy
            .send_event(RuntimeEvent::ExternalCommandsReady)
            .map_err(|_| PlatformError::Unavailable)?;
        self.runtime
            .pending_tray_commands
            .push_back(command(responder));
        Ok(response)
    }
}

impl Runtime {
    pub(super) fn apply_tray_platform_request(
        &mut self,
        request: crate::platform::PlatformRequest,
    ) -> Option<crate::platform::PlatformRequest> {
        match request {
            crate::platform::PlatformRequest::SetTrayIcon(options) => {
                if let Err(error) = self.set_tray_icon_now(options) {
                    tracing::warn!(%error, "could not set the tray icon");
                }
                None
            }
            crate::platform::PlatformRequest::RemoveTrayIcon(id) => {
                self.tray_icons.remove(&id);
                None
            }
            crate::platform::PlatformRequest::ShowTrayMenu(id) => {
                if let Err(error) = self.show_tray_menu_now(id) {
                    tracing::warn!(%error, "could not show the tray menu");
                }
                None
            }
            other => Some(other),
        }
    }

    pub(super) fn process_tray_commands(&mut self) {
        while let Some(command) = self.pending_tray_commands.pop_front() {
            match command {
                TrayCommand::Set { options, responder } => {
                    let result = self.set_tray_icon_now(options);
                    responder.complete(result);
                }
                TrayCommand::Remove { id, responder } => {
                    self.tray_icons.remove(&id);
                    responder.complete(Ok(()));
                }
                TrayCommand::ShowMenu { id, responder } => {
                    responder.complete(self.show_tray_menu_now(id));
                }
            }
        }
    }

    pub(super) fn set_tray_icon_now(
        &mut self,
        options: TrayIconOptions,
    ) -> Result<(), PlatformError> {
        if !self.tray_icons.contains_key(&options.id) && self.tray_icons.len() >= MAX_TRAY_ICONS {
            return Err(tray_error("the application already owns 32 tray icons"));
        }
        let id = options.id;
        let native = NativeTrayIcon::new(options, self.event_proxy.clone())?;
        self.tray_icons.insert(id, native);
        Ok(())
    }

    pub(super) fn show_tray_menu_now(&self, id: u32) -> Result<(), PlatformError> {
        let tray = self
            .tray_icons
            .get(&id)
            .ok_or_else(|| tray_error("the tray icon id is not registered"))?;
        tray.show_menu()
    }

    pub(super) fn invoke_tray_event(&mut self, event_loop: &ActiveEventLoop, event: TrayEvent) {
        if !self.tray_icons.contains_key(&event.tray_id) {
            return;
        }
        let Some(mut callback) = self.application_callbacks.tray_event.take() else {
            return;
        };
        let mut context = self.event_context();
        callback(event, &mut context);
        self.application_callbacks.tray_event = Some(callback);
        self.apply_application_context(event_loop, context);
    }
}

pub(crate) fn validate_tray_options(options: &TrayIconOptions) -> Result<(), PlatformError> {
    if options.id == 0 {
        return Err(tray_error("a tray icon id must be nonzero"));
    }
    for text in [options.tooltip.as_deref(), options.title.as_deref()]
        .into_iter()
        .flatten()
    {
        validate_text(text)?;
    }
    let mut ids = HashSet::new();
    let mut count = 0;
    validate_menu(&options.menu, 1, &mut count, &mut ids)
}

fn validate_menu(
    items: &[TrayMenuItem],
    depth: usize,
    count: &mut usize,
    ids: &mut HashSet<u32>,
) -> Result<(), PlatformError> {
    if depth > MAX_TRAY_MENU_DEPTH {
        return Err(tray_error(
            "a tray menu cannot be nested more than 8 levels",
        ));
    }
    for item in items {
        *count += 1;
        if *count > MAX_TRAY_MENU_ITEMS {
            return Err(tray_error("a tray menu cannot contain more than 256 items"));
        }
        match item {
            TrayMenuItem::Action { id, label, .. } => {
                if *id == 0 || !ids.insert(*id) {
                    return Err(tray_error(
                        "tray menu action ids must be nonzero and unique within the tray",
                    ));
                }
                validate_text(label)?;
            }
            TrayMenuItem::Submenu { label, items, .. } => {
                validate_text(label)?;
                validate_menu(items, depth + 1, count, ids)?;
            }
            TrayMenuItem::Separator => {}
        }
    }
    Ok(())
}

fn validate_text(text: &str) -> Result<(), PlatformError> {
    if text.is_empty() || text.len() > MAX_TRAY_TEXT_BYTES || text.contains('\0') {
        return Err(tray_error(
            "tray text must be nonempty, NUL-free, and at most 4096 UTF-8 bytes",
        ));
    }
    Ok(())
}

fn tray_error(message: impl Into<Arc<str>>) -> PlatformError {
    PlatformError::Platform(message.into())
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
pub(super) struct NativeTrayIcon {
    native: tray_icon::TrayIcon,
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
impl NativeTrayIcon {
    fn new(
        options: TrayIconOptions,
        proxy: EventLoopProxy<RuntimeEvent>,
    ) -> Result<Self, PlatformError> {
        install_native_menu_handlers(proxy);
        let icon = tray_icon::Icon::from_rgba(
            options.icon.rgba().to_vec(),
            options.icon.width(),
            options.icon.height(),
        )
        .map_err(|error| tray_error(error.to_string()))?;
        let menu = build_native_menu(options.id, &options.menu)?;
        let mut builder = tray_icon::TrayIconBuilder::new()
            .with_id(tray_native_id(options.id))
            .with_icon(icon)
            .with_icon_as_template(options.icon_is_template || options.icon.is_template())
            .with_menu_on_left_click(options.menu_on_left_click)
            .with_menu_on_right_click(true)
            .with_menu(Box::new(menu));
        if let Some(tooltip) = options.tooltip {
            builder = builder.with_tooltip(tooltip.as_ref());
        }
        if let Some(title) = options.title {
            builder = builder.with_title(title.as_ref());
        }
        let native = builder
            .build()
            .map_err(|error| tray_error(error.to_string()))?;
        if !options.visible {
            native
                .set_visible(false)
                .map_err(|error| tray_error(error.to_string()))?;
        }
        Ok(Self { native })
    }

    fn show_menu(&self) -> Result<(), PlatformError> {
        self.native.show_menu();
        Ok(())
    }

    fn rect(&self) -> Option<tray_icon::Rect> {
        self.native.rect()
    }
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn build_native_menu(
    tray_id: u32,
    items: &[TrayMenuItem],
) -> Result<tray_icon::menu::Menu, PlatformError> {
    use tray_icon::menu::{CheckMenuItem, IsMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};

    trait NativeMenuParent {
        fn append_item(&self, item: &dyn IsMenuItem) -> tray_icon::menu::Result<()>;
    }

    impl NativeMenuParent for Menu {
        fn append_item(&self, item: &dyn IsMenuItem) -> tray_icon::menu::Result<()> {
            self.append(item)
        }
    }

    impl NativeMenuParent for Submenu {
        fn append_item(&self, item: &dyn IsMenuItem) -> tray_icon::menu::Result<()> {
            self.append(item)
        }
    }

    fn append_items(
        parent: &impl NativeMenuParent,
        tray_id: u32,
        items: &[TrayMenuItem],
    ) -> Result<(), PlatformError> {
        for item in items {
            match item {
                TrayMenuItem::Action {
                    id,
                    label,
                    enabled,
                    checked,
                } => {
                    if *checked {
                        let native = CheckMenuItem::with_id(
                            tray_menu_native_id(tray_id, *id),
                            label.as_ref(),
                            *enabled,
                            true,
                            None,
                        );
                        parent
                            .append_item(&native)
                            .map_err(|error| tray_error(error.to_string()))?;
                    } else {
                        let native = MenuItem::with_id(
                            tray_menu_native_id(tray_id, *id),
                            label.as_ref(),
                            *enabled,
                            None,
                        );
                        parent
                            .append_item(&native)
                            .map_err(|error| tray_error(error.to_string()))?;
                    }
                }
                TrayMenuItem::Separator => parent
                    .append_item(&PredefinedMenuItem::separator())
                    .map_err(|error| tray_error(error.to_string()))?,
                TrayMenuItem::Submenu {
                    label,
                    enabled,
                    items,
                } => {
                    let submenu = Submenu::new(label.as_ref(), *enabled);
                    append_items(&submenu, tray_id, items)?;
                    parent
                        .append_item(&submenu)
                        .map_err(|error| tray_error(error.to_string()))?;
                }
            }
        }
        Ok(())
    }

    let menu = Menu::new();
    append_items(&menu, tray_id, items)?;
    Ok(menu)
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn tray_native_id(id: u32) -> String {
    format!("quickgui-tray-{id}")
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn tray_menu_native_id(tray_id: u32, item_id: u32) -> String {
    format!("quickgui-tray-{tray_id}-item-{item_id}")
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn parse_tray_native_id(value: &str) -> Option<u32> {
    value.strip_prefix("quickgui-tray-")?.parse().ok()
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn parse_tray_menu_native_id(value: &str) -> Option<(u32, u32)> {
    let value = value.strip_prefix("quickgui-tray-")?;
    let (tray, item) = value.split_once("-item-")?;
    Some((tray.parse().ok()?, item.parse().ok()?))
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn parse_application_menu_native_id(value: &str) -> Option<usize> {
    value.strip_prefix("quickgui-app-menu-")?.parse().ok()
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn parse_popup_menu_native_id(value: &str) -> Option<(u64, usize)> {
    let value = value.strip_prefix("quickgui-popup-menu-")?;
    let (popup, action) = value.split_once("-action-")?;
    Some((popup.parse().ok()?, action.parse().ok()?))
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
static TRAY_PROXY: Mutex<Option<EventLoopProxy<RuntimeEvent>>> = Mutex::new(None);

#[cfg(any(target_os = "macos", target_os = "windows"))]
pub(super) fn install_native_menu_handlers(proxy: EventLoopProxy<RuntimeEvent>) {
    use tray_icon::{MouseButton, MouseButtonState, TrayIconEvent};

    *TRAY_PROXY
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(proxy);
    TrayIconEvent::set_event_handler(Some(|native: TrayIconEvent| {
        let Some(tray_id) = parse_tray_native_id(native.id().as_ref()) else {
            return;
        };
        let event = match native {
            TrayIconEvent::Click {
                position,
                button,
                button_state,
                ..
            } => {
                let mut event = TrayEvent::simple(tray_id, TrayEventKind::Click);
                event.position = Some((position.x, position.y));
                event.button = Some(match button {
                    MouseButton::Left => TrayMouseButton::Left,
                    MouseButton::Right => TrayMouseButton::Right,
                    MouseButton::Middle => TrayMouseButton::Middle,
                });
                event.pressed = Some(matches!(button_state, MouseButtonState::Down));
                event
            }
            TrayIconEvent::DoubleClick {
                position, button, ..
            } => {
                let mut event = TrayEvent::simple(tray_id, TrayEventKind::DoubleClick);
                event.position = Some((position.x, position.y));
                event.button = Some(match button {
                    MouseButton::Left => TrayMouseButton::Left,
                    MouseButton::Right => TrayMouseButton::Right,
                    MouseButton::Middle => TrayMouseButton::Middle,
                });
                event
            }
            TrayIconEvent::Enter { position, .. } => {
                let mut event = TrayEvent::simple(tray_id, TrayEventKind::Enter);
                event.position = Some((position.x, position.y));
                event
            }
            TrayIconEvent::Move { position, .. } => {
                let mut event = TrayEvent::simple(tray_id, TrayEventKind::Move);
                event.position = Some((position.x, position.y));
                event
            }
            TrayIconEvent::Leave { position, .. } => {
                let mut event = TrayEvent::simple(tray_id, TrayEventKind::Leave);
                event.position = Some((position.x, position.y));
                event
            }
            _ => return,
        };
        let proxy = TRAY_PROXY
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        if let Some(proxy) = proxy {
            let _ = proxy.send_event(RuntimeEvent::Tray(event));
        }
    }));
    tray_icon::menu::MenuEvent::set_event_handler(Some(|native: tray_icon::menu::MenuEvent| {
        let proxy = TRAY_PROXY
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        let Some(proxy) = proxy else {
            return;
        };
        if let Some((tray_id, item_id)) = parse_tray_menu_native_id(native.id.as_ref()) {
            let _ = proxy.send_event(RuntimeEvent::Tray(TrayEvent::menu_item(tray_id, item_id)));
        } else if let Some((popup_id, action_id)) = parse_popup_menu_native_id(native.id.as_ref()) {
            let _ = proxy.send_event(RuntimeEvent::NativePopupMenuAction(popup_id, action_id));
        } else if let Some(action_id) = parse_application_menu_native_id(native.id.as_ref()) {
            let _ = proxy.send_event(RuntimeEvent::MenuAction(action_id));
        }
    }));
}

#[cfg(target_os = "linux")]
struct LinuxTray {
    id: u32,
    options: TrayIconOptions,
    proxy: EventLoopProxy<RuntimeEvent>,
}

#[cfg(target_os = "linux")]
impl ksni::Tray for LinuxTray {
    const MENU_ON_ACTIVATE: bool = false;

    fn id(&self) -> String {
        format!("quickgui-tray-{}", self.id)
    }

    fn title(&self) -> String {
        self.options
            .title
            .as_deref()
            .or(self.options.tooltip.as_deref())
            .unwrap_or("QuickGUI")
            .to_owned()
    }

    fn status(&self) -> ksni::Status {
        if self.options.visible {
            ksni::Status::Active
        } else {
            ksni::Status::Passive
        }
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        let mut data = self.options.icon.rgba().to_vec();
        for pixel in data.chunks_exact_mut(4) {
            pixel.rotate_right(1);
        }
        vec![ksni::Icon {
            width: self.options.icon.width() as i32,
            height: self.options.icon.height() as i32,
            data,
        }]
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip {
            title: self
                .options
                .tooltip
                .as_deref()
                .unwrap_or_default()
                .to_owned(),
            ..Default::default()
        }
    }

    fn activate(&mut self, x: i32, y: i32) {
        let mut event = TrayEvent::simple(self.id, TrayEventKind::Click);
        event.button = Some(TrayMouseButton::Left);
        event.position = Some((f64::from(x), f64::from(y)));
        let _ = self.proxy.send_event(RuntimeEvent::Tray(event));
    }

    fn secondary_activate(&mut self, x: i32, y: i32) {
        let mut event = TrayEvent::simple(self.id, TrayEventKind::Click);
        event.button = Some(TrayMouseButton::Middle);
        event.position = Some((f64::from(x), f64::from(y)));
        let _ = self.proxy.send_event(RuntimeEvent::Tray(event));
    }

    fn scroll(&mut self, delta: i32, orientation: ksni::Orientation) {
        let mut event = TrayEvent::simple(self.id, TrayEventKind::Scroll);
        event.scroll_delta = Some(delta);
        event.horizontal = Some(matches!(orientation, ksni::Orientation::Horizontal));
        let _ = self.proxy.send_event(RuntimeEvent::Tray(event));
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        linux_menu_items(self.id, &self.options.menu, self.proxy.clone())
    }
}

#[cfg(target_os = "linux")]
fn linux_menu_items(
    tray_id: u32,
    items: &[TrayMenuItem],
    proxy: EventLoopProxy<RuntimeEvent>,
) -> Vec<ksni::MenuItem<LinuxTray>> {
    items
        .iter()
        .map(|item| match item {
            TrayMenuItem::Action {
                id,
                label,
                enabled,
                checked,
            } => {
                let item_id = *id;
                let callback_proxy = proxy.clone();
                if *checked {
                    ksni::menu::CheckmarkItem {
                        label: label.to_string(),
                        enabled: *enabled,
                        checked: true,
                        activate: Box::new(move |_| {
                            let _ = callback_proxy.send_event(RuntimeEvent::Tray(
                                TrayEvent::menu_item(tray_id, item_id),
                            ));
                        }),
                        ..Default::default()
                    }
                    .into()
                } else {
                    ksni::menu::StandardItem {
                        label: label.to_string(),
                        enabled: *enabled,
                        activate: Box::new(move |_| {
                            let _ = callback_proxy.send_event(RuntimeEvent::Tray(
                                TrayEvent::menu_item(tray_id, item_id),
                            ));
                        }),
                        ..Default::default()
                    }
                    .into()
                }
            }
            TrayMenuItem::Separator => ksni::MenuItem::Separator,
            TrayMenuItem::Submenu {
                label,
                enabled,
                items,
            } => ksni::menu::SubMenu {
                label: label.to_string(),
                enabled: *enabled,
                submenu: linux_menu_items(tray_id, items, proxy.clone()),
                ..Default::default()
            }
            .into(),
        })
        .collect()
}

#[cfg(target_os = "linux")]
pub(super) struct NativeTrayIcon {
    handle: ksni::blocking::Handle<LinuxTray>,
}

#[cfg(target_os = "linux")]
impl NativeTrayIcon {
    fn new(
        options: TrayIconOptions,
        proxy: EventLoopProxy<RuntimeEvent>,
    ) -> Result<Self, PlatformError> {
        use ksni::blocking::TrayMethods;

        let tray = LinuxTray {
            id: options.id,
            options,
            proxy,
        };
        let handle = tray
            .spawn()
            .map_err(|error| tray_error(error.to_string()))?;
        Ok(Self { handle })
    }

    fn show_menu(&self) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported)
    }
}

#[cfg(target_os = "linux")]
impl Drop for NativeTrayIcon {
    fn drop(&mut self) {
        self.handle.shutdown().wait();
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
pub(super) struct NativeTrayIcon;

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
impl NativeTrayIcon {
    fn new(
        _options: TrayIconOptions,
        _proxy: EventLoopProxy<RuntimeEvent>,
    ) -> Result<Self, PlatformError> {
        Err(PlatformError::Unsupported)
    }

    fn show_menu(&self) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported)
    }
}

pub(super) fn clear_tray_handler_proxy() {
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    {
        *TRAY_PROXY
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn icon() -> TrayIconImage {
        TrayIconImage::from_rgba(vec![255, 0, 0, 255], 1, 1).unwrap()
    }

    #[test]
    fn validates_icon_dimensions_and_byte_count() {
        assert!(TrayIconImage::from_rgba(vec![0; 4], 1, 1).is_ok());
        assert!(TrayIconImage::from_rgba(vec![0; 3], 1, 1).is_err());
        assert!(TrayIconImage::from_rgba(Vec::<u8>::new(), 0, 0).is_err());
    }

    #[test]
    fn template_metadata_travels_with_the_artwork() {
        assert!(!icon().is_template());
        let template = icon().template(true);
        assert!(template.is_template());
        assert_ne!(icon(), template);
        assert!(!template.clone().template(false).is_template());
        assert!(!TrayIconOptions::new(1, icon()).icon_is_template);
        assert!(TrayIconOptions::new(1, template).icon.is_template());
    }

    #[test]
    fn from_path_marks_template_filenames() {
        let png = crate::Image::from_rgba(1, 1, vec![0, 0, 0, 255])
            .unwrap()
            .to_png()
            .unwrap();
        let template_path = std::env::temp_dir().join(format!(
            "quickgui-{}-statusTemplate.png",
            std::process::id()
        ));
        let plain_path =
            std::env::temp_dir().join(format!("quickgui-{}-status.png", std::process::id()));
        std::fs::write(&template_path, &png).unwrap();
        std::fs::write(&plain_path, &png).unwrap();
        let template = TrayIconImage::from_path(&template_path).unwrap();
        let plain = TrayIconImage::from_path(&plain_path).unwrap();
        let _ = std::fs::remove_file(template_path);
        let _ = std::fs::remove_file(plain_path);
        assert!(template.is_template());
        assert!(!plain.is_template());
        assert!(!template.template(false).is_template());
    }

    #[test]
    fn validates_bounded_unique_menu_ids() {
        let valid = TrayIconOptions::new(1, icon()).menu([
            TrayMenuItem::action(1, "Open"),
            TrayMenuItem::submenu("More", [TrayMenuItem::action(2, "Quit")]),
        ]);
        assert!(validate_tray_options(&valid).is_ok());

        let duplicate = TrayIconOptions::new(1, icon()).menu([
            TrayMenuItem::action(1, "Open"),
            TrayMenuItem::action(1, "Quit"),
        ]);
        assert!(validate_tray_options(&duplicate).is_err());
    }
}
