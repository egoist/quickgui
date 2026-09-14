use std::sync::Arc;

use std::{fmt, path::Path};

use crate::{Accelerator, Action, AnyAction, Image, ImageError, KeymapError, Keystroke};

/// Maximum action, role, separator, submenu, and system-menu nodes in one native menu tree.
pub const MAX_NATIVE_MENU_ITEMS: usize = 1_024;
/// Maximum nested native-menu depth.
pub const MAX_NATIVE_MENU_DEPTH: usize = 16;
/// Maximum UTF-8 bytes in one native menu or item label.
pub const MAX_NATIVE_MENU_TEXT_BYTES: usize = 4 * 1024;
/// Maximum aggregate UTF-8 label bytes in one native menu declaration.
pub const MAX_NATIVE_MENU_TOTAL_TEXT_BYTES: usize = 1024 * 1024;

#[derive(Clone, Debug, Eq, thiserror::Error, PartialEq)]
pub enum MenuError {
    #[error("native menu labels must be nonempty, NUL-free, and at most 4096 UTF-8 bytes")]
    InvalidText,
    #[error("a native menu tree cannot contain more than 1024 nodes")]
    TooManyItems,
    #[error("a native menu tree cannot be nested more than 16 levels")]
    TooDeep,
    #[error("native menu labels cannot retain more than 1 MiB of aggregate UTF-8 text")]
    TooMuchText,
}

/// A declarative application menu projected to the platform's native menu system.
///
/// Menus own typed actions rather than callbacks. Selecting an action item dispatches through the
/// same focused action path as a key binding, so keyboard shortcuts, menus, and programmatic
/// commands share one command implementation.
#[derive(Clone, Debug)]
pub struct Menu {
    pub name: Arc<str>,
    pub items: Vec<MenuItem>,
    pub disabled: bool,
}

impl Menu {
    pub fn new(name: impl Into<Arc<str>>) -> Self {
        Self {
            name: name.into(),
            items: Vec::new(),
            disabled: false,
        }
    }

    /// Replace this menu's items, matching GPUI's menu construction shape.
    pub fn items(mut self, items: impl IntoIterator<Item = MenuItem>) -> Self {
        self.items = items.into_iter().collect();
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Append one item for fluent, web-like menu construction.
    pub fn item(mut self, item: MenuItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn action<A: Action>(self, name: impl Into<Arc<str>>, action: A) -> Self {
        self.item(MenuItem::action(name, action))
    }

    pub fn separator(self) -> Self {
        self.item(MenuItem::separator())
    }

    pub fn submenu(self, menu: Menu) -> Self {
        self.item(MenuItem::submenu(menu))
    }

    pub fn push(&mut self, item: MenuItem) {
        self.items.push(item);
    }
}

/// A menu populated and managed by the operating system.
#[derive(Clone, Debug)]
pub struct OsMenu {
    pub name: Arc<str>,
    pub menu_type: SystemMenuType,
}

/// Native menus with operating-system-owned contents.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SystemMenuType {
    /// The macOS Services menu.
    Services,
    /// The operating system's window-management menu.
    Window,
    /// The operating system's help menu.
    Help,
    /// The operating system's recent-documents menu.
    ///
    /// macOS populates any menu that owns a `clearRecentDocuments:` item, so QuickGUI builds an
    /// "Open Recent" submenu holding one "Clear Menu" command and lets AppKit fill the rest.
    RecentDocuments,
}

/// Commands that should first follow the operating system's native responder chain.
///
/// If no native responder accepts the selector, QuickGUI falls back to dispatching the associated
/// typed action through the focused retained tree. This makes one Edit menu work for both embedded
/// NSView controls and GPU-rendered QuickGUI inputs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OsAction {
    Cut,
    Copy,
    Paste,
    SelectAll,
    Undo,
    Redo,
    /// Show the application's native About panel where the operating system provides one.
    About,
    /// Hide every window belonging to this application.
    HideApplication,
    /// Hide other applications while keeping this application visible.
    HideOtherApplications,
    /// Reveal applications hidden through the native application menu.
    ShowAllApplications,
    /// Request orderly application termination.
    Quit,
    /// Close the active window through its ordinary close policy.
    CloseWindow,
    /// Minimize the active window.
    MinimizeWindow,
    /// Toggle the active window's native zoom/maximized state.
    ZoomWindow,
    /// Toggle borderless/native fullscreen for the active window.
    ToggleFullscreen,
    /// Bring the application's windows to the front.
    BringAllToFront,
    /// Open the operating system's application help UI where available.
    ShowHelp,
    /// Paste the clipboard's plain text without its original styling.
    PasteAndMatchStyle,
    /// Delete the current selection without writing it to the clipboard.
    Delete,
    /// Start the operating system's speech synthesis for the current selection.
    StartSpeaking,
    /// Stop the operating system's speech synthesis.
    StopSpeaking,
    /// Select the next native window tab.
    SelectNextTab,
    /// Select the previous native window tab.
    SelectPreviousTab,
    /// Merge every window of this application into one native tab group.
    MergeAllWindows,
    /// Move the active native tab into its own window.
    MoveTabToNewWindow,
    /// Toggle the native tab bar for the active window.
    ToggleTabBar,
    /// Toggle the native tab overview for the active window.
    ToggleTabOverview,
}

