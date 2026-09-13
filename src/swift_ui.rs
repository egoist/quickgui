use std::{
    cell::RefCell,
    collections::HashSet,
    ffi::{CStr, CString, c_char, c_void},
    fmt,
    panic::{AssertUnwindSafe, catch_unwind},
    ptr::NonNull,
    rc::Rc,
    sync::Arc,
};

use objc2_foundation::MainThreadMarker;
use serde::Serialize;

use crate::{MacNativeView, Size, WindowHandle};

type ActionCallback = unsafe extern "C" fn(*mut c_void, u64);
type PresentationCallback = unsafe extern "C" fn(*mut c_void, u64, bool);
type ValueCallback = unsafe extern "C" fn(*mut c_void, u64, *const c_char);
type SubmitCallback = unsafe extern "C" fn(*mut c_void, u64);

unsafe extern "C" {
    fn quickgui_swift_ui_host_create(
        context: *mut c_void,
        callback: Option<ActionCallback>,
        presentation_callback: Option<PresentationCallback>,
        value_callback: Option<ValueCallback>,
        submit_callback: Option<SubmitCallback>,
    ) -> *mut c_void;
    fn quickgui_swift_ui_host_view(handle: *mut c_void) -> *mut c_void;
    fn quickgui_swift_ui_host_update(handle: *mut c_void, json: *const c_char) -> bool;
    fn quickgui_swift_ui_host_set_embedded_view(
        handle: *mut c_void,
        id: u64,
        view: *mut c_void,
        width: f64,
        height: f64,
    ) -> bool;
    fn quickgui_swift_ui_host_remove_embedded_view(handle: *mut c_void, id: u64);
    fn quickgui_swift_ui_host_fitting_size(handle: *mut c_void, width: *mut f64, height: *mut f64);
    fn quickgui_swift_ui_host_effect_inset(handle: *mut c_void) -> f64;
    fn quickgui_swift_ui_host_release(handle: *mut c_void);
}

/// Semantic role forwarded to a native SwiftUI button.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SwiftUiButtonRole {
    #[default]
    Default,
    Cancel,
    Destructive,
}

/// Native SwiftUI button style. Glass variants fall back to their bordered counterparts before
/// macOS 26.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SwiftUiButtonStyle {
    #[default]
    Automatic,
    Bordered,
    BorderedProminent,
    Borderless,
    Plain,
    Glass,
    GlassProminent,
}

/// Native control sizing forwarded to SwiftUI.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SwiftUiControlSize {
    Mini,
    Small,
    #[default]
    Regular,
    Large,
    ExtraLarge,
}

/// Border shape used by styled native SwiftUI buttons.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SwiftUiButtonBorderShape {
    #[default]
    Automatic,
    Capsule,
    RoundedRectangle,
    Circle,
}

/// Presentation of a SwiftUI `Label` used as a button label.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SwiftUiLabelStyle {
    #[default]
    Automatic,
    IconOnly,
    TitleAndIcon,
    TitleOnly,
}

/// Ordered SwiftUI view modifier supported by QuickGUI's native button bridge.
#[derive(Clone, Debug, PartialEq)]
pub enum SwiftUiModifier {
    ButtonStyle(SwiftUiButtonStyle),
    ButtonBorderShape {
        shape: SwiftUiButtonBorderShape,
        corner_radius: Option<f32>,
    },
    ControlSize(SwiftUiControlSize),
    LabelStyle(SwiftUiLabelStyle),
    Tint(Arc<str>),
    Disabled(bool),
}

/// One declarative button inside a [`MacSwiftUiHost`].
#[derive(Clone, Debug, PartialEq)]
pub struct SwiftUiButton {
    pub id: u64,
    pub label: Option<Arc<str>>,
    pub system_image: Option<Arc<str>>,
    pub role: SwiftUiButtonRole,
    pub target: Option<Arc<str>>,
    pub test_id: Option<Arc<str>>,
    pub modifiers: Vec<SwiftUiModifier>,
    pub has_action: bool,
}

/// One controlled native SwiftUI slider inside a [`MacSwiftUiHost`].
#[derive(Clone, Debug, PartialEq)]
pub struct SwiftUiSlider {
    pub id: u64,
    pub value: f64,
    pub minimum: f64,
    pub maximum: f64,
    pub step: Option<f64>,
    pub label: Option<Arc<str>>,
    pub test_id: Option<Arc<str>>,
    pub modifiers: Vec<SwiftUiModifier>,
    pub has_value_change: bool,
}

/// One controlled native SwiftUI toggle inside a [`MacSwiftUiHost`].
#[derive(Clone, Debug, PartialEq)]
pub struct SwiftUiToggle {
    pub id: u64,
    pub is_on: bool,
    pub label: Option<Arc<str>>,
    pub test_id: Option<Arc<str>>,
    pub modifiers: Vec<SwiftUiModifier>,
    pub has_value_change: bool,
}

/// One determinate or indeterminate native SwiftUI progress indicator.
#[derive(Clone, Debug, PartialEq)]
pub struct SwiftUiProgressView {
    pub id: u64,
    pub value: Option<f64>,
    pub total: f64,
    pub label: Option<Arc<str>>,
    pub current_value_label: Option<Arc<str>>,
    pub test_id: Option<Arc<str>>,
    pub modifiers: Vec<SwiftUiModifier>,
}

/// One controlled native SwiftUI stepper inside a [`MacSwiftUiHost`].
#[derive(Clone, Debug, PartialEq)]
pub struct SwiftUiStepper {
    pub id: u64,
    pub value: f64,
    pub minimum: f64,
    pub maximum: f64,
    pub step: f64,
    pub label: Option<Arc<str>>,
    pub test_id: Option<Arc<str>>,
    pub modifiers: Vec<SwiftUiModifier>,
    pub has_value_change: bool,
}

/// One controlled native SwiftUI text or secure field inside a [`MacSwiftUiHost`].
#[derive(Clone, Debug, PartialEq)]
pub struct SwiftUiTextField {
    pub id: u64,
    pub text: Arc<str>,
    pub placeholder: Option<Arc<str>>,
    pub secure: bool,
    pub test_id: Option<Arc<str>>,
    pub modifiers: Vec<SwiftUiModifier>,
    pub has_value_change: bool,
    pub has_submit: bool,
}

/// Native presentation style for a SwiftUI [`Picker`](https://developer.apple.com/documentation/swiftui/picker).
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SwiftUiPickerStyle {
    #[default]
    Automatic,
    Menu,
    Segmented,
    /// Segmented tab navigation. On macOS 27 this selects the neutral Xcode-style tabs role.
    Tabs,
    RadioGroup,
    Inline,
}

/// One selectable value declared ahead of a native SwiftUI picker interaction.
#[derive(Clone, Debug, PartialEq)]
pub struct SwiftUiPickerOption {
    pub value: Arc<str>,
    pub label: Arc<str>,
    pub system_image: Option<Arc<str>>,
    pub disabled: bool,
}

