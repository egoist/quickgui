use std::{
    any::Any,
    cell::RefCell,
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
    ffi::c_void,
    fmt,
    path::PathBuf,
    rc::Rc,
    sync::{
        Arc, Condvar, LazyLock, Mutex,
        atomic::{AtomicU32, Ordering},
    },
    task::{Context, Poll, Waker},
    time::{Duration, Instant},
};

use quickgui::{
    AccessibilityOrientation, AccessibilityRole, Accordion, AccordionItem, AnchorPlacement,
    AppInfo, AppPaths, AppRegion, AppRunStatus, AppRunner, AppRunnerWaker,
    Application as QuickGuiApplication, BoxShadow, Checkbox, Collapsible, Color, ContextMenuLayout,
    ContextMenuState, CursorGrabMode, CursorStyle, Dialog as CoreDialog, DialogKind, DisplayId,
    Element, ElementId, Field, Fieldset, FollowMode, FontWeight, GridTrack, Image, Insets,
    IntoElement, LayoutBoundsHandle, ListAlignment, ListState, MAX_BOX_SHADOWS_PER_ELEMENT,
    MAX_GROUP_STYLES_PER_ELEMENT, MAX_SLIDER_THUMBS, MAX_SPLITTER_PANES, MAX_TOGGLE_GROUP_ITEMS,
    MAX_TOOLBAR_ITEMS, MacOsVibrancy, MacOsVisualEffectState, Markdown, MarkdownStyle, Meter,
    PerformanceProfile, Point, PointerPhase, Popover, PopoverKind, PopoverMenu,
    PopoverMenuActivation, PopoverMenuItem, PopoverMenuItemKind, PopoverMenuItemState, Progress,
    QuitMode, Radio, RadioGroup, Slider, SliderOrientation, SliderState, Splitter,
    SplitterOrientation, SplitterState, StateAccessor, Svg, Switch, SystemPopover,
    TERMINAL_ANSI_COLOR_COUNT, Tab, Tabs, TaskbarProgressState, Terminal, TerminalOptions,
    TerminalPaddingColor, TerminalStatus, TerminalStyle, TerminalTheme, TextAlign, TitleBarStyle,
    Toggle, ToggleGroup, ToggleGroupItem, ToggleGroupState, ToggleState, Toolbar, ToolbarItem,
    ToolbarOrientation, ToolbarState, Tooltip, Transition, TransitionProperties, View, ViewContext,
    WindowAppearance, WindowBackgroundAppearance, WindowHandle, WindowKind, WindowOptions, button,
    div, svg as svg_element, text, text_area, text_input,
};
use quickgui::{Event, EventContext};
use serde::{Deserialize, Serialize};
// Base UI-aligned compound descriptors. Every type below is a core model the binding only
// translates a declaration into; nothing here re-implements a component.
use quickgui::{
    AnchorAlign, AnchorPlacementHandle, AnchorSide, DEFAULT_TOAST_LIMIT, DEFAULT_TOAST_TIMEOUT,
    DialogState, FieldValidationMode, FieldValidationTrigger, MAX_DIALOG_TRANSITION,
    MAX_FIELD_VALIDATION_DEBOUNCE, MAX_NUMBER_FIELD_SCRUB_SENSITIVITY, MAX_POPOVER_ALIGN_OFFSET,
    MAX_POPOVER_COLLISION_PADDING, MAX_POPOVER_HOVER_DELAY, MAX_POPOVER_SIDE_OFFSET,
    MAX_TOAST_DURATION, MAX_TOAST_SWIPE_THRESHOLD, MAX_TOOLTIP_COLLISION_PADDING,
    MAX_TOOLTIP_DELAY, MAX_TOOLTIP_GROUP_TIMEOUT, MAX_TOOLTIP_SIDE_OFFSET, MenuOrientation,
    MenuState, NumberFieldScrubDirection, PopoverHoverState, ProgressStatus, SliderThumbAlignment,
    TabsActivationDirection, TabsState, ToastParts, ToastSwipeDirection, TooltipCursorAxis,
    TooltipProvider, TooltipState, ValueFormat, anchor_placement,
};
// Declared option sources, virtual collections, and the remaining stateful field components.
// Every type below is a core model the binding only translates a declaration into.
use quickgui::{
    AutocompleteListState, AutocompleteOptionState, AutocompletePopoverLayout, AutocompleteState,
    Calendar, CalendarState, CalendarWeekday, CivilDate, CivilTime, ComboboxListState,
    ComboboxOptionState, ComboboxState, DateField, DateFieldOrder, DateFieldState, DateSegment,
    MAX_MENUBAR_MENUS, MAX_NUMBER_FIELD_PRECISION, MAX_TABLE_COLUMNS, MAX_TABLE_ROWS, MAX_TOASTS,
    Menubar, MenubarState, NumberField, NumberFieldState, PickerFilterMode, PickerItem,
    SelectListState, SelectOptionState, SelectPopoverLayout, SelectState, TableCellPosition,
    TableCellState, TableColumn, TableColumnAlign, TableEditEnded, TableHeaderState, TableLayout,
    TableRowState, TableSelection, TableSelectionMode, TableSort, TableSortDirection, TableState,
    TimeField, TimeFieldState, TimeSegment, Toast as CoreToast, ToastEntry, ToastId, ToastKind,
    ToastManager, ToastViewport, TreeLayout, TreeLoadChildren, TreeNode, TreeRow, TreeState,
};
#[cfg(target_os = "macos")]
use quickgui::{
    MacEmbeddedView, MacSwiftUiHost, SwiftUiButton, SwiftUiButtonBorderShape, SwiftUiButtonRole,
    SwiftUiButtonStyle, SwiftUiColorPicker, SwiftUiControlSize, SwiftUiDatePicker,
    SwiftUiDatePickerComponents, SwiftUiDatePickerStyle, SwiftUiElement, SwiftUiGauge,
    SwiftUiGaugeStyle, SwiftUiLabelStyle, SwiftUiModifier, SwiftUiPicker, SwiftUiPickerOption,
    SwiftUiPickerStyle, SwiftUiPopover, SwiftUiPopoverArrowEdge, SwiftUiPopoverAttachmentAnchor,
    SwiftUiProgressView, SwiftUiQuickGuiHost, SwiftUiSlider, SwiftUiStepper, SwiftUiTextField,
    SwiftUiToggle, native_view_with_outset,
};