impl OsAction {
    /// Whether this command participates in focused native text-control responder routing.
    pub const fn is_text_editing(self) -> bool {
        matches!(
            self,
            Self::Cut
                | Self::Copy
                | Self::Paste
                | Self::PasteAndMatchStyle
                | Self::Delete
                | Self::SelectAll
                | Self::Undo
                | Self::Redo
                | Self::StartSpeaking
                | Self::StopSpeaking
        )
    }
}

/// Native state indicator shown beside a menu item.
///
/// QuickGUI treats check and radio items as controlled state: selecting one dispatches its role or
/// typed action, and the next menu declaration supplies the resulting state. This avoids a second
/// mutable source of truth inside the platform menu host.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum MenuItemMark {
    #[default]
    None,
    Check,
    Radio,
}

/// A decoded, bounded RGBA image suitable for a native menu item.
#[derive(Clone)]
pub struct MenuIcon(Image);

impl MenuIcon {
    /// Construct an icon from tightly packed, straight-alpha RGBA8 pixels.
    pub fn from_rgba(
        width: u32,
        height: u32,
        rgba: impl Into<std::sync::Arc<[u8]>>,
    ) -> Result<Self, ImageError> {
        Image::from_rgba(width, height, rgba).map(Self)
    }

    /// Decode PNG, JPEG, TIFF, WebP, or the first frame of a GIF.
    pub fn decode(encoded: impl AsRef<[u8]>) -> Result<Self, ImageError> {
        Image::decode(encoded).map(Self)
    }

    /// Decode a supported image file with QuickGUI's normal image bounds.
    ///
    /// Paths whose stem ends in `Template` (optionally `@2x`) are marked for macOS template
    /// rendering. Call [`Self::template`] to override that convention.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ImageError> {
        let path = path.as_ref();
        Image::open(path).map(|image| Self(image.template(crate::is_template_image_path(path))))
    }

    /// Mark the artwork as a macOS template image.
    pub fn template(self, template: bool) -> Self {
        Self(self.0.template(template))
    }

    /// Whether this artwork is marked for macOS template rendering.
    pub fn is_template(&self) -> bool {
        self.0.is_template()
    }

    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    pub(crate) fn image(&self) -> &Image {
        &self.0
    }

    pub fn width(&self) -> u32 {
        self.0.width()
    }

    pub fn height(&self) -> u32 {
        self.0.height()
    }

    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    pub(crate) fn rgba(&self) -> &[u8] {
        self.0.rgba()
    }
}

