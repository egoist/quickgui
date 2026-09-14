//! QuickGUI is a small, damage-driven foundation for native desktop interfaces.
//!
//! It deliberately keeps the hot path narrow: application state is retained,
//! windows sleep while clean, long lists are virtualized, rectangles are
//! instanced in one draw call, and shaped text is cached by stable [`TextId`]s.

mod action;
mod animated_image;
mod animation;
mod assets;
mod autocomplete;
mod avatar;
mod background;
mod calendar;
mod canvas;
mod checkbox_group;
mod clipboard;
#[cfg(feature = "editor")]
mod code_block;
mod color;
mod combobox;
mod constrained_combobox;
mod context_menu;
mod cursor;
mod custom_shader;
mod custom_shader_renderer;
mod date_field;
mod dialog;
#[cfg(feature = "editor")]
mod diff_view;
mod disclosure;
mod display;
mod drawer;
#[cfg(feature = "editor")]
mod editor;
mod element;
mod entity;
mod event;
mod field;
mod find;
mod font;
mod foreground;
mod geometry;
mod global;
mod image;
mod image_renderer;
mod image_resource;
#[cfg(feature = "inspector")]
mod inspector;
mod keyboard;
mod keymap;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
mod macos_application;
#[cfg(target_os = "macos")]
mod macos_clipboard;
#[cfg(target_os = "macos")]
mod macos_key_equivalents;
#[cfg(target_os = "macos")]
mod macos_keyboard;
#[cfg(target_os = "macos")]
mod macos_menu;
#[cfg(target_os = "macos")]
mod macos_shell;
#[cfg(feature = "markdown")]
mod markdown;
mod menu;
mod menubar;
mod metrics;
#[cfg(target_os = "macos")]
mod native_view;
mod navigation_menu;
mod number_field;
mod otp_field;
mod paint_order;
mod path;
mod path_renderer;
mod picker;
mod platform;
mod popover;
mod popover_component;
mod popover_menu;
mod preview_card;
mod progress;
mod renderer;
mod router;
mod runtime;
mod scene;
mod scheduler;
mod scroll_area;
mod select;
mod selection_control;
mod separator;
mod slider;
mod spell;
mod splitter;
mod spring;
mod state_accessor;
mod styled_text;
mod svg;
mod svg_renderer;
#[cfg(all(target_os = "macos", feature = "swift-ui"))]
mod swift_ui;
#[cfg(feature = "editor")]
mod syntax;
mod table;
mod tabs;
#[cfg(feature = "terminal")]
mod terminal;
#[cfg(feature = "terminal")]
mod terminal_process;
mod text_input;
#[cfg(feature = "text-input-decorations")]
mod text_input_decorations;
#[cfg(feature = "text-input-decorations")]
pub use text_input_decorations::{TextInputGutter, TextInputIndentation};
mod component_extension;
pub use component_extension::ExtensionComponent;
mod toast;
mod toggle;
mod toolbar;
mod tooltip;
mod transition;
mod tree;
mod ui_tree;
mod undo;
mod virtual_list;
#[cfg(any(test, feature = "test-support"))]
mod visual_test;