mod dialog;
mod extension_services;
mod integrations;
mod queued_events;
mod system;

pub use dialog::{
    NativeDialogButton, NativeDialogOptions, NativeFileDialogFilter, NativeOpenDialogOptions,
    NativeSaveDialogOptions,
};
use dialog::{
    PendingDialog, native_dialog_configuration, native_open_dialog_options,
    native_save_dialog_options,
};

const PROTOCOL_MAGIC: &[u8; 4] = b"QGMB";
const PROTOCOL_VERSION: u16 = 31;
const ROOT_NODE: u32 = 0;
const ROOT_ELEMENT_ID: u64 = u64::MAX - 1;
const MAX_BATCH_BYTES: usize = 16 * 1024 * 1024;
const MAX_MUTATIONS: usize = 131_072;
const MAX_NODES: usize = 262_144;
const MAX_TREE_DEPTH: usize = 512;
const MAX_STRING_BYTES: usize = 1024 * 1024;
const MAX_QUEUED_EVENTS: usize = 8_192;
const MAX_HOST_COMMANDS: usize = 8_192;
const MAX_WINDOWS: usize = 256;
const NO_ANCHOR: u32 = u32::MAX;
/// Longest compound scope key or item value accepted from one component part property.
const MAX_COMPONENT_VALUE_BYTES: usize = 256;
/// Longest tooltip label retained from one `tooltip` property.
const MAX_TOOLTIP_TEXT_BYTES: usize = 1_024;
/// Longest declared CSS grid track list accepted from one template property.
const MAX_GRID_TRACK_LIST_BYTES: usize = 4_096;
/// Most explicit grid tracks materialized from one declared template.
const MAX_GRID_TRACKS: usize = 512;