/// One controlled native SwiftUI picker inside a [`MacSwiftUiHost`].
#[derive(Clone, Debug, PartialEq)]
pub struct SwiftUiPicker {
    pub id: u64,
    pub selection: Arc<str>,
    pub options: Vec<SwiftUiPickerOption>,
    pub style: SwiftUiPickerStyle,
    pub label: Option<Arc<str>>,
    pub test_id: Option<Arc<str>>,
    pub modifiers: Vec<SwiftUiModifier>,
    pub has_value_change: bool,
}

/// Named Rust surface for a picker created with [`SwiftUiPicker::segmented`].
///
/// SwiftUI itself models a segmented control as a `Picker` style, so this alias keeps one source of
/// truth while making the component discoverable by its conventional name.
pub type SwiftUiSegmentedControl = SwiftUiPicker;

/// Named Rust surface for a picker created with [`SwiftUiPicker::tabs`].
pub type SwiftUiSegmentedTabs = SwiftUiPicker;

/// Visible fields in a native SwiftUI date picker.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SwiftUiDatePickerComponents {
    Date,
    HourAndMinute,
    #[default]
    DateAndTime,
}

/// Native presentation style for a SwiftUI date picker.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SwiftUiDatePickerStyle {
    #[default]
    Automatic,
    Field,
    Graphical,
    StepperField,
}

/// One controlled native SwiftUI date picker.
///
/// Dates use seconds from the Unix epoch. This stays runtime-neutral for direct Rust callers while
/// preserving sub-second precision across the hosted JavaScript mutation protocol.
#[derive(Clone, Debug, PartialEq)]
pub struct SwiftUiDatePicker {
    pub id: u64,
    pub value: f64,
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
    pub components: SwiftUiDatePickerComponents,
    pub style: SwiftUiDatePickerStyle,
    pub label: Option<Arc<str>>,
    pub test_id: Option<Arc<str>>,
    pub modifiers: Vec<SwiftUiModifier>,
    pub has_value_change: bool,
}

/// One controlled native SwiftUI color picker.
#[derive(Clone, Debug, PartialEq)]
pub struct SwiftUiColorPicker {
    pub id: u64,
    pub selection: Arc<str>,
    pub supports_opacity: bool,
    pub label: Option<Arc<str>>,
    pub test_id: Option<Arc<str>>,
    pub modifiers: Vec<SwiftUiModifier>,
    pub has_value_change: bool,
}

/// Native presentation style for a SwiftUI gauge.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SwiftUiGaugeStyle {
    #[default]
    Automatic,
    AccessoryCircular,
    AccessoryCircularCapacity,
    AccessoryLinear,
    AccessoryLinearCapacity,
}

/// One native SwiftUI gauge.
#[derive(Clone, Debug, PartialEq)]
pub struct SwiftUiGauge {
    pub id: u64,
    pub value: f64,
    pub minimum: f64,
    pub maximum: f64,
    pub label: Option<Arc<str>>,
    pub current_value_label: Option<Arc<str>>,
    pub minimum_value_label: Option<Arc<str>>,
    pub maximum_value_label: Option<Arc<str>>,
    pub style: SwiftUiGaugeStyle,
    pub test_id: Option<Arc<str>>,
    pub modifiers: Vec<SwiftUiModifier>,
}

/// A retained QuickGUI renderer surface that can be mounted by a native SwiftUI descriptor.
///
/// The platform view is installed asynchronously at the normal window-creation boundary. Clones
/// retain the same surface identity and content-size state; dropping this value does not close the
/// surface's core window.
#[derive(Clone)]
pub struct MacEmbeddedView {
    window: WindowHandle,
    pub(crate) owner: WindowHandle,
    pub(crate) inner: Rc<RefCell<MacEmbeddedViewState>>,
}

#[derive(Clone, Debug)]
pub(crate) struct MacEmbeddedViewState {
    pub(crate) view: Option<MacNativeView>,
    pub(crate) content_size: Size,
    pub(crate) match_horizontal: bool,
    pub(crate) match_vertical: bool,
}

impl MacEmbeddedView {
    pub(crate) fn pending(
        window: WindowHandle,
        owner: WindowHandle,
        initial_size: Size,
        match_horizontal: bool,
        match_vertical: bool,
    ) -> Self {
        Self {
            window,
            owner,
            inner: Rc::new(RefCell::new(MacEmbeddedViewState {
                view: None,
                content_size: initial_size,
                match_horizontal,
                match_vertical,
            })),
        }
    }

    /// Stable core window handle used by the embedded QuickGUI subtree.
    pub const fn window_handle(&self) -> WindowHandle {
        self.window
    }

    /// Retained AppKit rendering view once the pending core window has been created.
    pub fn view(&self) -> Option<MacNativeView> {
        self.inner.borrow().view.clone()
    }

    /// Latest max-content measurement published by the embedded retained tree.
    pub fn content_size(&self) -> Size {
        self.inner.borrow().content_size
    }

    pub(crate) fn install(&self, view: MacNativeView) {
        self.inner.borrow_mut().view = Some(view);
    }

    pub(crate) fn update_content_size(&self, size: Size) -> bool {
        let mut state = self.inner.borrow_mut();
        let size = Size::new(size.width.max(1.0), size.height.max(1.0));
        if (state.content_size.width - size.width).abs() < 0.25
            && (state.content_size.height - size.height).abs() < 0.25
        {
            return false;
        }
        state.content_size = size;
        true
    }

    pub(crate) fn match_axes(&self) -> (bool, bool) {
        let state = self.inner.borrow();
        (state.match_horizontal, state.match_vertical)
    }
}

impl fmt::Debug for MacEmbeddedView {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MacEmbeddedView")
            .field("window", &self.window)
            .field("owner", &self.owner)
            .field("ready", &self.inner.borrow().view.is_some())
            .field("content_size", &self.inner.borrow().content_size)
            .finish()
    }
}

/// One ordinary QuickGUI subtree embedded back into a SwiftUI host.
#[derive(Clone, Debug)]
pub struct SwiftUiQuickGuiHost {
    pub id: u64,
    pub embedded: Option<MacEmbeddedView>,
    pub match_horizontal: bool,
    pub match_vertical: bool,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub test_id: Option<Arc<str>>,
}

impl SwiftUiQuickGuiHost {
    pub fn new(id: u64, embedded: MacEmbeddedView) -> Self {
        Self {
            id,
            embedded: Some(embedded),
            match_horizontal: false,
            match_vertical: false,
            width: None,
            height: None,
            test_id: None,
        }
    }

