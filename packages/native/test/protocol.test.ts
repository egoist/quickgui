import { describe, expect, test } from "bun:test";
import {
  MAX_COLLECTION_JSON_BYTES,
  MAX_COMPONENT_ITEMS,
  MAX_COMPONENT_JSON_BYTES,
  MAX_COMPONENT_VALUE_BYTES,
  MAX_COMPONENT_VALUES,
  MAX_DECLARED_OPTIONS,
  MAX_DECLARED_TREE_NODES,
  MAX_DRAG_JSON_BYTES,
  MAX_FILTERS_PER_ELEMENT,
  MAX_GRADIENT_STOPS,
  MAX_KEYMAP_JSON_BYTES,
  MAX_MENU_JSON_BYTES,
  MAX_MENU_HOVER_DELAY_MS,
  MAX_MENU_ITEMS,
  MAX_MENU_LINK_BYTES,
  MAX_SELECT_VALUES,
  MAX_COMBOBOX_VALUES,
  MAX_AVATAR_FALLBACK_DELAY_MS,
  MAX_CHECKBOX_GROUP_VALUES,
  MAX_DRAWER_SNAP_POINTS,
  MAX_ANCHOR_ALIGN_OFFSET,
  MAX_ANCHOR_COLLISION_PADDING,
  MAX_ANCHOR_SIDE_OFFSET,
  MAX_DIALOG_TRANSITION_MS,
  MAX_FIELD_VALIDATION_DEBOUNCE_MS,
  MAX_MENUBAR_MENUS,
  MAX_NUMBER_FIELD_SCRUB_SENSITIVITY,
  MAX_POPOVER_HOVER_DELAY_MS,
  MAX_TOAST_DURATION_MS,
  MAX_TOAST_SWIPE_THRESHOLD,
  MAX_TOOLTIP_DELAY_MS,
  MAX_TOOLTIP_GROUP_TIMEOUT_MS,
  MAX_NAVIGATION_MENU_DELAY_MS,
  MAX_NAVIGATION_MENU_ITEMS,
  MAX_OPTIONS_JSON_BYTES,
  MAX_OTP_LENGTH,
  MAX_PREVIEW_CARD_DELAY_MS,
  MAX_SCROLL_AREA_OVERFLOW_THRESHOLD,
  MAX_GROUP_STYLES_PER_ELEMENT,
  MAX_HOVER_GROUP_NAME_BYTES,
  MAX_STATE_STYLE_JSON_BYTES,
  MAX_STYLE_DECLARATION_BYTES,
  MAX_TABLE_COLUMNS,
  MAX_TABLE_ROWS,
  MAX_TOASTS,
  MAX_TOOLTIP_TEXT_BYTES,
  MutationBatch,
  NativeNodeTag,
  NativePart,
  NO_ANCHOR,
  PropertyCode,
  PROTOCOL_VERSION,
} from "../src/protocol.ts";