mod property {
    pub const DISPLAY: u16 = 1;
    pub const FLEX_DIRECTION: u16 = 2;
    pub const FLEX_WRAP: u16 = 3;
    pub const FLEX_GROW: u16 = 4;
    pub const FLEX_SHRINK: u16 = 5;
    pub const FLEX_BASIS: u16 = 6;
    pub const ALIGN_ITEMS: u16 = 7;
    pub const ALIGN_SELF: u16 = 8;
    pub const JUSTIFY_CONTENT: u16 = 9;
    pub const ALIGN_CONTENT: u16 = 10;
    pub const GAP: u16 = 11;
    pub const COLUMN_GAP: u16 = 12;
    pub const ROW_GAP: u16 = 13;
    pub const WIDTH: u16 = 14;
    pub const HEIGHT: u16 = 15;
    pub const MIN_WIDTH: u16 = 16;
    pub const MIN_HEIGHT: u16 = 17;
    pub const MAX_WIDTH: u16 = 18;
    pub const MAX_HEIGHT: u16 = 19;
    pub const PADDING: u16 = 20;
    pub const PADDING_TOP: u16 = 21;
    pub const PADDING_RIGHT: u16 = 22;
    pub const PADDING_BOTTOM: u16 = 23;
    pub const PADDING_LEFT: u16 = 24;
    pub const MARGIN: u16 = 25;
    pub const MARGIN_TOP: u16 = 26;
    pub const MARGIN_RIGHT: u16 = 27;
    pub const MARGIN_BOTTOM: u16 = 28;
    pub const MARGIN_LEFT: u16 = 29;
    pub const BACKGROUND_COLOR: u16 = 30;
    pub const COLOR: u16 = 31;
    pub const OPACITY: u16 = 32;
    pub const BORDER_WIDTH: u16 = 33;
    pub const BORDER_COLOR: u16 = 34;
    pub const BORDER_RADIUS: u16 = 35;
    pub const FONT_SIZE: u16 = 36;
    pub const FONT_WEIGHT: u16 = 37;
    pub const LINE_HEIGHT: u16 = 38;
    pub const TEXT_ALIGN: u16 = 39;
    pub const WHITE_SPACE: u16 = 40;
    pub const TEXT_OVERFLOW: u16 = 41;
    pub const LINE_CLAMP: u16 = 42;
    pub const OVERFLOW: u16 = 43;
    pub const OVERFLOW_X: u16 = 44;
    pub const OVERFLOW_Y: u16 = 45;
    pub const CURSOR: u16 = 46;
    pub const APP_REGION: u16 = 47;
    pub const DISABLED: u16 = 48;
    pub const ACCESSIBILITY_LABEL: u16 = 49;
    pub const ROLE: u16 = 50;
    pub const TAB_INDEX: u16 = 51;
    pub const POSITION: u16 = 52;
    pub const TOP: u16 = 53;
    pub const RIGHT: u16 = 54;
    pub const BOTTOM: u16 = 55;
    pub const LEFT: u16 = 56;
    pub const USER_SELECT: u16 = 57;
    pub const CLICK_LISTENER: u16 = 58;
    pub const HOVER_LISTENER: u16 = 59;
    pub const VISIBILITY: u16 = 60;
    pub const ASPECT_RATIO: u16 = 61;
    pub const VALUE: u16 = 62;
    pub const PLACEHOLDER: u16 = 63;
    pub const MULTILINE: u16 = 64;
    pub const INPUT_LISTENER: u16 = 65;
    pub const SUBMIT_LISTENER: u16 = 66;
    pub const STREAMING: u16 = 67;
    pub const MARKDOWN_CODE_BACKGROUND: u16 = 68;
    pub const MARKDOWN_BORDER_COLOR: u16 = 69;
    pub const MARKDOWN_MUTED_COLOR: u16 = 70;
    pub const MARKDOWN_LINK_COLOR: u16 = 71;
    pub const MARKDOWN_CODE_TEXT_COLOR: u16 = 72;
    pub const MARKDOWN_BLOCK_GAP: u16 = 73;
    pub const MARKDOWN_CODE_FONT_SIZE: u16 = 74;
    pub const SCROLL_TO_END_REVISION: u16 = 75;
    pub const PASSWORD: u16 = 76;
    pub const ESTIMATED_ITEM_HEIGHT: u16 = 77;
    pub const OVERSCAN: u16 = 78;
    pub const LIST_ALIGNMENT: u16 = 79;
    pub const FOLLOW_MODE: u16 = 80;
    pub const ANCHOR_TARGET: u16 = 81;
    pub const ANCHOR_PLACEMENT: u16 = 82;
    pub const ANCHOR_GAP: u16 = 83;
    pub const VIEWPORT_MARGIN: u16 = 84;
    pub const DISMISS_ON_ESCAPE: u16 = 85;
    pub const DISMISS_ON_POINTER_OUTSIDE: u16 = 86;
    pub const DISMISS_LISTENER: u16 = 87;
    pub const TERMINAL_PROGRAM: u16 = 88;
    pub const TERMINAL_ARGUMENTS: u16 = 89;
    pub const TERMINAL_WORKING_DIRECTORY: u16 = 90;
    pub const TERMINAL_ENVIRONMENT: u16 = 91;
    pub const TERMINAL_SCROLLBACK: u16 = 92;
    pub const TERMINAL_STATUS_LISTENER: u16 = 93;
    pub const HOVER_BACKGROUND_COLOR: u16 = 94;
    pub const HOVER_COLOR: u16 = 95;
    pub const ACTIVE_BACKGROUND_COLOR: u16 = 96;
    pub const ACTIVE_COLOR: u16 = 97;
    pub const TRANSITION: u16 = 98;
    pub const POINTER_LISTENER: u16 = 99;
    pub const FOCUS_ON_POINTER: u16 = 100;
    pub const FONT_FAMILY: u16 = 101;
    pub const TERMINAL_PALETTE: u16 = 102;
    pub const TERMINAL_CURSOR_COLOR: u16 = 103;
    pub const HIT_SLOP: u16 = 104;
    pub const HIT_SLOP_TOP: u16 = 105;
    pub const HIT_SLOP_RIGHT: u16 = 106;
    pub const HIT_SLOP_BOTTOM: u16 = 107;
    pub const HIT_SLOP_LEFT: u16 = 108;
    pub const OVERLAY: u16 = 109;
    pub const FOCUS_TRAP: u16 = 110;
    pub const RESTORE_PREVIOUS_FOCUS: u16 = 111;
    pub const AUTO_FOCUS: u16 = 112;
    pub const ACCESSIBILITY_MODAL: u16 = 113;
    pub const TERMINAL_PADDING_COLOR: u16 = 114;
    pub const TERMINAL_FONT_THICKEN: u16 = 115;
    pub const SWIFT_UI_SYSTEM_IMAGE: u16 = 116;
    pub const SWIFT_UI_BUTTON_STYLE: u16 = 117;
    pub const SWIFT_UI_CONTROL_SIZE: u16 = 118;
    pub const SWIFT_UI_MATCH_CONTENTS_HORIZONTAL: u16 = 119;
    pub const SWIFT_UI_MATCH_CONTENTS_VERTICAL: u16 = 120;
    pub const SWIFT_UI_TARGET: u16 = 121;
    pub const SWIFT_UI_TEST_ID: u16 = 122;
    pub const SWIFT_UI_MODIFIERS: u16 = 123;
    pub const SWIFT_UI_EMBEDDED_WINDOW: u16 = 124;
    pub const SWIFT_UI_IS_PRESENTED: u16 = 125;
    pub const SWIFT_UI_ATTACHMENT_ANCHOR: u16 = 126;
    pub const SWIFT_UI_ARROW_EDGE: u16 = 127;
    pub const BORDER_TOP_WIDTH: u16 = 129;
    pub const BORDER_RIGHT_WIDTH: u16 = 130;
    pub const BORDER_BOTTOM_WIDTH: u16 = 131;
    pub const BORDER_LEFT_WIDTH: u16 = 132;
    pub const BOX_SHADOW: u16 = 133;
    pub const PART: u16 = 134;
    pub const CHECKED: u16 = 135;
    pub const INDETERMINATE: u16 = 136;
    pub const SCOPE: u16 = 137;
    pub const PART_VALUE: u16 = 138;
    pub const ACTIVE_VALUE: u16 = 139;
    pub const ORIENTATION: u16 = 140;
    pub const ACTIVATE_ON_FOCUS: u16 = 141;
    pub const LOOP_FOCUS: u16 = 142;
    pub const KEEP_MOUNTED: u16 = 143;
    pub const OPEN: u16 = 144;
    pub const ITEM_INDEX: u16 = 145;
    pub const HEADING_LEVEL: u16 = 146;
    pub const REQUIRED: u16 = 147;
    pub const INVALID: u16 = 148;
    pub const VALIDATION_MESSAGE: u16 = 149;
    pub const TOUCHED: u16 = 150;
    pub const DIRTY: u16 = 151;
    pub const FILLED: u16 = 152;
    pub const TOOLTIP: u16 = 153;
    pub const TOOLTIP_PLACEMENT: u16 = 154;
    pub const TOOLTIP_DELAY: u16 = 155;
    pub const TOOLTIP_GAP: u16 = 156;
    pub const TOOLTIP_VIEWPORT_MARGIN: u16 = 157;
    pub const VARIANT: u16 = 158;
    pub const MENU: u16 = 159;
    pub const SELECT_LISTENER: u16 = 160;
    pub const CONTROLS: u16 = 161;
    pub const GRID_TEMPLATE_COLUMNS: u16 = 162;
    pub const GRID_TEMPLATE_ROWS: u16 = 163;
    pub const GRID_AUTO_FLOW: u16 = 164;
    pub const GRID_COLUMN_START: u16 = 165;
    pub const GRID_COLUMN_END: u16 = 166;
    pub const GRID_COLUMN_SPAN: u16 = 167;
    pub const GRID_ROW_START: u16 = 168;
    pub const GRID_ROW_END: u16 = 169;
    pub const GRID_ROW_SPAN: u16 = 170;
    pub const TRANSITION_PROPERTIES: u16 = 171;
    pub const TRANSITION_DURATION: u16 = 172;
    pub const TRANSITION_EASING: u16 = 173;
    pub const TRANSITION_MAX_FPS: u16 = 174;
    pub const MINIMUM: u16 = 175;
    pub const MAXIMUM: u16 = 176;
    pub const LOW: u16 = 177;
    pub const HIGH: u16 = 178;
    pub const OPTIMUM: u16 = 179;
    pub const VALUE_TEXT: u16 = 180;
    pub const PRESSED: u16 = 181;
    pub const OBJECT_FIT: u16 = 182;
    pub const SHADER_PARAMETERS: u16 = 183;
    pub const KEY_DOWN_LISTENER: u16 = 184;
    pub const KEY_UP_LISTENER: u16 = 185;
    pub const MOUSE_DOWN_LISTENER: u16 = 186;
    pub const MOUSE_UP_LISTENER: u16 = 187;
    pub const MOUSE_MOVE_LISTENER: u16 = 188;
    pub const DOUBLE_CLICK_LISTENER: u16 = 189;
    pub const SCROLL_LISTENER: u16 = 190;
    pub const CONTEXT_MENU_LISTENER: u16 = 191;
    pub const PINCH_LISTENER: u16 = 192;
    pub const ROTATION_LISTENER: u16 = 193;
    pub const SMART_MAGNIFY_LISTENER: u16 = 194;
    pub const PRESSURE_LISTENER: u16 = 195;
    pub const FOCUS_LISTENER: u16 = 196;
    pub const KEYMAP: u16 = 197;
    pub const ACTION_LISTENER: u16 = 198;
    pub const DRAGGABLE: u16 = 199;
    pub const DROP_KINDS: u16 = 200;
    pub const DRAG_LISTENER: u16 = 201;
    pub const DROP_LISTENER: u16 = 202;
    pub const VALUES: u16 = 203;
    pub const STEP: u16 = 204;
    pub const LARGE_STEP: u16 = 205;
    pub const ITEMS: u16 = 206;
    pub const COMPONENT_CHANGE_LISTENER: u16 = 207;
    pub const OPTIONS: u16 = 208;
    pub const INPUT_VALUE: u16 = 209;
    pub const FILTER_MODE: u16 = 210;
    pub const APPEARANCE: u16 = 211;
    pub const COLUMNS: u16 = 212;
    pub const ROW_COUNT: u16 = 213;
    pub const SORT_COLUMN: u16 = 214;
    pub const SORT_DIRECTION: u16 = 215;
    pub const SELECTION_MODE: u16 = 216;
    pub const SELECTION: u16 = 217;
    pub const ROW_INDEX: u16 = 218;
    pub const COLUMN_INDEX: u16 = 219;
    pub const NODES: u16 = 220;
    pub const EXPANDED: u16 = 221;
    pub const SELECTED_VALUE: u16 = 222;
    pub const SET_CHILDREN: u16 = 223;
    pub const PRECISION: u16 = 224;
    pub const TOASTS: u16 = 225;
    pub const SEGMENT_ORDER: u16 = 226;
    pub const SEGMENT: u16 = 227;
    pub const CIVIL_VALUE: u16 = 228;
    pub const CIVIL_MINIMUM: u16 = 229;
    pub const CIVIL_MAXIMUM: u16 = 230;
    pub const MENU_COUNT: u16 = 231;
    pub const FIRST_WEEKDAY: u16 = 232;
    pub const ROW_HEIGHT: u16 = 233;
    pub const HEADER_HEIGHT: u16 = 234;
    pub const GROUP: u16 = 235;
    pub const EDITING: u16 = 236;
    pub const DISCLOSURE: u16 = 237;
    pub const LOADING_LABEL: u16 = 238;
    pub const COMMIT_LISTENER: u16 = 239;
    // Extended text styling.
    pub const LETTER_SPACING: u16 = 240;
    pub const WORD_SPACING: u16 = 241;
    pub const TEXT_TRANSFORM: u16 = 242;
    pub const TEXT_SHADOW: u16 = 243;
    pub const TEXT_DECORATION_LINE: u16 = 244;
    pub const TEXT_DECORATION_COLOR: u16 = 245;
    pub const TEXT_DECORATION_STYLE: u16 = 246;
    pub const TEXT_DECORATION_THICKNESS: u16 = 247;
    pub const WORD_BREAK: u16 = 248;
    pub const OVERFLOW_WRAP: u16 = 249;
    pub const HYPHENS: u16 = 250;
    pub const TEXT_DIRECTION: u16 = 251;
    // Direction-relative layout.
    pub const DIRECTION: u16 = 252;
    pub const PADDING_START: u16 = 253;
    pub const PADDING_END: u16 = 254;
    pub const MARGIN_START: u16 = 255;
    pub const MARGIN_END: u16 = 256;
    pub const BORDER_START_WIDTH: u16 = 257;
    pub const BORDER_END_WIDTH: u16 = 258;
    // Extended box styling.
    pub const BACKGROUND_GRADIENT: u16 = 259;
    pub const BORDER_TOP_LEFT_RADIUS: u16 = 260;
    pub const BORDER_TOP_RIGHT_RADIUS: u16 = 261;
    pub const BORDER_BOTTOM_RIGHT_RADIUS: u16 = 262;
    pub const BORDER_BOTTOM_LEFT_RADIUS: u16 = 263;
    pub const BORDER_STYLE: u16 = 264;
    pub const OUTLINE_WIDTH: u16 = 265;
    pub const OUTLINE_COLOR: u16 = 266;
    pub const OUTLINE_OFFSET: u16 = 267;
    pub const OUTLINE_STYLE: u16 = 268;
    pub const BACKGROUND_IMAGE: u16 = 269;
    pub const BACKGROUND_SIZE: u16 = 270;
    pub const BACKGROUND_REPEAT: u16 = 271;
    pub const BACKGROUND_POSITION: u16 = 272;
    pub const FILTER: u16 = 273;
    pub const BACKDROP_FILTER: u16 = 274;
    pub const TRANSFORM: u16 = 275;
    pub const TRANSFORM_ORIGIN: u16 = 276;
    pub const MIX_BLEND_MODE: u16 = 277;
    // Hover, active, and focus state styling the core's `ElementStateStyle` supports.
    pub const HOVER_BACKGROUND_GRADIENT: u16 = 278;
    pub const HOVER_OUTLINE: u16 = 279;
    pub const HOVER_TRANSFORM: u16 = 280;
    pub const ACTIVE_BACKGROUND_GRADIENT: u16 = 281;
    pub const ACTIVE_OUTLINE: u16 = 282;
    pub const ACTIVE_TRANSFORM: u16 = 283;
    pub const FOCUS_BACKGROUND_COLOR: u16 = 284;
    pub const FOCUS_COLOR: u16 = 285;
    pub const FOCUS_BACKGROUND_GRADIENT: u16 = 286;
    pub const FOCUS_OUTLINE: u16 = 287;
    pub const FOCUS_TRANSFORM: u16 = 288;
    // Scroll snapping.
    pub const SCROLL_SNAP_TYPE: u16 = 289;
    pub const SCROLL_SNAP_ALIGN: u16 = 290;
    pub const SCROLL_SNAP_STOP: u16 = 291;
    // Base UI separators, avatars, checkbox groups, preview cards, scroll areas, OTP fields,
    // drawers, and navigation menus. Every one of these is declared ahead of the core's decision,
    // because a hosted renderer can never be asked a synchronous question.
    pub const DELAY: u16 = 292;
    pub const CLOSE_DELAY: u16 = 293;
    pub const LENGTH: u16 = 294;
    pub const MASK: u16 = 295;
    pub const READ_ONLY: u16 = 296;
    pub const AUTO_SUBMIT: u16 = 297;
    pub const SWIPE_DIRECTION: u16 = 298;
    pub const VIEWPORT_SIZE: u16 = 299;
    pub const CONTENT_SIZE: u16 = 300;
    pub const OVERFLOW_EDGE_THRESHOLD: u16 = 301;
    pub const DISABLE_POINTER_DISMISSAL: u16 = 302;
    // Base UI-aligned popover, tooltip, range, toast, tab, toolbar, field, and dialog props.
    // Every one is declared ahead of the core's decision, because the hosted boundary is never
    // asked a synchronous question.
    pub const SIDE: u16 = 303;
    pub const ALIGN: u16 = 304;
    pub const SIDE_OFFSET: u16 = 305;
    pub const ALIGN_OFFSET: u16 = 306;
    pub const COLLISION_PADDING: u16 = 307;
    pub const STICKY: u16 = 308;
    pub const ANCHOR_POINT: u16 = 309;
    pub const MODAL: u16 = 310;
    pub const OPEN_ON_HOVER: u16 = 311;
    pub const PROVIDER: u16 = 312;
    pub const TIMEOUT: u16 = 313;
    pub const HOVERABLE: u16 = 314;
    pub const TRACK_CURSOR_AXIS: u16 = 315;
    pub const CLOSE_ON_CLICK: u16 = 316;
    pub const MIN_STEPS_BETWEEN_VALUES: u16 = 317;
    pub const THUMB_ALIGNMENT: u16 = 318;
    pub const FORMAT: u16 = 319;
    pub const SMALL_STEP: u16 = 320;
    pub const ALLOW_WHEEL_SCRUB: u16 = 321;
    pub const SNAP_ON_STEP: u16 = 322;
    pub const LIMIT: u16 = 323;
    pub const PITCH: u16 = 324;
    pub const FOCUSABLE_WHEN_DISABLED: u16 = 325;
    pub const VALIDATION_MODE: u16 = 326;
    pub const VALIDATION_DEBOUNCE_TIME: u16 = 327;
    pub const PARENT: u16 = 328;
    pub const ENTER_DURATION: u16 = 329;
    pub const EXIT_DURATION: u16 = 330;
    pub const STACK_EXPANDED: u16 = 331;
    /// Base UI's `Menu.Root` `closeParentOnEsc`.
    pub const CLOSE_PARENT_ON_ESC: u16 = 332;
    /// Base UI's `Menu.LinkItem` `href`.
    pub const HREF: u16 = 333;
    /// Base UI's `multiple` on a select or combobox.
    pub const MULTIPLE: u16 = 334;
    /// Base UI's `Select.Positioner` `alignItemWithTrigger`.
    pub const ALIGN_ITEM_WITH_TRIGGER: u16 = 335;
    /// Base UI's combobox `autoHighlight`.
    pub const AUTO_HIGHLIGHT: u16 = 336;
    /// Base UI's combobox `openOnInputClick`.
    pub const OPEN_ON_INPUT_CLICK: u16 = 337;
    /// Base UI's combobox `highlightItemOnHover`.
    pub const HIGHLIGHT_ITEM_ON_HOVER: u16 = 338;
    pub const SWIFT_UI_PICKER_STYLE: u16 = 339;
    pub const SWIFT_UI_DATE_PICKER_COMPONENTS: u16 = 340;
    pub const SWIFT_UI_DATE_PICKER_STYLE: u16 = 341;
    pub const SWIFT_UI_COLOR_SUPPORTS_OPACITY: u16 = 342;
    pub const SWIFT_UI_GAUGE_STYLE: u16 = 343;
    pub const SWIFT_UI_GAUGE_MINIMUM_VALUE_LABEL: u16 = 344;
    pub const SWIFT_UI_GAUGE_MAXIMUM_VALUE_LABEL: u16 = 345;
    // Nested interaction-state styles: one bounded JSON declaration per state carrying everything
    // the core's `ElementStateStyle` can swap, plus the hover-group marker whose hover a
    // descendant's `groupHover` follows. The flat hover/active/focus codes above stay honoured.
    pub const HOVER_STYLE: u16 = 346;
    pub const ACTIVE_STYLE: u16 = 347;
    pub const FOCUS_STYLE: u16 = 348;
    pub const DISABLED_STYLE: u16 = 349;
    pub const INVALID_STYLE: u16 = 350;
    pub const DRAGGING_STYLE: u16 = 351;
    pub const DRAG_OVER_STYLE: u16 = 352;
    pub const GROUP_HOVER_STYLE: u16 = 353;
    pub const HOVER_GROUP: u16 = 354;
    pub const GROUP_ACTIVE_STYLE: u16 = 355;
    pub const FOCUS_WITHIN_STYLE: u16 = 356;
    /// Nested `selected` state style, painted while the element's `selected` flag is set.
    pub const SELECTED_STYLE: u16 = 357;
    /// Web-style `selected` flag on any element: the native accessibility state and the
    /// `selected` state style follow one declaration.
    pub const SELECTED: u16 = 358;
    pub const SCROLL_SNAP_X: u16 = 359;
    pub const SCROLL_SNAP_Y: u16 = 360;
    pub const MOTION: u16 = 361;
    pub const ANCHORED_LAYER: u16 = 362;
    pub const INPUT_PRESENTATION: u16 = 363;
    pub const OVERSCAN_PIXELS: u16 = 364;
    pub const BLOCK_POINTER: u16 = 365;
    pub const INPUT_SUBMIT_ON_ENTER: u16 = 366;
    pub const SCROLL_REQUEST: u16 = 367;
    pub const RICH_DOCUMENT: u16 = 368;
    pub const DOCUMENT_THEME: u16 = 369;
    pub const LAYOUT_ROUNDING: u16 = 370;
    pub const LAST: u16 = LAYOUT_ROUNDING;
}