    pub fn pending(id: u64) -> Self {
        Self {
            id,
            embedded: None,
            match_horizontal: false,
            match_vertical: false,
            width: None,
            height: None,
            test_id: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SwiftUiPopoverAttachmentAnchor {
    #[default]
    Center,
    Top,
    Bottom,
    Leading,
    Trailing,
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SwiftUiPopoverArrowEdge {
    Top,
    #[default]
    Bottom,
    Leading,
    Trailing,
}

/// A controlled native SwiftUI popover with explicit trigger and content slots.
#[derive(Clone, Debug)]
pub struct SwiftUiPopover {
    pub id: u64,
    pub is_presented: bool,
    pub attachment_anchor: SwiftUiPopoverAttachmentAnchor,
    pub arrow_edge: SwiftUiPopoverArrowEdge,
    pub trigger: Vec<SwiftUiElement>,
    pub content: Vec<SwiftUiElement>,
    pub test_id: Option<Arc<str>>,
}

impl SwiftUiPopover {
    pub fn new(id: u64) -> Self {
        Self {
            id,
            is_presented: false,
            attachment_anchor: SwiftUiPopoverAttachmentAnchor::Center,
            arrow_edge: SwiftUiPopoverArrowEdge::Bottom,
            trigger: Vec::new(),
            content: Vec::new(),
            test_id: None,
        }
    }
}

/// A node in the native SwiftUI descriptor tree.
#[derive(Clone, Debug)]
pub enum SwiftUiElement {
    Button(SwiftUiButton),
    Slider(SwiftUiSlider),
    Toggle(SwiftUiToggle),
    ProgressView(SwiftUiProgressView),
    Stepper(SwiftUiStepper),
    TextField(SwiftUiTextField),
    Picker(SwiftUiPicker),
    DatePicker(SwiftUiDatePicker),
    ColorPicker(SwiftUiColorPicker),
    Gauge(SwiftUiGauge),
    QuickGuiHost(SwiftUiQuickGuiHost),
    Popover(SwiftUiPopover),
}

impl From<SwiftUiButton> for SwiftUiElement {
    fn from(value: SwiftUiButton) -> Self {
        Self::Button(value)
    }
}

impl From<SwiftUiSlider> for SwiftUiElement {
    fn from(value: SwiftUiSlider) -> Self {
        Self::Slider(value)
    }
}

impl From<SwiftUiToggle> for SwiftUiElement {
    fn from(value: SwiftUiToggle) -> Self {
        Self::Toggle(value)
    }
}

impl From<SwiftUiProgressView> for SwiftUiElement {
    fn from(value: SwiftUiProgressView) -> Self {
        Self::ProgressView(value)
    }
}

impl From<SwiftUiStepper> for SwiftUiElement {
    fn from(value: SwiftUiStepper) -> Self {
        Self::Stepper(value)
    }
}

impl From<SwiftUiTextField> for SwiftUiElement {
    fn from(value: SwiftUiTextField) -> Self {
        Self::TextField(value)
    }
}

impl From<SwiftUiPicker> for SwiftUiElement {
    fn from(value: SwiftUiPicker) -> Self {
        Self::Picker(value)
    }
}

impl From<SwiftUiDatePicker> for SwiftUiElement {
    fn from(value: SwiftUiDatePicker) -> Self {
        Self::DatePicker(value)
    }
}

impl From<SwiftUiColorPicker> for SwiftUiElement {
    fn from(value: SwiftUiColorPicker) -> Self {
        Self::ColorPicker(value)
    }
}

impl From<SwiftUiGauge> for SwiftUiElement {
    fn from(value: SwiftUiGauge) -> Self {
        Self::Gauge(value)
    }
}

impl SwiftUiButton {
    pub fn new(id: u64) -> Self {
        Self {
            id,
            label: None,
            system_image: None,
            role: SwiftUiButtonRole::Default,
            target: None,
            test_id: None,
            modifiers: Vec::new(),
            has_action: false,
        }
    }

    pub fn label(mut self, label: impl Into<Arc<str>>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn system_image(mut self, name: impl Into<Arc<str>>) -> Self {
        self.system_image = Some(name.into());
        self
    }

    pub fn role(mut self, role: SwiftUiButtonRole) -> Self {
        self.role = role;
        self
    }

    pub fn target(mut self, target: impl Into<Arc<str>>) -> Self {
        self.target = Some(target.into());
        self
    }

    pub fn test_id(mut self, test_id: impl Into<Arc<str>>) -> Self {
        self.test_id = Some(test_id.into());
        self
    }

    pub fn modifier(mut self, modifier: SwiftUiModifier) -> Self {
        self.modifiers.push(modifier);
        self
    }

    pub fn modifiers(mut self, modifiers: impl IntoIterator<Item = SwiftUiModifier>) -> Self {
        self.modifiers.extend(modifiers);
        self
    }

    /// Convenience builder equivalent to Expo's `buttonStyle` modifier.
    pub fn style(mut self, style: SwiftUiButtonStyle) -> Self {
        self.modifiers.push(SwiftUiModifier::ButtonStyle(style));
        self
    }

    /// Convenience builder equivalent to Expo's `controlSize` modifier.
    pub fn control_size(mut self, size: SwiftUiControlSize) -> Self {
        self.modifiers.push(SwiftUiModifier::ControlSize(size));
        self
    }

    /// Convenience builder equivalent to Expo's `disabled` modifier.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.modifiers.push(SwiftUiModifier::Disabled(disabled));
        self
    }

    pub fn action(mut self, enabled: bool) -> Self {
        self.has_action = enabled;
        self
    }
}

impl SwiftUiSlider {
    pub fn new(id: u64, value: f64) -> Self {
        Self {
            id,
            value,
            minimum: 0.0,
            maximum: 1.0,
            step: None,
            label: None,
            test_id: None,
            modifiers: Vec::new(),
            has_value_change: false,
        }
    }

    pub fn range(mut self, minimum: f64, maximum: f64) -> Self {
        self.minimum = minimum;
        self.maximum = maximum;
        self
    }

    pub fn step(mut self, step: f64) -> Self {
        self.step = Some(step);
        self
    }

    pub fn label(mut self, label: impl Into<Arc<str>>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn test_id(mut self, test_id: impl Into<Arc<str>>) -> Self {
        self.test_id = Some(test_id.into());
        self
    }

    pub fn modifier(mut self, modifier: SwiftUiModifier) -> Self {
        self.modifiers.push(modifier);
        self
    }

    pub fn modifiers(mut self, modifiers: impl IntoIterator<Item = SwiftUiModifier>) -> Self {
        self.modifiers.extend(modifiers);
        self
    }

    pub fn on_value_change(mut self, enabled: bool) -> Self {
        self.has_value_change = enabled;
        self
    }
}

impl SwiftUiToggle {
    pub fn new(id: u64, is_on: bool) -> Self {
        Self {
            id,
            is_on,
            label: None,
            test_id: None,
            modifiers: Vec::new(),
            has_value_change: false,
        }
    }

    pub fn label(mut self, label: impl Into<Arc<str>>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn test_id(mut self, test_id: impl Into<Arc<str>>) -> Self {
        self.test_id = Some(test_id.into());
        self
    }

    pub fn modifier(mut self, modifier: SwiftUiModifier) -> Self {
        self.modifiers.push(modifier);
        self
    }

    pub fn modifiers(mut self, modifiers: impl IntoIterator<Item = SwiftUiModifier>) -> Self {
        self.modifiers.extend(modifiers);
        self
    }

    pub fn on_value_change(mut self, enabled: bool) -> Self {
        self.has_value_change = enabled;
        self
    }
}

impl SwiftUiProgressView {
    pub fn new(id: u64, value: f64, total: f64) -> Self {
        Self {
            id,
            value: Some(value),
            total,
            label: None,
            current_value_label: None,
            test_id: None,
            modifiers: Vec::new(),
        }
    }

    pub fn indeterminate(id: u64) -> Self {
        Self {
            id,
            value: None,
            total: 1.0,
            label: None,
            current_value_label: None,
            test_id: None,
            modifiers: Vec::new(),
        }
    }

    pub fn label(mut self, label: impl Into<Arc<str>>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn current_value_label(mut self, label: impl Into<Arc<str>>) -> Self {
        self.current_value_label = Some(label.into());
        self
    }

    pub fn test_id(mut self, test_id: impl Into<Arc<str>>) -> Self {
        self.test_id = Some(test_id.into());
        self
    }

    pub fn modifier(mut self, modifier: SwiftUiModifier) -> Self {
        self.modifiers.push(modifier);
        self
    }

    pub fn modifiers(mut self, modifiers: impl IntoIterator<Item = SwiftUiModifier>) -> Self {
        self.modifiers.extend(modifiers);
        self
    }
}

impl SwiftUiStepper {
    pub fn new(id: u64, value: f64) -> Self {
        Self {
            id,
            value,
            minimum: 0.0,
            maximum: 100.0,
            step: 1.0,
            label: None,
            test_id: None,
            modifiers: Vec::new(),
            has_value_change: false,
        }
    }

    pub fn range(mut self, minimum: f64, maximum: f64) -> Self {
        self.minimum = minimum;
        self.maximum = maximum;
        self
    }

    pub fn step(mut self, step: f64) -> Self {
        self.step = step;
        self
    }

    pub fn label(mut self, label: impl Into<Arc<str>>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn test_id(mut self, test_id: impl Into<Arc<str>>) -> Self {
        self.test_id = Some(test_id.into());
        self
    }

    pub fn modifier(mut self, modifier: SwiftUiModifier) -> Self {
        self.modifiers.push(modifier);
        self
    }

    pub fn modifiers(mut self, modifiers: impl IntoIterator<Item = SwiftUiModifier>) -> Self {
        self.modifiers.extend(modifiers);
        self
    }

    pub fn on_value_change(mut self, enabled: bool) -> Self {
        self.has_value_change = enabled;
        self
    }
}

impl SwiftUiTextField {
    pub fn new(id: u64, text: impl Into<Arc<str>>) -> Self {
        Self {
            id,
            text: text.into(),
            placeholder: None,
            secure: false,
            test_id: None,
            modifiers: Vec::new(),
            has_value_change: false,
            has_submit: false,
        }
    }

    pub fn placeholder(mut self, placeholder: impl Into<Arc<str>>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    pub fn secure(mut self, secure: bool) -> Self {
        self.secure = secure;
        self
    }

    pub fn test_id(mut self, test_id: impl Into<Arc<str>>) -> Self {
        self.test_id = Some(test_id.into());
        self
    }

    pub fn modifier(mut self, modifier: SwiftUiModifier) -> Self {
        self.modifiers.push(modifier);
        self
    }

    pub fn modifiers(mut self, modifiers: impl IntoIterator<Item = SwiftUiModifier>) -> Self {
        self.modifiers.extend(modifiers);
        self
    }

    pub fn on_value_change(mut self, enabled: bool) -> Self {
        self.has_value_change = enabled;
        self
    }

    pub fn on_submit(mut self, enabled: bool) -> Self {
        self.has_submit = enabled;
        self
    }
}

impl SwiftUiPickerOption {
    pub fn new(value: impl Into<Arc<str>>, label: impl Into<Arc<str>>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            system_image: None,
            disabled: false,
        }
    }

    pub fn system_image(mut self, name: impl Into<Arc<str>>) -> Self {
        self.system_image = Some(name.into());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl SwiftUiPicker {
    pub fn new(
        id: u64,
        selection: impl Into<Arc<str>>,
        options: impl IntoIterator<Item = SwiftUiPickerOption>,
    ) -> Self {
        Self {
            id,
            selection: selection.into(),
            options: options.into_iter().collect(),
            style: SwiftUiPickerStyle::Automatic,
            label: None,
            test_id: None,
            modifiers: Vec::new(),
            has_value_change: false,
        }
    }

    /// Convenience constructor for SwiftUI's segmented picker style.
    pub fn segmented(
        id: u64,
        selection: impl Into<Arc<str>>,
        options: impl IntoIterator<Item = SwiftUiPickerOption>,
    ) -> Self {
        Self::new(id, selection, options).style(SwiftUiPickerStyle::Segmented)
    }

    /// Convenience constructor for segmented tab navigation.
    pub fn tabs(
        id: u64,
        selection: impl Into<Arc<str>>,
        options: impl IntoIterator<Item = SwiftUiPickerOption>,
    ) -> Self {
        Self::new(id, selection, options).style(SwiftUiPickerStyle::Tabs)
    }

    pub fn style(mut self, style: SwiftUiPickerStyle) -> Self {
        self.style = style;
        self
    }

    pub fn label(mut self, label: impl Into<Arc<str>>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn test_id(mut self, test_id: impl Into<Arc<str>>) -> Self {
        self.test_id = Some(test_id.into());
        self
    }

    pub fn modifier(mut self, modifier: SwiftUiModifier) -> Self {
        self.modifiers.push(modifier);
        self
    }

    pub fn modifiers(mut self, modifiers: impl IntoIterator<Item = SwiftUiModifier>) -> Self {
        self.modifiers.extend(modifiers);
        self
    }

    pub fn on_value_change(mut self, enabled: bool) -> Self {
        self.has_value_change = enabled;
        self
    }
}

impl SwiftUiDatePicker {
    pub fn new(id: u64, value: f64) -> Self {
        Self {
            id,
            value,
            minimum: None,
            maximum: None,
            components: SwiftUiDatePickerComponents::DateAndTime,
            style: SwiftUiDatePickerStyle::Automatic,
            label: None,
            test_id: None,
            modifiers: Vec::new(),
            has_value_change: false,
        }
    }

    pub fn range(mut self, minimum: Option<f64>, maximum: Option<f64>) -> Self {
        self.minimum = minimum;
        self.maximum = maximum;
        self
    }

    pub fn components(mut self, components: SwiftUiDatePickerComponents) -> Self {
        self.components = components;
        self
    }

    pub fn style(mut self, style: SwiftUiDatePickerStyle) -> Self {
        self.style = style;
        self
    }

    pub fn label(mut self, label: impl Into<Arc<str>>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn test_id(mut self, test_id: impl Into<Arc<str>>) -> Self {
        self.test_id = Some(test_id.into());
        self
    }

    pub fn modifier(mut self, modifier: SwiftUiModifier) -> Self {
        self.modifiers.push(modifier);
        self
    }

    pub fn modifiers(mut self, modifiers: impl IntoIterator<Item = SwiftUiModifier>) -> Self {
        self.modifiers.extend(modifiers);
        self
    }

    pub fn on_value_change(mut self, enabled: bool) -> Self {
        self.has_value_change = enabled;
        self
    }
}

impl SwiftUiColorPicker {
    pub fn new(id: u64, selection: impl Into<Arc<str>>) -> Self {
        Self {
            id,
            selection: selection.into(),
            supports_opacity: true,
            label: None,
            test_id: None,
            modifiers: Vec::new(),
            has_value_change: false,
        }
    }

    pub fn supports_opacity(mut self, supports_opacity: bool) -> Self {
        self.supports_opacity = supports_opacity;
        self
    }

    pub fn label(mut self, label: impl Into<Arc<str>>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn test_id(mut self, test_id: impl Into<Arc<str>>) -> Self {
        self.test_id = Some(test_id.into());
        self
    }

    pub fn modifier(mut self, modifier: SwiftUiModifier) -> Self {
        self.modifiers.push(modifier);
        self
    }

    pub fn modifiers(mut self, modifiers: impl IntoIterator<Item = SwiftUiModifier>) -> Self {
        self.modifiers.extend(modifiers);
        self
    }

    pub fn on_value_change(mut self, enabled: bool) -> Self {
        self.has_value_change = enabled;
        self
    }
}

impl SwiftUiGauge {
    pub fn new(id: u64, value: f64) -> Self {
        Self {
            id,
            value,
            minimum: 0.0,
            maximum: 1.0,
            label: None,
            current_value_label: None,
            minimum_value_label: None,
            maximum_value_label: None,
            style: SwiftUiGaugeStyle::Automatic,
            test_id: None,
            modifiers: Vec::new(),
        }
    }

    pub fn range(mut self, minimum: f64, maximum: f64) -> Self {
        self.minimum = minimum;
        self.maximum = maximum;
        self
    }

    pub fn label(mut self, label: impl Into<Arc<str>>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn current_value_label(mut self, label: impl Into<Arc<str>>) -> Self {
        self.current_value_label = Some(label.into());
        self
    }

    pub fn minimum_value_label(mut self, label: impl Into<Arc<str>>) -> Self {
        self.minimum_value_label = Some(label.into());
        self
    }

    pub fn maximum_value_label(mut self, label: impl Into<Arc<str>>) -> Self {
        self.maximum_value_label = Some(label.into());
        self
    }

    pub fn style(mut self, style: SwiftUiGaugeStyle) -> Self {
        self.style = style;
        self
    }

    pub fn test_id(mut self, test_id: impl Into<Arc<str>>) -> Self {
        self.test_id = Some(test_id.into());
        self
    }

    pub fn modifier(mut self, modifier: SwiftUiModifier) -> Self {
        self.modifiers.push(modifier);
        self
    }

    pub fn modifiers(mut self, modifiers: impl IntoIterator<Item = SwiftUiModifier>) -> Self {
        self.modifiers.extend(modifiers);
        self
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BridgePickerOption<'a> {
    value: &'a str,
    label: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_image: Option<&'a str>,
    disabled: bool,
}

impl<'a> From<&'a SwiftUiPickerOption> for BridgePickerOption<'a> {
    fn from(option: &'a SwiftUiPickerOption) -> Self {
        Self {
            value: &option.value,
            label: &option.label,
            system_image: option.system_image.as_deref(),
            disabled: option.disabled,
        }
    }
}

#[derive(Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum BridgeElement<'a> {
    #[serde(rename = "button")]
    Button {
        id: u64,
        #[serde(skip_serializing_if = "Option::is_none")]
        label: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        system_image: Option<&'a str>,
        role: SwiftUiButtonRole,
        #[serde(skip_serializing_if = "Option::is_none")]
        target: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        test_id: Option<&'a str>,
        modifiers: Vec<BridgeModifier<'a>>,
        has_action: bool,
    },
    #[serde(rename = "slider")]
    Slider {
        id: u64,
        value: f64,
        minimum: f64,
        maximum: f64,
        #[serde(skip_serializing_if = "Option::is_none")]
        step: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        label: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        test_id: Option<&'a str>,
        modifiers: Vec<BridgeModifier<'a>>,
        has_value_change: bool,
    },
    #[serde(rename = "toggle")]
    Toggle {
        id: u64,
        is_on: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        label: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        test_id: Option<&'a str>,
        modifiers: Vec<BridgeModifier<'a>>,
        has_value_change: bool,
    },
    #[serde(rename = "progressView")]
    ProgressView {
        id: u64,
        #[serde(skip_serializing_if = "Option::is_none")]
        value: Option<f64>,
        total: f64,
        #[serde(skip_serializing_if = "Option::is_none")]
        label: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        current_value_label: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        test_id: Option<&'a str>,
        modifiers: Vec<BridgeModifier<'a>>,
    },
    #[serde(rename = "stepper")]
    Stepper {
        id: u64,
        value: f64,
        minimum: f64,
        maximum: f64,
        step: f64,
        #[serde(skip_serializing_if = "Option::is_none")]
        label: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        test_id: Option<&'a str>,
        modifiers: Vec<BridgeModifier<'a>>,
        has_value_change: bool,
    },
    #[serde(rename = "textField")]
    TextField {
        id: u64,
        text: &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        placeholder: Option<&'a str>,
        secure: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        test_id: Option<&'a str>,
        modifiers: Vec<BridgeModifier<'a>>,
        has_value_change: bool,
        has_submit: bool,
    },
    #[serde(rename = "picker")]
    Picker {
        id: u64,
        selection: &'a str,
        options: Vec<BridgePickerOption<'a>>,
        style: SwiftUiPickerStyle,
        #[serde(skip_serializing_if = "Option::is_none")]
        label: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        test_id: Option<&'a str>,
        modifiers: Vec<BridgeModifier<'a>>,
        has_value_change: bool,
    },
    #[serde(rename = "datePicker")]
    DatePicker {
        id: u64,
        value: f64,
        #[serde(skip_serializing_if = "Option::is_none")]
        minimum: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        maximum: Option<f64>,
        components: SwiftUiDatePickerComponents,
        style: SwiftUiDatePickerStyle,
        #[serde(skip_serializing_if = "Option::is_none")]
        label: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        test_id: Option<&'a str>,
        modifiers: Vec<BridgeModifier<'a>>,
        has_value_change: bool,
    },
    #[serde(rename = "colorPicker")]
    ColorPicker {
        id: u64,
        selection: &'a str,
        supports_opacity: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        label: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        test_id: Option<&'a str>,
        modifiers: Vec<BridgeModifier<'a>>,
        has_value_change: bool,
    },
    #[serde(rename = "gauge")]
    Gauge {
        id: u64,
        value: f64,
        minimum: f64,
        maximum: f64,
        #[serde(skip_serializing_if = "Option::is_none")]
        label: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        current_value_label: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        minimum_value_label: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        maximum_value_label: Option<&'a str>,
        style: SwiftUiGaugeStyle,
        #[serde(skip_serializing_if = "Option::is_none")]
        test_id: Option<&'a str>,
        modifiers: Vec<BridgeModifier<'a>>,
    },
    #[serde(rename = "quickGuiHost")]
    QuickGuiHost {
        id: u64,
        match_horizontal: bool,
        match_vertical: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        width: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        height: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        test_id: Option<&'a str>,
    },
    #[serde(rename = "popover")]
    Popover {
        id: u64,
        is_presented: bool,
        attachment_anchor: SwiftUiPopoverAttachmentAnchor,
        arrow_edge: SwiftUiPopoverArrowEdge,
        trigger: Vec<BridgeElement<'a>>,
        content: Vec<BridgeElement<'a>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        test_id: Option<&'a str>,
    },
}

impl<'a> From<&'a SwiftUiElement> for BridgeElement<'a> {
    fn from(element: &'a SwiftUiElement) -> Self {
        match element {
            SwiftUiElement::Button(button) => Self::Button {
                id: button.id,
                label: button.label.as_deref(),
                system_image: button.system_image.as_deref(),
                role: button.role,
                target: button.target.as_deref(),
                test_id: button.test_id.as_deref(),
                modifiers: button.modifiers.iter().map(BridgeModifier::from).collect(),
                has_action: button.has_action,
            },
            SwiftUiElement::Slider(slider) => Self::Slider {
                id: slider.id,
                value: slider.value,
                minimum: slider.minimum,
                maximum: slider.maximum,
                step: slider.step,
                label: slider.label.as_deref(),
                test_id: slider.test_id.as_deref(),
                modifiers: slider.modifiers.iter().map(BridgeModifier::from).collect(),
                has_value_change: slider.has_value_change,
            },
            SwiftUiElement::Toggle(toggle) => Self::Toggle {
                id: toggle.id,
                is_on: toggle.is_on,
                label: toggle.label.as_deref(),
                test_id: toggle.test_id.as_deref(),
                modifiers: toggle.modifiers.iter().map(BridgeModifier::from).collect(),
                has_value_change: toggle.has_value_change,
            },
            SwiftUiElement::ProgressView(progress) => Self::ProgressView {
                id: progress.id,
                value: progress.value,
                total: progress.total,
                label: progress.label.as_deref(),
                current_value_label: progress.current_value_label.as_deref(),
                test_id: progress.test_id.as_deref(),
                modifiers: progress
                    .modifiers
                    .iter()
                    .map(BridgeModifier::from)
                    .collect(),
            },
            SwiftUiElement::Stepper(stepper) => Self::Stepper {
                id: stepper.id,
                value: stepper.value,
                minimum: stepper.minimum,
                maximum: stepper.maximum,
                step: stepper.step,
                label: stepper.label.as_deref(),
                test_id: stepper.test_id.as_deref(),
                modifiers: stepper.modifiers.iter().map(BridgeModifier::from).collect(),
                has_value_change: stepper.has_value_change,
            },
            SwiftUiElement::TextField(field) => Self::TextField {
                id: field.id,
                text: &field.text,
                placeholder: field.placeholder.as_deref(),
                secure: field.secure,
                test_id: field.test_id.as_deref(),
                modifiers: field.modifiers.iter().map(BridgeModifier::from).collect(),
                has_value_change: field.has_value_change,
                has_submit: field.has_submit,
            },
            SwiftUiElement::Picker(picker) => Self::Picker {
                id: picker.id,
                selection: &picker.selection,
                options: picker
                    .options
                    .iter()
                    .map(BridgePickerOption::from)
                    .collect(),
                style: picker.style,
                label: picker.label.as_deref(),
                test_id: picker.test_id.as_deref(),
                modifiers: picker.modifiers.iter().map(BridgeModifier::from).collect(),
                has_value_change: picker.has_value_change,
            },
            SwiftUiElement::DatePicker(picker) => Self::DatePicker {
                id: picker.id,
                value: picker.value,
                minimum: picker.minimum,
                maximum: picker.maximum,
                components: picker.components,
                style: picker.style,
                label: picker.label.as_deref(),
                test_id: picker.test_id.as_deref(),
                modifiers: picker.modifiers.iter().map(BridgeModifier::from).collect(),
                has_value_change: picker.has_value_change,
            },
            SwiftUiElement::ColorPicker(picker) => Self::ColorPicker {
                id: picker.id,
                selection: &picker.selection,
                supports_opacity: picker.supports_opacity,
                label: picker.label.as_deref(),
                test_id: picker.test_id.as_deref(),
                modifiers: picker.modifiers.iter().map(BridgeModifier::from).collect(),
                has_value_change: picker.has_value_change,
            },
            SwiftUiElement::Gauge(gauge) => Self::Gauge {
                id: gauge.id,
                value: gauge.value,
                minimum: gauge.minimum,
                maximum: gauge.maximum,
                label: gauge.label.as_deref(),
                current_value_label: gauge.current_value_label.as_deref(),
                minimum_value_label: gauge.minimum_value_label.as_deref(),
                maximum_value_label: gauge.maximum_value_label.as_deref(),
                style: gauge.style,
                test_id: gauge.test_id.as_deref(),
                modifiers: gauge.modifiers.iter().map(BridgeModifier::from).collect(),
            },
            SwiftUiElement::QuickGuiHost(host) => Self::QuickGuiHost {
                id: host.id,
                match_horizontal: host.match_horizontal,
                match_vertical: host.match_vertical,
                width: host.width,
                height: host.height,
                test_id: host.test_id.as_deref(),
            },
            SwiftUiElement::Popover(popover) => Self::Popover {
                id: popover.id,
                is_presented: popover.is_presented,
                attachment_anchor: popover.attachment_anchor,
                arrow_edge: popover.arrow_edge,
                trigger: popover.trigger.iter().map(Self::from).collect(),
                content: popover.content.iter().map(Self::from).collect(),
                test_id: popover.test_id.as_deref(),
            },
        }
    }
}

#[derive(Serialize)]
#[serde(tag = "$type")]
enum BridgeModifier<'a> {
    #[serde(rename = "buttonStyle")]
    ButtonStyle { style: SwiftUiButtonStyle },
    #[serde(rename = "buttonBorderShape")]
    ButtonBorderShape {
        shape: SwiftUiButtonBorderShape,
        #[serde(rename = "cornerRadius", skip_serializing_if = "Option::is_none")]
        corner_radius: Option<f32>,
    },
    #[serde(rename = "controlSize")]
    ControlSize { size: SwiftUiControlSize },
    #[serde(rename = "labelStyle")]
    LabelStyle { style: SwiftUiLabelStyle },
    #[serde(rename = "tint")]
    Tint { color: &'a str },
    #[serde(rename = "disabled")]
    Disabled { disabled: bool },
}

impl<'a> From<&'a SwiftUiModifier> for BridgeModifier<'a> {
    fn from(modifier: &'a SwiftUiModifier) -> Self {
        match modifier {
            SwiftUiModifier::ButtonStyle(style) => Self::ButtonStyle { style: *style },
            SwiftUiModifier::ButtonBorderShape {
                shape,
                corner_radius,
            } => Self::ButtonBorderShape {
                shape: *shape,
                corner_radius: *corner_radius,
            },
            SwiftUiModifier::ControlSize(size) => Self::ControlSize { size: *size },
            SwiftUiModifier::LabelStyle(style) => Self::LabelStyle { style: *style },
            SwiftUiModifier::Tint(color) => Self::Tint { color },
            SwiftUiModifier::Disabled(disabled) => Self::Disabled {
                disabled: *disabled,
            },
        }
    }
}

#[allow(clippy::type_complexity)]
struct ActionContext {
    callback: Box<dyn Fn(u64)>,
    presentation_callback: Box<dyn Fn(u64, bool)>,
    value_callback: Box<dyn Fn(u64, &str)>,
    submit_callback: Box<dyn Fn(u64)>,
}

unsafe extern "C" fn dispatch_action(context: *mut c_void, id: u64) {
    let Some(context) = NonNull::new(context).map(|context| context.cast::<ActionContext>()) else {
        return;
    };
    // An application callback must never unwind across Swift's C ABI boundary.
    let _ = catch_unwind(AssertUnwindSafe(|| unsafe {
        (context.as_ref().callback)(id);
    }));
}

unsafe extern "C" fn dispatch_presentation(context: *mut c_void, id: u64, presented: bool) {
    let Some(context) = NonNull::new(context).map(|context| context.cast::<ActionContext>()) else {
        return;
    };
    let _ = catch_unwind(AssertUnwindSafe(|| unsafe {
        (context.as_ref().presentation_callback)(id, presented);
    }));
}

unsafe extern "C" fn dispatch_value(context: *mut c_void, id: u64, value: *const c_char) {
    let Some(context) = NonNull::new(context).map(|context| context.cast::<ActionContext>()) else {
        return;
    };
    let Some(value) = NonNull::new(value.cast_mut()) else {
        return;
    };
    let value = unsafe { CStr::from_ptr(value.as_ptr()) }.to_string_lossy();
    let _ = catch_unwind(AssertUnwindSafe(|| unsafe {
        (context.as_ref().value_callback)(id, &value);
    }));
}

unsafe extern "C" fn dispatch_submit(context: *mut c_void, id: u64) {
    let Some(context) = NonNull::new(context).map(|context| context.cast::<ActionContext>()) else {
        return;
    };
    let _ = catch_unwind(AssertUnwindSafe(|| unsafe {
        (context.as_ref().submit_callback)(id);
    }));
}

/// A retained `NSHostingView` whose SwiftUI contents are synchronized from Rust descriptors.
///
/// Construction, updates, sizing, and destruction must happen on AppKit's main thread. QuickGUI's
/// runtime satisfies that contract when this host is used from a retained view.
pub struct MacSwiftUiHost {
    handle: NonNull<c_void>,
    view: MacNativeView,
    action_context: Box<ActionContext>,
    last_payload: String,
    embedded_ids: HashSet<u64>,
}

impl MacSwiftUiHost {
    pub fn new(action: impl Fn(u64) + 'static) -> Result<Self, String> {
        Self::new_with_events(action, |_id, _presented| {})
    }

    pub fn new_with_events(
        action: impl Fn(u64) + 'static,
        presentation: impl Fn(u64, bool) + 'static,
    ) -> Result<Self, String> {
        Self::new_with_control_events(action, presentation, |_id, _value| {}, |_id| {})
    }

    /// Construct a host that also reports controlled value changes and text-field submissions.
    pub fn new_with_control_events(
        action: impl Fn(u64) + 'static,
        presentation: impl Fn(u64, bool) + 'static,
        value: impl Fn(u64, &str) + 'static,
        submit: impl Fn(u64) + 'static,
    ) -> Result<Self, String> {
        require_main_thread()?;
        let mut action_context = Box::new(ActionContext {
            callback: Box::new(action),
            presentation_callback: Box::new(presentation),
            value_callback: Box::new(value),
            submit_callback: Box::new(submit),
        });
        let context = (&mut *action_context as *mut ActionContext).cast::<c_void>();
        let handle = NonNull::new(unsafe {
            quickgui_swift_ui_host_create(
                context,
                Some(dispatch_action),
                Some(dispatch_presentation),
                Some(dispatch_value),
                Some(dispatch_submit),
            )
        })
        .ok_or_else(|| "SwiftUI did not create an NSHostingView".to_owned())?;
        let view_pointer = NonNull::new(unsafe { quickgui_swift_ui_host_view(handle.as_ptr()) })
            .ok_or_else(|| {
                unsafe { quickgui_swift_ui_host_release(handle.as_ptr()) };
                "SwiftUI created a host without an NSView".to_owned()
            })?;
        let view = unsafe { &*view_pointer.as_ptr().cast::<objc2_app_kit::NSView>() };
        Ok(Self {
            handle,
            view: MacNativeView::new(view),
            action_context,
            last_payload: String::new(),
            embedded_ids: HashSet::new(),
        })
    }

    pub fn view(&self) -> &objc2_app_kit::NSView {
        self.view.as_ns_view()
    }

    /// Synchronize the complete ordered SwiftUI descriptor tree. Returns whether the native root
    /// view or one of its embedded QuickGUI surfaces changed.
    pub fn sync(&mut self, elements: &[SwiftUiElement]) -> Result<bool, String> {
        require_main_thread()?;
        let bridge = elements.iter().map(BridgeElement::from).collect::<Vec<_>>();
        let payload = serde_json::to_string(&bridge)
            .map_err(|error| format!("could not encode SwiftUI elements: {error}"))?;

        let mut embedded = Vec::new();
        collect_embedded_views(elements, &mut embedded);
        let next_ids = embedded.iter().map(|(id, _)| *id).collect::<HashSet<_>>();
        let removed = self
            .embedded_ids
            .difference(&next_ids)
            .copied()
            .collect::<Vec<_>>();
        for id in removed {
            unsafe { quickgui_swift_ui_host_remove_embedded_view(self.handle.as_ptr(), id) };
        }
        let mut changed = next_ids != self.embedded_ids;
        for (id, embedded) in embedded {
            let Some(view) = embedded.view() else {
                continue;
            };
            let size = embedded.content_size();
            if !unsafe {
                quickgui_swift_ui_host_set_embedded_view(
                    self.handle.as_ptr(),
                    id,
                    view.as_ns_view() as *const _ as *mut c_void,
                    f64::from(size.width),
                    f64::from(size.height),
                )
            } {
                return Err(format!("SwiftUI rejected embedded QuickGUI view {id}"));
            }
            changed = true;
        }
        self.embedded_ids = next_ids;

        if payload != self.last_payload {
            let json = CString::new(payload.as_str())
                .map_err(|_| "encoded SwiftUI elements unexpectedly contained NUL".to_owned())?;
            if !unsafe { quickgui_swift_ui_host_update(self.handle.as_ptr(), json.as_ptr()) } {
                return Err("SwiftUI rejected its Rust element description".to_owned());
            }
            self.last_payload = payload;
            changed = true;
        }
        Ok(changed)
    }

    /// Compatibility convenience for the original button-only bridge.
    pub fn sync_buttons(&mut self, buttons: &[SwiftUiButton]) -> Result<bool, String> {
        let elements = buttons
            .iter()
            .cloned()
            .map(SwiftUiElement::Button)
            .collect::<Vec<_>>();
        self.sync(&elements)
    }

    /// Current fitting size of the hosted SwiftUI controls at their native size.
    ///
    /// The headroom the host keeps around content whose effects draw past its bounds, such as
    /// Liquid Glass, is excluded; [`Self::effect_inset`] reports it, and placing the view with
    /// [`crate::native_view_with_outset`] keeps it outside layout.
    pub fn fitting_size(&self) -> Result<Size, String> {
        require_main_thread()?;
        let mut width = 0.0;
        let mut height = 0.0;
        unsafe {
            quickgui_swift_ui_host_fitting_size(self.handle.as_ptr(), &mut width, &mut height);
        }
        if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
            return Err(format!(
                "SwiftUI returned an invalid fitting size {width}x{height}"
            ));
        }
        let inset = f64::from(self.effect_inset()?) * 2.0;
        Ok(Size::new(
            (width - inset).max(1.0) as f32,
            (height - inset).max(1.0) as f32,
        ))
    }

    /// Padding the host keeps on each side of its content for effects that draw past a control's
    /// bounds; zero unless the content contains such a control.
    pub fn effect_inset(&self) -> Result<f32, String> {
        require_main_thread()?;
        let inset = unsafe { quickgui_swift_ui_host_effect_inset(self.handle.as_ptr()) };
        if !inset.is_finite() || inset < 0.0 {
            return Err(format!("SwiftUI returned an invalid effect inset {inset}"));
        }
        Ok(inset as f32)
    }
}

fn collect_embedded_views<'a>(
    elements: &'a [SwiftUiElement],
    output: &mut Vec<(u64, &'a MacEmbeddedView)>,
) {
    for element in elements {
        match element {
            SwiftUiElement::Button(_)
            | SwiftUiElement::Slider(_)
            | SwiftUiElement::Toggle(_)
            | SwiftUiElement::ProgressView(_)
            | SwiftUiElement::Stepper(_)
            | SwiftUiElement::TextField(_)
            | SwiftUiElement::Picker(_)
            | SwiftUiElement::DatePicker(_)
            | SwiftUiElement::ColorPicker(_)
            | SwiftUiElement::Gauge(_) => {}
            SwiftUiElement::QuickGuiHost(host) => {
                if let Some(embedded) = &host.embedded {
                    output.push((host.id, embedded));
                }
            }
            SwiftUiElement::Popover(popover) => {
                collect_embedded_views(&popover.trigger, output);
                collect_embedded_views(&popover.content, output);
            }
        }
    }
}

impl fmt::Debug for MacSwiftUiHost {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MacSwiftUiHost")
            .field("handle", &self.handle)
            .field("view", &self.view)
            .finish_non_exhaustive()
    }
}

impl Drop for MacSwiftUiHost {
    fn drop(&mut self) {
        let _ = &self.action_context;
        unsafe { quickgui_swift_ui_host_release(self.handle.as_ptr()) };
    }
}

fn require_main_thread() -> Result<MainThreadMarker, String> {
    MainThreadMarker::new()
        .ok_or_else(|| "SwiftUI hosts must be accessed on the AppKit main thread".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn form_control_descriptors_encode_the_swift_contract() {
        let elements = vec![
            SwiftUiSlider::new(1, 0.4)
                .range(0.0, 1.0)
                .step(0.1)
                .label("Volume")
                .on_value_change(true)
                .into(),
            SwiftUiToggle::new(2, true)
                .label("Notifications")
                .on_value_change(true)
                .into(),
            SwiftUiProgressView::new(3, 3.0, 10.0)
                .label("Uploading")
                .current_value_label("3 of 10")
                .into(),
            SwiftUiStepper::new(4, 2.0)
                .range(1.0, 5.0)
                .step(1.0)
                .label("Copies")
                .into(),
            SwiftUiTextField::new(5, "secret")
                .placeholder("Password")
                .secure(true)
                .on_value_change(true)
                .on_submit(true)
                .into(),
            SwiftUiPicker::segmented(
                6,
                "grid",
                [
                    SwiftUiPickerOption::new("list", "List").system_image("list.bullet"),
                    SwiftUiPickerOption::new("grid", "Grid").system_image("square.grid.2x2"),
                ],
            )
            .label("Layout")
            .on_value_change(true)
            .into(),
            SwiftUiDatePicker::new(7, 1_725_091_200.25)
                .range(Some(1_704_067_200.0), None)
                .components(SwiftUiDatePickerComponents::Date)
                .style(SwiftUiDatePickerStyle::Field)
                .on_value_change(true)
                .into(),
            SwiftUiColorPicker::new(8, "#3366ffff")
                .label("Accent")
                .supports_opacity(false)
                .on_value_change(true)
                .into(),
            SwiftUiGauge::new(9, 0.72)
                .range(0.0, 1.0)
                .label("Battery")
                .current_value_label("72%")
                .style(SwiftUiGaugeStyle::AccessoryLinearCapacity)
                .into(),
        ];
        let encoded =
            serde_json::to_value(elements.iter().map(BridgeElement::from).collect::<Vec<_>>())
                .unwrap();

        assert_eq!(encoded[0]["type"], "slider");
        assert_eq!(encoded[0]["value"], 0.4);
        assert_eq!(encoded[0]["hasValueChange"], true);
        assert_eq!(encoded[1]["type"], "toggle");
        assert_eq!(encoded[1]["isOn"], true);
        assert_eq!(encoded[2]["type"], "progressView");
        assert_eq!(encoded[2]["currentValueLabel"], "3 of 10");
        assert_eq!(encoded[3]["type"], "stepper");
        assert_eq!(encoded[4]["type"], "textField");
        assert_eq!(encoded[4]["secure"], true);
        assert_eq!(encoded[4]["hasSubmit"], true);
        assert_eq!(encoded[5]["type"], "picker");
        assert_eq!(encoded[5]["style"], "segmented");
        assert_eq!(encoded[5]["selection"], "grid");
        assert_eq!(encoded[5]["options"][0]["systemImage"], "list.bullet");
        assert_eq!(encoded[6]["type"], "datePicker");
        assert_eq!(encoded[6]["components"], "date");
        assert_eq!(encoded[6]["style"], "field");
        assert_eq!(encoded[7]["type"], "colorPicker");
        assert_eq!(encoded[7]["supportsOpacity"], false);
        assert_eq!(encoded[8]["type"], "gauge");
        assert_eq!(encoded[8]["style"], "accessoryLinearCapacity");

        let tabs = SwiftUiPicker::tabs(
            10,
            "list",
            [
                SwiftUiPickerOption::new("list", "List"),
                SwiftUiPickerOption::new("grid", "Grid"),
            ],
        );
        assert_eq!(tabs.style, SwiftUiPickerStyle::Tabs);
    }
}