describe("binary mutation protocol", () => {
  test("writes one bounded little-endian batch", () => {
    const batch = new MutationBatch();
    batch.createElement(1, NativeNodeTag.View);
    batch.createText(2, "héllo");
    batch.setProperty(1, PropertyCode.Width, 320);
    batch.insert(1, 2);
    const bytes = batch.finish();
    const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);

    expect(bytes.subarray(0, 4).toString("utf8")).toBe("QGMB");
    expect(view.getUint16(4, true)).toBe(PROTOCOL_VERSION);
    expect(view.getUint32(6, true)).toBe(4);
    expect(view.getUint32(bytes.byteLength - 4, true)).toBe(NO_ANCHOR);
  });

  test("rejects non-finite numeric properties", () => {
    const batch = new MutationBatch();
    expect(() => batch.setProperty(1, PropertyCode.Width, Number.NaN)).toThrow("finite");
  });

  test("encodes native controls, SwiftUI reverse hosts, overlays, terminals, SVGs, paint, and pointer capture under the current protocol", () => {
    expect(PROTOCOL_VERSION).toBe(36);
    const batch = new MutationBatch();
    batch.createElement(1, NativeNodeTag.Input);
    batch.setProperty(1, PropertyCode.Value, "hello");
    batch.setProperty(1, PropertyCode.Password, true);
    batch.createElement(2, NativeNodeTag.Extension);
    batch.setProperty(2, PropertyCode.ExtensionPackage, "third-party");
    batch.setProperty(2, PropertyCode.ExtensionComponent, "document");
    batch.setProperty(2, PropertyCode.ExtensionProps, JSON.stringify({ streaming: true }));
    batch.setProperty(2, PropertyCode.ScrollToEndRevision, 3);
    batch.createElement(3, NativeNodeTag.VirtualList);
    batch.setProperty(3, PropertyCode.EstimatedItemHeight, 180);
    batch.setProperty(3, PropertyCode.FollowMode, "tail");
    batch.createElement(4, NativeNodeTag.View);
    batch.setProperty(4, PropertyCode.AnchorTarget, "1");
    batch.setProperty(4, PropertyCode.AnchorPlacement, "bottom-start");
    batch.setProperty(4, PropertyCode.AnchorGap, 8);
    batch.setProperty(4, PropertyCode.ViewportMargin, 12);
    batch.setProperty(4, PropertyCode.DismissOnEscape, false);
    batch.setProperty(4, PropertyCode.DismissOnPointerOutside, true);
    batch.setProperty(4, PropertyCode.DismissListener, true);
    batch.setProperty(4, PropertyCode.BorderTopWidth, 1);
    batch.setProperty(4, PropertyCode.BorderRightWidth, 2);
    batch.setProperty(4, PropertyCode.BorderBottomWidth, 3);
    batch.setProperty(4, PropertyCode.BorderLeftWidth, 4);
    batch.setProperty(
      4,
      PropertyCode.BoxShadow,
      '[{"offsetX":0,"offsetY":8,"blurRadius":24,"spreadRadius":-8,"color":4278190080,"inset":false}]',
    );
    batch.createElement(5, NativeNodeTag.Extension);
    batch.setProperty(5, PropertyCode.ExtensionPackage, "acme");
    batch.setProperty(5, PropertyCode.ExtensionComponent, "console");
    batch.setProperty(5, PropertyCode.ExtensionProps, JSON.stringify({ program: "/bin/zsh", arguments: ["-l"], environment: { TERM: "xterm-256color" } }));
    batch.setProperty(5, PropertyCode.FontFamily, "JetBrainsMono Nerd Font Mono");
    batch.setProperty(1, PropertyCode.HoverBackgroundColor, 0xff332211, true);
    batch.setProperty(1, PropertyCode.HoverColor, 0xffeeeeee, true);
    batch.setProperty(1, PropertyCode.ActiveBackgroundColor, 0xff221100, true);
    batch.setProperty(1, PropertyCode.ActiveColor, 0xffffffff, true);
    batch.setProperty(1, PropertyCode.Transition, 90);
    batch.setProperty(1, PropertyCode.PointerListener, true);
    batch.setProperty(1, PropertyCode.FocusOnPointer, false);
    batch.setProperty(1, PropertyCode.HitSlop, 2);
    batch.setProperty(1, PropertyCode.HitSlopTop, 3);
    batch.setProperty(1, PropertyCode.HitSlopRight, 4);
    batch.setProperty(1, PropertyCode.HitSlopBottom, 5);
    batch.setProperty(1, PropertyCode.HitSlopLeft, 6);
    batch.setProperty(4, PropertyCode.Overlay, true);
    batch.setProperty(4, PropertyCode.FocusTrap, true);
    batch.setProperty(4, PropertyCode.RestorePreviousFocus, true);
    batch.setProperty(1, PropertyCode.AutoFocus, true);
    batch.setProperty(4, PropertyCode.AccessibilityModal, true);
    batch.createElement(6, NativeNodeTag.Svg);
    batch.setProperty(6, PropertyCode.Value, "<svg/>");
    batch.createElement(7, NativeNodeTag.SwiftUIHost);
    batch.setProperty(7, PropertyCode.SwiftUIMatchContentsHorizontal, true);
    batch.setProperty(7, PropertyCode.SwiftUIMatchContentsVertical, true);
    batch.createElement(8, NativeNodeTag.SwiftUIButton);
    batch.setProperty(8, PropertyCode.SwiftUIButtonStyle, "glass");
    batch.setProperty(8, PropertyCode.SwiftUIControlSize, "large");
    batch.setProperty(8, PropertyCode.SwiftUISystemImage, "square.and.arrow.down");
    batch.setProperty(8, PropertyCode.SwiftUITarget, "save");
    batch.setProperty(8, PropertyCode.SwiftUITestId, "save-button");
    batch.setProperty(
      8,
      PropertyCode.SwiftUIModifiers,
      JSON.stringify([{ $type: "buttonStyle", style: "glass" }]),
    );
    batch.createElement(9, NativeNodeTag.SwiftUIQuickGUIHost);
    batch.setProperty(9, PropertyCode.SwiftUIEmbeddedWindow, 2);
    batch.createElement(10, NativeNodeTag.SwiftUIPopover);
    batch.setProperty(10, PropertyCode.SwiftUIIsPresented, true);
    batch.setProperty(10, PropertyCode.SwiftUIAttachmentAnchor, "bottom");
    batch.setProperty(10, PropertyCode.SwiftUIArrowEdge, "top");
    batch.createElement(11, NativeNodeTag.SwiftUIPopoverTrigger);
    batch.createElement(12, NativeNodeTag.SwiftUIPopoverContent);
    batch.createElement(13, NativeNodeTag.SwiftUISlider);
    batch.setProperty(13, PropertyCode.Value, 0.5);
    batch.setProperty(13, PropertyCode.Minimum, 0);
    batch.setProperty(13, PropertyCode.Maximum, 1);
    batch.setProperty(13, PropertyCode.Step, 0.1);
    batch.createElement(14, NativeNodeTag.SwiftUIToggle);
    batch.setProperty(14, PropertyCode.Checked, true);
    batch.createElement(15, NativeNodeTag.SwiftUIProgressView);
    batch.setProperty(15, PropertyCode.Value, 0.75);
    batch.setProperty(15, PropertyCode.Maximum, 1);
    batch.createElement(16, NativeNodeTag.SwiftUIStepper);
    batch.setProperty(16, PropertyCode.Value, 2);
    batch.setProperty(16, PropertyCode.Minimum, 0);
    batch.setProperty(16, PropertyCode.Maximum, 10);
    batch.setProperty(16, PropertyCode.Step, 1);
    batch.createElement(17, NativeNodeTag.SwiftUITextField);
    batch.setProperty(17, PropertyCode.Value, "Ada");
    batch.setProperty(17, PropertyCode.Placeholder, "Name");
    batch.setProperty(17, PropertyCode.Password, false);
    batch.setProperty(17, PropertyCode.InputListener, true);
    batch.setProperty(17, PropertyCode.SubmitListener, true);
    batch.createElement(18, NativeNodeTag.SwiftUIPicker);
    batch.setProperty(18, PropertyCode.Value, "grid");
    batch.setProperty(
      18,
      PropertyCode.Items,
      JSON.stringify([
        { value: "list", label: "List" },
        { value: "grid", label: "Grid" },
      ]),
    );
    batch.setProperty(18, PropertyCode.SwiftUIPickerStyle, "segmented");
    batch.setProperty(18, PropertyCode.InputListener, true);
    batch.createElement(19, NativeNodeTag.SwiftUIDatePicker);
    batch.setProperty(19, PropertyCode.CivilValue, "1725091200.25");
    batch.setProperty(19, PropertyCode.CivilMinimum, "1704067200");
    batch.setProperty(19, PropertyCode.CivilMaximum, "1767225600");
    batch.setProperty(19, PropertyCode.SwiftUIDatePickerComponents, "date");
    batch.setProperty(19, PropertyCode.SwiftUIDatePickerStyle, "field");
    batch.setProperty(19, PropertyCode.InputListener, true);
    batch.createElement(20, NativeNodeTag.SwiftUIColorPicker);
    batch.setProperty(20, PropertyCode.Value, "#3366ffff");
    batch.setProperty(20, PropertyCode.SwiftUIColorSupportsOpacity, false);
    batch.setProperty(20, PropertyCode.InputListener, true);
    batch.createElement(21, NativeNodeTag.SwiftUIGauge);
    batch.setProperty(21, PropertyCode.Value, 0.72);
    batch.setProperty(21, PropertyCode.Minimum, 0);
    batch.setProperty(21, PropertyCode.Maximum, 1);
    batch.setProperty(21, PropertyCode.ValueText, "72%");
    batch.setProperty(21, PropertyCode.SwiftUIGaugeMinimumValueLabel, "0%");
    batch.setProperty(21, PropertyCode.SwiftUIGaugeMaximumValueLabel, "100%");
    batch.setProperty(21, PropertyCode.SwiftUIGaugeStyle, "accessoryLinearCapacity");
    expect(batch.mutationCount).toBe(111);
    expect(batch.finish().byteLength).toBeGreaterThan(10);
  });

  test("encodes selection, tab, disclosure, field, and tooltip component parts", () => {
    const batch = new MutationBatch();
    batch.createElement(1, NativeNodeTag.Button);
    batch.setProperty(1, PropertyCode.Part, NativePart.Checkbox);
    batch.setProperty(1, PropertyCode.Checked, false);
    batch.setProperty(1, PropertyCode.Indeterminate, true);
    batch.createElement(2, NativeNodeTag.Button);
    batch.setProperty(2, PropertyCode.Part, NativePart.Tab);
    batch.setProperty(2, PropertyCode.Scope, "qg-tabs-1");
    batch.setProperty(2, PropertyCode.PartValue, "overview");
    batch.setProperty(2, PropertyCode.ActiveValue, "overview");
    batch.setProperty(2, PropertyCode.Orientation, "vertical");
    batch.setProperty(2, PropertyCode.ActivateOnFocus, true);
    batch.setProperty(2, PropertyCode.LoopFocus, false);
    batch.setProperty(2, PropertyCode.KeepMounted, true);
    batch.createElement(3, NativeNodeTag.View);
    batch.setProperty(3, PropertyCode.Part, NativePart.AccordionItem);
    batch.setProperty(3, PropertyCode.Open, true);
    batch.setProperty(3, PropertyCode.ItemIndex, 2);
    batch.setProperty(3, PropertyCode.HeadingLevel, 4);
    batch.createElement(4, NativeNodeTag.Input);
    batch.setProperty(4, PropertyCode.Part, NativePart.FieldControl);
    batch.setProperty(4, PropertyCode.Required, true);
    batch.setProperty(4, PropertyCode.Invalid, true);
    batch.setProperty(4, PropertyCode.Touched, true);
    batch.setProperty(4, PropertyCode.Dirty, true);
    batch.setProperty(4, PropertyCode.Filled, false);
    batch.setProperty(4, PropertyCode.ValidationMessage, "Enter an address");
    batch.setProperty(4, PropertyCode.Tooltip, "We never share it");
    batch.setProperty(4, PropertyCode.TooltipPlacement, "top");
    batch.setProperty(4, PropertyCode.TooltipDelay, 250);
    batch.setProperty(4, PropertyCode.TooltipGap, 7);
    batch.setProperty(4, PropertyCode.TooltipViewportMargin, 8);
    batch.createElement(5, NativeNodeTag.View);
    batch.setProperty(5, PropertyCode.Part, NativePart.DialogPopup);
    batch.setProperty(5, PropertyCode.Variant, "alertdialog");
    batch.setProperty(5, PropertyCode.Open, true);
    batch.setProperty(5, PropertyCode.DismissOnEscape, true);
    batch.setProperty(5, PropertyCode.DismissOnPointerOutside, false);

    expect(batch.mutationCount).toBe(37);
    expect(batch.finish().byteLength).toBeGreaterThan(10);
  });

  test("encodes declared popover and context menus", () => {
    const batch = new MutationBatch();
    batch.createElement(1, NativeNodeTag.Button);
    batch.setProperty(1, PropertyCode.Part, NativePart.PopoverMenuTrigger);
    batch.setProperty(1, PropertyCode.Open, true);
    batch.setProperty(1, PropertyCode.Controls, "2");
    batch.createElement(2, NativeNodeTag.View);
    batch.setProperty(2, PropertyCode.Part, NativePart.PopoverMenuPopup);
    batch.setProperty(
      2,
      PropertyCode.Menu,
      JSON.stringify({ items: [{ id: "open", label: "Open" }] }),
    );
    batch.setProperty(2, PropertyCode.SelectListener, true);
    batch.createElement(3, NativeNodeTag.View);
    batch.setProperty(3, PropertyCode.Part, NativePart.ContextMenuTrigger);
    batch.setProperty(3, PropertyCode.Menu, JSON.stringify({ items: [] }));
    batch.setProperty(3, PropertyCode.SelectListener, true);

    expect(batch.mutationCount).toBe(12);
    expect(batch.finish().byteLength).toBeGreaterThan(10);
  });

  test("encodes CSS grid, full transitions, images, shaders, and range parts", () => {
    const batch = new MutationBatch();
    batch.createElement(1, NativeNodeTag.View);
    batch.setProperty(1, PropertyCode.Display, "grid");
    batch.setProperty(1, PropertyCode.GridTemplateColumns, "200px 1fr");
    batch.setProperty(1, PropertyCode.GridTemplateRows, 3);
    batch.setProperty(1, PropertyCode.GridAutoFlow, "column dense");
    batch.setProperty(1, PropertyCode.GridColumnStart, 2);
    batch.setProperty(1, PropertyCode.GridColumnEnd, 5);
    batch.setProperty(1, PropertyCode.GridRowSpan, 2);
    batch.setProperty(1, PropertyCode.TransitionProperties, "opacity,box-shadow");
    batch.setProperty(1, PropertyCode.TransitionDuration, 180);
    batch.setProperty(1, PropertyCode.TransitionEasing, "ease-out");
    batch.setProperty(1, PropertyCode.TransitionMaxFps, 30);
    batch.createElement(2, NativeNodeTag.Image);
    batch.setProperty(2, PropertyCode.Value, "/assets/logo.png");
    batch.setProperty(2, PropertyCode.ObjectFit, "cover");
    batch.createElement(3, NativeNodeTag.Shader);
    batch.setProperty(3, PropertyCode.ShaderParameters, "[0.5,0,0,1]");
    batch.createElement(4, NativeNodeTag.View);
    batch.setProperty(4, PropertyCode.Part, NativePart.Progress);
    batch.setProperty(4, PropertyCode.Maximum, 12);
    batch.setProperty(4, PropertyCode.ValueText, "3 of 12 files");
    batch.createElement(5, NativeNodeTag.View);
    batch.setProperty(5, PropertyCode.Part, NativePart.Meter);
    batch.setProperty(5, PropertyCode.Minimum, 0);
    batch.setProperty(5, PropertyCode.Low, 25);
    batch.setProperty(5, PropertyCode.High, 75);
    batch.setProperty(5, PropertyCode.Optimum, 90);
    batch.createElement(6, NativeNodeTag.Button);
    batch.setProperty(6, PropertyCode.Part, NativePart.Toggle);
    batch.setProperty(6, PropertyCode.Pressed, true);

    expect(batch.mutationCount).toBe(30);
    expect(batch.finish().byteLength).toBeGreaterThan(10);
  });

  test("encodes declared key, mouse, gesture, keymap, and drag listeners", () => {
    const batch = new MutationBatch();
    batch.createElement(1, NativeNodeTag.View);
    batch.setProperty(1, PropertyCode.TabIndex, 0);
    batch.setProperty(1, PropertyCode.KeyDownListener, true);
    batch.setProperty(1, PropertyCode.KeyUpListener, true);
    batch.setProperty(1, PropertyCode.MouseDownListener, true);
    batch.setProperty(1, PropertyCode.MouseUpListener, true);
    batch.setProperty(1, PropertyCode.MouseMoveListener, true);
    batch.setProperty(1, PropertyCode.DoubleClickListener, true);
    batch.setProperty(1, PropertyCode.ScrollListener, true);
    batch.setProperty(1, PropertyCode.ContextMenuListener, true);
    batch.setProperty(1, PropertyCode.PinchListener, true);
    batch.setProperty(1, PropertyCode.RotationListener, true);
    batch.setProperty(1, PropertyCode.SmartMagnifyListener, true);
    batch.setProperty(1, PropertyCode.PressureListener, true);
    batch.setProperty(1, PropertyCode.FocusListener, true);
    batch.setProperty(1, PropertyCode.Keymap, JSON.stringify({ "CmdOrCtrl+S": "save" }));
    batch.setProperty(1, PropertyCode.ActionListener, true);
    batch.createElement(2, NativeNodeTag.View);
    batch.setProperty(2, PropertyCode.Draggable, JSON.stringify({ id: "row-7" }));
    batch.setProperty(2, PropertyCode.DragListener, true);
    batch.createElement(3, NativeNodeTag.View);
    batch.setProperty(3, PropertyCode.DropKinds, JSON.stringify(["local", "files"]));
    batch.setProperty(3, PropertyCode.DropListener, true);

    expect(batch.mutationCount).toBe(23);
    expect(batch.finish().byteLength).toBeGreaterThan(10);
  });

  test("keeps every component part name and bound stable", () => {
    expect(NativePart.Checkbox).toBe("checkbox");
    expect(NativePart.TabPanel).toBe("tab-panel");
    expect(NativePart.CollapsiblePanel).toBe("collapsible-panel");
    expect(NativePart.AccordionTrigger).toBe("accordion-trigger");
    expect(NativePart.FieldPassiveLabel).toBe("field-passive-label");
    expect(NativePart.FieldsetLegend).toBe("fieldset-legend");
    expect(NativePart.Dialog).toBe("dialog");
    expect(NativePart.DialogPopup).toBe("dialog-popup");
    expect(MAX_COMPONENT_VALUE_BYTES).toBe(256);
    expect(MAX_TOOLTIP_TEXT_BYTES).toBe(1024);
    expect(NativePart.PopoverMenuTrigger).toBe("popover-menu-trigger");
    expect(NativePart.PopoverMenuPopup).toBe("popover-menu-popup");
    expect(NativePart.ContextMenuTrigger).toBe("context-menu-trigger");
    expect(MAX_MENU_JSON_BYTES).toBe(512 * 1024);
    expect(NativePart.Progress).toBe("progress");
    expect(NativePart.MeterIndicator).toBe("meter-indicator");
    expect(NativePart.Toggle).toBe("toggle");
    expect(NativeNodeTag.Image).toBe(16);
    expect(NativeNodeTag.Shader).toBe(17);
    expect(NativeNodeTag.Extension).toBe(30);
    expect(MAX_KEYMAP_JSON_BYTES).toBe(64 * 1024);
    expect(MAX_DRAG_JSON_BYTES).toBe(64 * 1024);
    expect(NativePart.Slider).toBe("slider");
    expect(NativePart.SliderTrack).toBe("slider-track");
    expect(NativePart.SliderRange).toBe("slider-range");
    expect(NativePart.SliderThumb).toBe("slider-thumb");
    expect(NativePart.Splitter).toBe("splitter");
    expect(NativePart.SplitterPane).toBe("splitter-pane");
    expect(NativePart.SplitterHandle).toBe("splitter-handle");
    expect(NativePart.Toolbar).toBe("toolbar");
    expect(NativePart.ToolbarItem).toBe("toolbar-item");
    expect(NativePart.ToggleGroup).toBe("toggle-group");
    expect(NativePart.ToggleGroupItem).toBe("toggle-group-item");
    expect(MAX_COMPONENT_JSON_BYTES).toBe(64 * 1024);
    expect(MAX_COMPONENT_VALUES).toBe(64);
    expect(MAX_COMPONENT_ITEMS).toBe(256);
    expect(NativePart.Select).toBe("select");
    expect(NativePart.Combobox).toBe("combobox");
    expect(NativePart.Autocomplete).toBe("autocomplete");
    expect(NativePart.Option).toBe("option");
    expect(NativePart.Table).toBe("table");
    expect(NativePart.TableHeader).toBe("table-header");
    expect(NativePart.TableRow).toBe("table-row");
    expect(NativePart.TableCell).toBe("table-cell");
    expect(NativePart.Tree).toBe("tree");
    expect(NativePart.TreeRow).toBe("tree-row");
    expect(NativePart.NumberField).toBe("number-field");
    expect(NativePart.NumberFieldInput).toBe("number-field-input");
    expect(NativePart.NumberFieldIncrement).toBe("number-field-increment");
    expect(NativePart.NumberFieldDecrement).toBe("number-field-decrement");
    expect(NativePart.DateField).toBe("date-field");
    expect(NativePart.DateFieldSegment).toBe("date-field-segment");
    expect(NativePart.TimeField).toBe("time-field");
    expect(NativePart.TimeFieldSegment).toBe("time-field-segment");
    expect(NativePart.Calendar).toBe("calendar");
    expect(NativePart.CalendarWeek).toBe("calendar-week");
    expect(NativePart.CalendarDay).toBe("calendar-day");
    expect(NativePart.Menubar).toBe("menubar");
    expect(NativePart.MenubarItem).toBe("menubar-item");
    expect(NativePart.ToastViewport).toBe("toast-viewport");
    expect(NativePart.Toast).toBe("toast");
    expect(NativePart.ToastTitle).toBe("toast-title");
    expect(NativePart.ToastDescription).toBe("toast-description");
    expect(NativePart.ToastAction).toBe("toast-action");
    expect(NativePart.ToastClose).toBe("toast-close");
    expect(MAX_OPTIONS_JSON_BYTES).toBe(512 * 1024);
    expect(MAX_DECLARED_OPTIONS).toBe(4096);
    expect(MAX_COLLECTION_JSON_BYTES).toBe(2 * 1024 * 1024);
    expect(MAX_DECLARED_TREE_NODES).toBe(65_536);
    expect(MAX_TABLE_COLUMNS).toBe(512);
    expect(MAX_TABLE_ROWS).toBe(1_000_000);
    expect(MAX_TOASTS).toBe(8);
    expect(MAX_MENUBAR_MENUS).toBe(64);
  });

  test("encodes declared option sources, virtual collections, and stateful fields", () => {
    const batch = new MutationBatch();
    batch.createElement(1, NativeNodeTag.Button);
    batch.setProperty(1, PropertyCode.Part, NativePart.Select);
    batch.setProperty(1, PropertyCode.Scope, "theme");
    batch.setProperty(
      1,
      PropertyCode.Options,
      JSON.stringify([{ value: "light", label: "Light" }]),
    );
    batch.setProperty(1, PropertyCode.ActiveValue, "light");
    batch.setProperty(1, PropertyCode.FilterMode, "fuzzy");
    batch.setProperty(1, PropertyCode.Appearance, '{"width":240}');
    batch.setProperty(1, PropertyCode.CommitListener, true);

    batch.createElement(2, NativeNodeTag.View);
    batch.setProperty(2, PropertyCode.Part, NativePart.Table);
    batch.setProperty(2, PropertyCode.Scope, "files");
    batch.setProperty(2, PropertyCode.Columns, JSON.stringify([{ id: "name", label: "Name" }]));
    batch.setProperty(2, PropertyCode.RowCount, 1000);
    batch.setProperty(2, PropertyCode.RowHeight, 24);
    batch.setProperty(2, PropertyCode.HeaderHeight, 32);
    batch.setProperty(2, PropertyCode.SelectionMode, "multiple");
    batch.setProperty(2, PropertyCode.Selection, "[[0,2]]");
    batch.setProperty(2, PropertyCode.SortColumn, "name");
    batch.setProperty(2, PropertyCode.SortDirection, "ascending");
    batch.setProperty(2, PropertyCode.Editing, '{"row":0,"column":0}');

    batch.createElement(3, NativeNodeTag.View);
    batch.setProperty(3, PropertyCode.Part, NativePart.TableCell);
    batch.setProperty(3, PropertyCode.PartValue, "name");
    batch.setProperty(3, PropertyCode.ColumnIndex, 0);

    batch.createElement(4, NativeNodeTag.View);
    batch.setProperty(4, PropertyCode.Part, NativePart.Tree);
    batch.setProperty(4, PropertyCode.Nodes, '[{"id":"src","pending":true}]');
    batch.setProperty(4, PropertyCode.Expanded, '["src"]');
    batch.setProperty(4, PropertyCode.SelectedValue, "src");
    batch.setProperty(4, PropertyCode.SetChildren, '{"id":"src","children":[]}');
    batch.setProperty(4, PropertyCode.LoadingLabel, "Loading…");
    batch.setProperty(4, PropertyCode.Disclosure, "leading");

    batch.createElement(5, NativeNodeTag.View);
    batch.setProperty(5, PropertyCode.Part, NativePart.DateField);
    batch.setProperty(5, PropertyCode.CivilValue, "2026-09-03");
    batch.setProperty(5, PropertyCode.CivilMinimum, "2000-01-01");
    batch.setProperty(5, PropertyCode.CivilMaximum, "2099-12-31");
    batch.setProperty(5, PropertyCode.SegmentOrder, "mdy");
    batch.setProperty(5, PropertyCode.Segment, "year");

    batch.createElement(6, NativeNodeTag.View);
    batch.setProperty(6, PropertyCode.Part, NativePart.ToastViewport);
    batch.setProperty(6, PropertyCode.Toasts, '[{"id":"a","title":"Saved"}]');
    batch.setProperty(6, PropertyCode.MenuCount, 3);
    batch.setProperty(6, PropertyCode.FirstWeekday, 0);
    batch.setProperty(6, PropertyCode.Precision, 2);
    batch.setProperty(6, PropertyCode.InputValue, "al");
    batch.setProperty(6, PropertyCode.Group, "recent");
    batch.setProperty(6, PropertyCode.RowIndex, 4);

    expect(batch.mutationCount).toBe(48);
    expect(batch.finish().byteLength).toBeGreaterThan(10);
  });

  test("encodes declared range, ordering, and roving-focus component declarations", () => {
    const batch = new MutationBatch();
    batch.createElement(1, NativeNodeTag.View);
    batch.setProperty(1, PropertyCode.Part, NativePart.Slider);
    batch.setProperty(1, PropertyCode.Scope, "volume");
    batch.setProperty(1, PropertyCode.Values, "[10,60]");
    batch.setProperty(1, PropertyCode.Minimum, 0);
    batch.setProperty(1, PropertyCode.Maximum, 100);
    batch.setProperty(1, PropertyCode.Step, 5);
    batch.setProperty(1, PropertyCode.LargeStep, 25);
    batch.setProperty(1, PropertyCode.ComponentChangeListener, true);
    batch.createElement(2, NativeNodeTag.View);
    batch.setProperty(2, PropertyCode.Part, NativePart.SliderThumb);
    batch.setProperty(2, PropertyCode.Scope, "volume");
    batch.setProperty(2, PropertyCode.ItemIndex, 1);
    batch.createElement(3, NativeNodeTag.View);
    batch.setProperty(3, PropertyCode.Part, NativePart.Toolbar);
    batch.setProperty(3, PropertyCode.Items, JSON.stringify([{ value: "cut" }]));

    expect(batch.mutationCount).toBe(16);
    expect(batch.finish().byteLength).toBeGreaterThan(10);
  });

  test("encodes extended text, box, and layout styling under protocol v23", () => {
    const batch = new MutationBatch();
    batch.createElement(1, NativeNodeTag.Text);
    batch.setProperty(1, PropertyCode.TextAlign, "start");
    batch.setProperty(1, PropertyCode.LetterSpacing, 0.4);
    batch.setProperty(1, PropertyCode.WordSpacing, 2);
    batch.setProperty(1, PropertyCode.TextTransform, "uppercase");
    batch.setProperty(1, PropertyCode.TextShadow, "0 2px 4px #00000080");
    batch.setProperty(1, PropertyCode.TextDecorationLine, "underline overline");
    batch.setProperty(1, PropertyCode.TextDecorationColor, 0xff2266dd, true);
    batch.setProperty(1, PropertyCode.TextDecorationStyle, "wavy");
    batch.setProperty(1, PropertyCode.TextDecorationThickness, 2);
    batch.setProperty(1, PropertyCode.WordBreak, "break-all");
    batch.setProperty(1, PropertyCode.OverflowWrap, "anywhere");
    batch.setProperty(1, PropertyCode.Hyphens, "manual");
    batch.setProperty(1, PropertyCode.TextDirection, "rtl");

    batch.createElement(2, NativeNodeTag.View);
    batch.setProperty(2, PropertyCode.Direction, "rtl");
    batch.setProperty(2, PropertyCode.PaddingStart, 12);
    batch.setProperty(2, PropertyCode.PaddingEnd, 4);
    batch.setProperty(2, PropertyCode.MarginStart, 6);
    batch.setProperty(2, PropertyCode.MarginEnd, 2);
    batch.setProperty(2, PropertyCode.BorderStartWidth, 3);
    batch.setProperty(2, PropertyCode.BorderEndWidth, 1);

    batch.createElement(3, NativeNodeTag.View);
    batch.setProperty(
      3,
      PropertyCode.BackgroundGradient,
      "linear-gradient(135deg, #0f172a, #1e3a8a 60%, #38bdf8)",
    );
    batch.setProperty(3, PropertyCode.BorderTopLeftRadius, 16);
    batch.setProperty(3, PropertyCode.BorderTopRightRadius, 4);
    batch.setProperty(3, PropertyCode.BorderBottomRightRadius, 16);
    batch.setProperty(3, PropertyCode.BorderBottomLeftRadius, 4);
    batch.setProperty(3, PropertyCode.BorderStyle, "dashed");
    batch.setProperty(3, PropertyCode.OutlineWidth, 2);
    batch.setProperty(3, PropertyCode.OutlineColor, 0xff38bdf8, true);
    batch.setProperty(3, PropertyCode.OutlineOffset, 3);
    batch.setProperty(3, PropertyCode.OutlineStyle, "dotted");
    batch.setProperty(3, PropertyCode.BackgroundImage, "/assets/paper.png");
    batch.setProperty(3, PropertyCode.BackgroundSize, "cover");
    batch.setProperty(3, PropertyCode.BackgroundRepeat, "no-repeat");
    batch.setProperty(3, PropertyCode.BackgroundPosition, "center");
    batch.setProperty(3, PropertyCode.Filter, "saturate(1.4) blur(2px)");
    batch.setProperty(3, PropertyCode.BackdropFilter, "blur(18px)");
    batch.setProperty(3, PropertyCode.Transform, "rotate(3deg) scale(1.02)");
    batch.setProperty(3, PropertyCode.TransformOrigin, "left top");
    batch.setProperty(3, PropertyCode.MixBlendMode, "multiply");
    batch.setProperty(
      3,
      PropertyCode.HoverBackgroundGradient,
      "linear-gradient(90deg, #111827, #334155)",
    );
    batch.setProperty(3, PropertyCode.HoverOutline, "2px solid #f8fafc");
    batch.setProperty(3, PropertyCode.HoverTransform, "translate(0, -2px)");
    batch.setProperty(3, PropertyCode.ActiveTransform, "scale(0.98)");
    batch.setProperty(3, PropertyCode.ActiveOutline, "1px solid #94a3b8");
    batch.setProperty(
      3,
      PropertyCode.ActiveBackgroundGradient,
      "linear-gradient(90deg, #0b1220, #1f2937)",
    );
    batch.setProperty(3, PropertyCode.FocusBackgroundColor, 0xff1f2937, true);
    batch.setProperty(3, PropertyCode.FocusColor, 0xfff8fafc, true);
    batch.setProperty(
      3,
      PropertyCode.FocusBackgroundGradient,
      "radial-gradient(circle at 50% 0%, #1d4ed8, #0f172a)",
    );
    batch.setProperty(3, PropertyCode.FocusOutline, "2px solid #60a5fa");
    batch.setProperty(3, PropertyCode.FocusTransform, "scale(1.01)");

    batch.createElement(4, NativeNodeTag.View);
    batch.setProperty(4, PropertyCode.Position, "sticky");
    batch.setProperty(4, PropertyCode.Top, 0);
    batch.setProperty(4, PropertyCode.OverflowX, "scroll");
    batch.setProperty(4, PropertyCode.ScrollSnapType, "x mandatory");
    batch.setProperty(4, PropertyCode.ScrollSnapAlign, "start");
    batch.setProperty(4, PropertyCode.ScrollSnapStop, "always");

    expect(batch.mutationCount).toBe(60);
    expect(batch.finish().byteLength).toBeGreaterThan(10);
  });

  test("keeps every extended styling code inside the declared property space", () => {
    // The Rust binding rejects any property code past its own `property::LAST`, so the two must
    // stay in step whenever a declaration is added.
    expect(PropertyCode.ScrollSnapStop).toBe(291);
    expect(PropertyCode.HoverStyle).toBe(346);
    expect(PropertyCode.HoverGroup).toBe(354);
    expect(PropertyCode.FocusWithinStyle).toBe(356);
    expect(PropertyCode.SelectedStyle).toBe(357);
    expect(PropertyCode.Selected).toBe(358);
    expect(MAX_GROUP_STYLES_PER_ELEMENT).toBe(8);
    expect(MAX_STYLE_DECLARATION_BYTES).toBe(4096);
    expect(MAX_STATE_STYLE_JSON_BYTES).toBe(16 * 1024);
    expect(MAX_HOVER_GROUP_NAME_BYTES).toBe(256);
    expect(MAX_GRADIENT_STOPS).toBe(8);
    expect(MAX_FILTERS_PER_ELEMENT).toBe(8);
  });

  test("encodes every Base UI parity part under protocol v25", () => {
    const batch = new MutationBatch();
    batch.createElement(1, NativeNodeTag.View);
    batch.setProperty(1, PropertyCode.Part, NativePart.Separator);
    batch.setProperty(1, PropertyCode.Orientation, "vertical");

    batch.createElement(2, NativeNodeTag.View);
    batch.setProperty(2, PropertyCode.Part, NativePart.Avatar);
    batch.setProperty(2, PropertyCode.Scope, "member");
    batch.setProperty(2, PropertyCode.AccessibilityLabel, "Ada Lovelace");
    batch.createElement(3, NativeNodeTag.Image);
    batch.setProperty(3, PropertyCode.Part, NativePart.AvatarImage);
    batch.setProperty(3, PropertyCode.Scope, "member");
    batch.setProperty(3, PropertyCode.Value, "/tmp/ada.png");
    batch.createElement(4, NativeNodeTag.View);
    batch.setProperty(4, PropertyCode.Part, NativePart.AvatarFallback);
    batch.setProperty(4, PropertyCode.Scope, "member");
    batch.setProperty(4, PropertyCode.Delay, 200);

    batch.createElement(5, NativeNodeTag.View);
    batch.setProperty(5, PropertyCode.Part, NativePart.CheckboxGroup);
    batch.setProperty(5, PropertyCode.Scope, "colors");
    batch.setProperty(5, PropertyCode.Items, '["red","green","blue"]');
    batch.setProperty(5, PropertyCode.Values, '["green"]');
    batch.createElement(6, NativeNodeTag.Button);
    batch.setProperty(6, PropertyCode.Part, NativePart.CheckboxGroupItem);
    batch.setProperty(6, PropertyCode.Scope, "colors");
    batch.setProperty(6, PropertyCode.PartValue, "red");
    batch.createElement(7, NativeNodeTag.Button);
    batch.setProperty(7, PropertyCode.Part, NativePart.CheckboxGroupParent);
    batch.setProperty(7, PropertyCode.Scope, "colors");

    batch.createElement(8, NativeNodeTag.View);
    batch.setProperty(8, PropertyCode.Part, NativePart.PreviewCard);
    batch.setProperty(8, PropertyCode.Scope, "profile");
    batch.setProperty(8, PropertyCode.Open, false);
    batch.createElement(9, NativeNodeTag.Button);
    batch.setProperty(9, PropertyCode.Part, NativePart.PreviewCardTrigger);
    batch.setProperty(9, PropertyCode.Scope, "profile");
    batch.setProperty(9, PropertyCode.Delay, 600);
    batch.setProperty(9, PropertyCode.CloseDelay, 300);

    batch.createElement(10, NativeNodeTag.View);
    batch.setProperty(10, PropertyCode.Part, NativePart.ScrollArea);
    batch.setProperty(10, PropertyCode.Scope, "log");
    batch.setProperty(10, PropertyCode.ViewportSize, "[260,160]");
    batch.setProperty(10, PropertyCode.ContentSize, "[260,900]");
    batch.setProperty(10, PropertyCode.OverflowEdgeThreshold, 2);
    batch.createElement(11, NativeNodeTag.View);
    batch.setProperty(11, PropertyCode.Part, NativePart.ScrollAreaScrollbar);
    batch.setProperty(11, PropertyCode.Scope, "log");
    batch.setProperty(11, PropertyCode.Orientation, "vertical");
    batch.setProperty(11, PropertyCode.KeepMounted, true);

    batch.createElement(12, NativeNodeTag.View);
    batch.setProperty(12, PropertyCode.Part, NativePart.OtpField);
    batch.setProperty(12, PropertyCode.Scope, "code");
    batch.setProperty(12, PropertyCode.Length, 6);
    batch.setProperty(12, PropertyCode.Variant, "numeric");
    batch.setProperty(12, PropertyCode.Mask, true);
    batch.setProperty(12, PropertyCode.ReadOnly, false);
    batch.setProperty(12, PropertyCode.AutoSubmit, "verify");
    batch.createElement(13, NativeNodeTag.Input);
    batch.setProperty(13, PropertyCode.Part, NativePart.OtpFieldInput);
    batch.setProperty(13, PropertyCode.Scope, "code");
    batch.setProperty(13, PropertyCode.ItemIndex, 0);

    batch.createElement(14, NativeNodeTag.View);
    batch.setProperty(14, PropertyCode.Part, NativePart.Drawer);
    batch.setProperty(14, PropertyCode.Scope, "filters");
    batch.setProperty(14, PropertyCode.Variant, "modal");
    batch.setProperty(14, PropertyCode.SwipeDirection, "down");
    batch.setProperty(14, PropertyCode.Values, "[0.45,1]");
    batch.setProperty(14, PropertyCode.ItemIndex, 0);
    batch.setProperty(14, PropertyCode.DisablePointerDismissal, true);
    batch.createElement(15, NativeNodeTag.View);
    batch.setProperty(15, PropertyCode.Part, NativePart.DrawerSwipeArea);
    batch.setProperty(15, PropertyCode.Scope, "filters");

    batch.createElement(16, NativeNodeTag.View);
    batch.setProperty(16, PropertyCode.Part, NativePart.NavigationMenu);
    batch.setProperty(16, PropertyCode.Scope, "main-nav");
    batch.setProperty(16, PropertyCode.ActiveValue, "products");
    batch.setProperty(16, PropertyCode.Delay, 50);
    batch.setProperty(16, PropertyCode.CloseDelay, 50);
    batch.createElement(17, NativeNodeTag.Button);
    batch.setProperty(17, PropertyCode.Part, NativePart.NavigationMenuTrigger);
    batch.setProperty(17, PropertyCode.Scope, "main-nav");
    batch.setProperty(17, PropertyCode.PartValue, "products");

    expect(batch.mutationCount).toBe(80);
    const encoded = batch.finish();
    expect(encoded.readUInt16LE(4)).toBe(PROTOCOL_VERSION);
  });

  test("keeps every Base UI property code and bound inside the declared space", () => {
    // The Rust binding rejects any property code past its own `property::LAST`, so the two must
    // stay in step whenever a declaration is added.
    expect(PropertyCode.DisablePointerDismissal).toBe(302);
    expect(MAX_AVATAR_FALLBACK_DELAY_MS).toBe(10_000);
    expect(MAX_CHECKBOX_GROUP_VALUES).toBe(256);
    expect(MAX_PREVIEW_CARD_DELAY_MS).toBe(10_000);
    expect(MAX_SCROLL_AREA_OVERFLOW_THRESHOLD).toBe(256);
    expect(MAX_OTP_LENGTH).toBe(12);
    expect(MAX_DRAWER_SNAP_POINTS).toBe(8);
    expect(MAX_NAVIGATION_MENU_ITEMS).toBe(64);
    expect(MAX_NAVIGATION_MENU_DELAY_MS).toBe(10_000);

    // Every Base UI part name is the exact string the Rust binding matches on.
    expect(NativePart.Separator).toBe("separator");
    expect(NativePart.AvatarFallback).toBe("avatar-fallback");
    expect(NativePart.CheckboxGroupParent).toBe("checkbox-group-parent");
    expect(NativePart.PreviewCardPopup).toBe("preview-card-popup");
    expect(NativePart.ScrollAreaThumb).toBe("scroll-area-thumb");
    expect(NativePart.OtpFieldSeparator).toBe("otp-field-separator");
    expect(NativePart.DrawerSwipeArea).toBe("drawer-swipe-area");
    expect(NativePart.NavigationMenuLink).toBe("navigation-menu-link");
  });

  test("encodes every Base UI-aligned popover, tooltip, range, toast, and field part", () => {
    const batch = new MutationBatch();

    // A popover's trigger is mounted whether the surface is open or closed, so it carries the
    // whole declaration; every other part only repeats the scope.
    batch.createElement(1, NativeNodeTag.Button);
    batch.setProperty(1, PropertyCode.Part, NativePart.PopoverTrigger);
    batch.setProperty(1, PropertyCode.Scope, "account");
    batch.setProperty(1, PropertyCode.Open, true);
    batch.setProperty(1, PropertyCode.Modal, true);
    batch.setProperty(1, PropertyCode.OpenOnHover, true);
    batch.setProperty(1, PropertyCode.Delay, 300);
    batch.setProperty(1, PropertyCode.CloseDelay, 100);
    batch.setProperty(1, PropertyCode.Side, "top");
    batch.setProperty(1, PropertyCode.Align, "end");
    batch.setProperty(1, PropertyCode.SideOffset, 10);
    batch.setProperty(1, PropertyCode.AlignOffset, 4);
    batch.setProperty(1, PropertyCode.CollisionPadding, 12);
    batch.setProperty(1, PropertyCode.Sticky, false);
    batch.setProperty(1, PropertyCode.AnchorPoint, "120,48");
    batch.createElement(2, NativeNodeTag.View);
    batch.setProperty(2, PropertyCode.Part, NativePart.PopoverPositioner);
    batch.setProperty(2, PropertyCode.Scope, "account");
    batch.createElement(3, NativeNodeTag.View);
    batch.setProperty(3, PropertyCode.Part, NativePart.PopoverPopup);
    batch.setProperty(3, PropertyCode.Scope, "account");
    batch.createElement(4, NativeNodeTag.View);
    batch.setProperty(4, PropertyCode.Part, NativePart.PopoverArrow);
    batch.setProperty(4, PropertyCode.Scope, "account");
    batch.createElement(5, NativeNodeTag.View);
    batch.setProperty(5, PropertyCode.Part, NativePart.PopoverViewport);
    batch.setProperty(5, PropertyCode.Scope, "account");

    batch.createElement(6, NativeNodeTag.View);
    batch.setProperty(6, PropertyCode.Part, NativePart.TooltipProvider);
    batch.setProperty(6, PropertyCode.Scope, "hints");
    batch.setProperty(6, PropertyCode.Delay, 600);
    batch.setProperty(6, PropertyCode.Timeout, 400);
    batch.createElement(7, NativeNodeTag.Button);
    batch.setProperty(7, PropertyCode.Part, NativePart.TooltipTrigger);
    batch.setProperty(7, PropertyCode.Scope, "save-hint");
    batch.setProperty(7, PropertyCode.Provider, "hints");
    batch.setProperty(7, PropertyCode.Hoverable, true);
    batch.setProperty(7, PropertyCode.CloseOnClick, false);
    batch.setProperty(7, PropertyCode.TrackCursorAxis, "x");

    batch.createElement(8, NativeNodeTag.View);
    batch.setProperty(8, PropertyCode.Part, NativePart.Slider);
    batch.setProperty(8, PropertyCode.Scope, "volume");
    batch.setProperty(8, PropertyCode.MinStepsBetweenValues, 1);
    batch.setProperty(8, PropertyCode.ThumbAlignment, "edge");
    batch.setProperty(8, PropertyCode.Format, "percent");
    batch.createElement(9, NativeNodeTag.View);
    batch.setProperty(9, PropertyCode.Part, NativePart.SliderValue);
    batch.setProperty(9, PropertyCode.Scope, "volume");

    batch.createElement(10, NativeNodeTag.View);
    batch.setProperty(10, PropertyCode.Part, NativePart.NumberField);
    batch.setProperty(10, PropertyCode.Scope, "quantity");
    batch.setProperty(10, PropertyCode.SmallStep, 0.5);
    batch.setProperty(10, PropertyCode.LargeStep, 5);
    batch.setProperty(10, PropertyCode.SnapOnStep, true);
    batch.setProperty(10, PropertyCode.AllowWheelScrub, false);
    batch.setProperty(10, PropertyCode.Pitch, 2);
    batch.createElement(11, NativeNodeTag.View);
    batch.setProperty(11, PropertyCode.Part, NativePart.NumberFieldScrubArea);
    batch.setProperty(11, PropertyCode.Scope, "quantity");

    batch.createElement(12, NativeNodeTag.View);
    batch.setProperty(12, PropertyCode.Part, NativePart.ToastViewport);
    batch.setProperty(12, PropertyCode.Scope, "notices");
    batch.setProperty(12, PropertyCode.Timeout, 5000);
    batch.setProperty(12, PropertyCode.Limit, 3);
    batch.setProperty(12, PropertyCode.StackExpanded, true);
    batch.setProperty(12, PropertyCode.SwipeDirection, "left");
    batch.setProperty(12, PropertyCode.Pitch, 12);
    batch.createElement(13, NativeNodeTag.View);
    batch.setProperty(13, PropertyCode.Part, NativePart.ToastContent);
    batch.setProperty(13, PropertyCode.Scope, "notices");
    batch.setProperty(13, PropertyCode.PartValue, "saved");

    batch.createElement(14, NativeNodeTag.Button);
    batch.setProperty(14, PropertyCode.Part, NativePart.ToolbarButton);
    batch.setProperty(14, PropertyCode.Scope, "actions");
    batch.setProperty(14, PropertyCode.PartValue, "cut");
    batch.createElement(15, NativeNodeTag.View);
    batch.setProperty(15, PropertyCode.Part, NativePart.ToolbarSeparator);
    batch.setProperty(15, PropertyCode.Scope, "actions");

    batch.createElement(16, NativeNodeTag.View);
    batch.setProperty(16, PropertyCode.Part, NativePart.Field);
    batch.setProperty(16, PropertyCode.Scope, "email");
    batch.setProperty(16, PropertyCode.ValidationMode, "onChange");
    batch.setProperty(16, PropertyCode.ValidationDebounceTime, 250);
    batch.createElement(17, NativeNodeTag.View);
    batch.setProperty(17, PropertyCode.Part, NativePart.FieldValidity);
    batch.setProperty(17, PropertyCode.Scope, "email");

    batch.createElement(18, NativeNodeTag.Button);
    batch.setProperty(18, PropertyCode.Part, NativePart.Checkbox);
    batch.setProperty(18, PropertyCode.Parent, true);
    batch.setProperty(18, PropertyCode.Values, "[true,false]");
    batch.setProperty(18, PropertyCode.ReadOnly, true);

    batch.createElement(19, NativeNodeTag.View);
    batch.setProperty(19, PropertyCode.Part, NativePart.Dialog);
    batch.setProperty(19, PropertyCode.Scope, "confirm");
    batch.setProperty(19, PropertyCode.EnterDuration, 0);
    batch.setProperty(19, PropertyCode.ExitDuration, 160);
    batch.createElement(20, NativeNodeTag.View);
    batch.setProperty(20, PropertyCode.Part, NativePart.DialogViewport);
    batch.setProperty(20, PropertyCode.Scope, "confirm");

    expect(batch.mutationCount).toBe(99);
    const encoded = batch.finish();
    expect(encoded.readUInt16LE(4)).toBe(PROTOCOL_VERSION);
  });

  test("keeps every Base UI-aligned property code and bound inside the declared space", () => {
    // The Rust binding rejects any property code past its own `property::LAST`, so the two must
    // stay in step whenever a declaration is added.
    expect(PropertyCode.Side).toBe(303);
    expect(PropertyCode.StackExpanded).toBe(331);
    expect(MAX_ANCHOR_SIDE_OFFSET).toBe(256);
    expect(MAX_ANCHOR_ALIGN_OFFSET).toBe(4096);
    expect(MAX_ANCHOR_COLLISION_PADDING).toBe(512);
    expect(MAX_POPOVER_HOVER_DELAY_MS).toBe(10_000);
    expect(MAX_TOOLTIP_DELAY_MS).toBe(10_000);
    expect(MAX_TOOLTIP_GROUP_TIMEOUT_MS).toBe(10_000);
    expect(MAX_TOAST_DURATION_MS).toBe(60_000);
    expect(MAX_TOAST_SWIPE_THRESHOLD).toBe(512);
    expect(MAX_FIELD_VALIDATION_DEBOUNCE_MS).toBe(10_000);
    expect(MAX_DIALOG_TRANSITION_MS).toBe(10_000);
    expect(MAX_NUMBER_FIELD_SCRUB_SENSITIVITY).toBe(256);

    // Every part name below is the exact string the Rust binding matches on.
    expect(NativePart.PopoverPositioner).toBe("popover-positioner");
    expect(NativePart.PopoverViewport).toBe("popover-viewport");
    expect(NativePart.TooltipProvider).toBe("tooltip-provider");
    expect(NativePart.TooltipArrow).toBe("tooltip-arrow");
    expect(NativePart.SliderIndicator).toBe("slider-indicator");
    expect(NativePart.NumberFieldScrubAreaCursor).toBe("number-field-scrub-area-cursor");
    expect(NativePart.ProgressValue).toBe("progress-value");
    expect(NativePart.MeterTrack).toBe("meter-track");
    expect(NativePart.ToastPositioner).toBe("toast-positioner");
    expect(NativePart.ToolbarGroup).toBe("toolbar-group");
    expect(NativePart.FieldItem).toBe("field-item");
    expect(NativePart.DialogViewport).toBe("dialog-viewport");
  });

  test("encodes every Base UI-aligned menu, select, and combobox part under protocol v30", () => {
    const batch = new MutationBatch();
    // The whole `Menu.Root` declaration travels on the trigger, which is the one part the core
    // keeps mounted whether the level is open or closed.
    batch.createElement(1, NativeNodeTag.Button);
    batch.setProperty(1, PropertyCode.Part, NativePart.MenuTrigger);
    batch.setProperty(1, PropertyCode.Scope, "edit");
    batch.setProperty(1, PropertyCode.Open, true);
    batch.setProperty(1, PropertyCode.Modal, true);
    batch.setProperty(1, PropertyCode.Orientation, "horizontal");
    batch.setProperty(1, PropertyCode.LoopFocus, false);
    batch.setProperty(1, PropertyCode.CloseParentOnEsc, true);
    batch.setProperty(1, PropertyCode.OpenOnHover, true);
    batch.setProperty(1, PropertyCode.Delay, 100);

    // A row is an ordinary child node: the core owns its identity, semantics, and activation.
    batch.createElement(2, NativeNodeTag.View);
    batch.setProperty(2, PropertyCode.Part, NativePart.MenuLinkItem);
    batch.setProperty(2, PropertyCode.Scope, "edit");
    batch.setProperty(2, PropertyCode.PartValue, "docs");
    batch.setProperty(2, PropertyCode.Href, "https://example.invalid/docs");
    batch.setProperty(2, PropertyCode.CloseOnClick, false);

    // A select declares Base UI's `multiple`, `readOnly`, `required`, `modal`, and
    // `alignItemWithTrigger` ahead of every decision the core makes about them.
    batch.createElement(3, NativeNodeTag.Button);
    batch.setProperty(3, PropertyCode.Part, NativePart.Select);
    batch.setProperty(3, PropertyCode.Multiple, true);
    batch.setProperty(3, PropertyCode.ReadOnly, true);
    batch.setProperty(3, PropertyCode.Required, true);
    batch.setProperty(3, PropertyCode.AlignItemWithTrigger, true);
    batch.setProperty(3, PropertyCode.Values, '["alpha","bravo"]');

    // A combobox declares the filter policy and the highlight behavior the core answers.
    batch.createElement(4, NativeNodeTag.Input);
    batch.setProperty(4, PropertyCode.Part, NativePart.Combobox);
    batch.setProperty(4, PropertyCode.FilterMode, "startsWith");
    batch.setProperty(4, PropertyCode.AutoHighlight, true);
    batch.setProperty(4, PropertyCode.OpenOnInputClick, false);
    batch.setProperty(4, PropertyCode.HighlightItemOnHover, false);

    const bytes = batch.finish();
    expect(bytes.readUInt16LE(4)).toBe(PROTOCOL_VERSION);
    expect(batch.mutationCount).toBe(29);
  });

  test("keeps every menu, select, and combobox code and part name stable", () => {
    expect(PropertyCode.CloseParentOnEsc).toBe(332);
    expect(PropertyCode.Href).toBe(333);
    expect(PropertyCode.Multiple).toBe(334);
    expect(PropertyCode.AlignItemWithTrigger).toBe(335);
    expect(PropertyCode.AutoHighlight).toBe(336);
    expect(PropertyCode.OpenOnInputClick).toBe(337);
    expect(PropertyCode.HighlightItemOnHover).toBe(338);
    expect(MAX_MENU_HOVER_DELAY_MS).toBe(10_000);
    expect(MAX_MENU_LINK_BYTES).toBe(8 * 1024);
    expect(MAX_MENU_ITEMS).toBe(2_048);
    expect(MAX_SELECT_VALUES).toBe(256);
    expect(MAX_COMBOBOX_VALUES).toBe(64);

    // Every part name below is the exact string the Rust binding matches on.
    expect(NativePart.Menu).toBe("menu");
    expect(NativePart.MenuTrigger).toBe("menu-trigger");
    expect(NativePart.MenuPopup).toBe("menu-popup");
    expect(NativePart.MenuSubmenuTrigger).toBe("menu-submenu-trigger");
    expect(NativePart.MenuRadioItemIndicator).toBe("menu-radio-item-indicator");
    expect(NativePart.MenuCheckboxItemIndicator).toBe("menu-checkbox-item-indicator");
    expect(NativePart.SelectItemText).toBe("select-item-text");
    expect(NativePart.SelectScrollUpArrow).toBe("select-scroll-up-arrow");
    expect(NativePart.SelectScrollDownArrow).toBe("select-scroll-down-arrow");
    expect(NativePart.ComboboxChipRemove).toBe("combobox-chip-remove");
    expect(NativePart.ComboboxCollection).toBe("combobox-collection");
    expect(NativePart.ComboboxEmpty).toBe("combobox-empty");
  });
});