/// Base64 transport for optional byte payloads carried inside JSON options and results.
///
/// The C ABI passes bulk bytes as length-delimited spans; the rare byte fields nested inside JSON
/// declarations (icons, clipboard data) ride as base64 strings instead of a second channel.
pub(crate) mod base64_bytes {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(
        value: &Option<Vec<u8>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match value {
            Some(bytes) => serializer.serialize_some(&STANDARD.encode(bytes)),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<Vec<u8>>, D::Error> {
        let value: Option<String> = Option::deserialize(deserializer)?;
        value
            .map(|text| STANDARD.decode(text).map_err(serde::de::Error::custom))
            .transpose()
    }
}

#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct NativeImageSource {
    /// Encoded image bytes, or raw RGBA8 when width and height are both supplied.
    #[serde(with = "base64_bytes")]
    pub data: Option<Vec<u8>>,
    /// Image path used when data is omitted.
    pub path: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct NativeAppOptions {
    pub name: Option<String>,
    pub version: Option<String>,
    pub identifier: Option<String>,
    pub resource_dir: Option<String>,
    pub config_dir: Option<String>,
    pub data_dir: Option<String>,
    pub local_data_dir: Option<String>,
    pub cache_dir: Option<String>,
    pub log_dir: Option<String>,
    pub runtime_dir: Option<String>,
    pub temp_dir: Option<String>,
    /// `default`, `last-window-closed`, or `explicit`.
    pub quit_mode: Option<String>,
    /// OpenType files registered before readiness. Relative paths use `resource_dir`.
    pub fonts: Option<Vec<String>>,
}

#[derive(Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct NativeWindowOptions {
    pub title: Option<String>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub x: Option<f64>,
    pub y: Option<f64>,
    /// `normal`, `maximized`, or `fullscreen`.
    pub initial_state: Option<String>,
    pub display_id: Option<String>,
    /// Set to false to remove QuickGUI's default minimum size.
    pub minimum_size_enabled: Option<bool>,
    pub minimum_width: Option<f64>,
    pub minimum_height: Option<f64>,
    pub maximum_width: Option<f64>,
    pub maximum_height: Option<f64>,
    pub represented_file: Option<String>,
    pub document_edited: Option<bool>,
    pub tabbing_identifier: Option<String>,
    pub background: Option<u32>,
    pub performance_profile: Option<String>,
    pub appearance: Option<String>,
    pub vibrancy: Option<String>,
    pub visual_effect_state: Option<String>,
    pub title_bar_style: Option<String>,
    pub kind: Option<String>,
    pub focus: Option<bool>,
    pub focusable: Option<bool>,
    pub show: Option<bool>,
    pub movable: Option<bool>,
    pub resizable: Option<bool>,
    pub minimizable: Option<bool>,
    pub maximizable: Option<bool>,
    pub closable: Option<bool>,
    pub decorated: Option<bool>,
    pub shadow: Option<bool>,
    pub content_protected: Option<bool>,
    pub window_level: Option<String>,
    pub skip_taskbar: Option<bool>,
    pub visible_on_all_workspaces: Option<bool>,
    pub opacity: Option<f64>,
    pub icon: Option<NativeImageSource>,
    pub taskbar_progress_state: Option<String>,
    pub taskbar_progress: Option<f64>,
    pub taskbar_overlay_icon: Option<NativeImageSource>,
    pub taskbar_overlay_description: Option<String>,
    pub cursor_visible: Option<bool>,
    pub cursor_grab: Option<String>,
    pub cursor_hit_test: Option<bool>,
    pub cursor_x: Option<f64>,
    pub cursor_y: Option<f64>,
    /// Bounded JSON encoding of a per-window native menu. Omitted windows inherit the app menu.
    pub menu: Option<String>,
    /// Persisted geometry and display identity captured with `window.getRestoreState()`.
    ///
    /// The core re-validates every field, so a stale value can never place a window off every
    /// connected display.
    pub restore_state: Option<system::NativeWindowRestoreState>,
    pub line_scroll_pixels: Option<f64>,
    pub key_sequence_timeout_ms: Option<f64>,
    pub reduce_motion: Option<bool>,
    pub traffic_light_x: Option<f64>,
    pub traffic_light_y: Option<f64>,
    pub transparent: Option<bool>,
    pub blur: Option<bool>,
    pub popover_placement: Option<String>,
    pub popover_gap: Option<f64>,
    pub popover_offset_x: Option<f64>,
    pub popover_offset_y: Option<f64>,
    pub popover_viewport_margin: Option<f64>,
    pub popover_dismiss_on_escape: Option<bool>,
    pub popover_dismiss_on_pointer_outside: Option<bool>,
    pub popover_grab: Option<bool>,
    pub popover_accepts_key_focus: Option<bool>,
}

/// One bounded event the host delivers to the application thread.
#[derive(Clone, Debug, Default)]
pub struct NativeEvent {
    pub kind: String,
    pub window: u32,
    pub target: u32,
    pub value: Option<String>,
    pub paths: Option<Vec<String>>,
    pub data: Option<Vec<u8>>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub error: Option<String>,
}

impl NativeEvent {
    pub(crate) fn reply(
        kind: &str,
        request: u32,
        value: Option<String>,
        error: Option<String>,
    ) -> Self {
        Self {
            kind: kind.to_owned(),
            window: 0,
            target: request,
            value,
            paths: None,
            data: None,
            width: None,
            height: None,
            error,
        }
    }
}

/// Application-thread callback that receives one event: the native trampoline copies every span
/// before returning, so the host never waits on the application thread.
///
/// `flags` bit 0 marks a present `value`, bit 1 a present `extra` JSON object (`paths`, `width`,
/// `height`, `error`), and bit 2 present `data` bytes.
pub type EventCallback = unsafe extern "C" fn(
    kind: *const u8,
    kind_len: usize,
    window: u32,
    target: u32,
    flags: u32,
    value: *const u8,
    value_len: usize,
    extra: *const u8,
    extra_len: usize,
    data: *const u8,
    data_len: usize,
    context: *mut c_void,
);

#[derive(Clone, Copy)]
struct EventSink {
    callback: EventCallback,
    context: usize,
}

impl EventSink {
    fn deliver(&self, event: &NativeEvent) {
        let mut flags = 0_u32;
        let value = event.value.as_deref().unwrap_or_default();
        if event.value.is_some() {
            flags |= 1;
        }
        let extra = if event.paths.is_some()
            || event.width.is_some()
            || event.height.is_some()
            || event.error.is_some()
        {
            flags |= 2;
            let mut object = serde_json::Map::new();
            if let Some(paths) = &event.paths {
                object.insert("paths".to_owned(), serde_json::json!(paths));
            }
            if let Some(width) = event.width {
                object.insert("width".to_owned(), serde_json::json!(width));
            }
            if let Some(height) = event.height {
                object.insert("height".to_owned(), serde_json::json!(height));
            }
            if let Some(error) = &event.error {
                object.insert("error".to_owned(), serde_json::json!(error));
            }
            serde_json::Value::Object(object).to_string()
        } else {
            String::new()
        };
        let data = event.data.as_deref().unwrap_or_default();
        if event.data.is_some() {
            flags |= 4;
        }
        // SAFETY: the callback is a purego trampoline that copies every span before it
        // returns; the spans outlive the call because `event` is borrowed for its duration.
        unsafe {
            (self.callback)(
                event.kind.as_ptr(),
                event.kind.len(),
                event.window,
                event.target,
                flags,
                value.as_ptr(),
                value.len(),
                extra.as_ptr(),
                extra.len(),
                data.as_ptr(),
                data.len(),
                self.context as *mut c_void,
            );
        }
    }
}

enum HostCommand {
    CreateApp {
        app: u32,
        options: NativeAppOptions,
    },
    CreateWindow {
        app: u32,
        window: u32,
        options: NativeWindowOptions,
        initial_batch: Vec<u8>,
    },
    CreateSystemPopover {
        app: u32,
        window: u32,
        parent: u32,
        anchor: u32,
        options: NativeWindowOptions,
        initial_batch: Vec<u8>,
    },
    #[cfg(target_os = "macos")]
    CreateEmbeddedView {
        app: u32,
        window: u32,
        parent: u32,
        match_horizontal: bool,
        match_vertical: bool,
        options: NativeWindowOptions,
        initial_batch: Vec<u8>,
    },
    ApplyBatch {
        app: u32,
        window: u32,
        batch: Vec<u8>,
    },
    /// A fire-and-forget system command; failures are fatal host errors.
    Mutation {
        app: u32,
        command: system::SystemCommand,
    },
    /// A system command whose result returns as one `command` event carrying `request`.
    Request {
        app: u32,
        request: u32,
        command: system::SystemCommand,
    },
    CloseWindow {
        app: u32,
        window: u32,
    },
    FocusNode {
        app: u32,
        window: u32,
        node: u32,
    },
    ShowAlertDialog {
        app: u32,
        window: Option<u32>,
        request: u32,
        options: NativeDialogOptions,
    },
    ShowOpenDialog {
        app: u32,
        window: Option<u32>,
        request: u32,
        options: NativeOpenDialogOptions,
    },
    ShowSaveDialog {
        app: u32,
        window: Option<u32>,
        request: u32,
        options: NativeSaveDialogOptions,
    },
    /// Make the application ready; the outcome returns as one `app-ready` event.
    PrepareApp {
        app: u32,
        request: u32,
    },
    DestroyApp {
        app: u32,
    },
}

#[derive(Default)]
struct HostState {
    running: bool,
    app: Option<u32>,
    commands: VecDeque<HostCommand>,
    /// Events published before the application registered its callback.
    pending_events: VecDeque<NativeEvent>,
    exit_code: Option<i32>,
    failure: Option<String>,
    waker: Option<AppRunnerWaker>,
}

struct HostCoordinator {
    next_app: AtomicU32,
    next_window: AtomicU32,
    state: Mutex<HostState>,
    sink: Mutex<Option<EventSink>>,
    changed: Condvar,
}

impl HostCoordinator {
    fn new() -> Self {
        Self {
            next_app: AtomicU32::new(1),
            next_window: AtomicU32::new(1),
            state: Mutex::new(HostState::default()),
            sink: Mutex::new(None),
            changed: Condvar::new(),
        }
    }