impl fmt::Debug for MenuIcon {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MenuIcon")
            .field("width", &self.width())
            .field("height", &self.height())
            .field("bytes", &self.0.byte_len())
            .finish()
    }
}

/// One action, separator, or nested menu in a Menu.
#[derive(Clone, Debug)]
pub enum MenuItem {
    Separator,
    Submenu(Menu),
    SystemMenu(OsMenu),
    Action {
        name: Arc<str>,
        action: AnyAction,
        os_action: Option<OsAction>,
        checked: bool,
        mark: MenuItemMark,
        icon: Option<MenuIcon>,
        disabled: bool,
        hidden: bool,
        shortcut: Option<Keystroke>,
    },
    /// A standard operating-system command that does not require an application action type.
    Role {
        name: Arc<str>,
        role: OsAction,
        checked: bool,
        mark: MenuItemMark,
        icon: Option<MenuIcon>,
        disabled: bool,
        hidden: bool,
        shortcut: Option<Keystroke>,
    },
}

impl MenuItem {
    pub fn action<A: Action>(name: impl Into<Arc<str>>, action: A) -> Self {
        Self::Action {
            name: name.into(),
            action: AnyAction::new(action),
            os_action: None,
            checked: false,
            mark: MenuItemMark::None,
            icon: None,
            disabled: false,
            hidden: false,
            shortcut: None,
        }
    }

    pub fn os_action<A: Action>(name: impl Into<Arc<str>>, action: A, os_action: OsAction) -> Self {
        Self::Action {
            name: name.into(),
            action: AnyAction::new(action),
            os_action: Some(os_action),
            checked: false,
            mark: MenuItemMark::None,
            icon: None,
            disabled: false,
            hidden: false,
            shortcut: None,
        }
    }

    /// Construct a standard native application, window, help, or editing command.
    pub fn role(name: impl Into<Arc<str>>, role: OsAction) -> Self {
        Self::Role {
            name: name.into(),
            role,
            checked: false,
            mark: MenuItemMark::None,
            icon: None,
            disabled: false,
            hidden: false,
            shortcut: None,
        }
    }

    pub fn separator() -> Self {
        Self::Separator
    }

    pub fn submenu(menu: Menu) -> Self {
        Self::Submenu(menu)
    }

    pub fn os_submenu(name: impl Into<Arc<str>>, menu_type: SystemMenuType) -> Self {
        Self::SystemMenu(OsMenu {
            name: name.into(),
            menu_type,
        })
    }

    pub fn checked(mut self, value: bool) -> Self {
        match &mut self {
            Self::Action { checked, mark, .. } | Self::Role { checked, mark, .. } => {
                *checked = value;
                *mark = MenuItemMark::Check;
            }
            Self::Separator | Self::Submenu(_) | Self::SystemMenu(_) => {}
        }
        self
    }

    /// Display a mutually-exclusive native radio indicator with controlled checked state.
    pub fn radio(mut self, value: bool) -> Self {
        match &mut self {
            Self::Action { checked, mark, .. } | Self::Role { checked, mark, .. } => {
                *checked = value;
                *mark = MenuItemMark::Radio;
            }
            Self::Separator | Self::Submenu(_) | Self::SystemMenu(_) => {}
        }
        self
    }

    /// Attach an image to a normal native menu item.
    pub fn icon(mut self, value: MenuIcon) -> Self {
        match &mut self {
            Self::Action { icon, .. } | Self::Role { icon, .. } => *icon = Some(value),
            Self::Separator | Self::Submenu(_) | Self::SystemMenu(_) => {}
        }
        self
    }

    /// Override the platform key equivalent with an explicit keystroke.
    ///
    /// An explicit shortcut wins over the keymap binding QuickGUI would otherwise display.
    pub fn keystroke(mut self, value: Keystroke) -> Self {
        match &mut self {
            Self::Action { shortcut, .. } | Self::Role { shortcut, .. } => *shortcut = Some(value),
            Self::Separator | Self::Submenu(_) | Self::SystemMenu(_) => {}
        }
        self
    }