pub use action::{Action, ActionListener, AnyAction, MAX_ACTION_LISTENERS_PER_ELEMENT};
pub use animated_image::{
    AnimatedImage, AnimatedImageFrame, AnimationRepeat, MAX_ANIMATED_IMAGE_BYTES,
    MAX_ANIMATION_FRAMES, MIN_ANIMATION_FRAME_DURATION,
};
pub use animation::{
    Animation, AnimationExt, AnimationPhase, Interpolate, MAX_ANIMATION_FPS, MAX_ANIMATION_STAGES,
    MAX_DECLARATIVE_ANIMATIONS_PER_WINDOW, bounce, ease_in_out, ease_out_quint, linear,
    pulsating_between, quadratic,
};
pub use assets::{
    AssetBytes, AssetError, AssetSource, Assets, BundledAssets, FontSource, MAX_ASSET_BYTES,
    MAX_ASSET_LIST_ENTRIES, MAX_ASSET_LIST_PATH_BYTES, MAX_ASSET_PATH_BYTES,
    MAX_BUNDLED_ASSET_BYTES, MAX_BUNDLED_ASSETS, MAX_CUSTOM_FONT_BYTES, MAX_CUSTOM_FONT_FACES,
    MAX_CUSTOM_FONT_FACES_PER_FILE, MAX_CUSTOM_FONT_TOTAL_BYTES, MAX_CUSTOM_FONTS,
};
pub use autocomplete::{
    AutocompleteListState, AutocompleteOptionState, AutocompletePopoverLayout,
    AutocompleteSelectionBehavior, AutocompleteState, MAX_AUTOCOMPLETE_VALUE_BYTES,
    MAX_AUTOCOMPLETE_VISIBLE_ROWS,
};
pub use avatar::{Avatar, AvatarLoadingStatus, AvatarState, MAX_AVATAR_FALLBACK_DELAY, avatar};
pub use background::{BackgroundTaskError, MAX_PENDING_BACKGROUND_TASKS, TaskSpawnError};
pub use calendar::{
    CALENDAR_WEEK_DAYS, Calendar, CalendarNextDay, CalendarNextMonth, CalendarNextWeek,
    CalendarNextYear, CalendarPreviousDay, CalendarPreviousMonth, CalendarPreviousWeek,
    CalendarPreviousYear, CalendarSelect, CalendarState, CalendarWeekEnd, CalendarWeekStart,
    CalendarWeekday, MAX_CALENDAR_WEEKS, calendar, calendar_key_bindings,
};
pub use canvas::Canvas;
pub use checkbox_group::{
    CheckboxGroup, CheckboxGroupState, MAX_CHECKBOX_GROUP_VALUES, checkbox_group,
};
pub use clipboard::{
    ClipboardBookmark, ClipboardData, ClipboardEntry, ClipboardError, ClipboardImage,
    ClipboardImageFormat, ClipboardItem, ClipboardString, ExternalPaths,
    MAX_CLIPBOARD_BOOKMARK_TITLE_BYTES, MAX_CLIPBOARD_BOOKMARK_URL_BYTES, MAX_CLIPBOARD_DATA_BYTES,
    MAX_CLIPBOARD_DECODED_IMAGE_BYTES, MAX_CLIPBOARD_ENTRIES, MAX_CLIPBOARD_IMAGE_BYTES,
    MAX_CLIPBOARD_METADATA_BYTES, MAX_CLIPBOARD_MIME_TYPE_BYTES, MAX_CLIPBOARD_PATH_BYTES,
    MAX_CLIPBOARD_PATHS, MAX_CLIPBOARD_TEXT_BYTES, MAX_CLIPBOARD_TOTAL_PATH_BYTES,
};
#[cfg(feature = "editor")]
pub use code_block::{
    CodeBlock, CodeBlockStyle, CodeBlockUpdate, MAX_CODE_BLOCK_LINES, MAX_CODE_BLOCK_SOURCE_BYTES,
};
pub use color::Color;
pub use combobox::{
    ComboboxConfirm, ComboboxFirst, ComboboxLast, ComboboxNext, ComboboxPageDown, ComboboxPageUp,
    ComboboxPrevious, combobox_key_bindings, select_key_bindings,
};
pub use constrained_combobox::{
    ComboboxItemPartState, ComboboxListState, ComboboxOptionState, ComboboxPartState,
    ComboboxPopoverLayout, ComboboxState, MAX_COMBOBOX_VALUES, MAX_COMBOBOX_VISIBLE_ROWS,
};
pub use context_menu::{
    CONTEXT_MENU_SUBMENU_AIM_DELAY, CONTEXT_MENU_SUBMENU_HOVER_DELAY, ContextMenuLayout,
    ContextMenuState,
};
pub use cursor::CursorStyle;
pub use custom_shader::{
    CUSTOM_SHADER_PARAMETER_VECTORS, CustomShader, CustomShaderError,
    MAX_CUSTOM_SHADER_SOURCE_BYTES, ShaderParameters,
};
pub use custom_shader_renderer::{
    MAX_CUSTOM_SHADER_INSTANCES_PER_FRAME, MAX_CUSTOM_SHADER_PIPELINES_PER_WINDOW,
};
pub use date_field::{
    CivilDate, CivilPeriod, CivilTime, DateField, DateFieldClearSegment, DateFieldDecrement,
    DateFieldIncrement, DateFieldNextSegment, DateFieldOrder, DateFieldPreviousSegment,
    DateFieldSegment, DateFieldSegmentMaximum, DateFieldSegmentMinimum, DateFieldState,
    DateSegment, MAX_CIVIL_YEAR, MAX_DATE_FIELD_PLACEHOLDER_BYTES, MIN_CIVIL_YEAR, TimeField,
    TimeFieldSegment, TimeFieldState, TimeSegment, TimeSegments, date_field,
    date_field_key_bindings, time_field, time_field_key_bindings,
};
pub use dialog::{Dialog, DialogKind, DialogState, MAX_DIALOG_TRANSITION};
#[cfg(feature = "editor")]
pub use diff_view::{
    DiffDocument, DiffFile, DiffFileKind, DiffHunk, DiffIndicators, DiffLayout, DiffLine,
    DiffLineKind, DiffTheme, DiffView, DiffViewStyle, MAX_DIFF_FILES, MAX_DIFF_LINES,
    MAX_DIFF_SOURCE_BYTES,
};
pub use disclosure::{
    Accordion, AccordionItem, AccordionItemState, AccordionState, AccordionStateError, Collapsible,
    CollapsibleState, MAX_ACCORDION_OPEN_ITEMS,
};
pub use display::{
    Display, DisplayError, DisplayId, DisplayUuid, Displays, MAX_DISPLAY_NAME_BYTES, MAX_DISPLAYS,
};
pub use display::{DisplayEvent, MAX_DISPLAY_COLOR_DEPTH, MAX_DISPLAY_EVENTS};
pub use drawer::{
    DEFAULT_DRAWER_DISMISS_VELOCITY, Drawer, DrawerGesture, DrawerModality, DrawerState,
    MAX_DRAWER_DISMISS_VELOCITY, MAX_DRAWER_SNAP_POINTS, MAX_NESTED_DRAWERS, SwipeDirection,
    drawer_popup,
};
#[cfg(feature = "editor")]
pub use editor::{Editor, EditorStyle, EditorUpdate, MAX_EDITOR_SOURCE_BYTES};
pub use element::{
    AccessibilityAutoComplete, AccessibilityLive, AccessibilityOrientation, AccessibilityPopover,
    AccessibilityRole, AccessibilitySortDirection, AccessibilityValueRange, AnchorAlign,
    AnchorPlacement, AnchorPlacementHandle, AnchorSide, AppRegion, BackgroundImage,
    BackgroundPosition, BackgroundRepeat, BackgroundSize, Element, ElementId, ElementStateStyle,
    ElementUpdate, FocusHandle, GridTrack, IntoElement, LayoutBoundsHandle,
    MAX_BACKGROUND_IMAGE_TILES, MAX_BOX_SHADOWS_PER_ELEMENT, MAX_CONTAINER_QUERIES_PER_WINDOW,
    MAX_CONTAINER_QUERY_DEPTH, MAX_CORNER_RADIUS, MAX_GRID_TRACKS, MAX_GROUP_STYLES_PER_ELEMENT,
    MAX_HOVER_GROUP_NAME_BYTES, MAX_KEY_LISTENERS_PER_ELEMENT, MAX_MOUSE_LISTENERS_PER_ELEMENT,
    MAX_OUTLINE_OFFSET, MAX_OUTLINE_WIDTH, Outline, ResolvedAnchorPlacement, ToggleState,
    UserSelect, Visibility, anchor_placement, button, canvas, container_query, custom_shader, div,
    form, img, overlay, path, styled_text_area, styled_text_input, submit_button, svg, text,
    text_area, text_input,
};
#[cfg(target_os = "macos")]
pub use element::{native_view, native_view_with_outset};
pub use entity::{
    Entity, EntityId, EventEmitter, MAX_ENTITY_EVENT_DELIVERIES_PER_TURN,
    MAX_ENTITY_EVENTS_PER_CALLBACK, MAX_ENTITY_NOTIFICATIONS_PER_EVENT,
    MAX_ENTITY_SUBSCRIPTIONS_PER_WINDOW, MAX_OBSERVED_ENTITIES_PER_WINDOW,
    MAX_PENDING_ENTITY_EVENTS, Subscription, WeakEntity,
};
pub use event::{
    ContextMenuEvent, DispatchPhase, DragOrigin, DragStartEvent, DropEvent, DroppedFiles,
    DroppedText, DroppedUrl, Event, EventContext, ExternalDragEndEvent, ExternalDragOperation,
    ExternalDragPayload, ExternalDragText, ExternalDragUrl, ExternalDragUrlError, FileDragPaths,
    FormField, FormSubmitEvent, GesturePhase, Key, KeyDownEvent, KeyUpEvent,
    MAX_ACTIVE_TOUCHES_PER_WINDOW, MAX_DROPPED_FILES, MAX_EXTERNAL_DRAG_FILES,
    MAX_EXTERNAL_DRAG_PATH_BYTES, MAX_EXTERNAL_DRAG_TEXT_BYTES, MAX_EXTERNAL_DRAG_TOTAL_PATH_BYTES,
    MAX_EXTERNAL_DRAG_URL_BYTES, MAX_FORM_FIELDS, MAX_FORM_SUBMISSIONS_PER_EVENT,
    MAX_NATIVE_POPUP_MENUS_PER_EVENT, MAX_PENDING_TARGETED_ACTIONS, MAX_PINCH_DELTA_PER_EVENT,
    MAX_ROTATION_DEGREES_PER_EVENT, MAX_SCROLL_LINES_PER_EVENT, MAX_SCROLL_PIXELS_PER_EVENT,
    MAX_TARGETED_ACTIONS_PER_EVENT, MAX_TOUCH_COORDINATE, MAX_VALIDATION_ISSUES,
    MAX_VALIDATION_MESSAGE_BYTES, Modifiers, MouseButton, MouseDownEvent, MouseExitEvent,
    MouseMoveEvent, MousePressureEvent, MouseUpEvent, PinchEvent, PointerEvent, PointerPhase,
    PressureStage, RotationEvent, ScrollDelta, ScrollWheelEvent, SmartMagnifyEvent, TouchEvent,
    TouchId, TouchPhase, ValidationIssue, ValidationReport,
};
pub use field::{
    Field, FieldState, FieldValidationMode, FieldValidationTrigger, Fieldset,
    MAX_FIELD_VALIDATION_DEBOUNCE,
};
pub use find::{
    FIND_BAR_KEY_CONTEXT, FindBar, FindClose, FindNext, FindOptions, FindPrevious, FindReplace,
    FindReplaceAll, FindState, MAX_FIND_MATCHES, MAX_FIND_QUERY_BYTES, MAX_FIND_REPLACEMENT_BYTES,
    find_bar_key_bindings,
};
pub use font::{
    Font, FontFallbacks, FontFamily, FontFeature, FontFeatureTag, FontFeatureTagError,
    FontFeatures, MAX_FONT_FALLBACKS, MAX_FONT_FAMILY_BYTES, MAX_FONT_FEATURES, font,
};
pub use foreground::{
    AsyncContextError, AsyncViewContext, AsyncViewUpdate, ForegroundTaskSpawnError,
    ForegroundTimer, MAX_FOREGROUND_POLLS_PER_TURN, MAX_FOREGROUND_TASKS_PER_APPLICATION,
    MAX_FOREGROUND_TASKS_PER_WINDOW, MAX_FOREGROUND_TIMERS_PER_APPLICATION,
    MAX_FOREGROUND_TIMERS_PER_TASK, MAX_FOREGROUND_UPDATES_PER_TASK, Task,
};
pub use geometry::{Insets, Point, Rect, Size, Vector};
pub use global::{
    Global, MAX_APPLICATION_GLOBALS, MAX_GLOBAL_NOTIFICATIONS_PER_EVENT,
    MAX_GLOBAL_OBSERVER_DELIVERIES_PER_TURN, MAX_GLOBAL_SUBSCRIPTIONS_PER_WINDOW,
    MAX_OBSERVED_GLOBALS_PER_WINDOW, MAX_PENDING_GLOBAL_NOTIFICATIONS,
};
pub use glyphon::{Style as FontStyle, Weight as FontWeight};
pub use image::{
    DEFAULT_SYSTEM_IMAGE_POINT_SIZE, DEFAULT_SYSTEM_IMAGE_SCALE, MAX_IMAGE_DATA_URL_BYTES,
    MAX_IMAGE_REPRESENTATION_SCALE, MAX_IMAGE_REPRESENTATIONS, MIN_IMAGE_REPRESENTATION_SCALE,
};
pub use image::{
    Image, ImageError, ImageResource, ImageSource, MAX_DECODED_IMAGE_BYTES,
    MAX_ENCODED_IMAGE_BYTES, MAX_IMAGE_DIMENSION, ObjectFit, is_template_image_path,
};
pub use image_renderer::{MAX_GPU_IMAGE_CACHE_BYTES, MAX_GPU_IMAGE_CACHE_ENTRIES};
pub use image_resource::{
    IMAGE_LOADING_DELAY, ImageResourceStats, MAX_CPU_IMAGE_CACHE_BYTES,
    MAX_IMAGE_RESOURCE_CACHE_ENTRIES, MAX_PENDING_IMAGE_LOADS,
};
#[cfg(feature = "inspector")]
pub use inspector::{
    INSPECTOR_PANEL_WIDTH, InspectorAccessibility, InspectorElementKind, InspectorFrameDamage,
    InspectorHitRegion, InspectorMode, InspectorNode, InspectorSnapshot, MAX_INSPECTOR_NODES,
    MAX_INSPECTOR_TEXT_BYTES,
};
pub use keyboard::{
    KeyboardLayout, KeyboardLayoutError, MAX_KEYBOARD_LAYOUT_ID_BYTES,
    MAX_KEYBOARD_LAYOUT_NAME_BYTES,
};
pub use keymap::{Accelerator, MAX_ACCELERATOR_BYTES};
pub use keymap::{
    ContextPredicate, KeyBinding, KeyContext, Keymap, KeymapError, KeymapMatch, Keystroke,
};
#[cfg(feature = "markdown")]
pub use markdown::{
    MAX_MARKDOWN_BLOCKS, MAX_MARKDOWN_NESTING_DEPTH, MAX_MARKDOWN_SOURCE_BYTES, Markdown,
    MarkdownBlock, MarkdownCodeBlock, MarkdownInlineRun, MarkdownInlineStyle, MarkdownListItem,
    MarkdownStyle, MarkdownTableAlign, MarkdownUpdate,
};
pub use menu::{
    MAX_NATIVE_MENU_DEPTH, MAX_NATIVE_MENU_ITEMS, MAX_NATIVE_MENU_TEXT_BYTES,
    MAX_NATIVE_MENU_TOTAL_TEXT_BYTES, Menu, MenuError, MenuIcon, MenuItem, MenuItemMark, OsAction,
    OsMenu, SystemMenuType,
};
pub use menubar::{
    MAX_MENUBAR_MENUS, Menubar, MenubarClose, MenubarFirst, MenubarItem, MenubarLast, MenubarNext,
    MenubarOpen, MenubarPrevious, MenubarState, menubar, menubar_key_bindings,
};
pub use metrics::{FrameMetrics, PipelineMetrics, RenderStats};