    fn allocate_app(&self) -> std::result::Result<u32, String> {
        let app = self
            .next_app
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| {
                id.checked_add(1).filter(|next| *next != 0)
            })
            .map_err(|_| "QuickGUI hosted app id space exhausted".to_owned())?
            .max(1);
        Ok(app)
    }

    fn allocate_window(&self) -> std::result::Result<u32, String> {
        self.next_window
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| {
                id.checked_add(1).filter(|next| *next != 0)
            })
            .map(|id| id.max(1))
            .map_err(|_| "QuickGUI hosted window id space exhausted".to_owned())
    }

    fn enqueue(&self, command: HostCommand) -> std::result::Result<(), String> {
        let waker = {
            let mut state = lock(&self.state);
            if let Some(error) = state.failure.clone() {
                return Err(error);
            }
            if state.commands.len() >= MAX_HOST_COMMANDS {
                return Err("QuickGUI host command queue is full".to_owned());
            }
            state.commands.push_back(command);
            state.waker.clone()
        };
        self.changed.notify_all();
        if let Some(waker) = waker {
            waker.wake();
        }
        Ok(())
    }

    fn begin(&self) -> std::result::Result<(), String> {
        let mut state = lock(&self.state);
        if state.running {
            return Err("the QuickGUI native host is already running".to_owned());
        }
        state.running = true;
        Ok(())
    }

    fn finish(&self) {
        let mut state = lock(&self.state);
        state.running = false;
        state.waker = None;
        self.changed.notify_all();
    }

    fn wait_for_commands(&self) -> std::result::Result<VecDeque<HostCommand>, String> {
        let mut state = lock(&self.state);
        while state.commands.is_empty() && state.failure.is_none() {
            state = wait(&self.changed, state);
        }
        if let Some(error) = state.failure.clone() {
            return Err(error);
        }
        Ok(std::mem::take(&mut state.commands))
    }

    fn take_commands(&self) -> std::result::Result<VecDeque<HostCommand>, String> {
        let mut state = lock(&self.state);
        if let Some(error) = state.failure.clone() {
            return Err(error);
        }
        Ok(std::mem::take(&mut state.commands))
    }

    fn set_app(&self, app: u32) -> std::result::Result<(), String> {
        let mut state = lock(&self.state);
        if state.app.is_some() {
            return Err("a QuickGUI hosted app is already active".to_owned());
        }
        state.app = Some(app);
        state.exit_code = None;
        state.failure = None;
        Ok(())
    }

    fn set_waker(&self, waker: AppRunnerWaker) {
        lock(&self.state).waker = Some(waker);
    }

    /// Make the host loop's blocking pump return so queued native events reach the application.
    fn wake(&self) {
        let waker = lock(&self.state).waker.clone();
        if let Some(waker) = waker {
            waker.wake();
        }
    }

    /// Register the application thread's event callback and flush everything published so far.
    fn set_sink(&self, sink: Option<EventSink>) {
        *lock(&self.sink) = sink;
        if sink.is_some() {
            let pending = std::mem::take(&mut lock(&self.state).pending_events);
            self.publish_events(pending);
        }
    }

    fn publish_events(&self, events: impl IntoIterator<Item = NativeEvent>) {
        let sink = *lock(&self.sink);
        match sink {
            Some(sink) => {
                for event in events {
                    sink.deliver(&event);
                }
            }
            None => {
                let mut state = lock(&self.state);
                for event in events {
                    if state.pending_events.len() >= MAX_QUEUED_EVENTS {
                        break;
                    }
                    state.pending_events.push_back(event);
                }
            }
        }
    }

    fn publish_exit(&self, code: i32) {
        {
            let mut state = lock(&self.state);
            state.exit_code = Some(code.max(0));
            state.waker = None;
        }
        self.changed.notify_all();
        self.publish_events([NativeEvent::reply("exit", code.max(0) as u32, None, None)]);
    }

    fn fail(&self, error: String) {
        let waker = {
            let mut state = lock(&self.state);
            if state.failure.is_none() {
                state.failure = Some(error.clone());
            }
            state.waker.clone()
        };
        self.changed.notify_all();
        if let Some(waker) = waker {
            waker.wake();
        }
        self.publish_events([NativeEvent::reply("host-error", 0, None, Some(error))]);
    }

    #[cfg(test)]
    fn failure(&self) -> Option<String> {
        lock(&self.state).failure.clone()
    }
}