    /// Override the platform key equivalent with an Electron accelerator string.
    ///
    /// Use [`Self::try_accelerator`] when the accelerator comes from data instead of a literal;
    /// this builder keeps the fluent chain infallible by dropping an unparsable declaration after
    /// logging it.
    pub fn accelerator(self, value: impl AsRef<str>) -> Self {
        let value = value.as_ref();
        match Accelerator::parse(value) {
            Ok(stroke) => self.keystroke(stroke),
            Err(error) => {
                tracing::warn!(accelerator = value, %error, "ignoring an invalid menu accelerator");
                self
            }
        }
    }

    /// Override the platform key equivalent, reporting an unparsable accelerator.
    pub fn try_accelerator(self, value: impl AsRef<str>) -> Result<Self, KeymapError> {
        Accelerator::parse(value.as_ref()).map(|stroke| self.keystroke(stroke))
    }

    /// The explicit key equivalent declared for this item, if any.
    pub fn shortcut(&self) -> Option<&Keystroke> {
        match self {
            Self::Action { shortcut, .. } | Self::Role { shortcut, .. } => shortcut.as_ref(),
            Self::Separator | Self::Submenu(_) | Self::SystemMenu(_) => None,
        }
    }

    /// Keep a declared item out of the presented native menu without removing its action id.
    pub fn hidden(mut self, value: bool) -> Self {
        match &mut self {
            Self::Action { hidden, .. } | Self::Role { hidden, .. } => *hidden = value,
            Self::Separator | Self::Submenu(_) | Self::SystemMenu(_) => {}
        }
        self
    }

    pub fn is_hidden(&self) -> bool {
        match self {
            Self::Action { hidden, .. } | Self::Role { hidden, .. } => *hidden,
            Self::Separator | Self::Submenu(_) | Self::SystemMenu(_) => false,
        }
    }

    pub fn is_checked(&self) -> bool {
        matches!(
            self,
            Self::Action { checked: true, .. } | Self::Role { checked: true, .. }
        )
    }

    pub fn mark(&self) -> MenuItemMark {
        match self {
            Self::Action { mark, .. } | Self::Role { mark, .. } => *mark,
            Self::Separator | Self::Submenu(_) | Self::SystemMenu(_) => MenuItemMark::None,
        }
    }

    pub fn menu_icon(&self) -> Option<&MenuIcon> {
        match self {
            Self::Action { icon, .. } | Self::Role { icon, .. } => icon.as_ref(),
            Self::Separator | Self::Submenu(_) | Self::SystemMenu(_) => None,
        }
    }

    pub fn disabled(mut self, value: bool) -> Self {
        match &mut self {
            Self::Action { disabled, .. } | Self::Role { disabled, .. } => *disabled = value,
            Self::Submenu(menu) => menu.disabled = value,
            Self::Separator | Self::SystemMenu(_) => {}
        }
        self
    }

    pub fn is_disabled(&self) -> bool {
        match self {
            Self::Action { disabled, .. } | Self::Role { disabled, .. } => *disabled,
            Self::Submenu(menu) => menu.disabled,
            Self::Separator | Self::SystemMenu(_) => false,
        }
    }

    /// Convenience inverse of disabled.
    pub fn enabled(self, value: bool) -> Self {
        self.disabled(!value)
    }

    pub fn is_enabled(&self) -> bool {
        !self.is_disabled()
    }
}

pub(crate) struct MenuAction {
    pub action: Option<AnyAction>,
    pub os_action: Option<OsAction>,
    pub disabled: bool,
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    pub checked: bool,
    /// Explicit key equivalent that outranks keymap derivation for this item.
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    pub shortcut: Option<Keystroke>,
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    pub hidden: bool,
}