#[cfg(test)]
mod allocation_tests;
#[cfg(target_os = "macos")]
pub use native_view::{MAX_NATIVE_VIEW_OUTSET, MacNativeView};
pub use navigation_menu::{
    DEFAULT_NAVIGATION_MENU_CLOSE_DELAY, DEFAULT_NAVIGATION_MENU_DELAY, MAX_NAVIGATION_MENU_DELAY,
    MAX_NAVIGATION_MENU_ITEMS, NAVIGATION_MENU_HORIZONTAL_KEY_CONTEXT,
    NAVIGATION_MENU_VERTICAL_KEY_CONTEXT, NavigationMenu, NavigationMenuActivationDirection,
    NavigationMenuClose, NavigationMenuEntry, NavigationMenuFirst, NavigationMenuItem,
    NavigationMenuLast, NavigationMenuNext, NavigationMenuOrientation, NavigationMenuPrevious,
    NavigationMenuState, navigation_menu, navigation_menu_key_bindings,
};
pub use number_field::{
    DEFAULT_NUMBER_FIELD_SCRUB_SENSITIVITY, MAX_NUMBER_FIELD_PRECISION,
    MAX_NUMBER_FIELD_SCRUB_SENSITIVITY, MAX_NUMBER_FIELD_TEXT_BYTES, NUMBER_FIELD_REPEAT_DELAY,
    NUMBER_FIELD_REPEAT_INTERVAL, NumberField, NumberFieldFormat, NumberFieldPartState,
    NumberFieldScrubDirection, NumberFieldState, NumberFieldStepSize, number_field,
    number_field_root,
};
pub use otp_field::{
    MAX_OTP_LENGTH, OTP_FIELD_KEY_CONTEXT, OtpField, OtpFieldBackspace, OtpFieldDelete,
    OtpFieldFirst, OtpFieldLast, OtpFieldNext, OtpFieldPrevious, OtpFieldState, OtpValidationType,
    otp_field, otp_field_key_bindings,
};
pub use path::{
    Background, FillOptions, FillRule, GradientColorSpace, LineCap, LineJoin, LinearColorStop,
    LinearGradient, MAX_PATH_BYTES, MAX_PATH_COMMANDS, MAX_PATH_COORDINATE, MAX_PATH_DASH_SEGMENTS,
    MAX_PATH_VERTICES, Path, PathBuilder, PathError, PathStyle, StrokeOptions, linear_color_stop,
    linear_gradient,
};
pub use path::{
    ColorStops, Gradient, GradientAngle, GradientCenter, GradientDirection, GradientKind,
    MAX_GRADIENT_STOPS, RadialGradientExtent, RadialGradientShape,
};
pub use path_renderer::{MAX_GPU_PATH_VERTICES, MAX_GPU_PATHS_PER_FRAME};
pub use picker::{
    MAX_PICKER_ITEM_TEXT_BYTES, MAX_PICKER_ITEMS, MAX_PICKER_QUERY_BYTES,
    MAX_PICKER_QUERY_GRAPHEMES, MAX_PICKER_RESULTS, MAX_PICKER_TEXT_BYTES, PickerConfirm,
    PickerError, PickerFilter, PickerFilterMode, PickerFirst, PickerItem, PickerLast, PickerLayout,
    PickerMatch, PickerNext, PickerPageDown, PickerPageUp, PickerPrevious, PickerState,
    picker_key_bindings,
};
// Application-shell services: activation policy, Dock attention, and process packaging.
pub use platform::{
    AboutPanelOptions, FileDialogFilter, FileIconResponse, FileIconSize,
    MAX_ABOUT_PANEL_TEXT_BYTES, MAX_ACTIVE_PLATFORM_DIALOGS, MAX_DOCK_BADGE_BYTES,
    MAX_FILE_DIALOG_FILTER_BYTES, MAX_FILE_DIALOG_FILTER_EXTENSIONS, MAX_FILE_DIALOG_FILTERS,
    MAX_OPEN_URLS, MAX_OPEN_URLS_TOTAL_BYTES, MAX_PENDING_NOTIFICATION_PERMISSION_REQUESTS,
    MAX_PENDING_PLATFORM_REQUESTS, MAX_PENDING_SYSTEM_NOTIFICATIONS, MAX_PLATFORM_PATH_BYTES,
    MAX_PLATFORM_REQUESTS_PER_EVENT, MAX_PLATFORM_TEXT_BYTES, MAX_PLATFORM_URL_BYTES,
    MAX_PROMPT_BUTTON_BYTES, MAX_PROMPT_BUTTONS, MAX_SELECTED_PATHS,
    MAX_SELECTED_PATHS_TOTAL_BYTES, MAX_SYSTEM_NOTIFICATION_ACTION_BYTES,
    MAX_SYSTEM_NOTIFICATION_ACTIONS, MAX_SYSTEM_NOTIFICATION_ATTACHMENTS,
    MAX_SYSTEM_NOTIFICATION_BODY_BYTES, MAX_SYSTEM_NOTIFICATION_CATEGORIES,
    MAX_SYSTEM_NOTIFICATION_ICON_BYTES, MAX_SYSTEM_NOTIFICATION_OPTION_BYTES,
    MAX_SYSTEM_NOTIFICATION_REPLY_BYTES, MAX_SYSTEM_NOTIFICATION_TAG_BYTES,
    MAX_SYSTEM_NOTIFICATION_TITLE_BYTES, MAX_TASKBAR_OVERLAY_DESCRIPTION_BYTES,
    MAX_USER_TASK_TEXT_BYTES, MAX_USER_TASKS, NotificationPermissionResponse,
    NotificationPermissionStatus, OpenUrls, PathPromptOptions, PathPromptResponse, PlatformError,
    PlatformResponse, PromptButton, PromptLevel, SavePathOptions, SavePathResponse, ShellResponse,
    SystemNotification, SystemNotificationAction, SystemNotificationActionKind,
    SystemNotificationAttachment, SystemNotificationResponse, SystemNotificationSound, UserTask,
};
pub use platform::{
    ActivationPolicy, ApplicationsFolderSupport, ColorPanelMode, DockAttention,
    DockAttentionRequest, MAX_BIOMETRIC_REASON_BYTES, MAX_FILE_PREVIEW_NAME_BYTES,
    MAX_MESSAGE_BOX_CHECKBOX_BYTES, MAX_SHARE_ITEM_TEXT_BYTES, MAX_SHARE_ITEMS, MessageBoxCheckbox,
    MessageBoxOptions, MessageBoxResponse, MessageBoxResponseFuture, ShareItem,
    is_application_packaged,
};
pub use popover::{
    MAX_GRABBING_POPOVERS, PopoverAnchor, PopoverConstraintAdjustment, PopoverGravity,
    PopoverOptions,
};
pub use popover_component::{
    DEFAULT_POPOVER_HOVER_DELAY, MAX_POPOVER_ALIGN_OFFSET, MAX_POPOVER_ARROW_SIZE,
    MAX_POPOVER_COLLISION_PADDING, MAX_POPOVER_HOVER_DELAY, MAX_POPOVER_SIDE_OFFSET, Popover,
    PopoverHoverState, PopoverKind, PopoverPartState, SystemPopover,
};
pub use popover_menu::{
    DEFAULT_MENU_CLOSE_DELAY, DEFAULT_MENU_HOVER_DELAY, MAX_POPOVER_MENU_DEPTH,
    MAX_POPOVER_MENU_ITEM_TEXT_BYTES, MAX_POPOVER_MENU_ITEMS, MAX_POPOVER_MENU_LINK_BYTES,
    MAX_POPOVER_MENU_TEXT_BYTES, MAX_POPOVER_MENU_TYPEAHEAD_BYTES, MenuCloseParent,
    MenuItemPartState, MenuOrientation, MenuPartState, MenuState, OpenMenuLink,
    POPOVER_MENU_HORIZONTAL_KEY_CONTEXT, POPOVER_MENU_KEY_CONTEXT, POPOVER_MENU_TYPEAHEAD_TIMEOUT,
    PopoverMenu, PopoverMenuActivate, PopoverMenuActivation, PopoverMenuClose, PopoverMenuError,
    PopoverMenuFirst, PopoverMenuItem, PopoverMenuItemKind, PopoverMenuItemState, PopoverMenuLast,
    PopoverMenuNext, PopoverMenuOpenSubmenu, PopoverMenuPrevious,
    popover_menu_horizontal_key_bindings, popover_menu_key_bindings,
};
pub use preview_card::{
    DEFAULT_PREVIEW_CARD_CLOSE_DELAY, DEFAULT_PREVIEW_CARD_DELAY, MAX_PREVIEW_CARD_DELAY,
    PreviewCard, PreviewCardState, preview_card_trigger,
};
pub use progress::{
    Meter, Progress, ProgressPartState, ProgressStatus, ValueFormat, meter, progress,
};
pub use quickgui_system::{
    AppInfo, AppPaths, BatteryState, BatteryStatus, ColorScheme, IdleState,
    MAX_APP_IDENTIFIER_BYTES, MAX_APP_NAME_BYTES, MAX_APP_VERSION_BYTES, MAX_IDLE_THRESHOLD,
    MAX_POWER_ASSERTION_REASON_BYTES, MAX_PREFERRED_LANGUAGES, MAX_RELAUNCH_ARGUMENT_BYTES,
    MAX_RELAUNCH_ARGUMENTS, MAX_RELAUNCH_VALUE_BYTES, MAX_SYSTEM_LOCALE_BYTES,
    MAX_SYSTEM_LOCALES_TOTAL_BYTES, MAX_SYSTEM_TEXT_BYTES, OperatingSystem, OperatingSystemFamily,
    PermissionKind, PermissionManager, PermissionStatus, PowerAssertion, PowerAssertionKind,
    PowerMonitor, PowerSource, PowerState, RelaunchOptions, RelaunchRequest, RelaunchedProcess,
    SessionState, SystemBitness, SystemColor, SystemColorRole, SystemInfo, SystemIntegrationError,
    SystemPreferences, ThermalState,
};
#[cfg(feature = "updater")]
pub use quickgui_system::{
    AvailableUpdate, DEFAULT_MAX_EXPANDED_UPDATE_BYTES, DEFAULT_MAX_UPDATE_BYTES, InstalledUpdate,
    MAX_UPDATE_ARCHIVE_ENTRIES, MAX_UPDATE_INSTALLER_ARGUMENT_BYTES,
    MAX_UPDATE_INSTALLER_ARGUMENTS, MAX_UPDATE_MANIFEST_BYTES, MAX_UPDATE_SIGNATURE_BYTES,
    UpdateCancellation, UpdateClient, UpdateInstallDisposition, UpdateInstallOptions,
    UpdateProgress, WindowsUpdateInstallMode, default_update_target,
};
#[cfg(feature = "crash-reporter")]
pub use quickgui_system::{
    BacktracePolicy, CRASH_REPORT_SCHEMA_VERSION, CrashKind, CrashLocation, CrashReport,
    CrashReporter, CrashReporterOptions, DEFAULT_MAX_CRASH_REPORT_BYTES, DEFAULT_MAX_CRASH_REPORTS,
    DEFAULT_WATCHDOG_HANG_THRESHOLD, DEFAULT_WATCHDOG_INTERVAL, MAX_CRASH_BACKTRACE_BYTES,
    MAX_CRASH_EXTRA_PARAMETERS, MAX_CRASH_MESSAGE_BYTES, MAX_CRASH_PARAMETER_KEY_BYTES,
    MAX_CRASH_PARAMETER_VALUE_BYTES, MAX_CRASH_REPORT_BYTES, MAX_CRASH_REPORTS,
    MAX_CRASH_UPLOAD_REPORTS, MAX_WATCHDOG_INTERVAL, MIN_WATCHDOG_INTERVAL, UploadSummary,
    Watchdog, WatchdogOptions,
};
pub use quickgui_system::{
    CpuUsage, CpuUsageSampler, MAX_CPU_SAMPLE_INTERVAL, ProcessMetrics, SystemMemory,
};
// Window lifecycle events, stacking/input policy, and persistable restore geometry.
pub use renderer::MAX_GRADIENTS_PER_FRAME;
pub use router::{
    MAX_ROUTE_DEPTH, MAX_ROUTE_DESTINATION_BYTES, MAX_ROUTE_HISTORY_ENTRIES, MAX_ROUTE_ID_BYTES,
    MAX_ROUTE_PATTERN_BYTES, MAX_ROUTE_QUERY_PAIRS, MAX_ROUTE_SEGMENTS, MAX_ROUTES,
    RouteDefinition, RouteLocation, RouteMatch, RouteParameter, RouteQueryPair, Router,
    RouterError, RouterSnapshot,
};
pub use runtime::{
    App, AppError, Application, ClickListener, ContextMenuListener, CursorGrabMode,
    DesktopIntegrationSupport, DismissListener, Drag, DragListener, DropListener,
    FormInvalidListener, FormSubmitListener, GlobalShortcutEvent, HoverListener, InputListener,
    KeyDownListener, KeyUpListener, MAX_ACTION_LISTENERS_PER_WINDOW, MAX_APPLICATION_WINDOWS,
    MAX_CHILD_WINDOW_CLOSE_LISTENERS_PER_WINDOW, MAX_GLOBAL_SHORTCUT_ACCELERATOR_BYTES,
    MAX_GLOBAL_SHORTCUTS, MAX_KEY_LISTENERS_PER_WINDOW, MAX_MOUSE_LISTENERS_PER_WINDOW,
    MAX_PENDING_NATIVE_POPUP_MENUS, MAX_PENDING_WINDOW_COMMANDS, MAX_SYSTEM_WINDOW_TABS,
    MAX_TRAY_ENCODED_ICON_BYTES, MAX_TRAY_ICON_DIMENSION, MAX_TRAY_ICONS, MAX_TRAY_MENU_DEPTH,
    MAX_TRAY_MENU_ITEMS, MAX_TRAY_TEXT_BYTES, MAX_WINDOW_COMMANDS_PER_EVENT,
    MAX_WINDOW_DOCUMENT_PATH_BYTES, MAX_WINDOW_LOGICAL_COORDINATE, MAX_WINDOW_LOGICAL_DIMENSION,
    MAX_WINDOW_TABBING_IDENTIFIER_BYTES, MAX_WINDOW_TITLE_BYTES, MacOsVibrancy,
    MacOsVisualEffectState, MouseDownListener, MouseExitListener, MouseMoveListener,
    MousePressureListener, MouseUpListener, PerformanceProfile, PinchListener, PointerListener,
    PowerEvent, QuitMode, QuitReason, QuitRequest, RotationListener, ScrollWheelListener,
    SmartMagnifyListener, SubmitListener, TaskbarProgressState, TitleBarStyle, TouchListener,
    TrayEvent, TrayEventKind, TrayIconImage, TrayIconOptions, TrayMenuItem, TrayMouseButton, View,
    ViewContext, WindowAppearance, WindowBackgroundAppearance, WindowBounds, WindowCommandError,
    WindowHandle, WindowInvalidator, WindowKind, WindowLevel, WindowOptions, WindowRegistry,
    WindowState, WindowTabState,
};
#[cfg(not(target_arch = "wasm32"))]
pub use runtime::{AppRunStatus, AppRunner, AppRunnerWaker};
#[cfg(any(
    target_os = "macos",
    target_os = "windows",
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "openbsd",
    target_os = "netbsd"
))]
pub use runtime::{
    MAX_DEEP_LINK_ARGUMENTS, MAX_SECOND_INSTANCE_ARGUMENTS, MAX_SECOND_INSTANCE_MESSAGE_BYTES,
    MAX_SINGLE_INSTANCE_IDENTIFIER_BYTES, SecondInstanceEvent, SingleInstanceError,
};
#[cfg(any(test, feature = "test-support"))]
pub use runtime::{
    MAX_TEST_EFFECT_TURNS, TestAppContext, TestAppError, TestApplicationShell, TestWindowHandle,
    VisualTestContext,
};
pub use runtime::{MAX_WINDOW_ASPECT_RATIO, ResolvedWindowRestoreState, WindowRestoreState};
pub use scene::{
    BorderStyle, BoxShadow, ColorMatrix, Corners, CustomShaderPrimitive, Filter, Filters,
    ImagePrimitive, MAX_BOX_SHADOW_BLUR_RADIUS, MAX_BOX_SHADOW_EXTENT, MAX_FILTERS_PER_ELEMENT,
    PathPrimitive, Quad, Scene, ScenePlane, Shadow, SvgPrimitive, TextAlign, TextId, TextOverflow,
    TextRun, TextShaping, TextStyle, TextWrap, WhiteSpace,
};
// Compositing layers: transforms, subtree filters, backdrop effects, and blend modes.
pub use scene::{
    BlendMode, DropShadow, LayerEffects, MAX_BLUR_RADIUS, MAX_LAYER_DEPTH, MAX_LAYER_TEXTURE_BYTES,
    MAX_LAYERS_PER_FRAME, Transform2D,
};
// Direction-relative layout, sticky positioning, scroll snapping, and text styling additions.
pub use element::{Direction, SnapAlign, SnapStrictness, StickyInsets};
pub use scene::{
    Hyphens, MAX_TEXT_SHADOW_SAMPLES, MAX_TEXT_SPACING, OverflowWrap, TextDirection, TextShadow,
    TextTransform, WordBreak,
};
pub use scroll_area::{
    DEFAULT_SCROLL_AREA_LINE_HEIGHT, DEFAULT_SCROLL_AREA_OVERFLOW_THRESHOLD,
    MAX_SCROLL_AREA_OVERFLOW_THRESHOLD, MIN_SCROLL_AREA_THUMB_LENGTH, ScrollArea,
    ScrollAreaOrientation, ScrollAreaState, ScrollAreaStyleState, scroll_area_viewport,
};
pub use select::{
    DEFAULT_SELECT_VALUE_SEPARATOR, MAX_SELECT_TYPEAHEAD_BYTES, MAX_SELECT_VALUES,
    MAX_SELECT_VISIBLE_ROWS, SELECT_SCROLL_ARROW_INTERVAL, SELECT_TYPEAHEAD_TIMEOUT,
    SelectListState, SelectOptionState, SelectPartState, SelectPopoverLayout, SelectPopupParts,
    SelectState,
};
pub use selection_control::{
    Checkbox, Radio, RadioGroup, Switch, checkbox, radio, radio_group, switch,
};
pub use separator::{Separator, SeparatorOrientation, separator};
pub use slider::{
    MAX_SLIDER_THUMBS, Slider, SliderDecrement, SliderIncrement, SliderLargeDecrement,
    SliderLargeIncrement, SliderMaximum, SliderMinimum, SliderOrientation, SliderPointerChange,
    SliderState, SliderThumb, SliderThumbAlignment, SliderThumbState, slider, slider_key_bindings,
};
#[cfg(any(test, feature = "test-support"))]
pub use spell::TestSpellCheckProvider;
pub use spell::{
    Autocorrection, IgnoreWord, LearnWord, LookUpSelection, MAX_DEFINITION_LOOKUP_BYTES,
    MAX_MISSPELLED_RANGES, MAX_SPELL_GUESSES, MAX_SPELL_WORD_BYTES, MAX_SPELLCHECK_BYTES,
    Misspelling, MisspellingKind, NoSpellCheckProvider, ReplaceWord, SPELL_CHECK_SETTLE_DELAY,
    SPELLING_MENU_ID_PREFIX, SpellCheckProvider, SpellDocumentTag, SpellingMenuLabels,
    TextCheckingOverrides, TextCheckingPolicy, TextServiceError, TextSubstitution,
    clear_spell_check_provider, default_text_checking, grammar_highlight_style,
    has_spell_check_provider, misspelling_highlight_style, set_default_text_checking,
    set_grammar_highlight_style, set_misspelling_highlight_style, set_shared_spell_check_provider,
    set_spell_check_provider, show_definition_for, spell_check_provider, spelling_menu_items,
    word_range_at,
};
pub use splitter::{
    MAX_SPLITTER_PANES, Splitter, SplitterCollapse, SplitterDecrease, SplitterHandle,
    SplitterIncrease, SplitterMaximum, SplitterMinimum, SplitterOrientation, SplitterPane,
    SplitterState, splitter_key_bindings,
};
pub use spring::{SpringAnimation, SpringConfig, SpringPlayback, SpringState, SpringTarget};
pub use state_accessor::StateAccessor;
pub use styled_text::{
    HighlightStyle, MAX_HIGHLIGHT_FONT_FAMILY_BYTES, MAX_TEXT_HIGHLIGHTS, StyledText,
    TextHighlight, TextUnderline, styled_text,
};
pub use svg::{
    MAX_SVG_RASTER_DIMENSION, MAX_SVG_RASTER_PIXELS, MAX_SVG_SOURCE_BYTES, Svg, SvgError,
    SvgTransform,
};
pub use svg_renderer::{MAX_GPU_SVG_CACHE_BYTES, MAX_GPU_SVG_CACHE_ENTRIES};
#[cfg(all(target_os = "macos", feature = "swift-ui"))]
pub use swift_ui::{
    MacEmbeddedView, MacSwiftUiHost, SwiftUiButton, SwiftUiButtonBorderShape, SwiftUiButtonRole,
    SwiftUiButtonStyle, SwiftUiColorPicker, SwiftUiControlSize, SwiftUiDatePicker,
    SwiftUiDatePickerComponents, SwiftUiDatePickerStyle, SwiftUiElement, SwiftUiGauge,
    SwiftUiGaugeStyle, SwiftUiLabelStyle, SwiftUiModifier, SwiftUiPicker, SwiftUiPickerOption,
    SwiftUiPickerStyle, SwiftUiPopover, SwiftUiPopoverArrowEdge, SwiftUiPopoverAttachmentAnchor,
    SwiftUiProgressView, SwiftUiQuickGuiHost, SwiftUiSegmentedControl, SwiftUiSegmentedTabs,
    SwiftUiSlider, SwiftUiStepper, SwiftUiTextField, SwiftUiToggle,
};
#[cfg(feature = "editor")]
pub use syntax::{
    MAX_SYNTAX_HIGHLIGHTS, MAX_SYNTAX_LINE_BYTES, MAX_SYNTAX_SOURCE_BYTES, SyntaxLanguage,
    SyntaxTheme, highlight_syntax,
};
pub use table::{
    MAX_TABLE_COLUMN_WIDTH, MAX_TABLE_COLUMNS, MAX_TABLE_ROWS, MAX_TABLE_SELECTION_RANGES,
    MIN_TABLE_COLUMN_WIDTH, TABLE_COLUMN_RESIZE_STEP, TableActivate, TableCancelEdit,
    TableCellPosition, TableCellState, TableColumn, TableColumnAlign, TableCommitEdit,
    TableEditEnded, TableExtendSelectionDown, TableExtendSelectionUp, TableFirstRow,
    TableHeaderState, TableLastRow, TableLayout, TableMoveColumnLeft, TableMoveColumnRight,
    TableNextColumn, TableNextRow, TablePageDown, TablePageUp, TablePreviousColumn,
    TablePreviousRow, TableResizeColumnLarger, TableResizeColumnSmaller, TableRowState,
    TableSelectAll, TableSelection, TableSelectionChanged, TableSelectionMode, TableSort,
    TableSortDirection, TableState, TableToggleSelection, table_key_bindings,
};
pub use tabs::{
    Tab, TabState, Tabs, TabsActivationDirection, TabsActivationMovement, TabsIndicatorGeometry,
    TabsOrientation, TabsState,
};
#[cfg(feature = "terminal")]
pub use terminal::{
    MAX_TERMINAL_ARGUMENTS, MAX_TERMINAL_ENVIRONMENT, MAX_TERMINAL_SCROLLBACK,
    MAX_TERMINAL_STRING_BYTES, TERMINAL_ANSI_COLOR_COUNT, Terminal, TerminalAgent,
    TerminalAgentStatus, TerminalCursor, TerminalCursorStyle, TerminalError, TerminalOptions,
    TerminalPaddingColor, TerminalScrollState, TerminalSnapshot, TerminalStatus, TerminalStyle,
    TerminalTheme,
};
pub use text_input::CARET_BLINK_HALF_PERIOD;
pub use toast::{
    DEFAULT_TOAST_LIMIT, DEFAULT_TOAST_SWIPE_THRESHOLD, DEFAULT_TOAST_TIMEOUT, MAX_TOAST_DURATION,
    MAX_TOAST_SWIPE_THRESHOLD, MAX_TOAST_TEXT_BYTES, MAX_TOASTS, Toast, ToastEntry, ToastId,
    ToastKind, ToastManager, ToastParts, ToastSwipeChange, ToastSwipeDirection, ToastViewport,
    toast_viewport,
};
pub use toggle::{
    MAX_TOGGLE_GROUP_ITEMS, Toggle, ToggleGroup, ToggleGroupEntry, ToggleGroupFirst,
    ToggleGroupItem, ToggleGroupLast, ToggleGroupNext, ToggleGroupPrevious, ToggleGroupSelection,
    ToggleGroupState, toggle, toggle_group_key_bindings,
};
pub use toolbar::{
    MAX_TOOLBAR_ITEMS, Toolbar, ToolbarEntry, ToolbarFirst, ToolbarItem, ToolbarLast, ToolbarNext,
    ToolbarOrientation, ToolbarPrevious, ToolbarState, toolbar, toolbar_key_bindings,
};
pub use tooltip::{
    DEFAULT_TOOLTIP_DELAY, DEFAULT_TOOLTIP_GROUP_TIMEOUT, DEFAULT_TOOLTIP_HOVER_DELAY,
    MAX_TOOLTIP_COLLISION_PADDING, MAX_TOOLTIP_CONTENT_NODES, MAX_TOOLTIP_DELAY,
    MAX_TOOLTIP_GROUP_TIMEOUT, MAX_TOOLTIP_SIDE_OFFSET, MAX_TOOLTIPS_PER_WINDOW, Tooltip,
    TooltipCursorAxis, TooltipPartState, TooltipProvider, TooltipState,
};
pub use transition::{MAX_STYLE_TRANSITIONS_PER_WINDOW, Transition, TransitionProperties};
pub use tree::{
    DEFAULT_TREE_LOADING_LABEL, MAX_TREE_DEPTH, MAX_TREE_LABEL_BYTES, MAX_TREE_NODES,
    MAX_TREE_TEXT_BYTES, TreeActivate, TreeCollapseOrParent, TreeError, TreeExpandOrChild,
    TreeFirst, TreeLast, TreeLayout, TreeLoadChildren, TreeNext, TreeNode, TreePageDown,
    TreePageUp, TreePrevious, TreeRow, TreeState, TreeToggle, tree_key_bindings,
};
pub use ui_tree::{MAX_FOCUSED_EVENT_PATH, MAX_MOUSE_EVENT_PATH, MAX_STATIC_TEXT_COPY_BYTES};
pub use ui_tree::{
    MAX_SCROLL_SNAP_CONTAINERS_PER_WINDOW, MAX_SCROLL_SNAP_POINTS_PER_WINDOW,
    MAX_STICKY_ELEMENTS_PER_WINDOW,
};
pub use undo::{
    MAX_UNDO_ACTION_NAME_BYTES, MAX_UNDO_ENTRIES, MAX_UNDO_GROUP_ENTRIES, Redo, Undo, UndoEntry,
    UndoManager, UndoableChange, undo_key_bindings,
};
pub use virtual_list::{
    FollowMode, ListAlignment, ListOffset, ListState, ListStateStats, MAX_LIST_ITEM_HEIGHT,
    MAX_LIST_ITEMS, MAX_LIST_OVERSCAN_ITEMS, MAX_MOUNTED_LIST_ITEMS, VirtualList, VisibleRows,
};
#[cfg(any(test, feature = "test-support"))]
pub use visual_test::{
    MAX_VISUAL_TEST_BYTES, MAX_VISUAL_TEST_DIMENSION, VisualDifference, VisualSnapshot,
    VisualTestError, VisualTolerance,
};

/// Run one window with the default application and window configuration.
pub fn run<V: View>(view: V) -> Result<(), AppError> {
    Application::new().run(move |cx| {
        cx.open_window(WindowOptions::default(), view);
    })
}

#[cfg(feature = "file-watcher")]
pub use quickgui_system::{FileWatchEvent, FileWatcher, MAX_WATCH_EVENT_PATHS, MAX_WATCH_ROOTS};

#[doc(hidden)]
pub mod extension_api;

#[doc(hidden)]
pub mod extensions;

#[cfg(feature = "terminal")]
use ui_tree::static_selection_color;

#[cfg(not(target_arch = "wasm32"))]
pub use quickgui_system::{
    AutoStart, AutoStartMode, AutoStartOptions, ProtocolRegistration, ProtocolRegistrationOptions,
    SecureStorage,
};