static HOST: LazyLock<HostCoordinator> = LazyLock::new(HostCoordinator::new);

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn wait<'a, T>(
    condvar: &Condvar,
    guard: std::sync::MutexGuard<'a, T>,
) -> std::sync::MutexGuard<'a, T> {
    condvar
        .wait(guard)
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Report a finished asynchronous request through one reply event.
pub(crate) fn publish_reply(
    kind: &str,
    request: u32,
    result: std::result::Result<serde_json::Value, String>,
) {
    let event = match result {
        Ok(mut value) => {
            strip_nulls(&mut value);
            NativeEvent::reply(kind, request, Some(value.to_string()), None)
        }
        Err(error) => NativeEvent::reply(kind, request, None, Some(error)),
    };
    HOST.publish_events([event]);
}

/// Drop `null` members so optional fields decode as absent on the application side.
pub(crate) fn strip_nulls(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(object) => {
            object.retain(|_, member| !member.is_null());
            for member in object.values_mut() {
                strip_nulls(member);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                strip_nulls(item);
            }
        }
        _ => {}
    }
}

mod anchored_layer;
mod base_ui;
mod capi;
mod collections;
mod commands;
mod components;
mod document;
mod events;
mod input_presentation;
mod menus;
mod motion;
mod pickers;
mod popover_menu;
mod popovers;
mod router;
mod runtime;
mod scroll_request;
mod styles;
mod tree;
mod view;

pub use capi::*;
use commands::*;

use base_ui::*;
use collections::*;
use components::*;
use events::*;
use menus::*;
use pickers::*;
use popover_menu::*;
use popovers::*;
pub use router::*;
use runtime::*;
use styles::*;
use tree::*;
use view::*;

#[cfg(test)]
mod tests;