pub(crate) fn collect_menu_actions(menus: &[Menu]) -> Vec<MenuAction> {
    fn collect(menu: &Menu, actions: &mut Vec<MenuAction>) {
        for item in &menu.items {
            match item {
                MenuItem::Action {
                    action,
                    os_action,
                    disabled,
                    checked,
                    hidden,
                    shortcut,
                    ..
                } => actions.push(MenuAction {
                    action: Some(action.clone()),
                    os_action: *os_action,
                    disabled: *disabled,
                    checked: *checked,
                    shortcut: shortcut.clone(),
                    hidden: *hidden,
                }),
                MenuItem::Role {
                    role,
                    disabled,
                    checked,
                    hidden,
                    shortcut,
                    ..
                } => actions.push(MenuAction {
                    action: None,
                    os_action: Some(*role),
                    disabled: *disabled,
                    checked: *checked,
                    shortcut: shortcut.clone(),
                    hidden: *hidden,
                }),
                MenuItem::Submenu(menu) => collect(menu, actions),
                MenuItem::Separator | MenuItem::SystemMenu(_) => {}
            }
        }
    }

    let mut actions = Vec::new();
    for menu in menus {
        collect(menu, &mut actions);
    }
    actions
}

pub(crate) fn validate_menus(menus: &[Menu]) -> Result<(), MenuError> {
    fn validate_text(text: &str, total: &mut usize) -> Result<(), MenuError> {
        if text.is_empty() || text.len() > MAX_NATIVE_MENU_TEXT_BYTES || text.contains('\0') {
            return Err(MenuError::InvalidText);
        }
        *total = total
            .checked_add(text.len())
            .ok_or(MenuError::TooMuchText)?;
        if *total > MAX_NATIVE_MENU_TOTAL_TEXT_BYTES {
            return Err(MenuError::TooMuchText);
        }
        Ok(())
    }

    fn validate_menu(
        menu: &Menu,
        depth: usize,
        count: &mut usize,
        text: &mut usize,
    ) -> Result<(), MenuError> {
        if depth > MAX_NATIVE_MENU_DEPTH {
            return Err(MenuError::TooDeep);
        }
        validate_text(&menu.name, text)?;
        for item in &menu.items {
            *count = count.checked_add(1).ok_or(MenuError::TooManyItems)?;
            if *count > MAX_NATIVE_MENU_ITEMS {
                return Err(MenuError::TooManyItems);
            }
            match item {
                MenuItem::Action { name, .. } | MenuItem::Role { name, .. } => {
                    validate_text(name, text)?;
                }
                MenuItem::Submenu(menu) => validate_menu(menu, depth + 1, count, text)?,
                MenuItem::SystemMenu(menu) => validate_text(&menu.name, text)?,
                MenuItem::Separator => {}
            }
        }
        Ok(())
    }

    let mut count = 0;
    let mut text = 0;
    for menu in menus {
        validate_menu(menu, 1, &mut count, &mut text)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::any::TypeId;

    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    struct OpenFile;

    #[derive(Clone, Debug, PartialEq)]
    struct ToggleSidebar;

    #[test]
    fn preserves_nested_action_order_and_state() {
        let menus = [Menu::new("File")
            .action("Open", OpenFile)
            .submenu(
                Menu::new("View").item(MenuItem::action("Sidebar", ToggleSidebar).checked(true)),
            )
            .item(MenuItem::action("Disabled", OpenFile).disabled(true))];

        let actions = collect_menu_actions(&menus);
        assert_eq!(actions.len(), 3);
        assert_eq!(
            actions[0].action.as_ref().unwrap().type_id(),
            TypeId::of::<OpenFile>()
        );
        assert_eq!(
            actions[1].action.as_ref().unwrap().type_id(),
            TypeId::of::<ToggleSidebar>()
        );
        assert!(actions[1].checked);
        assert!(actions[2].disabled);
    }

    #[test]
    fn gpui_shaped_builders_cover_disabled_and_system_items() {
        let menu = Menu::new("Application")
            .items([
                MenuItem::os_submenu("Services", SystemMenuType::Services),
                MenuItem::separator(),
            ])
            .disabled(true);
        assert_eq!(menu.name.as_ref(), "Application");
        assert_eq!(menu.items.len(), 2);
        assert!(menu.disabled);
        assert!(MenuItem::submenu(Menu::new("Nested").disabled(true)).is_disabled());
    }

    #[test]
    fn roles_radio_marks_and_icons_remain_in_core_action_order() {
        let icon = MenuIcon::from_rgba(1, 1, [255, 0, 0, 255].as_slice()).unwrap();
        let menus = [Menu::new("Window")
            .item(MenuItem::role("Minimize", OsAction::MinimizeWindow))
            .item(
                MenuItem::action("Mode", ToggleSidebar)
                    .radio(true)
                    .icon(icon),
            )];

        let actions = collect_menu_actions(&menus);
        assert_eq!(actions.len(), 2);
        assert!(actions[0].action.is_none());
        assert_eq!(actions[0].os_action, Some(OsAction::MinimizeWindow));
        assert_eq!(menus[0].items[1].mark(), MenuItemMark::Radio);
        assert!(actions[1].checked);
    }

    #[test]
    fn explicit_accelerators_and_hidden_flags_reach_the_collected_actions() {
        let menus = [Menu::new("File")
            .item(MenuItem::action("Save As", OpenFile).accelerator("CmdOrCtrl+Shift+S"))
            .item(
                MenuItem::role("Close", OsAction::CloseWindow)
                    .keystroke(Keystroke::parse("cmd-w").unwrap()),
            )
            .item(MenuItem::action("Debug", OpenFile).hidden(true))];

        let actions = collect_menu_actions(&menus);
        assert_eq!(actions.len(), 3);
        assert_eq!(
            actions[0].shortcut,
            Some(crate::Accelerator::parse("CmdOrCtrl+Shift+S").unwrap())
        );
        assert_eq!(
            actions[1].shortcut,
            Some(Keystroke::parse("cmd-w").unwrap())
        );
        assert_eq!(actions[2].shortcut, None);
        assert!(!actions[0].hidden);
        assert!(actions[2].hidden);
        assert!(menus[0].items[2].is_hidden());
        assert_eq!(
            menus[0].items[1].shortcut(),
            Some(&Keystroke::parse("cmd-w").unwrap())
        );
    }

    #[test]
    fn invalid_accelerators_are_reported_and_never_silently_bound() {
        assert!(
            MenuItem::action("Save", OpenFile)
                .try_accelerator("Cmd+Nonsense")
                .is_err()
        );
        // The infallible builder keeps the chain usable and simply declares no shortcut.
        assert_eq!(
            MenuItem::action("Save", OpenFile)
                .accelerator("Cmd+Nonsense")
                .shortcut(),
            None
        );
        assert!(
            MenuItem::separator()
                .accelerator("Cmd+S")
                .shortcut()
                .is_none()
        );
    }

    #[test]
    fn native_menu_validation_bounds_depth_count_and_text() {
        assert_eq!(
            validate_menus(&[Menu::new("")]),
            Err(MenuError::InvalidText)
        );

        let mut nested = Menu::new("leaf");
        for depth in 0..MAX_NATIVE_MENU_DEPTH {
            nested = Menu::new(format!("level-{depth}")).submenu(nested);
        }
        assert_eq!(validate_menus(&[nested]), Err(MenuError::TooDeep));

        let oversized = Menu::new("File").items(
            (0..=MAX_NATIVE_MENU_ITEMS)
                .map(|_| MenuItem::separator())
                .collect::<Vec<_>>(),
        );
        assert_eq!(validate_menus(&[oversized]), Err(MenuError::TooManyItems));
    }
}
