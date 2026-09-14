import { createRenderer as createUniversalRenderer } from "@solidjs/universal";
import {
  createContext,
  createSignal,
  flush as flushSolid,
  getOwner,
  omit,
  onCleanup,
  runWithOwner,
  Show,
  type Element as SolidElement,
  untrack,
  useContext,
} from "solid-js";
import {
  type ColorValue,
  type NativeElementName,
  type NativeEventListener,
  type NativePartName,
  type PopoverPlacement,
  MAX_COLLECTION_JSON_BYTES,
  MAX_COMPONENT_JSON_BYTES,
  MAX_COMPONENT_VALUE_BYTES,
  MAX_DRAG_JSON_BYTES,
  MAX_OPTIONS_JSON_BYTES,
  MAX_KEYMAP_JSON_BYTES,
  MAX_MENU_JSON_BYTES,
  MAX_MENU_LINK_BYTES,
  MAX_GROUP_STYLES_PER_ELEMENT,
  MAX_HOVER_GROUP_NAME_BYTES,
  MAX_STATE_STYLE_JSON_BYTES,
  MAX_STYLE_DECLARATION_BYTES,
  MAX_TOOLTIP_TEXT_BYTES,
  NativeNode,
  NativePart,
  PropertyCode,
  QuickGuiEvent,
  Window,
  cleanupNativeNodes,
  createNativeElement,
  createNativeSentinel,
  createNativeText,
  getNativeFirstChild,
  getNativeNextSibling,
  getNativeParent,
  insertNativeNode,
  isNativeText,
  parseColor,
  removeNativeNode,
  replaceNativeText,
  setNativeEventListener,
  setNativeProperty,
  type WindowRenderer,
} from "@quickgui/native";
import {
  expandStyleHelper,
  flexDeclaration,
  helperProperties,
  isDirectStyleName,
} from "./style-helpers.ts";
import type { StyleHelpers } from "./style-helpers.generated.ts";
export type { StyleHelpers } from "./style-helpers.generated.ts";

type PropertyInput = unknown;
type PropertyEntry = {
  code: PropertyCode;
  color?: boolean;
  normalize?: (value: PropertyInput) => boolean | number | string | null;
};

const properties: Record<string, PropertyEntry> = {
  extensionPackage: { code: PropertyCode.ExtensionPackage },
  extensionComponent: { code: PropertyCode.ExtensionComponent },
  extensionProps: { code: PropertyCode.ExtensionProps },
  display: { code: PropertyCode.Display },
  flexDirection: { code: PropertyCode.FlexDirection },
  flexWrap: { code: PropertyCode.FlexWrap },
  flexGrow: { code: PropertyCode.FlexGrow },
  flexShrink: { code: PropertyCode.FlexShrink },
  flexBasis: { code: PropertyCode.FlexBasis },
  alignItems: { code: PropertyCode.AlignItems },
  alignSelf: { code: PropertyCode.AlignSelf },
  justifyContent: { code: PropertyCode.JustifyContent },
  alignContent: { code: PropertyCode.AlignContent },
  gap: { code: PropertyCode.Gap },
  columnGap: { code: PropertyCode.ColumnGap },
  rowGap: { code: PropertyCode.RowGap },
  width: { code: PropertyCode.Width },
  height: { code: PropertyCode.Height },
  minWidth: { code: PropertyCode.MinWidth },
  minHeight: { code: PropertyCode.MinHeight },
  maxWidth: { code: PropertyCode.MaxWidth },
  maxHeight: { code: PropertyCode.MaxHeight },
  padding: { code: PropertyCode.Padding },
  paddingTop: { code: PropertyCode.PaddingTop },
  paddingRight: { code: PropertyCode.PaddingRight },
  paddingBottom: { code: PropertyCode.PaddingBottom },
  paddingLeft: { code: PropertyCode.PaddingLeft },
  margin: { code: PropertyCode.Margin },
  marginTop: { code: PropertyCode.MarginTop },
  marginRight: { code: PropertyCode.MarginRight },
  marginBottom: { code: PropertyCode.MarginBottom },
  marginLeft: { code: PropertyCode.MarginLeft },
  color: { code: PropertyCode.Color, color: true },
  textColor: { code: PropertyCode.Color, color: true },
  // Flat legacy state names. The nested `hover`, `active`, and `focus` objects are canonical; the
  // Rust binding overlays these on top of them so a partial migration paints what both declared.
  hoverColor: { code: PropertyCode.HoverColor, color: true },
  activeColor: { code: PropertyCode.ActiveColor, color: true },

  opacity: { code: PropertyCode.Opacity },
  borderWidth: { code: PropertyCode.BorderWidth },
  borderTopWidth: { code: PropertyCode.BorderTopWidth },
  borderRightWidth: { code: PropertyCode.BorderRightWidth },
  borderBottomWidth: { code: PropertyCode.BorderBottomWidth },
  borderLeftWidth: { code: PropertyCode.BorderLeftWidth },
  borderColor: { code: PropertyCode.BorderColor, color: true },
  borderRadius: { code: PropertyCode.BorderRadius },
  boxShadow: {
    code: PropertyCode.BoxShadow,
    normalize: normalizeBoxShadow,
  },
  fontSize: { code: PropertyCode.FontSize },
  fontFamily: { code: PropertyCode.FontFamily },
  fontWeight: { code: PropertyCode.FontWeight },
  lineHeight: { code: PropertyCode.LineHeight },
  textAlign: { code: PropertyCode.TextAlign },
  whiteSpace: { code: PropertyCode.WhiteSpace },
  textOverflow: { code: PropertyCode.TextOverflow },
  lineClamp: { code: PropertyCode.LineClamp },
  WebkitLineClamp: { code: PropertyCode.LineClamp },
  overflow: { code: PropertyCode.Overflow },
  overflowX: { code: PropertyCode.OverflowX },
  overflowY: { code: PropertyCode.OverflowY },
  cursor: { code: PropertyCode.Cursor },
  appRegion: { code: PropertyCode.AppRegion },
  disabled: { code: PropertyCode.Disabled },
  ariaLabel: { code: PropertyCode.AccessibilityLabel },
  accessibilityLabel: { code: PropertyCode.AccessibilityLabel },
  role: { code: PropertyCode.Role },
  tabIndex: { code: PropertyCode.TabIndex },
  focusOnPointer: { code: PropertyCode.FocusOnPointer },
  hitSlop: { code: PropertyCode.HitSlop },
  hitSlopTop: { code: PropertyCode.HitSlopTop },
  hitSlopRight: { code: PropertyCode.HitSlopRight },
  hitSlopBottom: { code: PropertyCode.HitSlopBottom },
  hitSlopLeft: { code: PropertyCode.HitSlopLeft },
  position: { code: PropertyCode.Position },
  top: { code: PropertyCode.Top },
  right: { code: PropertyCode.Right },
  bottom: { code: PropertyCode.Bottom },
  left: { code: PropertyCode.Left },
  userSelect: { code: PropertyCode.UserSelect },
  visibility: { code: PropertyCode.Visibility },
  aspectRatio: { code: PropertyCode.AspectRatio },
  value: { code: PropertyCode.Value },
  content: { code: PropertyCode.Value },
  source: { code: PropertyCode.Value },
  placeholder: { code: PropertyCode.Placeholder },
  multiline: { code: PropertyCode.Multiline },
  scrollToEndRevision: { code: PropertyCode.ScrollToEndRevision },
  estimatedItemHeight: { code: PropertyCode.EstimatedItemHeight },
  overscan: { code: PropertyCode.Overscan },
  overscanPixels: { code: PropertyCode.OverscanPixels },
  itemHeights: { code: PropertyCode.ItemHeights, normalize: normalizeItemHeights },
  listAlignment: { code: PropertyCode.ListAlignment },
  followMode: { code: PropertyCode.FollowMode },
  anchorPlacement: { code: PropertyCode.AnchorPlacement },
  anchorGap: { code: PropertyCode.AnchorGap },
  viewportMargin: { code: PropertyCode.ViewportMargin },
  dismissOnEscape: { code: PropertyCode.DismissOnEscape },
  dismissOnPointerOutside: { code: PropertyCode.DismissOnPointerOutside },
  overlay: { code: PropertyCode.Overlay },
  focusTrap: { code: PropertyCode.FocusTrap },
  restorePreviousFocus: { code: PropertyCode.RestorePreviousFocus },
  autoFocus: { code: PropertyCode.AutoFocus },
  ariaModal: { code: PropertyCode.AccessibilityModal },
  label: { code: PropertyCode.Value },
  systemImage: { code: PropertyCode.SwiftUISystemImage },
  buttonStyle: { code: PropertyCode.SwiftUIButtonStyle },
  controlSize: { code: PropertyCode.SwiftUIControlSize },
  target: { code: PropertyCode.SwiftUITarget },
  testID: { code: PropertyCode.SwiftUITestId },
  pickerStyle: { code: PropertyCode.SwiftUIPickerStyle },
  datePickerComponents: { code: PropertyCode.SwiftUIDatePickerComponents },
  datePickerStyle: { code: PropertyCode.SwiftUIDatePickerStyle },
  supportsOpacity: { code: PropertyCode.SwiftUIColorSupportsOpacity },
  gaugeStyle: { code: PropertyCode.SwiftUIGaugeStyle },
  gaugeMinimumValueLabel: {
    code: PropertyCode.SwiftUIGaugeMinimumValueLabel,
  },
  gaugeMaximumValueLabel: {
    code: PropertyCode.SwiftUIGaugeMaximumValueLabel,
  },
  embeddedWindow: { code: PropertyCode.SwiftUIEmbeddedWindow },
  isPresented: { code: PropertyCode.SwiftUIIsPresented },
  attachmentAnchor: { code: PropertyCode.SwiftUIAttachmentAnchor },
  arrowEdge: { code: PropertyCode.SwiftUIArrowEdge },
  part: { code: PropertyCode.Part },
  scope: { code: PropertyCode.Scope, normalize: normalizeComponentValue },
  partValue: { code: PropertyCode.PartValue, normalize: normalizeComponentValue },
  activeValue: { code: PropertyCode.ActiveValue, normalize: normalizeComponentValue },
  checked: { code: PropertyCode.Checked },
  indeterminate: { code: PropertyCode.Indeterminate },
  orientation: { code: PropertyCode.Orientation },
  activateOnFocus: { code: PropertyCode.ActivateOnFocus },
  loopFocus: { code: PropertyCode.LoopFocus },
  keepMounted: { code: PropertyCode.KeepMounted },
  open: { code: PropertyCode.Open },
  itemIndex: { code: PropertyCode.ItemIndex },
  headingLevel: { code: PropertyCode.HeadingLevel },
  required: { code: PropertyCode.Required },
  invalid: { code: PropertyCode.Invalid },
  selected: { code: PropertyCode.Selected },
  validationMessage: { code: PropertyCode.ValidationMessage },
  touched: { code: PropertyCode.Touched },
  dirty: { code: PropertyCode.Dirty },
  filled: { code: PropertyCode.Filled },
  tooltip: { code: PropertyCode.Tooltip, normalize: normalizeTooltipText },
  tooltipPlacement: { code: PropertyCode.TooltipPlacement },
  tooltipDelay: { code: PropertyCode.TooltipDelay },
  tooltipGap: { code: PropertyCode.TooltipGap },
  tooltipViewportMargin: { code: PropertyCode.TooltipViewportMargin },
  variant: { code: PropertyCode.Variant },
  menu: { code: PropertyCode.Menu },
  gridTemplateColumns: {
    code: PropertyCode.GridTemplateColumns,
    normalize: normalizeGridTemplate,
  },
  gridTemplateRows: {
    code: PropertyCode.GridTemplateRows,
    normalize: normalizeGridTemplate,
  },
  gridAutoFlow: { code: PropertyCode.GridAutoFlow },
  gridColumnStart: { code: PropertyCode.GridColumnStart },
  gridColumnEnd: { code: PropertyCode.GridColumnEnd },
  gridColumnSpan: { code: PropertyCode.GridColumnSpan },
  gridRowStart: { code: PropertyCode.GridRowStart },
  gridRowEnd: { code: PropertyCode.GridRowEnd },
  gridRowSpan: { code: PropertyCode.GridRowSpan },
  transitionProperty: {
    code: PropertyCode.TransitionProperties,
    normalize: normalizeTransitionProperties,
  },
  transitionDuration: {
    code: PropertyCode.TransitionDuration,
    normalize: normalizeMilliseconds,
  },
  transitionTimingFunction: { code: PropertyCode.TransitionEasing },
  transitionEasing: { code: PropertyCode.TransitionEasing },
  transitionMaxFps: { code: PropertyCode.TransitionMaxFps },
  min: { code: PropertyCode.Minimum },
  max: { code: PropertyCode.Maximum },
  low: { code: PropertyCode.Low },
  high: { code: PropertyCode.High },
  optimum: { code: PropertyCode.Optimum },
  valueText: { code: PropertyCode.ValueText },
  pressed: { code: PropertyCode.Pressed },
  values: { code: PropertyCode.Values, normalize: normalizeComponentJson },
  items: { code: PropertyCode.Items, normalize: normalizeComponentJson },
  step: { code: PropertyCode.Step },
  largeStep: { code: PropertyCode.LargeStep },
  options: { code: PropertyCode.Options, normalize: normalizeOptionsJson },
  inputValue: { code: PropertyCode.InputValue },
  filterMode: { code: PropertyCode.FilterMode },
  appearance: {
    code: PropertyCode.Appearance,
    normalize: normalizeAppearanceJson,
  },
  columns: { code: PropertyCode.Columns, normalize: normalizeCollectionJson },
  rowCount: { code: PropertyCode.RowCount },
  sortColumn: { code: PropertyCode.SortColumn },
  sortDirection: { code: PropertyCode.SortDirection },
  selectionMode: { code: PropertyCode.SelectionMode },
  selection: {
    code: PropertyCode.Selection,
    normalize: normalizeCollectionJson,
  },
  rowIndex: { code: PropertyCode.RowIndex },
  columnIndex: { code: PropertyCode.ColumnIndex },
  nodes: { code: PropertyCode.Nodes, normalize: normalizeCollectionJson },
  expanded: { code: PropertyCode.Expanded, normalize: normalizeCollectionJson },
  selectedValue: {
    code: PropertyCode.SelectedValue,
    normalize: normalizeComponentValue,
  },
  setChildren: {
    code: PropertyCode.SetChildren,
    normalize: normalizeCollectionJson,
  },
  precision: { code: PropertyCode.Precision },
  toasts: { code: PropertyCode.Toasts, normalize: normalizeCollectionJson },
  segmentOrder: { code: PropertyCode.SegmentOrder },
  segment: { code: PropertyCode.Segment },
  civilValue: { code: PropertyCode.CivilValue },
  civilMinimum: { code: PropertyCode.CivilMinimum },
  civilMaximum: { code: PropertyCode.CivilMaximum },
  menuCount: { code: PropertyCode.MenuCount },
  firstWeekday: { code: PropertyCode.FirstWeekday },
  rowHeight: { code: PropertyCode.RowHeight },
  headerHeight: { code: PropertyCode.HeaderHeight },
  // An option-like part's group label. The parts route their own `group` prop here so the
  // node-level `group` keeps Tailwind's hover-group meaning.
  optionGroup: {
    code: PropertyCode.Group,
    normalize: normalizeComponentValue,
  },
  editing: { code: PropertyCode.Editing, normalize: normalizeCollectionJson },
  disclosure: { code: PropertyCode.Disclosure },
  loadingLabel: { code: PropertyCode.LoadingLabel },
  objectFit: { code: PropertyCode.ObjectFit },

  // Base UI separators, avatars, checkbox groups, preview cards, scroll areas, OTP fields,
  // drawers, and navigation menus.
  src: { code: PropertyCode.Value },
  delay: { code: PropertyCode.Delay, normalize: normalizeMilliseconds },
  closeDelay: { code: PropertyCode.CloseDelay, normalize: normalizeMilliseconds },
  length: { code: PropertyCode.Length },
  mask: { code: PropertyCode.Mask },
  readOnly: { code: PropertyCode.ReadOnly },
  autoSubmit: {
    code: PropertyCode.AutoSubmit,
    normalize: normalizeComponentValue,
  },
  swipeDirection: { code: PropertyCode.SwipeDirection },
  viewportSize: { code: PropertyCode.ViewportSize, normalize: normalizeExtent },
  contentSize: { code: PropertyCode.ContentSize, normalize: normalizeExtent },
  overflowEdgeThreshold: { code: PropertyCode.OverflowEdgeThreshold },
  disablePointerDismissal: { code: PropertyCode.DisablePointerDismissal },
  fit: { code: PropertyCode.ObjectFit },
  shaderParameters: {
    code: PropertyCode.ShaderParameters,
    normalize: normalizeShaderParameters,
  },

  // Extended text styling. Every one of these inherits through the subtree exactly as the Rust
  // core's own typography does.
  letterSpacing: { code: PropertyCode.LetterSpacing },
  wordSpacing: { code: PropertyCode.WordSpacing },
  textTransform: { code: PropertyCode.TextTransform },
  textShadow: {
    code: PropertyCode.TextShadow,
    normalize: normalizeStyleDeclaration,
  },
  textDecoration: { code: PropertyCode.TextDecorationLine },
  textDecorationLine: { code: PropertyCode.TextDecorationLine },
  textDecorationColor: { code: PropertyCode.TextDecorationColor, color: true },
  textDecorationStyle: { code: PropertyCode.TextDecorationStyle },
  textDecorationThickness: { code: PropertyCode.TextDecorationThickness },
  wordBreak: { code: PropertyCode.WordBreak },
  overflowWrap: { code: PropertyCode.OverflowWrap },
  wordWrap: { code: PropertyCode.OverflowWrap },
  hyphens: { code: PropertyCode.Hyphens },
  textDirection: { code: PropertyCode.TextDirection },

  // Direction-relative layout.
  direction: { code: PropertyCode.Direction },
  paddingStart: { code: PropertyCode.PaddingStart },
  paddingInlineStart: { code: PropertyCode.PaddingStart },
  paddingEnd: { code: PropertyCode.PaddingEnd },
  paddingInlineEnd: { code: PropertyCode.PaddingEnd },
  marginStart: { code: PropertyCode.MarginStart },
  marginInlineStart: { code: PropertyCode.MarginStart },
  marginEnd: { code: PropertyCode.MarginEnd },
  marginInlineEnd: { code: PropertyCode.MarginEnd },
  borderStartWidth: { code: PropertyCode.BorderStartWidth },
  borderInlineStartWidth: { code: PropertyCode.BorderStartWidth },
  borderEndWidth: { code: PropertyCode.BorderEndWidth },
  borderInlineEndWidth: { code: PropertyCode.BorderEndWidth },

  // Extended box styling.
  borderTopLeftRadius: { code: PropertyCode.BorderTopLeftRadius },
  borderTopRightRadius: { code: PropertyCode.BorderTopRightRadius },
  borderBottomRightRadius: { code: PropertyCode.BorderBottomRightRadius },
  borderBottomLeftRadius: { code: PropertyCode.BorderBottomLeftRadius },
  borderStyle: { code: PropertyCode.BorderStyle },
  outlineWidth: { code: PropertyCode.OutlineWidth },
  outlineColor: { code: PropertyCode.OutlineColor, color: true },
  outlineOffset: { code: PropertyCode.OutlineOffset },
  outlineStyle: { code: PropertyCode.OutlineStyle },
  bgImage: { code: PropertyCode.BackgroundImage },
  bgSize: { code: PropertyCode.BackgroundSize },
  bgRepeat: { code: PropertyCode.BackgroundRepeat },
  bgPosition: { code: PropertyCode.BackgroundPosition },
  filter: { code: PropertyCode.Filter, normalize: normalizeStyleDeclaration },
  backdropFilter: {
    code: PropertyCode.BackdropFilter,
    normalize: normalizeStyleDeclaration,
  },
  transform: {
    code: PropertyCode.Transform,
    normalize: normalizeStyleDeclaration,
  },
  transformOrigin: { code: PropertyCode.TransformOrigin },
  mixBlendMode: { code: PropertyCode.MixBlendMode },

  // Flat legacy state styling; `hover: { outline, transform }` and friends are canonical.
  hoverOutline: {
    code: PropertyCode.HoverOutline,
    normalize: normalizeStyleDeclaration,
  },
  hoverTransform: {
    code: PropertyCode.HoverTransform,
    normalize: normalizeStyleDeclaration,
  },
  activeOutline: {
    code: PropertyCode.ActiveOutline,
    normalize: normalizeStyleDeclaration,
  },
  activeTransform: {
    code: PropertyCode.ActiveTransform,
    normalize: normalizeStyleDeclaration,
  },
  focusColor: { code: PropertyCode.FocusColor, color: true },
  focusOutline: {
    code: PropertyCode.FocusOutline,
    normalize: normalizeStyleDeclaration,
  },
  focusTransform: {
    code: PropertyCode.FocusTransform,
    normalize: normalizeStyleDeclaration,
  },

  // Scroll snapping.
  scrollSnapType: { code: PropertyCode.ScrollSnapType },
  scrollSnapAlign: { code: PropertyCode.ScrollSnapAlign },
  scrollSnapStop: { code: PropertyCode.ScrollSnapStop },

  // Base UI-aligned popovers, tooltips, range parts, toasts, tabs, toolbars, fields, selection
  // controls, and dialogs. Each of these is declared ahead of the core's decision; nothing here
  // is a value JavaScript computes for itself.
  side: { code: PropertyCode.Side },
  align: { code: PropertyCode.Align },
  sideOffset: { code: PropertyCode.SideOffset },
  alignOffset: { code: PropertyCode.AlignOffset },
  collisionPadding: { code: PropertyCode.CollisionPadding },
  sticky: { code: PropertyCode.Sticky },
  modal: { code: PropertyCode.Modal },
  openOnHover: { code: PropertyCode.OpenOnHover },
  provider: { code: PropertyCode.Provider, normalize: normalizeComponentValue },
  timeout: { code: PropertyCode.Timeout, normalize: normalizeMilliseconds },
  hoverable: { code: PropertyCode.Hoverable },
  trackCursorAxis: { code: PropertyCode.TrackCursorAxis },
  closeOnClick: { code: PropertyCode.CloseOnClick },
  minStepsBetweenValues: { code: PropertyCode.MinStepsBetweenValues },
  thumbAlignment: { code: PropertyCode.ThumbAlignment },
  format: { code: PropertyCode.Format },
  smallStep: { code: PropertyCode.SmallStep },
  allowWheelScrub: { code: PropertyCode.AllowWheelScrub },
  snapOnStep: { code: PropertyCode.SnapOnStep },
  limit: { code: PropertyCode.Limit },
  stackExpanded: { code: PropertyCode.StackExpanded },
  pitch: { code: PropertyCode.Pitch },
  focusableWhenDisabled: { code: PropertyCode.FocusableWhenDisabled },
  validationMode: { code: PropertyCode.ValidationMode },
  validationDebounceTime: {
    code: PropertyCode.ValidationDebounceTime,
    normalize: normalizeMilliseconds,
  },
  parent: { code: PropertyCode.Parent },
  enterDuration: {
    code: PropertyCode.EnterDuration,
    normalize: normalizeMilliseconds,
  },
  exitDuration: {
    code: PropertyCode.ExitDuration,
    normalize: normalizeMilliseconds,
  },

  // Base UI-aligned menus, selects, and comboboxes. Every one is a declaration the core reads
  // before it decides anything, because the hosted boundary is never asked a question.
  closeParentOnEsc: { code: PropertyCode.CloseParentOnEsc },
  href: { code: PropertyCode.Href, normalize: normalizeMenuLink },
  multiple: { code: PropertyCode.Multiple },
  alignItemWithTrigger: { code: PropertyCode.AlignItemWithTrigger },
  autoHighlight: { code: PropertyCode.AutoHighlight },
  openOnInputClick: { code: PropertyCode.OpenOnInputClick },
  highlightItemOnHover: { code: PropertyCode.HighlightItemOnHover },
};

/** Background properties that accept either one color or one declared gradient. */
const backgroundProperties: Record<string, { color: PropertyCode; gradient: PropertyCode }> = {
  bg: {
    color: PropertyCode.BackgroundColor,
    gradient: PropertyCode.BackgroundGradient,
  },
  bgGradient: {
    color: PropertyCode.BackgroundColor,
    gradient: PropertyCode.BackgroundGradient,
  },
  hoverBg: {
    color: PropertyCode.HoverBackgroundColor,
    gradient: PropertyCode.HoverBackgroundGradient,
  },
  activeBg: {
    color: PropertyCode.ActiveBackgroundColor,
    gradient: PropertyCode.ActiveBackgroundGradient,
  },
  focusBg: {
    color: PropertyCode.FocusBackgroundColor,
    gradient: PropertyCode.FocusBackgroundGradient,
  },
};

/**
 * Property codes whose `false` is a declaration, not an absence.
 *
 * The Rust core defaults some of these to `true`, so the renderer must transmit the explicit
 * negative instead of clearing the property.
 */
const explicitFalseProperties = new Set<PropertyCode>([
  PropertyCode.Disabled,
  PropertyCode.DismissOnEscape,
  PropertyCode.DismissOnPointerOutside,
  PropertyCode.FocusOnPointer,
  PropertyCode.Checked,
  PropertyCode.Indeterminate,
  PropertyCode.Pressed,
  PropertyCode.ActivateOnFocus,
  PropertyCode.LoopFocus,
  PropertyCode.KeepMounted,
  PropertyCode.Open,
  PropertyCode.Required,
  PropertyCode.Invalid,
  PropertyCode.Touched,
  PropertyCode.Dirty,
  PropertyCode.Filled,
  PropertyCode.Sticky,
  PropertyCode.Modal,
  PropertyCode.OpenOnHover,
  PropertyCode.Hoverable,
  PropertyCode.CloseOnClick,
  PropertyCode.AllowWheelScrub,
  PropertyCode.SnapOnStep,
  PropertyCode.ReadOnly,
  PropertyCode.FocusableWhenDisabled,
  PropertyCode.Parent,
  PropertyCode.StackExpanded,
  PropertyCode.CloseParentOnEsc,
  PropertyCode.Multiple,
  PropertyCode.AlignItemWithTrigger,
  PropertyCode.AutoHighlight,
  PropertyCode.OpenOnInputClick,
  PropertyCode.HighlightItemOnHover,
  PropertyCode.SwiftUIColorSupportsOpacity,
]);

const colorProperties = new Set([
  PropertyCode.BackgroundColor,
  PropertyCode.Color,
  PropertyCode.HoverBackgroundColor,
  PropertyCode.HoverColor,
  PropertyCode.ActiveBackgroundColor,
  PropertyCode.ActiveColor,
  PropertyCode.BorderColor,
  PropertyCode.TextDecorationColor,
  PropertyCode.OutlineColor,
  PropertyCode.FocusBackgroundColor,
  PropertyCode.FocusColor,
]);

type StylePropertyState = {
  style: Record<string, unknown>;
};

// One flattened declaration owns every visual field. Weak ownership follows the retained node.
const stylePropertyStates = new WeakMap<NativeNode, StylePropertyState>();

function stylePropertyState(node: NativeNode): StylePropertyState {
  let state = stylePropertyStates.get(node);
  if (!state) {
    state = { style: {} };
    stylePropertyStates.set(node, state);
  }
  return state;
}

function applyStyleProperties(
  node: NativeNode,
  state: StylePropertyState,
  fields: Set<string>,
): void {
  for (const field of fields) {
    applyProperty(node, field, state.style[field] ?? null);
  }
}

function setProperty(
  node: NativeNode,
  name: string,
  value: PropertyInput,
  previous?: PropertyInput,
) {
  if (name === "style") {
    setStyle(node, value);
    return;
  }
  if (isDirectStyleName(name))
    throw new TypeError(`QuickGUI style property ${name} must be declared inside style`);
  applyProperty(node, name, value, previous);
}

function applyProperty(
  node: NativeNode,
  name: string,
  value: PropertyInput,
  previous?: PropertyInput,
) {
  if (name === "children" || name === "ref" || name === "key") return;
  if (name === "anchor") {
    if (value === null || value === undefined || value === false) {
      setNativeProperty(node, PropertyCode.AnchorTarget, null);
      setNativeProperty(node, PropertyCode.AnchorPoint, null);
    } else if (value instanceof NativeNode) {
      setNativeProperty(node, PropertyCode.AnchorPoint, null);
      setNativeProperty(node, PropertyCode.AnchorTarget, String(value.id));
    } else if (
      isRecord(value) &&
      typeof value.x === "number" &&
      typeof value.y === "number" &&
      Number.isFinite(value.x) &&
      Number.isFinite(value.y)
    ) {
      // Base UI's virtual element: a logical point the core anchors to directly.
      setNativeProperty(node, PropertyCode.AnchorTarget, null);
      setNativeProperty(node, PropertyCode.AnchorPoint, `${value.x},${value.y}`);
    } else {
      throw new TypeError("QuickGUI popover anchor must be a NativeNode or an { x, y } point");
    }
    return;
  }
  if (name === "controls") {
    if (value === null || value === undefined || value === false) {
      setNativeProperty(node, PropertyCode.Controls, null);
    } else if (value instanceof NativeNode) {
      setNativeProperty(node, PropertyCode.Controls, String(value.id));
    } else {
      throw new TypeError("QuickGUI controls target must be a NativeNode");
    }
    return;
  }
  if (name === "transition") {
    setTransition(node, value);
    return;
  }
  const backgroundEntry = backgroundProperties[name];
  if (backgroundEntry) {
    setBackground(node, backgroundEntry, value);
    return;
  }
  if (name === "outline") {
    setOutline(node, value);
    return;
  }
  if (name === "borderRadius") {
    setNativeProperty(node, PropertyCode.BorderRadius, normalizeCornerRadius(value));
    return;
  }
  const event = eventName(name);
  if (event) {
    setNativeEventListener(
      node,
      event,
      typeof value === "function"
        ? (nativeEvent) => {
            try {
              (value as NativeEventListener)(nativeEvent);
            } finally {
              // Solid 2 batches external writes until the host marks the event boundary.
              flushSolid();
            }
          }
        : undefined,
    );
    return;
  }
  if (name === "aria-label") name = "ariaLabel";
  if (name === "aria-modal") name = "ariaModal";
  if (name === "type") {
    setNativeProperty(node, PropertyCode.Password, value === "password");
    return;
  }
  if (name === "keymap") {
    setNativeProperty(node, PropertyCode.Keymap, encodeKeymap(value));
    return;
  }
  if (name === "draggable") {
    setNativeProperty(node, PropertyCode.Draggable, encodeDragSource(value));
    return;
  }
  if (name === "dropKinds") {
    setNativeProperty(node, PropertyCode.DropKinds, encodeDropKinds(value));
    return;
  }
  if (name === "matchContents") {
    setMatchContents(node, value);
    return;
  }
  if (name === "modifiers") {
    setNativeProperty(node, PropertyCode.SwiftUIModifiers, encodeSwiftUiModifiers(value));
    return;
  }
  if (name === "group") {
    // Tailwind's group marker: `true` opens an unnamed group and a string names one for
    // `groupHover: { group }` and `groupActive: { group }`. Option-like parts route their own
    // group label elsewhere.
    setNativeProperty(
      node,
      PropertyCode.HoverGroup,
      typeof value === "string" ? normalizeHoverGroupName(value) : value === true ? true : null,
    );
    return;
  }
  const stateCode = stateStyleCodes[name];
  if (stateCode !== undefined && isStateStyleDeclaration(name, value, previous)) {
    setNativeProperty(node, stateCode, encodeStateStyle(name, value));
    return;
  }
  const entry = properties[name];
  if (!entry) return;
  const normalized = entry.normalize ? entry.normalize(value) : normalizeValue(value, entry.code);
  setNativeProperty(node, entry.code, normalized, { color: !!entry.color });
}

function setMatchContents(node: NativeNode, value: PropertyInput): void {
  let horizontal = false;
  let vertical = false;
  if (value === true) {
    horizontal = true;
    vertical = true;
  } else if (isRecord(value)) {
    horizontal = value.horizontal === true;
    vertical = value.vertical === true;
  }
  setNativeProperty(node, PropertyCode.SwiftUIMatchContentsHorizontal, horizontal || null);
  setNativeProperty(node, PropertyCode.SwiftUIMatchContentsVertical, vertical || null);
}

/**
 * Flatten a `style` prop — one object, or an array of objects and falsy entries nested to any
 * depth — into the single object the renderer diffs, the React Native way: entries merge left to
 * right and later values win, so a conditional style is one expression. A nested interaction
 * state merges one level deep, so a later `hover` adds to or overrides individual keys of an
 * earlier `hover` instead of replacing it; a later `hover: null` removes it.
 */
export function flattenStyle(style: JSX.StyleProp): JSX.Style {
  const flattened: Record<string, unknown> = {};
  mergeStyleInto(flattened, style);
  return flattened as JSX.Style;
}

function mergeStyleInto(target: Record<string, unknown>, style: unknown): void {
  if (Array.isArray(style)) {
    for (const entry of style) mergeStyleInto(target, entry);
    return;
  }
  // `false`, `null`, and `undefined` entries are skipped, as is anything that is not a style.
  if (!isRecord(style)) return;
  for (const [name, value] of Object.entries(style)) {
    if (name.includes("-")) throw new TypeError(`QuickGUI style keys must use camelCase: ${name}`);
    const helper =
      name === "flex" && (typeof value === "number" || typeof value === "string")
        ? flexDeclaration(value)
        : expandStyleHelper(name, value);
    if (helper !== undefined) {
      Object.assign(target, helper);
      continue;
    }
    const current = target[name];
    if (!(name in stateStyleCodes) || !isStateEntries(name, value)) {
      target[name] = value;
    } else if (!isStateEntries(name, current)) {
      target[name] = value;
    } else if (groupStates.has(name)) {
      target[name] = mergeGroupStateEntries(current, value);
    } else {
      target[name] = { ...(current as object), ...(value as object) };
    }
  }
}

/** Whether a value is a state declaration that can be merged into: an object, or a group list. */
function isStateEntries(name: string, value: unknown): boolean {
  return isRecord(value) || (groupStates.has(name) && Array.isArray(value));
}

/**
 * Merge two group-state declarations: entries following the same group — both unnamed, or both
 * naming the same one — merge key by key, and entries following different groups accumulate, so a
 * style array can add a second group to follow without losing the first.
 */
function mergeGroupStateEntries(current: unknown, value: unknown): unknown {
  const entries = groupStateEntries(current);
  for (const entry of groupStateEntries(value)) {
    const index = entries.findIndex((existing) => existing.group === entry.group);
    if (index === -1) entries.push(entry);
    else entries[index] = { ...entries[index], ...entry };
  }
  return entries.length === 1 ? entries[0] : entries;
}

function groupStateEntries(value: unknown): Record<string, unknown>[] {
  if (Array.isArray(value)) {
    return value.filter(isRecord).map((entry) => ({ ...entry }));
  }
  return isRecord(value) ? [{ ...value }] : [];
}

function setStyle(node: NativeNode, value: PropertyInput): void {
  const next = flattenStyle(value as JSX.StyleProp) as Record<string, unknown>;
  const state = stylePropertyState(node);
  const old = state.style;
  state.style = next;
  const fields = new Set<string>();
  for (const name of Object.keys(old)) {
    if (helperProperties.has(name)) {
      if (!Object.is(next[name], old[name])) fields.add(name);
    } else if (!(name in next)) applyProperty(node, name, null, old[name]);
  }
  for (const [name, nextValue] of Object.entries(next)) {
    if (helperProperties.has(name)) {
      if (!Object.is(nextValue, old[name])) fields.add(name);
    } else if (!Object.is(nextValue, old[name])) applyProperty(node, name, nextValue, old[name]);
  }
  applyStyleProperties(node, state, fields);
}

/**
 * Declare either a solid background color or one bounded core gradient.
 *
 * The Rust binding parses the CSS `linear-gradient()` / `radial-gradient()` / `conic-gradient()`
 * grammar and the equivalent object form into the core's own `Gradient`, so the renderer only has
 * to decide which of the two properties one declaration belongs to and clear the other.
 */
function setBackground(
  node: NativeNode,
  entry: { color: PropertyCode; gradient: PropertyCode },
  value: PropertyInput,
): void {
  if (value === null || value === undefined || value === false) {
    setNativeProperty(node, entry.color, null);
    setNativeProperty(node, entry.gradient, null);
    return;
  }
  if (isRecord(value) || (typeof value === "string" && isGradient(value))) {
    setNativeProperty(node, entry.color, null);
    setNativeProperty(node, entry.gradient, normalizeStyleDeclaration(value));
    return;
  }
  setNativeProperty(node, entry.gradient, null);
  setNativeProperty(node, entry.color, parseColor(value as number | string), {
    color: true,
  });
}

function isGradient(value: string): boolean {
  return /(?:^|\s)(?:linear|radial|conic)-gradient\(/.test(value.trim());
}

/**
 * Property codes for the nested interaction-state objects, one bounded JSON declaration each.
 *
 * Every state is a declaration the core resolves on its own: which element is hovered, pressed,
 * visibly focused, disabled, invalid, dragging, a compatible drop target, or inside a hovered
 * group is never asked of JavaScript.
 */
const stateStyleCodes: Record<string, PropertyCode> = {
  hover: PropertyCode.HoverStyle,
  active: PropertyCode.ActiveStyle,
  focus: PropertyCode.FocusStyle,
  disabled: PropertyCode.DisabledStyle,
  invalid: PropertyCode.InvalidStyle,
  dragging: PropertyCode.DraggingStyle,
  dragOver: PropertyCode.DragOverStyle,
  groupHover: PropertyCode.GroupHoverStyle,
  groupActive: PropertyCode.GroupActiveStyle,
  focusWithin: PropertyCode.FocusWithinStyle,
  selected: PropertyCode.SelectedStyle,
};

/** States declared once per group they follow, so they accept a list of entries. */
const groupStates = new Set(["groupHover", "groupActive"]);

/** States painted while the pointer rests on some other element, so they cannot pick a cursor. */
const pointerlessStates = new Set(["groupHover", "groupActive", "focusWithin"]);

/** The paint-only properties a state may swap in: exactly what the core's `ElementStateStyle` carries. */
const stateStyleProperties =
  "bg, bgGradient, color, borderColor, borderWidth, borderRadius, outline, " +
  "boxShadow, opacity, cursor, transform, and transformOrigin";

type EncodedStateStyle = {
  /** The named hover group a `groupHover` follows. */
  group?: string;
  backgroundColor?: number;
  background?: string;
  color?: number;
  borderColor?: number;
  borderWidth?: number;
  borderRadius?: number;
  outline?: string;
  boxShadow?: EncodedBoxShadow[];
  opacity?: number;
  cursor?: string;
  transform?: string;
  transformOrigin?: string;
};

/**
 * Whether `name` declares a nested state style.
 *
 * `disabled`, `invalid`, and `selected` are also boolean flags, so a withdrawal is a state style
 * only when the value it withdraws was one; the other state names have no second meaning.
 */
function isStateStyleDeclaration(
  name: string,
  value: PropertyInput,
  previous: PropertyInput,
): boolean {
  if (!(name in stateStyleCodes)) return false;
  if (isRecord(value) || (groupStates.has(name) && Array.isArray(value))) {
    return true;
  }
  if (value === null || value === undefined || value === false) {
    return isRecord(previous) || !(name in properties);
  }
  return false;
}

/**
 * Encode one nested state as the bounded JSON declaration the Rust binding parses into the core's
 * own `ElementStateStyle`. A group state may be a list of entries, one per group it follows, which
 * the core layers in declaration order.
 *
 * Colors are validated and packed here, exactly as every base color property is, so a declaration
 * the core cannot paint throws at the JavaScript boundary instead of silently painting nothing.
 * Layout properties are refused for the same reason: a state is paint-only in the core.
 */
function encodeStateStyle(state: string, value: PropertyInput): string | null {
  const entries = groupStates.has(state) && Array.isArray(value) ? value : [value];
  const encoded: EncodedStateStyle[] = [];
  for (const entry of entries) {
    if (entry === null || entry === undefined || entry === false) continue;
    if (!isRecord(entry)) {
      throw new TypeError(`QuickGUI \`${state}\` must be a style object`);
    }
    const declaration = encodeStateEntry(state, entry);
    if (declaration !== null) encoded.push(declaration);
  }
  if (encoded.length === 0) return null;
  if (encoded.length > MAX_GROUP_STYLES_PER_ELEMENT) {
    throw new TypeError(
      `QuickGUI \`${state}\` follows at most ${MAX_GROUP_STYLES_PER_ELEMENT} groups`,
    );
  }
  return bounded(
    JSON.stringify(encoded.length === 1 ? encoded[0] : encoded),
    MAX_STATE_STYLE_JSON_BYTES,
    `\`${state}\` style declarations`,
  );
}

function encodeStateEntry(state: string, value: Record<string, unknown>): EncodedStateStyle | null {
  const encoded: EncodedStateStyle = {};
  for (const [name, declared] of Object.entries(value)) {
    if (declared === null || declared === undefined || declared === false) continue;
    switch (name) {
      case "bg":
      case "bgGradient": {
        if (isRecord(declared) || (typeof declared === "string" && isGradient(declared))) {
          const gradient = normalizeStyleDeclaration(declared);
          if (gradient !== null) encoded.background = gradient;
        } else {
          encoded.backgroundColor = parseColor(declared as number | string);
        }
        break;
      }
      case "color":
      case "borderColor":
        encoded[name] = parseColor(declared as number | string);
        break;
      case "textColor":
        encoded.color = parseColor(declared as number | string);
        break;
      case "borderWidth":
      case "borderRadius":
        encoded[name] = stateLength(state, name, declared);
        break;
      case "opacity":
        if (typeof declared !== "number" || !Number.isFinite(declared)) {
          throw new TypeError(`QuickGUI \`${state}\` opacity must be a finite number`);
        }
        encoded.opacity = declared;
        break;
      case "outline":
        encoded.outline = normalizeOutlineShorthand(declared);
        break;
      case "boxShadow": {
        const shadows = parseBoxShadowList(declared);
        if (shadows !== null) encoded.boxShadow = shadows;
        break;
      }
      case "cursor":
        if (pointerlessStates.has(state)) {
          throw new TypeError(
            `QuickGUI \`${state}\` styles cannot declare a cursor; the pointer rests on another element`,
          );
        }
        encoded.cursor = String(declared);
        break;
      case "group":
        if (!groupStates.has(state)) {
          throw new TypeError(
            `QuickGUI only \`groupHover\` and \`groupActive\` follow a named group; \`${state}\` cannot declare \`group\``,
          );
        }
        encoded.group = normalizeHoverGroupName(declared);
        break;
      case "transform": {
        const transform = normalizeStyleDeclaration(declared);
        if (transform !== null) encoded.transform = transform;
        break;
      }
      case "transformOrigin":
        encoded.transformOrigin = String(declared).trim();
        break;
      default:
        throw new TypeError(
          `QuickGUI \`${state}\` styles are paint-only and accept ${stateStyleProperties}, not \`${name}\``,
        );
    }
  }
  return Object.keys(encoded).length === 0 ? null : encoded;
}

/** A hover group name is non-empty and bounded exactly as the core bounds it. */
function normalizeHoverGroupName(value: unknown): string {
  const name = String(value).trim();
  if (name === "") {
    throw new TypeError("QuickGUI hover group names cannot be empty");
  }
  if (inputTextEncoder.encode(name).length > MAX_HOVER_GROUP_NAME_BYTES) {
    throw new TypeError(
      `QuickGUI hover group names are limited to ${MAX_HOVER_GROUP_NAME_BYTES} bytes`,
    );
  }
  return name;
}

/**
 * An option-like part's `group` is its option group label, never a hover group, so it travels
 * under the option-group property and leaves the node-level `group` its Tailwind meaning.
 */
function optionPartProps(props: JSX.OptionProps): object {
  return universal.mergeProps(omit(props, "group"), {
    get optionGroup() {
      return props.group;
    },
  }) as object;
}

/** A state length is whole logical pixels: the core swaps one paint-only width or radius. */
function stateLength(state: string, name: string, value: unknown): number {
  const length = typeof value === "number" ? value : normalizeLength(String(value));
  if (typeof length !== "number" || !Number.isFinite(length)) {
    throw new TypeError(`QuickGUI \`${state}\` ${name} must be a number of logical pixels`);
  }
  return length;
}

/**
 * Validate the CSS `outline` shorthand a state declares and pass it on whole; the Rust binding
 * splits it with the grammar the base `outline` already uses, and `none` removes the ring.
 */
function normalizeOutlineShorthand(value: unknown): string {
  if (typeof value === "number") return `${value}px`;
  const shorthand = String(value).trim();
  if (shorthand === "" || shorthand.toLowerCase() === "none") return "none";
  for (const token of splitCssTokens(shorthand)) {
    if (token === "solid" || token === "dashed" || token === "dotted") continue;
    if (typeof normalizeLength(token) === "number") continue;
    parseColor(token);
  }
  return normalizeStyleDeclaration(shorthand) ?? "none";
}

/**
 * Split the CSS `outline` shorthand into the width, style, and color the core declares separately.
 */
function setOutline(node: NativeNode, value: PropertyInput): void {
  const clear = () => {
    for (const code of [
      PropertyCode.OutlineWidth,
      PropertyCode.OutlineColor,
      PropertyCode.OutlineStyle,
    ]) {
      setNativeProperty(node, code, null);
    }
  };
  if (value === null || value === undefined || value === false) {
    clear();
    return;
  }
  clear();
  if (typeof value === "number") {
    setNativeProperty(node, PropertyCode.OutlineWidth, value);
    return;
  }
  const shorthand = String(value).trim();
  if (shorthand === "" || shorthand === "none") {
    setNativeProperty(node, PropertyCode.OutlineStyle, "none");
    return;
  }
  for (const token of shorthand.split(/\s+/).filter(Boolean)) {
    if (token === "solid" || token === "dashed" || token === "dotted") {
      setNativeProperty(node, PropertyCode.OutlineStyle, token);
      continue;
    }
    const width = normalizeLength(token);
    if (typeof width === "number") {
      setNativeProperty(node, PropertyCode.OutlineWidth, width);
      continue;
    }
    setNativeProperty(node, PropertyCode.OutlineColor, parseColor(token), {
      color: true,
    });
  }
}

/** A uniform radius stays a number; a one-to-four value shorthand travels as its CSS text. */
function normalizeCornerRadius(value: PropertyInput): number | string | null {
  if (value === null || value === undefined || value === false) return null;
  if (typeof value === "number") return value;
  const text = String(value).trim();
  if (text === "") return null;
  if (/\s/.test(text)) {
    if (text.split(/\s+/).length > 4) {
      throw new TypeError("QuickGUI borderRadius accepts one to four corner radii");
    }
    return text;
  }
  return normalizeLength(text);
}

/**
 * Bound one gradient, filter, transform, outline, or text-shadow declaration.
 *
 * An array joins into a CSS list, an object becomes the JSON form the Rust binding deserializes,
 * and anything past `MAX_STYLE_DECLARATION_BYTES` is refused instead of reaching the boundary.
 */
function normalizeStyleDeclaration(value: PropertyInput): string | null {
  if (value === null || value === undefined || value === false) return null;
  const text = Array.isArray(value)
    ? value.map((entry) => String(entry)).join(" ")
    : isRecord(value)
      ? JSON.stringify(value)
      : String(value).trim();
  if (text === "") return null;
  if (text.length > MAX_STYLE_DECLARATION_BYTES) {
    throw new TypeError(
      `QuickGUI style declarations are limited to ${MAX_STYLE_DECLARATION_BYTES} bytes`,
    );
  }
  return text;
}

function normalizeValue(
  value: PropertyInput,
  code: PropertyCode,
): boolean | number | string | null {
  if (value === null || value === undefined) return null;
  if (value === false) return explicitFalseProperties.has(code) ? false : null;
  if (colorProperties.has(code)) return parseColor(value as number | string);
  if (typeof value === "number" || typeof value === "boolean") return value;
  if (isLengthProperty(code)) return normalizeLength(String(value));
  return String(value);
}

function normalizeLength(value: string | undefined): number | string | null {
  if (value === undefined) return null;
  const trimmed = value.trim();
  if (trimmed.endsWith("px")) {
    const number = Number(trimmed.slice(0, -2));
    return Number.isFinite(number) ? number : null;
  }
  if (trimmed === "0") return 0;
  const number = Number(trimmed);
  return Number.isFinite(number) ? number : trimmed;
}

/** Public paint property names mapped to the transition names the Rust core reads. */
const transitionProperties = new Map<string, string>([
  ["all", "all"],
  ["bg", "background-color"],
  ["border-color", "border-color"],
  ["border-width", "border-width"],
  ["border-radius", "border-radius"],
  ["color", "color"],
  ["box-shadow", "box-shadow"],
  ["opacity", "opacity"],
]);

const transitionEasings = new Set(["linear", "ease", "ease-in", "ease-out", "ease-in-out"]);

/** Duration in milliseconds from a `120ms`, `0.2s`, or plain-number declaration. */
function normalizeMilliseconds(value: PropertyInput): number | null {
  if (value === null || value === undefined || value === false) return null;
  if (typeof value === "number") {
    return Number.isFinite(value) ? value : null;
  }
  const text = String(value).trim();
  const match = text.match(/^(\d*\.?\d+)(ms|s)?$/);
  if (!match) return null;
  return Number(match[1]) * (match[2] === "s" ? 1_000 : 1);
}

/**
 * Declare a complete paint transition.
 *
 * The Rust core owns interpolation, cadence, and which paint properties can transition; this only
 * translates the CSS-shaped declaration into the property, duration, and easing the core reads.
 */
function setTransition(node: NativeNode, value: PropertyInput): void {
  const clear = () => {
    for (const code of [
      PropertyCode.Transition,
      PropertyCode.TransitionProperties,
      PropertyCode.TransitionDuration,
      PropertyCode.TransitionEasing,
      PropertyCode.TransitionMaxFps,
    ]) {
      setNativeProperty(node, code, null);
    }
  };
  if (value === null || value === undefined || value === false) {
    clear();
    return;
  }
  if (typeof value === "number") {
    clear();
    setNativeProperty(node, PropertyCode.Transition, value);
    return;
  }
  if (isRecord(value)) {
    clear();
    const duration = normalizeMilliseconds(value.duration as PropertyInput);
    if (duration !== null) {
      setNativeProperty(node, PropertyCode.TransitionDuration, duration);
    }
    const declared = value.property ?? value.properties;
    if (declared !== undefined && declared !== null) {
      const names = (Array.isArray(declared) ? declared : [declared])
        .map((name) => normalizeTransitionProperty(String(name)))
        .join(",");
      setNativeProperty(node, PropertyCode.TransitionProperties, names);
    }
    if (typeof value.easing === "string" || typeof value.timingFunction === "string") {
      setNativeProperty(
        node,
        PropertyCode.TransitionEasing,
        normalizeTransitionEasing(String(value.easing ?? value.timingFunction)),
      );
    }
    if (typeof value.maxFps === "number") {
      setNativeProperty(node, PropertyCode.TransitionMaxFps, value.maxFps);
    }
    if (value.delay !== undefined && Number(value.delay) !== 0) {
      throw new TypeError("QuickGUI transitions do not support a non-zero delay");
    }
    return;
  }
  if (typeof value !== "string") {
    throw new TypeError("QuickGUI transition must use the CSS transition shorthand");
  }
  const shorthand = value.trim();
  clear();
  if (shorthand === "" || shorthand === "none") return;

  const declared = new Set<string>();
  let sharedDuration: number | undefined;
  let easing: string | undefined;
  for (const declaration of splitCssList(shorthand)) {
    const tokens = declaration.split(/\s+/).filter(Boolean);
    const property = tokens.find((token) => transitionProperties.has(token));
    if (!property) {
      throw new TypeError(
        `QuickGUI cannot transition \`${declaration}\`; the core transitions bg, border-color, border-width, border-radius, color, box-shadow, and opacity`,
      );
    }
    declared.add(transitionProperties.get(property)!);
    const times = Array.from(
      declaration.matchAll(/(?:^|\s)(\d*\.?\d+)(ms|s)(?=\s|$)/g),
      (match) => Number(match[1]) * (match[2] === "s" ? 1_000 : 1),
    );
    const duration = times[0] ?? 0;
    if ((times[1] ?? 0) !== 0) {
      throw new TypeError("QuickGUI transitions do not support a non-zero delay");
    }
    if (sharedDuration !== undefined && sharedDuration !== duration) {
      throw new TypeError("QuickGUI transition properties must share one duration");
    }
    sharedDuration = duration;
    const declaredEasing = tokens.find((token) => transitionEasings.has(token));
    if (declaredEasing) easing = declaredEasing;
  }
  setNativeProperty(node, PropertyCode.Transition, sharedDuration ?? 0);
  setNativeProperty(node, PropertyCode.TransitionProperties, Array.from(declared).join(","));
  if (easing) {
    setNativeProperty(node, PropertyCode.TransitionEasing, normalizeTransitionEasing(easing));
  }
}

function normalizeTransitionProperty(name: string): string {
  const normalized = transitionProperties.get(name.trim());
  if (!normalized) {
    throw new TypeError(`QuickGUI cannot transition \`${name}\``);
  }
  return normalized;
}

function normalizeTransitionProperties(value: PropertyInput): string | null {
  if (value === null || value === undefined || value === false) return null;
  return String(value)
    .split(",")
    .map((name) => transitionProperties.get(name.trim()) ?? name.trim())
    .join(",");
}

function normalizeTransitionEasing(name: string): string {
  const normalized = name.trim();
  if (!transitionEasings.has(normalized)) {
    throw new TypeError(`QuickGUI does not expose the \`${normalized}\` easing curve`);
  }
  return normalized === "ease" ? "ease-in-out" : normalized;
}

/** Normalize a CSS grid track list. A plain number declares that many equal `1fr` tracks. */
function normalizeGridTemplate(value: PropertyInput): number | string | null {
  if (value === null || value === undefined || value === false) return null;
  if (typeof value === "number") {
    return Number.isFinite(value) ? value : null;
  }
  const tracks = Array.isArray(value) ? value.join(" ") : String(value);
  const normalized = tracks.trim().replace(/\s+/g, " ");
  return normalized === "" || normalized === "none" ? null : normalized;
}

/** Bounded shader parameter floats, packed into the core's four fixed vectors. */
function normalizeShaderParameters(value: PropertyInput): string | null {
  if (value === null || value === undefined || value === false) return null;
  if (!Array.isArray(value)) {
    throw new TypeError("QuickGUI shader parameters must be an array of numbers");
  }
  const floats = value.flat(2).map((entry) => {
    const number = Number(entry);
    if (!Number.isFinite(number)) {
      throw new TypeError("QuickGUI shader parameters must be finite numbers");
    }
    return number;
  });
  if (floats.length > MAX_SHADER_PARAMETER_FLOATS) {
    throw new RangeError(`QuickGUI exposes ${MAX_SHADER_PARAMETER_FLOATS} shader parameter floats`);
  }
  return JSON.stringify(floats);
}

/** Four four-component vectors, matching the Rust core's fixed shader uniform. */
export const MAX_SHADER_PARAMETER_FLOATS = 16;

function splitCssList(value: string): string[] {
  const values: string[] = [];
  let start = 0;
  let depth = 0;
  for (let index = 0; index < value.length; index += 1) {
    const character = value[index];
    if (character === "(") depth += 1;
    else if (character === ")") depth = Math.max(0, depth - 1);
    else if (character === "," && depth === 0) {
      values.push(value.slice(start, index).trim());
      start = index + 1;
    }
  }
  values.push(value.slice(start).trim());
  return values.filter(Boolean);
}

const MAX_BOX_SHADOWS_PER_ELEMENT = 8;

type EncodedBoxShadow = {
  offsetX: number;
  offsetY: number;
  blurRadius: number;
  spreadRadius: number;
  color: number | null;
  inset: boolean;
};

function normalizeBoxShadow(value: PropertyInput): string | null {
  const shadows = parseBoxShadowList(value);
  return shadows === null || shadows.length === 0 ? null : JSON.stringify(shadows);
}

/** The declared shadow entries; `none` is an empty list and an absent declaration is `null`. */
function parseBoxShadowList(value: PropertyInput): EncodedBoxShadow[] | null {
  if (value === null || value === undefined || value === false) return null;
  if (typeof value !== "string") {
    throw new TypeError("QuickGUI boxShadow must use the CSS box-shadow shorthand");
  }
  const shorthand = value.trim();
  if (shorthand === "" || shorthand.toLowerCase() === "none") return [];

  const declarations = splitCssList(shorthand);
  if (declarations.length > MAX_BOX_SHADOWS_PER_ELEMENT) {
    throw new TypeError(
      `QuickGUI boxShadow supports at most ${MAX_BOX_SHADOWS_PER_ELEMENT} shadows`,
    );
  }
  return declarations.map(parseBoxShadowDeclaration);
}

function parseBoxShadowDeclaration(declaration: string): EncodedBoxShadow {
  const lengths: number[] = [];
  let color: number | null = null;
  let hasColor = false;
  let inset = false;

  for (const token of splitCssTokens(declaration)) {
    if (token.toLowerCase() === "inset") {
      if (inset) throw new TypeError("QuickGUI boxShadow repeats `inset`");
      inset = true;
      continue;
    }
    const length = parseShadowLength(token);
    if (length !== undefined) {
      if (lengths.length === 4) {
        throw new TypeError("QuickGUI boxShadow accepts two to four length values");
      }
      lengths.push(length);
      continue;
    }
    if (hasColor) {
      throw new TypeError("QuickGUI boxShadow accepts one color per shadow");
    }
    hasColor = true;
    if (token.toLowerCase() !== "currentcolor") {
      color = parseColor(token);
    }
  }

  if (lengths.length < 2) {
    throw new TypeError("QuickGUI boxShadow requires horizontal and vertical offsets");
  }
  const blurRadius = lengths[2] ?? 0;
  if (blurRadius < 0) {
    throw new TypeError("QuickGUI boxShadow blur radius cannot be negative");
  }
  return {
    offsetX: lengths[0]!,
    offsetY: lengths[1]!,
    blurRadius,
    spreadRadius: lengths[3] ?? 0,
    color,
    inset,
  };
}

function splitCssTokens(value: string): string[] {
  const tokens: string[] = [];
  let start = 0;
  let depth = 0;
  for (let index = 0; index < value.length; index += 1) {
    const character = value[index]!;
    if (character === "(") depth += 1;
    else if (character === ")") {
      depth -= 1;
      if (depth < 0) {
        throw new TypeError("QuickGUI boxShadow has unbalanced parentheses");
      }
    } else if (/\s/.test(character) && depth === 0) {
      const token = value.slice(start, index).trim();
      if (token) tokens.push(token);
      start = index + 1;
    }
  }
  if (depth !== 0) {
    throw new TypeError("QuickGUI boxShadow has unbalanced parentheses");
  }
  const token = value.slice(start).trim();
  if (token) tokens.push(token);
  return tokens;
}

function parseShadowLength(value: string): number | undefined {
  const match = value.match(/^([+-]?(?:\d+(?:\.\d*)?|\.\d+))(?:px)?$/i);
  if (!match) return undefined;
  const length = Number(match[1]);
  return Number.isFinite(length) ? length : undefined;
}

const textEncoder = new TextEncoder();

function normalizeComponentValue(value: PropertyInput): string | null {
  if (value === null || value === undefined || value === false) return null;
  const text = String(value);
  if (text.length === 0) return null;
  if (textEncoder.encode(text).length > MAX_COMPONENT_VALUE_BYTES) {
    throw new TypeError(
      `QuickGUI component scopes and values are limited to ${MAX_COMPONENT_VALUE_BYTES} bytes`,
    );
  }
  return text;
}

/**
 * Bound one declared `Menu.LinkItem` destination exactly as the Rust core bounds it.
 *
 * A URL is longer than a component scope, so it carries its own bound rather than borrowing the
 * scope's; anything longer is refused before it can cross the boundary.
 */
function normalizeMenuLink(value: PropertyInput): string | null {
  if (value === null || value === undefined || value === false) return null;
  const text = String(value);
  if (text.length === 0) return null;
  if (textEncoder.encode(text).length > MAX_MENU_LINK_BYTES) {
    throw new TypeError(`QuickGUI menu links are bounded to ${MAX_MENU_LINK_BYTES} bytes`);
  }
  return text;
}

/**
 * Serialize one declared component list into the bounded JSON the Rust binding decodes.
 *
 * Slider thumb values, splitter pane sizes, toggle-group pressed values, and ordered toolbar or
 * toggle-group items all travel as one declaration so the core can answer a keypress without ever
 * asking JavaScript a synchronous question.
 */
/**
 * Serialize one declared `{ width, height }` extent as the bounded `[width, height]` pair the
 * Rust binding decodes.
 *
 * QuickGUI has no layout observer at the hosted boundary, so a scroll area's viewport and content
 * extents are declared ahead of the core's decision exactly like every other bounded property.
 */
function normalizeExtent(value: PropertyInput): string | null {
  if (value === null || value === undefined || value === false) return null;
  const pair = Array.isArray(value)
    ? value
    : typeof value === "object"
      ? [(value as { width?: number }).width ?? 0, (value as { height?: number }).height ?? 0]
      : null;
  if (!pair || pair.length < 2) return null;
  for (const component of pair) {
    if (typeof component !== "number" || !Number.isFinite(component)) {
      throw new TypeError("QuickGUI extents must be finite numbers");
    }
  }
  return JSON.stringify([pair[0], pair[1]]);
}

function normalizeItemHeights(value: PropertyInput): string | null {
  if (value === null || value === undefined || value === false) return null;
  if (!Array.isArray(value)) {
    throw new TypeError("QuickGUI item heights must be an array of numbers");
  }
  const heights = Array.from(value, (height) => {
    if (typeof height !== "number" || !Number.isFinite(height) || height <= 0) {
      throw new TypeError("QuickGUI item heights must be positive finite numbers");
    }
    return height;
  });
  const encoded = JSON.stringify(heights);
  if (textEncoder.encode(encoded).length > 1024 * 1024) {
    throw new TypeError("QuickGUI item-height declarations are limited to 1048576 bytes");
  }
  return encoded;
}

function normalizeComponentJson(value: PropertyInput): string | null {
  if (value === null || value === undefined || value === false) return null;
  const encoded = Array.isArray(value) ? JSON.stringify(value) : String(value);
  if (encoded.length === 0 || encoded === "[]") return encoded === "[]" ? encoded : null;
  if (textEncoder.encode(encoded).length > MAX_COMPONENT_JSON_BYTES) {
    throw new TypeError(
      `QuickGUI component declarations are limited to ${MAX_COMPONENT_JSON_BYTES} bytes`,
    );
  }
  return encoded;
}

/**
 * Serialize one declared option source, column list, node source, or toast queue.
 *
 * Every declaration crossing the boundary is bounded exactly as the Rust binding bounds it, so an
 * oversized source is refused here instead of being silently dropped by the decoder.
 */
function boundedJson(limit: number, what: string) {
  return (value: PropertyInput): string | null => {
    if (value === null || value === undefined || value === false) return null;
    const encoded = typeof value === "string" ? value : JSON.stringify(value ?? null);
    if (encoded === undefined) return null;
    if (textEncoder.encode(encoded).length > limit) {
      throw new TypeError(`QuickGUI ${what} declarations are limited to ${limit} bytes`);
    }
    return encoded;
  };
}

function normalizeOptionsJson(value: PropertyInput): string | null {
  return boundedJson(MAX_OPTIONS_JSON_BYTES, "option source")(value);
}

function normalizeCollectionJson(value: PropertyInput): string | null {
  return boundedJson(MAX_COLLECTION_JSON_BYTES, "collection")(value);
}

function normalizeAppearanceJson(value: PropertyInput): string | null {
  return boundedJson(MAX_COMPONENT_JSON_BYTES, "component appearance")(value);
}

function normalizeTooltipText(value: PropertyInput): string | null {
  if (value === null || value === undefined || value === false) return null;
  const text = String(value);
  if (text.length === 0) return null;
  if (textEncoder.encode(text).length > MAX_TOOLTIP_TEXT_BYTES) {
    throw new TypeError(`QuickGUI tooltip text is limited to ${MAX_TOOLTIP_TEXT_BYTES} bytes`);
  }
  return text;
}

function isLengthProperty(code: PropertyCode): boolean {
  return (
    code === PropertyCode.FlexBasis ||
    (code >= PropertyCode.Gap && code <= PropertyCode.MarginLeft) ||
    code === PropertyCode.BorderWidth ||
    (code >= PropertyCode.BorderTopWidth && code <= PropertyCode.BorderLeftWidth) ||
    code === PropertyCode.BorderRadius ||
    code === PropertyCode.FontSize ||
    code === PropertyCode.LineHeight ||
    (code >= PropertyCode.Top && code <= PropertyCode.Left) ||
    code === PropertyCode.AnchorGap ||
    code === PropertyCode.ViewportMargin ||
    (code >= PropertyCode.HitSlop && code <= PropertyCode.HitSlopLeft) ||
    code === PropertyCode.LetterSpacing ||
    code === PropertyCode.WordSpacing ||
    code === PropertyCode.TextDecorationThickness ||
    (code >= PropertyCode.PaddingStart && code <= PropertyCode.BorderEndWidth) ||
    (code >= PropertyCode.BorderTopLeftRadius && code <= PropertyCode.BorderBottomLeftRadius) ||
    code === PropertyCode.OutlineWidth ||
    code === PropertyCode.OutlineOffset
  );
}

function eventName(
  name: string,
):
  | "click"
  | "mouseenter"
  | "mouseleave"
  | "input"
  | "submit"
  | "dismiss"
  | "pointer"
  | "presentationchange"
  | "menuselect"
  | "keydown"
  | "keyup"
  | "mousedown"
  | "mouseup"
  | "mousemove"
  | "dblclick"
  | "wheel"
  | "contextmenu"
  | "pinch"
  | "rotate"
  | "smartmagnify"
  | "pressure"
  | "focus"
  | "blur"
  | "action"
  | "dragstart"
  | "dragend"
  | "drop"
  | "filesdropped"
  | "componentchange"
  | "commit"
  | undefined {
  switch (name.toLowerCase()) {
    case "onclick":
    case "on:click":
    case "onpress":
      return "click";
    case "onmouseenter":
    case "onpointerenter":
      return "mouseenter";
    case "onmouseleave":
    case "onpointerleave":
      return "mouseleave";
    case "oninput":
    case "onchange":
      return "input";
    case "onsubmit":
      return "submit";
    case "ondismiss":
    case "on:dismiss":
      return "dismiss";
    case "onpointer":
    case "on:pointer":
      return "pointer";
    case "onselect":
    case "on:select":
    case "onmenuselect":
      return "menuselect";
    case "oncomponentchange":
    case "on:componentchange":
      return "componentchange";
    case "oncommit":
    case "on:commit":
    case "onactivate":
      return "commit";
    case "onkeydown":
    case "on:keydown":
      return "keydown";
    case "onkeyup":
    case "on:keyup":
      return "keyup";
    case "onmousedown":
    case "onpointerdown":
      return "mousedown";
    case "onmouseup":
    case "onpointerup":
      return "mouseup";
    case "onmousemove":
    case "onpointermove":
      return "mousemove";
    case "ondoubleclick":
    case "ondblclick":
      return "dblclick";
    case "onwheel":
    case "onscrollwheel":
      return "wheel";
    case "oncontextmenu":
    case "on:contextmenu":
      return "contextmenu";
    case "onpinch":
      return "pinch";
    case "onrotate":
    case "onrotation":
      return "rotate";
    case "onsmartmagnify":
      return "smartmagnify";
    case "onpressure":
      return "pressure";
    case "onfocus":
      return "focus";
    case "onblur":
      return "blur";
    case "onaction":
    case "on:action":
      return "action";
    case "ondragstart":
      return "dragstart";
    case "ondragend":
      return "dragend";
    case "ondrop":
      return "drop";
    case "onfilesdropped":
      return "filesdropped";
    case "onispresentedchange":
    case "onpresentationchange":
    case "on:presentationchange":
      return "presentationchange";
    default:
      return undefined;
  }
}


function encodeSwiftUiModifiers(value: unknown): string | null {
  if (value === null || value === undefined) return null;
  if (!Array.isArray(value)) {
    throw new TypeError("QuickGUI SwiftUI modifiers must be an array");
  }
  const modifiers = value.map((modifier, index) => {
    if (!isRecord(modifier) || typeof modifier.$type !== "string") {
      throw new TypeError(`QuickGUI SwiftUI modifier ${index} must contain a $type`);
    }
    switch (modifier.$type) {
      case "buttonStyle":
        return {
          $type: modifier.$type,
          style: swiftUiEnum(
            modifier.style,
            [
              "automatic",
              "bordered",
              "borderedProminent",
              "borderless",
              "glass",
              "glassProminent",
              "plain",
            ],
            modifier.$type,
          ),
        };
      case "buttonBorderShape": {
        const shape = swiftUiEnum(
          modifier.shape,
          ["automatic", "capsule", "roundedRectangle", "circle"],
          modifier.$type,
        );
        const cornerRadius = modifier.cornerRadius;
        if (
          cornerRadius !== undefined &&
          (typeof cornerRadius !== "number" || !Number.isFinite(cornerRadius) || cornerRadius < 0)
        ) {
          throw new TypeError(
            "QuickGUI buttonBorderShape cornerRadius must be a non-negative finite number",
          );
        }
        return {
          $type: modifier.$type,
          shape,
          ...(cornerRadius === undefined ? {} : { cornerRadius }),
        };
      }
      case "controlSize":
        return {
          $type: modifier.$type,
          size: swiftUiEnum(
            modifier.size,
            ["mini", "small", "regular", "large", "extraLarge"],
            modifier.$type,
          ),
        };
      case "labelStyle":
        return {
          $type: modifier.$type,
          style: swiftUiEnum(
            modifier.style,
            ["automatic", "iconOnly", "titleAndIcon", "titleOnly"],
            modifier.$type,
          ),
        };
      case "tint":
        if (typeof modifier.color !== "string" || modifier.color.length === 0) {
          throw new TypeError("QuickGUI tint color must be a non-empty string");
        }
        return { $type: modifier.$type, color: modifier.color };
      case "disabled":
        if (typeof modifier.disabled !== "boolean") {
          throw new TypeError("QuickGUI disabled modifier must contain a boolean");
        }
        return { $type: modifier.$type, disabled: modifier.disabled };
      default:
        throw new TypeError(`unsupported QuickGUI SwiftUI modifier \`${modifier.$type}\``);
    }
  });
  return JSON.stringify(modifiers);
}

function swiftUiEnum(value: unknown, allowed: readonly string[], modifier: string): string {
  if (typeof value === "string" && allowed.includes(value)) return value;
  throw new TypeError(`QuickGUI ${modifier} must be one of ${allowed.join(", ")}`);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

const universal = createUniversalRenderer<NativeNode>({
  createElement(tag, staticProps) {
    // Intrinsic span uses the same native text container as Text.
    const name = (tag === "span" ? "text" : tag) as NativeElementName;
    if (
      ![
        "view",
        "div",
        "text",
        "button",
        "input",
        "textarea",
        "virtual-list",
        "extension",
        "svg",
        "image",
        "shader",
        "swift-ui-host",
        "swift-ui-button",
        "swift-ui-quickgui-host",
        "swift-ui-popover",
        "swift-ui-popover-trigger",
        "swift-ui-popover-content",
        "swift-ui-slider",
        "swift-ui-toggle",
        "swift-ui-progress-view",
        "swift-ui-stepper",
        "swift-ui-text-field",
        "swift-ui-picker",
        "swift-ui-date-picker",
        "swift-ui-color-picker",
        "swift-ui-gauge",
      ].includes(name)
    ) {
      throw new TypeError(`unknown QuickGUI element <${tag}>`);
    }
    const node = createNativeElement(name);
    if (staticProps) {
      for (const [name, value] of Object.entries(staticProps)) setProperty(node, name, value);
    }
    return node;
  },
  createTextNode: createNativeText,
  createSentinel: createNativeSentinel,
  replaceText: replaceNativeText,
  isTextNode: isNativeText,
  setProperty,
  insertNode: insertNativeNode,
  removeNode: removeNativeNode,
  cleanupNodes: cleanupNativeNodes,
  getParentNode: getNativeParent,
  getFirstChild: getNativeFirstChild,
  getNextSibling: getNativeNextSibling,
});

const nativeRender = universal.render;

/** Unstyled block/flex/grid container. */
export function View(props: JSX.NativeProps): NativeNode {
  const node = universal.createElement("view");
  universal.spread(node, props);
  return node;
}

/** Unstyled text-semantic container whose string children remain individually reactive. */
export function Text(props: JSX.NativeProps): NativeNode {
  const node = universal.createElement("text");
  universal.spread(node, props);
  return node;
}

/** Unstyled, focusable native button with web-style arrow-cursor behavior by default. */
export function Button(props: JSX.NativeProps): NativeNode {
  const node = universal.createElement("button");
  universal.spread(node, props);
  return node;
}

/** Controlled, unstyled single-line native text input. */
export function Input(props: JSX.InputProps): NativeNode {
  const node = universal.createElement("input");
  universal.spread(node, props);
  return node;
}

/** Controlled, unstyled multiline native text area. */
export function TextArea(props: JSX.InputProps): NativeNode {
  const node = universal.createElement("textarea");
  universal.spread(node, props);
  return node;
}

/** Unstyled variable-height list; only visible child blocks are mounted by QuickGUI core. */
export function VirtualList(props: JSX.VirtualListProps): NativeNode {
  const node = universal.createElement("virtual-list");
  universal.spread(node, props);
  return node;
}

export interface ExtensionComponentProps extends Omit<JSX.NativeProps, "children" | "onComponentChange"> {
  package: string;
  component: string;
  properties: unknown;
  onEvent?: (kind: string, value: unknown, event: QuickGuiEvent) => void;
}

/** Mount any package-owned native component through the shared extension ABI. */
export function ExtensionComponent(props: ExtensionComponentProps): NativeNode {
  const node = universal.createElement("extension");
  universal.spread(node, universal.mergeProps(omit(props, "package", "component", "properties", "onEvent"), {
    get extensionPackage() { return props.package; },
    get extensionComponent() { return props.component; },
    get extensionProps() { return JSON.stringify(props.properties); },
    onComponentChange(event: QuickGuiEvent) {
      const payload = JSON.parse(event.value ?? "{}");
      props.onEvent?.(payload.kind, payload.value, event);
    },
  }) as object);
  return node;
}


/** Parsed-once retained SVG mask tinted by the inherited `color` style. */
export function Svg(props: JSX.SvgProps): NativeNode {
  const node = universal.createElement("svg");
  universal.spread(node, props);
  return node;
}


export type PointerPhase = "down" | "move" | "up" | "cancel";

export interface CapturedPointerEvent {
  phase: PointerPhase;
  position: { x: number; y: number };
  origin: { x: number; y: number };
  localPosition: { x: number; y: number };
  localOrigin: { x: number; y: number };
  delta: { x: number; y: number };
  button: "left" | "right" | "middle" | "back" | "forward" | "other";
}

/** Decode a Rust-core captured pointer payload. */
export function capturedPointerFromEvent(event: QuickGuiEvent): CapturedPointerEvent {
  if (!event.value) throw new TypeError("QuickGUI pointer event has no payload");
  return JSON.parse(event.value) as CapturedPointerEvent;
}

export type PopoverOpenChangeReason = "trigger-press" | "dismiss" | "hover";

/** The placement a closed popover reports until the core has placed it once. */
const unresolvedPlacement: AnchorPlacementDetails = {
  side: "bottom",
  align: "start",
  anchorHidden: false,
  anchorWidth: 0,
  anchorHeight: 0,
  availableWidth: 0,
  availableHeight: 0,
};

/** Decode one resolved placement payload. */
function placementFromDetails(details: ComponentChangeDetails): AnchorPlacementDetails | undefined {
  const placement = details.placement;
  if (!placement || typeof placement.side !== "string") return undefined;
  return {
    side: placement.side,
    align: placement.align ?? "start",
    anchorHidden: placement.anchorHidden === true,
    anchorWidth: placement.anchorWidth ?? 0,
    anchorHeight: placement.anchorHeight ?? 0,
    availableWidth: placement.availableWidth ?? 0,
    availableHeight: placement.availableHeight ?? 0,
  };
}

/** Positioning Base UI declares on `Popover.Positioner` and QuickGUI also accepts on the root. */
interface AnchorPositioning {
  side?: "top" | "bottom" | "left" | "right" | undefined;
  align?: "start" | "center" | "end" | undefined;
  sideOffset?: number | undefined;
  alignOffset?: number | undefined;
  collisionPadding?: number | undefined;
  sticky?: boolean | undefined;
  anchor?: NativeNode | { x: number; y: number } | undefined;
}

export interface PopoverOpenChangeDetails {
  reason: PopoverOpenChangeReason;
  event: QuickGuiEvent;
}

type PopoverSurface = "popover" | "system-popover";

interface PopoverContextValue {
  surface: PopoverSurface;
  scope: string;
  open: () => boolean;
  anchor: () => NativeNode | undefined;
  dismissOnEscape: () => boolean;
  dismissOnPointerOutside: () => boolean;
  registerTrigger: (node: NativeNode) => void;
  unregisterTrigger: (node: NativeNode) => void;
  toggleFromTrigger: (node: NativeNode, event: QuickGuiEvent) => void;
  dismiss: (event: QuickGuiEvent) => void;
  /** The core's resolved placement, republished every time it changes. */
  placement: () => AnchorPlacementDetails;
  reportPlacement: (next: AnchorPlacementDetails, event: QuickGuiEvent) => void;
  adoptOpen: (open: boolean, event: QuickGuiEvent) => void;
  /** Positioning declared on the root, overridden by whatever the positioner declares. */
  positioning: () => AnchorPositioning;
  declarePositioning: (positioning: AnchorPositioning) => void;
  modal: () => boolean | undefined;
  openOnHover: () => boolean | undefined;
  delay: () => number | undefined;
  closeDelay: () => number | undefined;
}

const PopoverContext = createContext<PopoverContextValue>();

function createPopoverRoot(surface: PopoverSurface, props: JSX.PopoverRootProps): NativeNode {
  const [uncontrolledOpen, setUncontrolledOpen] = createSignal(
    untrack(() => props.defaultOpen ?? false),
  );
  const [anchor, setAnchor] = createSignal<NativeNode | undefined>(undefined, {
    ownedWrite: true,
  });
  const [placement, setPlacement] = createSignal<AnchorPlacementDetails>(unresolvedPlacement);
  const [declared, setDeclared] = createSignal<AnchorPositioning>({}, { ownedWrite: true });
  const triggers = new Set<NativeNode>();
  const open = () => props.open ?? uncontrolledOpen();

  const changeOpen = (nextOpen: boolean, reason: PopoverOpenChangeReason, event: QuickGuiEvent) => {
    if (props.open === undefined) setUncontrolledOpen(nextOpen);
    props.onOpenChange?.(nextOpen, { reason, event });
  };

  const context: PopoverContextValue = {
    surface,
    scope: createComponentScope("qg-popover"),
    open,
    anchor,
    placement,
    reportPlacement(next, event) {
      setPlacement(next);
      props.onPlacementChange?.(next, event);
    },
    adoptOpen(nextOpen, event) {
      if (nextOpen === open()) return;
      changeOpen(nextOpen, "hover", event);
    },
    // The positioner's own declaration wins wherever it has one; the root supplies the rest.
    positioning: () => {
      const override = declared();
      return {
        side: override.side ?? props.side,
        align: override.align ?? props.align,
        sideOffset: override.sideOffset ?? props.sideOffset,
        alignOffset: override.alignOffset ?? props.alignOffset,
        collisionPadding: override.collisionPadding ?? props.collisionPadding,
        sticky: override.sticky ?? props.sticky,
        anchor: override.anchor ?? props.anchor,
      };
    },
    declarePositioning: setDeclared,
    modal: () => props.modal,
    openOnHover: () => props.openOnHover,
    delay: () => props.delay,
    closeDelay: () => props.closeDelay,
    dismissOnEscape: () => props.dismissOnEscape ?? true,
    dismissOnPointerOutside: () => props.dismissOnPointerOutside ?? true,
    registerTrigger(node) {
      triggers.add(node);
      if (!anchor()) setAnchor(node);
    },
    unregisterTrigger(node) {
      triggers.delete(node);
      if (anchor() === node) setAnchor(triggers.values().next().value);
    },
    toggleFromTrigger(node, event) {
      setAnchor(node);
      changeOpen(!open(), "trigger-press", event);
    },
    dismiss(event) {
      changeOpen(false, "dismiss", event);
    },
  };

  return PopoverContext({
    value: context,
    get children() {
      return props.children as SolidElement;
    },
  }) as unknown as NativeNode;
}

/** Logical root for an in-window popover. It does not create a native element. */
export function PopoverRoot(props: JSX.PopoverRootProps): NativeNode {
  return createPopoverRoot("popover", props);
}

/** Logical root for a native-window popover. It does not create a native window by itself. */
export function SystemPopoverRoot(props: JSX.PopoverRootProps): NativeNode {
  return createPopoverRoot("system-popover", props);
}

/**
 * Trigger button shared by in-window and system popover roots.
 *
 * For the in-window surface this is the one part the core keeps mounted whether the popover is
 * open or closed, so it carries the whole declaration — the controlled open value, the preferred
 * side and alignment, the bounded offsets, `modal`, and the hover deadlines — and the core reports
 * the placement it really resolved to back through it.
 */
export function PopoverTrigger(props: JSX.PopoverTriggerProps): NativeNode {
  const context = useContext(PopoverContext);
  let trigger: NativeNode | undefined;
  const inWindow = context.surface === "popover";
  const positioning = () => context.positioning();
  const part = inWindow
    ? {
        part: NativePart.PopoverTrigger,
        scope: context.scope,
        get open() {
          return context.open();
        },
        get modal() {
          return context.modal();
        },
        get openOnHover() {
          return props.openOnHover ?? context.openOnHover();
        },
        get delay() {
          return props.delay ?? context.delay();
        },
        get closeDelay() {
          return props.closeDelay ?? context.closeDelay();
        },
        get side() {
          return positioning().side;
        },
        get align() {
          return positioning().align;
        },
        get sideOffset() {
          return positioning().sideOffset;
        },
        get alignOffset() {
          return positioning().alignOffset;
        },
        get collisionPadding() {
          return positioning().collisionPadding;
        },
        get sticky() {
          return positioning().sticky;
        },
        get anchor() {
          return positioning().anchor;
        },
        onComponentChange: componentChangeReader((details, event) => {
          const placement = placementFromDetails(details);
          if (placement) context.reportPlacement(placement, event);
          if (typeof details.open === "boolean") context.adoptOpen(details.open, event);
        }),
      }
    : {};
  const forwarded = universal.mergeProps(omit(props, "openOnHover", "delay", "closeDelay"), part, {
    ref: [
      (node: NativeNode) => {
        trigger = node;
        context.registerTrigger(node);
      },
      props.ref,
    ].filter((value): value is (node: NativeNode) => void => typeof value === "function"),
    onClick(event: QuickGuiEvent) {
      props.onClick?.(event);
      if (!event.defaultPrevented && trigger) context.toggleFromTrigger(trigger, event);
    },
  }) as JSX.PopoverTriggerProps;
  const node = universal.createElement("button");
  universal.spread(node, forwarded);
  onCleanup(() => {
    if (trigger) context.unregisterTrigger(trigger);
  });
  return node;
}

/**
 * Read the placement the retained tree really resolved to inside a `Popover.Root` subtree.
 *
 * A declared `side`/`align` is only a preference: the core flips the side and re-aligns the cross
 * axis whenever the popup does not fit, and publishes the answer during the paint it was already
 * performing. Style from this the way Base UI styles from `data-side` and `data-align`.
 */
export function usePopoverPlacement(): () => AnchorPlacementDetails {
  return requirePopoverSurface("popover", "usePopoverPlacement").placement;
}

/** Application-owned popover positioner. Base UI declares the placement props here. */
export function PopoverPositioner(props: JSX.PopoverPositionerProps): NativeNode {
  const context = requirePopoverSurface("popover", "Popover.Positioner");
  // The positioner is unmounted while the popover is closed, so its declaration is routed to the
  // trigger, which the core keeps mounted either way.
  context.declarePositioning({
    get side() {
      return props.side;
    },
    get align() {
      return props.align;
    },
    get sideOffset() {
      return props.sideOffset;
    },
    get alignOffset() {
      return props.alignOffset;
    },
    get collisionPadding() {
      return props.collisionPadding;
    },
    get sticky() {
      return props.sticky;
    },
    get anchor() {
      return props.anchor;
    },
  } as AnchorPositioning);
  onCleanup(() => context.declarePositioning({}));
  return createPartNode(
    "view",
    omit(
      props,
      "side",
      "align",
      "sideOffset",
      "alignOffset",
      "collisionPadding",
      "sticky",
      "anchor",
    ),
    { part: NativePart.PopoverPositioner, scope: context.scope },
  );
}

/** Portal boundary. QuickGUI's retained overlay node is the portal, so it is the positioner. */
export function PopoverPortal(props: JSX.NativeProps): NativeNode {
  const context = requirePopoverSurface("popover", "Popover.Portal");
  return createPartNode("view", props, {
    part: NativePart.PopoverPortal,
    scope: context.scope,
  });
}

/** The popup itself, carrying the core's focus containment, restoration, and dismissal. */
export function PopoverPopup(props: JSX.NativeProps): NativeNode {
  const context = requirePopoverSurface("popover", "Popover.Popup");
  return createPartNode("view", omit(props, "onDismiss"), {
    part: NativePart.PopoverPopup,
    scope: context.scope,
    get dismissOnEscape() {
      return context.dismissOnEscape();
    },
    get dismissOnPointerOutside() {
      return context.dismissOnPointerOutside();
    },
    onDismiss(event: QuickGuiEvent) {
      (props as JSX.NativeProps).onDismiss?.(event);
      context.dismiss(event);
    },
  });
}

/** Arrow pinned to the popup edge that really faces the anchor. */
export function PopoverArrow(props: JSX.NativeProps): NativeNode {
  const context = requirePopoverSurface("popover", "Popover.Arrow");
  return createPartNode("view", props, {
    part: NativePart.PopoverArrow,
    scope: context.scope,
  });
}

/** Scrollable popup body. */
export function PopoverViewport(props: JSX.NativeProps): NativeNode {
  const context = requirePopoverSurface("popover", "Popover.Viewport");
  return createPartNode("view", props, {
    part: NativePart.PopoverViewport,
    scope: context.scope,
  });
}

/** Pointer-blocking backdrop behind a modal popup. */
export function PopoverBackdrop(props: JSX.NativeProps): NativeNode {
  const context = requirePopoverSurface("popover", "Popover.Backdrop");
  return createPartNode("view", props, {
    part: NativePart.PopoverBackdrop,
    scope: context.scope,
  });
}

/** Popup title, which names the popup for assistive technology. */
export function PopoverTitle(props: JSX.NativeProps): NativeNode {
  const context = requirePopoverSurface("popover", "Popover.Title");
  return createPartNode("view", props, {
    part: NativePart.PopoverTitle,
    scope: context.scope,
  });
}

/** Popup description, which describes the popup for assistive technology. */
export function PopoverDescription(props: JSX.NativeProps): NativeNode {
  const context = requirePopoverSurface("popover", "Popover.Description");
  return createPartNode("view", props, {
    part: NativePart.PopoverDescription,
    scope: context.scope,
  });
}

/** Close control. The core owns its role and accessible name. */
export function PopoverClose(props: JSX.NativeProps): NativeNode {
  const context = requirePopoverSurface("popover", "Popover.Close");
  return createPartNode("button", omit(props, "onClick"), {
    part: NativePart.PopoverClose,
    scope: context.scope,
    onClick: forwardClick(props.onClick, (event) => context.dismiss(event)),
  });
}

function requirePopoverSurface(expected: PopoverSurface, component: string): PopoverContextValue {
  const context = useContext(PopoverContext);
  if (context.surface !== expected) {
    const root = expected === "popover" ? "Popover.Root" : "SystemPopover.Root";
    throw new TypeError(`${component} must be used inside <${root}>`);
  }
  return context;
}

function createInWindowPopoverContent(
  props: JSX.PopoverContentProps,
  context: PopoverContextValue,
  anchor: NativeNode,
): NativeNode {
  const surface = universal.mergeProps(
    omit(props, "width", "height", "placement", "gap", "viewportMargin"),
    {
      get style() {
        return [props.style, { width: props.width, height: props.height }] as JSX.StyleProp;
      },
    },
  ) as JSX.NativeProps;
  const node = universal.createElement("view");
  const forwarded = universal.mergeProps(surface, {
    anchor,
    get anchorPlacement() {
      return props.placement ?? "bottom-start";
    },
    get anchorGap() {
      return props.gap ?? 6;
    },
    get viewportMargin() {
      return props.viewportMargin ?? 8;
    },
    get dismissOnEscape() {
      return context.dismissOnEscape();
    },
    get dismissOnPointerOutside() {
      return context.dismissOnPointerOutside();
    },
    onDismiss(event: QuickGuiEvent) {
      context.dismiss(event);
    },
  }) as object;
  universal.spread(node, forwarded);
  return node;
}

/** Popover content rendered in the current window's retained overlay plane. */
export function PopoverContent(props: JSX.PopoverContentProps): NativeNode {
  const context = requirePopoverSurface("popover", "Popover.Content");
  return Show({
    keyed: true,
    get when() {
      return context.open() ? context.anchor() : undefined;
    },
    children: (anchor) => createInWindowPopoverContent(props, context, anchor),
  }) as unknown as NativeNode;
}

function createSystemPopoverContent(
  props: JSX.PopoverContentProps,
  context: PopoverContextValue,
  anchor: NativeNode,
): NativeNode {
  const owner = getOwner();
  const placeholder = createNativeSentinel();
  const surface = universal.mergeProps(
    omit(props, "width", "height", "placement", "gap", "viewportMargin"),
    {
      get style() {
        return [props.style, { width: props.width, height: props.height }] as JSX.StyleProp;
      },
    },
  ) as JSX.NativeProps;
  let systemWindow: Window | undefined;
  let disposing = false;

  // Initial JSX is rendered before its owner Window has a native handle. The microtask also makes
  // later mounts use the same lifecycle path instead of special-casing initial render.
  queueMicrotask(() => {
    if (disposing) return;
    systemWindow = new Window({
      title: "QuickGUI System Popover",
      anchor,
      width: props.width,
      height: props.height,
      placement: props.placement ?? "bottom-start",
      gap: props.gap ?? 6,
      viewportMargin: props.viewportMargin ?? 8,
      dismissOnEscape: context.dismissOnEscape(),
      dismissOnPointerOutside: context.dismissOnPointerOutside(),
      renderer: (window) =>
        runWithOwner(owner, () =>
          createRenderer(() => {
            const node = universal.createElement("view");
            universal.spread(node, surface);
            return node;
          })(window),
        ),
    });
    systemWindow.onClose(() => {
      systemWindow = undefined;
      if (disposing) return;
      // Leave the native close/disposal stack before controlled state unmounts this portal.
      queueMicrotask(() => {
        if (disposing) return;
        try {
          context.dismiss(new QuickGuiEvent("dismiss", placeholder));
        } finally {
          flushSolid();
        }
      });
    });
  });

  onCleanup(() => {
    disposing = true;
    systemWindow?.close();
    systemWindow = undefined;
  });

  return placeholder;
}

/** Popover content rendered through a separate Solid renderer in a native child window. */
export function SystemPopoverContent(props: JSX.PopoverContentProps): NativeNode {
  const context = requirePopoverSurface("system-popover", "SystemPopover.Content");
  return Show({
    keyed: true,
    get when() {
      return context.open() ? context.anchor() : undefined;
    },
    children: (anchor) => createSystemPopoverContent(props, context, anchor),
  }) as unknown as NativeNode;
}

/** Base-UI-shaped compound parts for an in-window retained popover. */
export const Popover = Object.assign(PopoverRoot, {
  Root: PopoverRoot,
  Trigger: PopoverTrigger,
  Content: PopoverContent,
  Portal: PopoverPortal,
  Backdrop: PopoverBackdrop,
  Positioner: PopoverPositioner,
  Popup: PopoverPopup,
  Arrow: PopoverArrow,
  Viewport: PopoverViewport,
  Title: PopoverTitle,
  Description: PopoverDescription,
  Close: PopoverClose,
});

/** Compound popover parts whose content uses a native child window. */
export const SystemPopover = Object.assign(SystemPopoverRoot, {
  Root: SystemPopoverRoot,
  Trigger: PopoverTrigger,
  Content: SystemPopoverContent,
});

// ---------------------------------------------------------------------------
// Compound component parts
//
// Every part below is one ordinary native node that declares which Rust core part descriptor the
// binding must rebuild. The core owns identity, semantics, keyboard behavior, and whether an
// inactive panel is mounted at all; Solid owns only the controlled value, the compound context
// that saves applications from repeating it, and the unstyled element tree.
// ---------------------------------------------------------------------------

let nextComponentScope = 1;

/**
 * Allocate one bounded scope key shared by every part of a compound component instance.
 *
 * The Rust binding hashes it into the same `ElementId` the core component would have used, so
 * derived part identities and accessibility relationships resolve with no registry and no
 * synchronous question asked of JavaScript.
 */
function createComponentScope(prefix: string): string {
  return `${prefix}-${nextComponentScope++}`;
}

/**
 * Read one compound context without requiring a reactive owner.
 *
 * A part rendered outside a component tree — a direct call in a test, or a fragment built ahead of
 * its parent — has no owner at all, and a missing compound context is an ordinary answer rather
 * than a failure: the part simply mounts standalone.
 */
function optionalContext<T>(context: ReturnType<typeof createContext<T | null>>): T | null {
  return getOwner() ? useContext(context) : null;
}

function createHostNode(element: NativeElementName, props: unknown): NativeNode {
  const node = universal.createElement(element);
  universal.spread(node, props as object);
  return node;
}

function createPartNode(element: NativeElementName, props: unknown, part: unknown): NativeNode {
  const node = universal.createElement(element);
  universal.spread(node, universal.mergeProps(props as object, part as object) as object);
  return node;
}

function forwardClick(
  handler: ((event: QuickGuiEvent) => void) | undefined,
  activate: (event: QuickGuiEvent) => void,
): (event: QuickGuiEvent) => void {
  return (event) => {
    handler?.(event);
    if (!event.defaultPrevented) activate(event);
  };
}

export type CheckedState = boolean | "indeterminate";

/**
 * Controlled, unstyled checkbox root carrying the core's exact on/off/mixed toggle state.
 *
 * Inside a `CheckboxGroup.Root` the checkbox becomes a member of that group: `value` names the
 * declared value it toggles, `parent` makes it the group's derived parent checkbox, and the core
 * owns the checked state, the mixed parent state, and the click behavior for both.
 */
export function CheckboxRoot(props: JSX.CheckboxProps): NativeNode {
  const group = optionalContext(CheckboxGroupContext);
  if (group && (props.value !== undefined || props.parent === true)) {
    return createPartNode(
      "button",
      omit(props, "checked", "defaultChecked", "onCheckedChange", "value", "parent", "onClick"),
      {
        part: props.parent ? NativePart.CheckboxGroupParent : NativePart.CheckboxGroupItem,
        scope: group.scope,
        get partValue() {
          return props.parent ? undefined : props.value;
        },
      },
    );
  }
  const [uncontrolled, setUncontrolled] = createSignal<CheckedState>(
    untrack(() => props.defaultChecked ?? false),
  );
  const checked = () => props.checked ?? uncontrolled();
  return createPartNode(
    "button",
    omit(
      props,
      "checked",
      "defaultChecked",
      "onCheckedChange",
      "value",
      "parent",
      "childrenChecked",
    ),
    {
      part: NativePart.Checkbox,
      // A standalone parent checkbox declares its children's checked booleans and the core folds
      // them into on, mixed, or off; nothing derives the mixed state in JavaScript.
      get parent() {
        return props.parent === true ? true : undefined;
      },
      get values() {
        return props.parent === true && props.childrenChecked
          ? props.childrenChecked.slice()
          : undefined;
      },
      get checked() {
        return checked() === true;
      },
      get indeterminate() {
        return checked() === "indeterminate";
      },
      onClick: forwardClick(props.onClick, (event) => {
        const next = checked() !== true;
        if (props.checked === undefined) setUncontrolled(next);
        props.onCheckedChange?.(next, event);
      }),
    },
  );
}

/** Application-owned checkbox mark, hidden from the control's accessible name by the core. */
export function CheckboxIndicator(props: JSX.NativeProps): NativeNode {
  const group = optionalContext(CheckboxGroupContext);
  if (group) {
    return createPartNode("view", props, {
      part: NativePart.CheckboxGroupIndicator,
      scope: group.scope,
    });
  }
  return createPartNode("view", props, { part: NativePart.CheckboxIndicator });
}

/** Base-UI-shaped compound parts for a controlled checkbox. */
export const Checkbox = Object.assign(CheckboxRoot, {
  Root: CheckboxRoot,
  Indicator: CheckboxIndicator,
});

interface RadioGroupContextValue {
  value: () => string | undefined;
  select: (value: string, event: QuickGuiEvent) => void;
}

const RadioGroupContext = createContext<RadioGroupContextValue | null>(null);

/** Semantic radio-group root. The core supplies roving Tab and arrow behavior from the tree. */
export function RadioGroupRoot(props: JSX.RadioGroupProps): NativeNode {
  const [uncontrolled, setUncontrolled] = createSignal(untrack(() => props.defaultValue));
  const value = () => props.value ?? uncontrolled();
  const context: RadioGroupContextValue = {
    value,
    select(next, event) {
      if (props.value === undefined) setUncontrolled(next);
      props.onValueChange?.(next, event);
    },
  };
  return createPartNode("view", omit(props, "value", "defaultValue", "onValueChange", "children"), {
    part: NativePart.RadioGroup,
    get children() {
      return RadioGroupContext({
        value: context,
        get children() {
          return props.children as SolidElement;
        },
      });
    },
  });
}

/** Controlled radio root. Inside a `RadioGroup` its selection comes from the group value. */
export function RadioRoot(props: JSX.RadioProps): NativeNode {
  const group = useContext(RadioGroupContext);
  const [uncontrolled, setUncontrolled] = createSignal(
    untrack(() => props.defaultChecked ?? false),
  );
  const checked = () => (group ? group.value() === props.value : (props.checked ?? uncontrolled()));
  return createPartNode(
    "button",
    omit(props, "value", "checked", "defaultChecked", "onCheckedChange"),
    {
      part: NativePart.Radio,
      get checked() {
        return checked();
      },
      get partValue() {
        return props.value;
      },
      onClick: forwardClick(props.onClick, (event) => {
        if (group) {
          if (props.value !== undefined) group.select(props.value, event);
          return;
        }
        if (props.checked === undefined) setUncontrolled(true);
        props.onCheckedChange?.(true, event);
      }),
    },
  );
}

/** Application-owned radio dot, hidden from the control's accessible name by the core. */
export function RadioIndicator(props: JSX.NativeProps): NativeNode {
  return createPartNode("view", props, { part: NativePart.RadioIndicator });
}

/** Base-UI-shaped compound parts for a controlled radio button. */
export const Radio = Object.assign(RadioRoot, {
  Root: RadioRoot,
  Indicator: RadioIndicator,
});

/** Semantic group for related radio roots. */
export const RadioGroup = Object.assign(RadioGroupRoot, {
  Root: RadioGroupRoot,
});

/** Controlled, unstyled switch root/track carrying the core's switch role. */
export function SwitchRoot(props: JSX.SwitchProps): NativeNode {
  const [uncontrolled, setUncontrolled] = createSignal(
    untrack(() => props.defaultChecked ?? false),
  );
  const checked = () => props.checked ?? uncontrolled();
  return createPartNode("button", omit(props, "checked", "defaultChecked", "onCheckedChange"), {
    part: NativePart.Switch,
    get checked() {
      return checked();
    },
    onClick: forwardClick(props.onClick, (event) => {
      const next = !checked();
      if (props.checked === undefined) setUncontrolled(next);
      props.onCheckedChange?.(next, event);
    }),
  });
}

/** Application-owned switch thumb, hidden from the control's accessible name by the core. */
export function SwitchThumb(props: JSX.NativeProps): NativeNode {
  return createPartNode("view", props, { part: NativePart.SwitchThumb });
}

/** Base-UI-shaped compound parts for a controlled switch. */
export const Switch = Object.assign(SwitchRoot, {
  Root: SwitchRoot,
  Thumb: SwitchThumb,
});

interface TabsContextValue {
  scope: string;
  state: () => TabsState;
  value: () => string | undefined;
  orientation: () => "horizontal" | "vertical";
  activation: () => "manual" | "automatic";
  loop: () => boolean;
  keepMounted: () => boolean;
  select: (value: string, event: QuickGuiEvent) => void;
}

const TabsContext = createContext<TabsContextValue | null>(null);
const TabValueContext = createContext<(() => string) | null>(null);

function requireTabs(component: string): TabsContextValue {
  const context = useContext(TabsContext);
  if (!context) {
    throw new TypeError(`${component} must be used inside <Tabs.Root>`);
  }
  return context;
}

/** Shared declaration every tab part repeats so the Rust binding decodes it without a registry. */
function tabsPartProps(context: TabsContextValue): object {
  return {
    get scope() {
      return context.scope;
    },
    get activeValue() {
      return context.value();
    },
    get orientation() {
      return context.orientation();
    },
    get activateOnFocus() {
      return context.activation() === "automatic";
    },
    get loopFocus() {
      return context.loop();
    },
    get keepMounted() {
      return context.keepMounted();
    },
  };
}

/** Controlled, unstyled tab set. The core owns roving focus, arrow keys, and panel mounting. */
export function TabsRoot(props: JSX.TabsRootProps): NativeNode {
  const scope = createComponentScope("qg-tabs");
  const [uncontrolled, setUncontrolled] = createSignal(untrack(() => props.defaultValue));
  const [tabsState, setTabsState] = createSignal<TabsState>(settledTabs);
  const value = () => props.value ?? uncontrolled();
  const context: TabsContextValue = {
    scope,
    state: tabsState,
    value,
    orientation: () => props.orientation ?? "horizontal",
    activation: () => props.activation ?? "manual",
    loop: () => props.loop ?? true,
    keepMounted: () => props.keepMounted === true,
    select(next, event) {
      if (props.value === undefined) setUncontrolled(next);
      props.onValueChange?.(next, event);
    },
  };
  return createPartNode(
    "view",
    omit(
      props,
      "value",
      "defaultValue",
      "onValueChange",
      "onTabsStateChange",
      "orientation",
      "activation",
      "loop",
      "keepMounted",
      "children",
    ),
    universal.mergeProps(tabsPartProps(context), {
      part: NativePart.Tabs,
      onComponentChange: componentChangeReader((details, event) => {
        if (!details.activationDirection) return;
        const next: TabsState = {
          activationDirection: details.activationDirection as TabsActivationDirection,
          indicator: details.indicator ?? null,
        };
        setTabsState(next);
        props.onTabsStateChange?.(next, event);
      }),
      get children() {
        return TabsContext({
          value: context,
          get children() {
            return props.children as SolidElement;
          },
        });
      },
    }),
  );
}

/** Everything the core decided about one tab set. */
export interface TabsState {
  /**
   * The side the selection travelled toward, matching Base UI's `data-activation-direction`.
   *
   * The core records it from the tab positions the declaration gave it, so a transition can run
   * the right way without JavaScript comparing indices itself.
   */
  activationDirection: TabsActivationDirection;
  /** The active tab's laid-out box, published by the core during paint. */
  indicator: TabsIndicatorGeometry | null;
}

const settledTabs: TabsState = { activationDirection: "none", indicator: null };

/** Read the live tab-set state inside a `Tabs.Root` subtree. */
export function useTabsState(): () => TabsState {
  const context = optionalContext(TabsContext);
  return context ? context.state : () => settledTabs;
}

/** Tab-list root. The core attaches its exact arrow/Home/End navigation behavior here. */
export function TabsList(props: JSX.NativeProps): NativeNode {
  const context = requireTabs("Tabs.List");
  return createPartNode(
    "view",
    props,
    universal.mergeProps(tabsPartProps(context), { part: NativePart.TabsList }),
  );
}

/** One controlled tab. Activation, roles, and the panel relationship come from the core. */
export function TabsTab(props: JSX.TabsTabProps): NativeNode {
  const context = requireTabs("Tabs.Tab");
  const value = () => props.value;
  return createPartNode(
    "button",
    omit(props, "value", "index", "children"),
    universal.mergeProps(tabsPartProps(context), {
      part: NativePart.Tab,
      get partValue() {
        return props.value;
      },
      get itemIndex() {
        return props.index;
      },
      onClick: forwardClick(props.onClick, (event) => context.select(props.value, event)),
      get children() {
        return TabValueContext({
          value,
          get children() {
            return props.children as SolidElement;
          },
        });
      },
    }),
  );
}

/** Decorative indicator mounted by the core only while its tab is active. */
export function TabsIndicator(props: JSX.TabsIndicatorProps): NativeNode {
  const context = requireTabs("Tabs.Indicator");
  const inherited = useContext(TabValueContext);
  return createPartNode(
    "view",
    omit(props, "value", "placement"),
    universal.mergeProps(tabsPartProps(context), {
      part: NativePart.TabIndicator,
      get partValue() {
        return props.value ?? inherited?.() ?? context.value();
      },
      // A declared placement asks the core to keep the indicator anchored to the tab that is
      // really active and to publish that tab's laid-out box back through `useTabsState`.
      get anchorPlacement() {
        return props.placement;
      },
    }),
  );
}

/** One tab panel. The core omits it, or retains it hidden with `keepMounted`, when inactive. */
export function TabsPanel(props: JSX.TabsPanelProps): NativeNode {
  const context = requireTabs("Tabs.Panel");
  return createPartNode(
    "view",
    omit(props, "value"),
    universal.mergeProps(tabsPartProps(context), {
      part: NativePart.TabPanel,
      get partValue() {
        return props.value;
      },
    }),
  );
}

/** Base-UI-shaped compound parts for a controlled tab set. */
export const Tabs = Object.assign(TabsRoot, {
  Root: TabsRoot,
  List: TabsList,
  Tab: TabsTab,
  Indicator: TabsIndicator,
  Panel: TabsPanel,
});

interface CollapsibleContextValue {
  scope: string;
  open: () => boolean;
  disabled: () => boolean;
  keepMounted: () => boolean;
  toggle: (event: QuickGuiEvent) => void;
}

const CollapsibleContext = createContext<CollapsibleContextValue | null>(null);

function requireCollapsible(component: string): CollapsibleContextValue {
  const context = useContext(CollapsibleContext);
  if (!context) {
    throw new TypeError(`${component} must be used inside <Collapsible.Root>`);
  }
  return context;
}

function collapsiblePartProps(context: CollapsibleContextValue): object {
  return {
    get scope() {
      return context.scope;
    },
    get open() {
      return context.open();
    },
    get disabled() {
      return context.disabled();
    },
    get keepMounted() {
      return context.keepMounted();
    },
  };
}

/** Controlled, unstyled disclosure root. */
export function CollapsibleRoot(props: JSX.CollapsibleRootProps): NativeNode {
  const scope = createComponentScope("qg-collapsible");
  const [uncontrolled, setUncontrolled] = createSignal(untrack(() => props.defaultOpen ?? false));
  const open = () => props.open ?? uncontrolled();
  const context: CollapsibleContextValue = {
    scope,
    open,
    disabled: () => props.disabled === true,
    keepMounted: () => props.keepMounted === true,
    toggle(event) {
      const next = !open();
      if (props.open === undefined) setUncontrolled(next);
      props.onOpenChange?.(next, event);
    },
  };
  return createPartNode(
    "view",
    omit(props, "open", "defaultOpen", "onOpenChange", "keepMounted", "children"),
    universal.mergeProps(collapsiblePartProps(context), {
      part: NativePart.Collapsible,
      get children() {
        return CollapsibleContext({
          value: context,
          get children() {
            return props.children as SolidElement;
          },
        });
      },
    }),
  );
}

/** Disclosure button. Expanded state and the panel relationship come from the core. */
export function CollapsibleTrigger(props: JSX.NativeProps): NativeNode {
  const context = requireCollapsible("Collapsible.Trigger");
  return createPartNode(
    "button",
    props,
    universal.mergeProps(collapsiblePartProps(context), {
      part: NativePart.CollapsibleTrigger,
      onClick: forwardClick(props.onClick, (event) => context.toggle(event)),
    }),
  );
}

/** Disclosure panel. The core omits it, or retains it hidden with `keepMounted`, when closed. */
export function CollapsiblePanel(props: JSX.NativeProps): NativeNode {
  const context = requireCollapsible("Collapsible.Panel");
  return createPartNode(
    "view",
    props,
    universal.mergeProps(collapsiblePartProps(context), {
      part: NativePart.CollapsiblePanel,
    }),
  );
}

/** Base-UI-shaped compound parts for a controlled disclosure. */
export const Collapsible = Object.assign(CollapsibleRoot, {
  Root: CollapsibleRoot,
  Trigger: CollapsibleTrigger,
  Panel: CollapsiblePanel,
});

interface AccordionContextValue {
  scope: string;
  isOpen: (value: string) => boolean;
  toggle: (value: string, event: QuickGuiEvent) => void;
  disabled: () => boolean;
  keepMounted: () => boolean;
  headingLevel: () => number;
}

interface AccordionItemContextValue {
  value: () => string;
  index: () => number;
  open: () => boolean;
  disabled: () => boolean;
}

const AccordionContext = createContext<AccordionContextValue | null>(null);
const AccordionItemContext = createContext<AccordionItemContextValue | null>(null);

function requireAccordion(component: string): AccordionContextValue {
  const context = useContext(AccordionContext);
  if (!context) {
    throw new TypeError(`${component} must be used inside <Accordion.Root>`);
  }
  return context;
}

function requireAccordionItem(component: string): AccordionItemContextValue {
  const context = useContext(AccordionItemContext);
  if (!context) {
    throw new TypeError(`${component} must be used inside <Accordion.Item>`);
  }
  return context;
}

function accordionItemPartProps(
  accordion: AccordionContextValue,
  item: AccordionItemContextValue,
): object {
  return {
    get scope() {
      return accordion.scope;
    },
    get partValue() {
      return item.value();
    },
    get itemIndex() {
      return item.index();
    },
    get open() {
      return item.open();
    },
    get disabled() {
      return item.disabled();
    },
    get keepMounted() {
      return accordion.keepMounted();
    },
    get headingLevel() {
      return accordion.headingLevel();
    },
  };
}

function accordionOpenValues(value: string | readonly string[] | null | undefined): string[] {
  if (value === null || value === undefined) return [];
  return typeof value === "string" ? [value] : [...value];
}

/** Controlled, unstyled accordion root supporting single or multiple open items. */
export function AccordionRoot(props: JSX.AccordionRootProps): NativeNode {
  const scope = createComponentScope("qg-accordion");
  const [uncontrolled, setUncontrolled] = createSignal<string[]>(
    untrack(() => accordionOpenValues(props.defaultValue)),
  );
  const open = () =>
    props.value === undefined ? uncontrolled() : accordionOpenValues(props.value);
  const context: AccordionContextValue = {
    scope,
    isOpen: (value) => open().includes(value),
    disabled: () => props.disabled === true,
    keepMounted: () => props.keepMounted === true,
    headingLevel: () => props.headingLevel ?? 3,
    toggle(value, event) {
      const current = open();
      const next = current.includes(value)
        ? current.filter((candidate) => candidate !== value)
        : props.multiple
          ? [...current, value]
          : [value];
      if (props.value === undefined) setUncontrolled(next);
      props.onValueChange?.(props.multiple ? next : (next[0] ?? null), event);
    },
  };
  return createPartNode(
    "view",
    omit(
      props,
      "value",
      "defaultValue",
      "onValueChange",
      "multiple",
      "keepMounted",
      "headingLevel",
      "children",
    ),
    {
      part: NativePart.Accordion,
      scope,
      get children() {
        return AccordionContext({
          value: context,
          get children() {
            return props.children as SolidElement;
          },
        });
      },
    },
  );
}

/** One accordion item. Its trigger, header, and panel identities derive from this value. */
export function AccordionItem(props: JSX.AccordionItemProps): NativeNode {
  const accordion = requireAccordion("Accordion.Item");
  const item: AccordionItemContextValue = {
    value: () => props.value,
    index: () => props.index ?? 0,
    open: () => accordion.isOpen(props.value),
    disabled: () => props.disabled === true || accordion.disabled(),
  };
  return createPartNode(
    "view",
    omit(props, "value", "index", "children"),
    universal.mergeProps(accordionItemPartProps(accordion, item), {
      part: NativePart.AccordionItem,
      get children() {
        return AccordionItemContext({
          value: item,
          get children() {
            return props.children as SolidElement;
          },
        });
      },
    }),
  );
}

/** Accordion heading that contains only this item's trigger. */
export function AccordionHeader(props: JSX.NativeProps): NativeNode {
  const accordion = requireAccordion("Accordion.Header");
  const item = requireAccordionItem("Accordion.Header");
  return createPartNode(
    "view",
    props,
    universal.mergeProps(accordionItemPartProps(accordion, item), {
      part: NativePart.AccordionHeader,
    }),
  );
}

/** Accordion disclosure button for one item. */
export function AccordionTrigger(props: JSX.NativeProps): NativeNode {
  const accordion = requireAccordion("Accordion.Trigger");
  const item = requireAccordionItem("Accordion.Trigger");
  return createPartNode(
    "button",
    props,
    universal.mergeProps(accordionItemPartProps(accordion, item), {
      part: NativePart.AccordionTrigger,
      onClick: forwardClick(props.onClick, (event) => accordion.toggle(item.value(), event)),
    }),
  );
}

/** Accordion panel mounted as a named region by the core while its item is open. */
export function AccordionPanel(props: JSX.NativeProps): NativeNode {
  const accordion = requireAccordion("Accordion.Panel");
  const item = requireAccordionItem("Accordion.Panel");
  return createPartNode(
    "view",
    props,
    universal.mergeProps(accordionItemPartProps(accordion, item), {
      part: NativePart.AccordionPanel,
    }),
  );
}

/** Base-UI-shaped compound parts for a controlled accordion. */
export const Accordion = Object.assign(AccordionRoot, {
  Root: AccordionRoot,
  Item: AccordionItem,
  Header: AccordionHeader,
  Trigger: AccordionTrigger,
  Panel: AccordionPanel,
});

interface FieldContextValue {
  scope: string;
  disabled: () => boolean;
  invalid: () => boolean;
  required: () => boolean;
  touched: () => boolean;
  dirty: () => boolean;
  filled: () => boolean;
  validationMessage: () => string | undefined;
}

interface FieldsetContextValue {
  disabled: () => boolean;
}

const FieldContext = createContext<FieldContextValue | null>(null);
const FieldsetContext = createContext<FieldsetContextValue | null>(null);

function requireField(component: string): FieldContextValue {
  const context = useContext(FieldContext);
  if (!context) {
    throw new TypeError(`${component} must be used inside <Field.Root>`);
  }
  return context;
}

function fieldPartProps(context: FieldContextValue): object {
  return {
    get scope() {
      return context.scope;
    },
    get disabled() {
      return context.disabled();
    },
    get invalid() {
      return context.invalid();
    },
    get required() {
      return context.required();
    },
    get touched() {
      return context.touched();
    },
    get dirty() {
      return context.dirty();
    },
    get filled() {
      return context.filled();
    },
    get validationMessage() {
      return context.validationMessage();
    },
  };
}

/** Controlled, unstyled labelling and validation composition for one form control. */
export function FieldRoot(props: JSX.FieldRootProps): NativeNode {
  const scope = createComponentScope("qg-field");
  const fieldset = useContext(FieldsetContext);
  const context: FieldContextValue = {
    scope,
    disabled: () => props.disabled === true || fieldset?.disabled() === true,
    invalid: () => props.invalid === true,
    required: () => props.required === true,
    touched: () => props.touched === true,
    dirty: () => props.dirty === true,
    filled: () => props.filled === true,
    validationMessage: () => props.validationMessage,
  };
  return createPartNode(
    "view",
    omit(props, "children"),
    universal.mergeProps(fieldPartProps(context), {
      part: NativePart.Field,
      get children() {
        return FieldContext({
          value: context,
          get children() {
            return props.children as SolidElement;
          },
        });
      },
    }),
  );
}

/** Visible label. The core forwards its clicks to the control unless `passive` is declared. */
export function FieldLabel(props: JSX.FieldLabelProps): NativeNode {
  const context = requireField("Field.Label");
  return createPartNode(
    "view",
    omit(props, "passive"),
    universal.mergeProps(fieldPartProps(context), {
      get part() {
        return props.passive ? NativePart.FieldPassiveLabel : NativePart.FieldLabel;
      },
    }),
  );
}

/**
 * The labelled control itself.
 *
 * The core part sets this element's identity, so the control must be the part rather than a
 * wrapper around one. `element` selects which native element the control renders.
 */
export function FieldControl(props: JSX.FieldControlProps): NativeNode {
  const context = requireField("Field.Control");
  return createPartNode(
    props.element ?? "input",
    omit(props, "element"),
    universal.mergeProps(fieldPartProps(context), {
      part: NativePart.FieldControl,
    }),
  );
}

/** Supplementary help described to assistive technology by the core. */
export function FieldDescription(props: JSX.NativeProps): NativeNode {
  const context = requireField("Field.Description");
  return createPartNode(
    "view",
    props,
    universal.mergeProps(fieldPartProps(context), {
      part: NativePart.FieldDescription,
    }),
  );
}

/**
 * One field item, Base UI's structural row inside a field.
 *
 * The core gives it an identity and propagates the field's disabled state; the layout stays
 * entirely application-owned.
 */
export function FieldItem(props: JSX.NativeProps): NativeNode {
  const context = requireField("Field.Item");
  return createPartNode(
    "view",
    props,
    universal.mergeProps(fieldPartProps(context), {
      part: NativePart.FieldItem,
    }),
  );
}

/**
 * Live validity readout.
 *
 * The core owns which triggers the declared `validationMode` answers and how long it waits before
 * each one; `useFieldValidation()` reports exactly those answers.
 */
export function FieldValidity(props: JSX.FieldValidityProps): NativeNode {
  const context = requireField("Field.Validity");
  return createPartNode(
    "view",
    omit(props, "visible"),
    universal.mergeProps(fieldPartProps(context), {
      part: NativePart.FieldValidity,
      get open() {
        return props.visible ?? true;
      },
    }),
  );
}

/** Visible error. The core removes it from layout while the controlled field is valid. */
export function FieldError(props: JSX.NativeProps): NativeNode {
  const context = requireField("Field.Error");
  return createPartNode(
    "view",
    props,
    universal.mergeProps(fieldPartProps(context), {
      part: NativePart.FieldError,
    }),
  );
}

/** Base-UI-shaped compound parts for one labelled, validated control. */
export const Field = Object.assign(FieldRoot, {
  Root: FieldRoot,
  Item: FieldItem,
  Label: FieldLabel,
  Control: FieldControl,
  Validity: FieldValidity,
  Description: FieldDescription,
  Error: FieldError,
});

/** Controlled group semantics for related fields. */
export function FieldsetRoot(props: JSX.FieldsetRootProps): NativeNode {
  const scope = createComponentScope("qg-fieldset");
  const context: FieldsetContextValue = {
    disabled: () => props.disabled === true,
  };
  return createPartNode("view", omit(props, "children"), {
    part: NativePart.Fieldset,
    scope,
    get disabled() {
      return context.disabled();
    },
    get children() {
      return FieldsetContext({
        value: context,
        get children() {
          return props.children as SolidElement;
        },
      });
    },
  });
}

/** Group legend named to assistive technology by the core. */
export function FieldsetLegend(props: JSX.NativeProps): NativeNode {
  return createPartNode("view", props, { part: NativePart.FieldsetLegend });
}

/** Group description named to assistive technology by the core. */
export function FieldsetDescription(props: JSX.NativeProps): NativeNode {
  return createPartNode("view", props, {
    part: NativePart.FieldsetDescription,
  });
}

/** Direct group control that inherits the fieldset's disabled state. */
export function FieldsetControl(props: JSX.FieldControlProps): NativeNode {
  const context = useContext(FieldsetContext);
  return createPartNode(props.element ?? "input", omit(props, "element"), {
    part: NativePart.FieldsetControl,
    get disabled() {
      return props.disabled === true || context?.disabled() === true;
    },
  });
}

export type DialogOpenChangeReason = "trigger-press" | "close-press" | "dismiss";

export interface DialogOpenChangeDetails {
  reason: DialogOpenChangeReason;
  event: QuickGuiEvent;
}

interface DialogContextValue {
  scope: string;
  variant: "dialog" | "alertdialog";
  open: () => boolean;
  dismissOnEscape: () => boolean;
  dismissOnBackdrop: () => boolean;
  enterDuration: () => number | undefined;
  exitDuration: () => number | undefined;
  change: (open: boolean, reason: DialogOpenChangeReason, event: QuickGuiEvent) => void;
  complete: (open: boolean, event: QuickGuiEvent) => void;
}

const DialogContext = createContext<DialogContextValue | null>(null);

function requireDialog(component: string): DialogContextValue {
  const context = useContext(DialogContext);
  if (!context) {
    throw new TypeError(`${component} must be used inside <Dialog.Root> or <AlertDialog.Root>`);
  }
  return context;
}

function dialogPartProps(context: DialogContextValue): object {
  return {
    get scope() {
      return context.scope;
    },
    get variant() {
      return context.variant;
    },
    get open() {
      return context.open();
    },
  };
}

function createDialogRoot(
  variant: "dialog" | "alertdialog",
  props: JSX.DialogRootProps,
): NativeNode {
  const scope = createComponentScope(variant === "alertdialog" ? "qg-alert-dialog" : "qg-dialog");
  const [uncontrolled, setUncontrolled] = createSignal(untrack(() => props.defaultOpen ?? false));
  const open = () => props.open ?? uncontrolled();
  const context: DialogContextValue = {
    scope,
    variant,
    open,
    dismissOnEscape: () => props.dismissOnEscape ?? true,
    dismissOnBackdrop: () => props.dismissOnBackdrop ?? variant !== "alertdialog",
    enterDuration: () => props.enterDuration,
    exitDuration: () => props.exitDuration,
    change(next, reason, event) {
      if (props.open === undefined) setUncontrolled(next);
      props.onOpenChange?.(next, { reason, event });
    },
    complete(next, event) {
      // Base UI's `onOpenChangeComplete`: the core held the surface mounted for exactly the
      // declared exit transition and is telling JavaScript it has finished.
      props.onOpenChangeComplete?.(next, event);
    },
  };
  return DialogContext({
    value: context,
    get children() {
      return props.children as SolidElement;
    },
  }) as unknown as NativeNode;
}

/** Logical root for a controlled in-window modal dialog. It creates no native element. */
export function DialogRoot(props: JSX.DialogRootProps): NativeNode {
  return createDialogRoot("dialog", props);
}

/** Logical root for a consequential alert dialog whose backdrop does not dismiss by default. */
export function AlertDialogRoot(props: JSX.DialogRootProps): NativeNode {
  return createDialogRoot("alertdialog", props);
}

/** Trigger button carrying the core's dialog popover and expanded accessibility state. */
export function DialogTrigger(props: JSX.NativeProps): NativeNode {
  const context = requireDialog("Dialog.Trigger");
  return createPartNode(
    "button",
    props,
    universal.mergeProps(dialogPartProps(context), {
      part: NativePart.DialogTrigger,
      onClick: forwardClick(props.onClick, (event) => context.change(true, "trigger-press", event)),
    }),
  );
}

/**
 * Viewport portal, focus trap, and focus-restoration boundary for the dialog.
 *
 * The Rust core mounts it only while the dialog is open, so a closed dialog contributes no
 * overlay, layout, paint, input, or accessibility node.
 */
export function DialogPortal(props: JSX.NativeProps): NativeNode {
  const context = requireDialog("Dialog.Portal");
  return createPartNode(
    "view",
    props,
    universal.mergeProps(dialogPartProps(context), {
      part: NativePart.Dialog,
      get enterDuration() {
        return context.enterDuration();
      },
      get exitDuration() {
        return context.exitDuration();
      },
      onComponentChange: componentChangeReader((details, event) => {
        if (typeof details.openChangeComplete === "boolean")
          context.complete(details.openChangeComplete, event);
      }),
    }),
  );
}

/**
 * Scrollable dialog body.
 *
 * The core owns the overflow, so a long dialog scrolls inside the popup rather than growing past
 * the window.
 */
export function DialogViewport(props: JSX.NativeProps): NativeNode {
  const context = requireDialog("Dialog.Viewport");
  return createPartNode(
    "view",
    props,
    universal.mergeProps(dialogPartProps(context), {
      part: NativePart.DialogViewport,
    }),
  );
}

/** Application-owned backdrop filling the portal. */
export function DialogBackdrop(props: JSX.NativeProps): NativeNode {
  const context = requireDialog("Dialog.Backdrop");
  return createPartNode(
    "view",
    props,
    universal.mergeProps(dialogPartProps(context), {
      part: NativePart.DialogBackdrop,
    }),
  );
}

/** Modal surface. Escape and backdrop dismissal are decided ahead of time by the core. */
export function DialogPopup(props: JSX.NativeProps): NativeNode {
  const context = requireDialog("Dialog.Popup");
  return createPartNode(
    "view",
    props,
    universal.mergeProps(dialogPartProps(context), {
      part: NativePart.DialogPopup,
      get dismissOnEscape() {
        return context.dismissOnEscape();
      },
      get dismissOnPointerOutside() {
        return context.dismissOnBackdrop();
      },
      onDismiss(event: QuickGuiEvent) {
        (props as JSX.NativeProps).onDismiss?.(event);
        if (!event.defaultPrevented) context.change(false, "dismiss", event);
      },
    }),
  );
}

/** Visible dialog title used as the popup's accessible name. */
export function DialogTitle(props: JSX.NativeProps): NativeNode {
  const context = requireDialog("Dialog.Title");
  return createPartNode(
    "view",
    props,
    universal.mergeProps(dialogPartProps(context), {
      part: NativePart.DialogTitle,
    }),
  );
}

/** Visible dialog description used as the popup's accessible description. */
export function DialogDescription(props: JSX.NativeProps): NativeNode {
  const context = requireDialog("Dialog.Description");
  return createPartNode(
    "view",
    props,
    universal.mergeProps(dialogPartProps(context), {
      part: NativePart.DialogDescription,
    }),
  );
}

/** Close control. Its accessible name comes from `aria-label`, defaulting to `Close`. */
export function DialogClose(props: JSX.NativeProps): NativeNode {
  const context = requireDialog("Dialog.Close");
  return createPartNode(
    "button",
    props,
    universal.mergeProps(dialogPartProps(context), {
      part: NativePart.DialogClose,
      onClick: forwardClick(props.onClick, (event) => context.change(false, "close-press", event)),
    }),
  );
}

/** Base-UI-shaped compound parts for a controlled in-window modal dialog. */
export const Dialog = Object.assign(DialogRoot, {
  Viewport: DialogViewport,
  Root: DialogRoot,
  Trigger: DialogTrigger,
  Portal: DialogPortal,
  Backdrop: DialogBackdrop,
  Popup: DialogPopup,
  Title: DialogTitle,
  Description: DialogDescription,
  Close: DialogClose,
});

/** Compound parts for a consequential alert dialog. */
export const AlertDialog = Object.assign(AlertDialogRoot, {
  Viewport: DialogViewport,
  Root: AlertDialogRoot,
  Trigger: DialogTrigger,
  Portal: DialogPortal,
  Backdrop: DialogBackdrop,
  Popup: DialogPopup,
  Title: DialogTitle,
  Description: DialogDescription,
  Close: DialogClose,
});

/** Base-UI-shaped compound parts for a semantic field group. */
export const Fieldset = Object.assign(FieldsetRoot, {
  Root: FieldsetRoot,
  Legend: FieldsetLegend,
  Description: FieldsetDescription,
  Control: FieldsetControl,
});

/** Retained image node. A path decodes on the core's bounded worker pool. */
export function Image(props: JSX.ImageProps): NativeNode {
  return createHostNode("image", props);
}

/** Retained application shader surface painted by validated WGSL. */
export function Shader(props: JSX.ShaderProps): NativeNode {
  return createHostNode("shader", props);
}

// ---------------------------------------------------------------------------
// Range and feedback parts
// ---------------------------------------------------------------------------

/** Determinate or indeterminate progress root carrying the core's exact value range. */
/** Everything the core decided about one progress bar or meter. */
export interface GaugeState {
  status: ProgressStatus;
  /** The value formatted through the declared `format`, or `null` without one. */
  displayValue: string | null;
  /** How far the value has travelled, from 0 to 1, or `null` while indeterminate. */
  completion: number | null;
}

const settledGauge: GaugeState = {
  status: "indeterminate",
  displayValue: null,
  completion: null,
};

interface GaugeContextValue {
  state: () => GaugeState;
}

const GaugeContext = createContext<GaugeContextValue | null>(null);

/**
 * Read the live status inside a `Progress.Root` or `Meter.Root` subtree.
 *
 * `status` is the core's own derived value — Base UI's `data-progressing`, `data-complete`, and
 * `data-indeterminate` — and `displayValue` is what the declared `format` produced.
 */
export function useGaugeState(): () => GaugeState {
  const context = optionalContext(GaugeContext);
  return context ? context.state : () => settledGauge;
}

function createGaugeRoot(
  part: NativePartName,
  props: JSX.ProgressProps | JSX.MeterProps,
  declaration: object,
): NativeNode {
  const [state, setState] = createSignal<GaugeState>(settledGauge);
  const context: GaugeContextValue = { state };
  return createPartNode(
    "view",
    omit(props as JSX.ProgressProps, "onStatusChange", "children"),
    universal.mergeProps(declaration, {
      part,
      onComponentChange: componentChangeReader((details, event) => {
        if (!details.status) return;
        const next: GaugeState = {
          status: details.status,
          displayValue: details.displayValue ?? null,
          completion: details.completion ?? null,
        };
        setState(next);
        (props as JSX.ProgressProps).onStatusChange?.(next, event);
      }),
      get children() {
        return GaugeContext({
          value: context,
          get children() {
            return (props as JSX.ProgressProps).children as SolidElement;
          },
        });
      },
    }),
  );
}

/** Progress track. The core derives its identity from the compound scope. */
export function ProgressTrack(props: JSX.NativeScopedProps): NativeNode {
  return createPartNode("view", props, { part: NativePart.ProgressTrack });
}

/** Progress label. The core points the root's accessible name at it. */
export function ProgressLabel(props: JSX.NativeScopedProps): NativeNode {
  return createPartNode("view", props, { part: NativePart.ProgressLabel });
}

/** Progress value readout, described by the core through the root. */
export function ProgressValue(props: JSX.NativeScopedProps): NativeNode {
  return createPartNode("view", props, { part: NativePart.ProgressValue });
}

/** Meter track. */
export function MeterTrack(props: JSX.NativeScopedProps): NativeNode {
  return createPartNode("view", props, { part: NativePart.MeterTrack });
}

/** Meter label. */
export function MeterLabel(props: JSX.NativeScopedProps): NativeNode {
  return createPartNode("view", props, { part: NativePart.MeterLabel });
}

/** Meter value readout. */
export function MeterValue(props: JSX.NativeScopedProps): NativeNode {
  return createPartNode("view", props, { part: NativePart.MeterValue });
}

export function ProgressRoot(props: JSX.ProgressProps): NativeNode {
  return createGaugeRoot(NativePart.Progress, props, {});
}

/** Application-owned progress fill, hidden from the accessible name by the core. */
export function ProgressIndicator(props: JSX.NativeScopedProps): NativeNode {
  return createPartNode("view", props, { part: NativePart.ProgressIndicator });
}

/** Base-UI-shaped compound parts for a progress indicator. */
export const Progress = Object.assign(ProgressRoot, {
  Root: ProgressRoot,
  Track: ProgressTrack,
  Indicator: ProgressIndicator,
  Label: ProgressLabel,
  Value: ProgressValue,
});

/** Static measurement gauge with optional low, high, and optimum markers. */
export function MeterRoot(props: JSX.MeterProps): NativeNode {
  return createGaugeRoot(NativePart.Meter, props, {});
}

/** Application-owned meter fill, hidden from the accessible name by the core. */
export function MeterIndicator(props: JSX.NativeScopedProps): NativeNode {
  return createPartNode("view", props, { part: NativePart.MeterIndicator });
}

/** Base-UI-shaped compound parts for a meter. */
export const Meter = Object.assign(MeterRoot, {
  Root: MeterRoot,
  Track: MeterTrack,
  Indicator: MeterIndicator,
  Label: MeterLabel,
  Value: MeterValue,
});

/** Controlled toggle button. A toggle is a button that stays pressed, not a checkbox. */
export function ToggleRoot(props: JSX.ToggleProps): NativeNode {
  const [uncontrolled, setUncontrolled] = createSignal(
    untrack(() => props.defaultPressed ?? false),
  );
  const pressed = () => props.pressed ?? uncontrolled();
  return createPartNode("button", omit(props, "pressed", "defaultPressed", "onPressedChange"), {
    part: NativePart.Toggle,
    get pressed() {
      return pressed();
    },
    onClick: forwardClick(props.onClick, (event) => {
      const next = !pressed();
      if (props.pressed === undefined) setUncontrolled(next);
      props.onPressedChange?.(next, event);
    }),
  });
}

/** Application-owned toggle indicator, hidden from the accessible name by the core. */
export function ToggleIndicator(props: JSX.NativeProps): NativeNode {
  return createPartNode("view", props, { part: NativePart.ToggleIndicator });
}

/** Base-UI-shaped compound parts for a toggle button. */
export const Toggle = Object.assign(ToggleRoot, {
  Root: ToggleRoot,
  Indicator: ToggleIndicator,
});

// ---------------------------------------------------------------------------
// Declared range, ordering, and roving-focus components
//
// Every value below is declared ahead of the core's decision. The Rust core owns clamping, step
// snapping, thumb ordering, splitter size conservation, wrapping arrow navigation, disabled-item
// skipping, and the single roving Tab stop; JavaScript declares the state and receives whatever
// the core decided as one asynchronous `componentchange` payload.
// ---------------------------------------------------------------------------

/** The payload of a native `componentchange` event. */
export interface ComponentChangeDetails {
  /** Slider thumb values, in ascending thumb order. */
  values?: readonly number[];
  /** Splitter pane sizes in logical pixels, conserved across the whole splitter. */
  sizes?: readonly number[];
  /** The toolbar item that now owns the single roving Tab stop. */
  active?: string | null;
  /** The pressed toggle-group values, in declared item order. */
  pressed?: readonly string[];
  /** Selected option value, selected tree node, number-field value, or civil value. */
  value?: string | number | null;
  /** Free-form editing text a picker retains. */
  inputValue?: string;
  /** Whether a picker's own native popover is open, or which menubar menu is. */
  open?: boolean | number | null;
  /** The number field's exact editing text. */
  text?: string;
  /** Whether the number field's editing text parses into range. */
  valid?: boolean;
  /** The range of rows a collection is virtualizing. */
  visibleRange?: VisibleRange;
  /** Selected table rows as inclusive `[start, end]` ranges. */
  selectedRanges?: readonly (readonly number[])[];
  /** The table's sort state, or `null` once it is cleared. */
  sort?: TableSortState | null;
  /** The table's active cell, or `null` once it is cleared. */
  activeCell?: TableCell | null;
  /** Retained widths of the resizable columns, keyed by declared identifier. */
  columnWidths?: Readonly<Record<string, number>>;
  /** Declared column identifiers in their current display order. */
  columnOrder?: readonly string[];
  /** The inline edit the core just ended, with its own commit decision. */
  editEnded?: TableEditEndDetails;
  /** Expanded tree node identifiers. */
  expanded?: readonly string[];
  /** The pending tree branch asking for its children. */
  loadChildren?: string;
  /** Toast identifiers the core's queue dismissed. */
  dismissed?: readonly string[];
  /** The calendar day, or menubar menu index, that now owns the single Tab stop. */
  focused?: string | number;
  /** The month a calendar is displaying, as `YYYY-MM`. */
  month?: string;
  /** Checked values of a checkbox group, in the declared order. */
  checkedValues?: readonly string[];
  /** The avatar load status the core retained, matching Base UI's own values. */
  loadingStatus?: AvatarLoadingStatus;
  /** The OTP code that just became complete. */
  complete?: string;
  /** A scroll area's clamped offset. */
  offset?: { x: number; y: number };
  /** Whether a pointer or wheel gesture is currently moving a scroll area's viewport. */
  scrolling?: boolean;
  /** Whether the pointer is inside a scroll area's root. */
  hovering?: boolean;
  hasOverflowX?: boolean;
  hasOverflowY?: boolean;
  overflowXStart?: boolean;
  overflowXEnd?: boolean;
  overflowYStart?: boolean;
  overflowYEnd?: boolean;
  /** The drawer snap point the core moved to. */
  snapPoint?: number;
  /** Whether a drawer swipe is in flight. */
  swiping?: boolean;
  /** The drawer's live dismissing displacement, in logical pixels. */
  swipeOffset?: number;
  /** The direction the user's attention travelled between navigation panels. */
  activationDirection?: NavigationMenuActivationDirection | TabsActivationDirection;
  /** Where the retained tree really placed an anchored surface. */
  placement?: AnchorPlacementDetails;
  /** Whether a slider thumb is being dragged right now. */
  dragging?: boolean;
  /** Whether the core's own commit boundary fired on this frame. */
  committed?: boolean;
  /** The value formatted through the declared `format`. */
  displayValue?: string | null;
  /** Whether a number field's scrub gesture is in flight. */
  scrubbing?: boolean;
  /** Whether the control refuses changes while staying focusable. */
  readOnly?: boolean;
  /** Whether the control is required for form submission. */
  required?: boolean;
  /** The queue a toast viewport is showing, newest first. */
  toasts?: readonly ToastStackEntry[];
  /** The active tab's laid-out box, in logical window coordinates. */
  indicator?: TabsIndicatorGeometry | null;
  /** A progress bar's derived status. */
  status?: ProgressStatus;
  /** How far a progress bar or meter has travelled, from 0 to 1. */
  completion?: number | null;
  /** Which triggers the core's declared validation mode answers. */
  validation?: FieldValidationTriggers;
  /** How long the core waits before validating on each trigger, in milliseconds. */
  validationDelay?: FieldValidationDelays;
  /** Base UI's `onOpenChangeComplete`: the transition the core just finished. */
  openChangeComplete?: boolean;
  /** The side a menu popup was really placed on. */
  side?: "top" | "bottom" | "left" | "right";
  /** The cross-axis alignment a menu popup really used. */
  align?: "start" | "center" | "end";
  /** Whether a menu's anchor left the collision viewport entirely. */
  anchorHidden?: boolean;
  /** Whether the roving highlight is on this menu row. */
  highlighted?: boolean;
  /** Whether a menu row refuses activation. */
  disabled?: boolean;
  /** A checkable menu row's state, or `null` for a row that is not checkable. */
  checked?: boolean | null;
  /** The stable identifier of the menu row the core just activated. */
  activated?: string;
  /** The destination a `Menu.LinkItem` carried into the core's own open-URL path. */
  href?: string;
  /** Every value a multiple select holds, in source order. */
  selectedValues?: readonly string[];
  /** Every value a multiple combobox holds as a chip, in chip order. */
  chipValues?: readonly string[];
  /** The label of each chip a multiple combobox holds. */
  chipLabels?: readonly string[];
  /** The joined label text a select's `Value` part renders. */
  valueText?: string | null;
  /** The core's own select or combobox part state, styled from the way Base UI styles `data-*`. */
  state?: Record<string, unknown>;
}

/** The side and alignment an anchored surface really resolved to. */
export interface AnchorPlacementDetails {
  side: "top" | "bottom" | "left" | "right";
  align: "start" | "center" | "end";
  /** Whether the anchor has been scrolled or clipped out of view. */
  anchorHidden: boolean;
  anchorWidth: number;
  anchorHeight: number;
  /** Room left between the anchor and the viewport edge on the resolved side. */
  availableWidth: number;
  availableHeight: number;
}

/** One toast's place in the core's own stack. */
export interface ToastStackEntry {
  id: string;
  index: number;
  type: ToastType;
  /** Older than the provider's `limit`, and styled back rather than silenced. */
  limited: boolean;
  expanded: boolean;
  swiping: boolean;
  /** Live swipe displacement in logical pixels, for the application to translate by. */
  swipeMovement: number;
  /** `index * pitch`, computed by the core from the declared stack pitch. */
  offset: number;
}

/** The side the tab selection travelled toward when the active tab changed. */
export type TabsActivationDirection = "none" | "left" | "right" | "up" | "down";

/** The active tab's laid-out box, published by the core during paint. */
export interface TabsIndicatorGeometry {
  left: number;
  top: number;
  width: number;
  height: number;
}

/** A progress bar's derived status, matching Base UI's own values. */
export type ProgressStatus = "progressing" | "complete" | "indeterminate";

/** Base UI's toast `type`. */
export type ToastType = "info" | "success" | "warning" | "error" | "loading";

/** Which triggers the core's declared validation mode answers. */
export interface FieldValidationTriggers {
  change: boolean;
  blur: boolean;
  submit: boolean;
}

/** How long the core waits before validating on each trigger, in milliseconds. */
export interface FieldValidationDelays {
  change: number | null;
  blur: number | null;
  submit: number | null;
}

/** Decode the payload of a native `componentchange` event. */
export function componentChangeFromEvent(event: QuickGuiEvent): ComponentChangeDetails | undefined {
  if (!event.value) return undefined;
  try {
    const parsed = JSON.parse(event.value) as ComponentChangeDetails;
    return typeof parsed === "object" && parsed !== null ? parsed : undefined;
  } catch {
    return undefined;
  }
}

function componentChangeListener<T>(
  read: (details: ComponentChangeDetails) => T | undefined,
  apply: (next: T, event: QuickGuiEvent) => void,
): (event: QuickGuiEvent) => void {
  return (event) => {
    const details = componentChangeFromEvent(event);
    if (!details) return;
    const next = read(details);
    if (next === undefined) return;
    apply(next, event);
  };
}

/**
 * Controlled slider root.
 *
 * `values` carries one entry per thumb, so a single-thumb slider and a range slider are the same
 * component. The core answers arrows, Page keys, Home, End, and captured pointer drags; it reports
 * the snapped, ordered, clamped result through `onValueChange`.
 */
export function SliderRoot(props: JSX.SliderProps): NativeNode {
  const [uncontrolled, setUncontrolled] = createSignal<readonly number[]>(
    untrack(() => props.defaultValue ?? [0]),
  );
  const [state, setState] = createSignal<SliderState>(settledSlider);
  const values = () => props.value ?? uncontrolled();
  const context: SliderContextValue = { state };
  return createPartNode(
    "view",
    omit(props, "value", "defaultValue", "onValueChange", "onValueCommitted", "children"),
    {
      part: NativePart.Slider,
      get values() {
        return values().slice();
      },
      onComponentChange: componentChangeReader((details, event) => {
        if (!details.values) return;
        const next = details.values;
        setState({
          values: next,
          dragging: details.dragging === true,
          displayValue: details.displayValue ?? null,
        });
        if (props.value === undefined) setUncontrolled(next);
        props.onValueChange?.(next, event);
        // `onValueCommitted` is the core's own pointer boundary, not a debounce in JavaScript.
        if (details.committed === true) props.onValueCommitted?.(next, event);
      }),
      get children() {
        return SliderContext({
          value: context,
          get children() {
            return props.children as SolidElement;
          },
        });
      },
    },
  );
}

/** Everything the core decided about one slider since the last frame. */
export interface SliderState {
  values: readonly number[];
  /** Whether a captured drag is in flight, matching Base UI's `data-dragging`. */
  dragging: boolean;
  /** The value formatted through the declared `format`, or `null` without one. */
  displayValue: string | null;
}

const settledSlider: SliderState = {
  values: [],
  dragging: false,
  displayValue: null,
};

interface SliderContextValue {
  state: () => SliderState;
}

const SliderContext = createContext<SliderContextValue | null>(null);

/** Read the live slider state inside a `Slider.Root` subtree. */
export function useSliderState(): () => SliderState {
  const context = optionalContext(SliderContext);
  return context ? context.state : () => settledSlider;
}

/** Application-owned slider track painted inside the interactive Control. */
export function SliderTrack(props: JSX.NativeScopedProps): NativeNode {
  return createPartNode("view", props, { part: NativePart.SliderTrack });
}

/** Application-owned slider fill, hidden from the accessible name by the core. */
export function SliderRange(props: JSX.NativeScopedProps): NativeNode {
  return createPartNode("view", props, { part: NativePart.SliderRange });
}

/** Base UI's name for the range fill. It decorates exactly the same core part. */
export function SliderIndicator(props: JSX.NativeScopedProps): NativeNode {
  return createPartNode("view", props, { part: NativePart.SliderIndicator });
}

/** Clickable control box the track and thumbs are laid out inside. */
export function SliderControl(props: JSX.NativeScopedProps): NativeNode {
  return createPartNode("view", props, { part: NativePart.SliderControl });
}

/** Slider label. The core points the root's accessible name at it. */
export function SliderLabel(props: JSX.NativeScopedProps): NativeNode {
  return createPartNode("view", props, { part: NativePart.SliderLabel });
}

/**
 * Slider value readout.
 *
 * The core applies the declared `format` and reports the result, so the text below is the value
 * the core formatted rather than one JavaScript re-derived.
 */
export function SliderValue(props: JSX.NativeScopedProps): NativeNode {
  return createPartNode("view", props, { part: NativePart.SliderValue });
}

/** Application-owned slider thumb. A range slider gives each thumb its own keyboard focus. */
export function SliderThumb(props: JSX.SliderThumbProps): NativeNode {
  return createPartNode("view", omit(props, "index"), {
    part: NativePart.SliderThumb,
    get itemIndex() {
      return props.index ?? props.itemIndex;
    },
  });
}

/** Base-UI-shaped compound parts for a slider. */
export const Slider = Object.assign(SliderRoot, {
  Root: SliderRoot,
  Label: SliderLabel,
  Value: SliderValue,
  Control: SliderControl,
  Track: SliderTrack,
  Range: SliderRange,
  Indicator: SliderIndicator,
  Thumb: SliderThumb,
});

/**
 * Controlled splitter root.
 *
 * `value` carries one size per pane. The core conserves the total across captured drags and typed
 * keyboard resizing and reports every pane size together through `onSizesChange`.
 */
export function SplitterRoot(props: JSX.SplitterProps): NativeNode {
  const [uncontrolled, setUncontrolled] = createSignal<readonly number[]>(
    untrack(() => props.defaultValue ?? []),
  );
  const sizes = () => props.value ?? uncontrolled();
  return createPartNode("view", omit(props, "value", "defaultValue", "onSizesChange", "panes"), {
    part: NativePart.Splitter,
    get values() {
      return sizes().slice();
    },
    get items() {
      return props.panes ? props.panes.slice() : undefined;
    },
    onComponentChange: componentChangeListener(
      (details) => details.sizes,
      (next, event) => {
        if (props.value === undefined) setUncontrolled(next);
        props.onSizesChange?.(next, event);
      },
    ),
  });
}

/** Application-owned splitter pane. */
export function SplitterPane(props: JSX.SplitterPaneProps): NativeNode {
  return createPartNode("view", props, { part: NativePart.SplitterPane });
}

/** Application-owned splitter handle carrying the core's numeric resize semantics. */
export function SplitterHandle(props: JSX.SplitterPaneProps): NativeNode {
  return createPartNode("view", props, { part: NativePart.SplitterHandle });
}

/** Base-UI-shaped compound parts for an adjustable splitter. */
export const Splitter = Object.assign(SplitterRoot, {
  Root: SplitterRoot,
  Pane: SplitterPane,
  Handle: SplitterHandle,
});

/**
 * Toolbar root with a single roving Tab stop.
 *
 * `items` declares the ordered navigation model. The core answers arrows, Home, and End on the
 * focused item, skips disabled items, and reports the moved Tab stop through `onActiveChange`.
 */
export function ToolbarRoot(props: JSX.ToolbarProps): NativeNode {
  const [uncontrolled, setUncontrolled] = createSignal<string | undefined>(
    untrack(() => props.defaultActive),
  );
  const active = () => props.active ?? uncontrolled();
  return createPartNode("view", omit(props, "active", "defaultActive", "onActiveChange"), {
    part: NativePart.Toolbar,
    get activeValue() {
      return active();
    },
    onComponentChange: componentChangeListener(
      (details) => details.active ?? undefined,
      (next, event) => {
        if (props.active === undefined) setUncontrolled(next);
        props.onActiveChange?.(next, event);
      },
    ),
  });
}

/** Application-owned toolbar item. Exactly one enabled item stays in the Tab sequence. */
export function ToolbarItem(props: JSX.ComponentItemProps): NativeNode {
  return createPartNode("button", props, { part: NativePart.ToolbarItem });
}

/** One toolbar command, projected with the core's Button role. */
export function ToolbarButton(props: JSX.ComponentItemProps): NativeNode {
  return createPartNode("button", props, { part: NativePart.ToolbarButton });
}

/** One toolbar link, projected with the core's Link role. */
export function ToolbarLink(props: JSX.ComponentItemProps): NativeNode {
  return createPartNode("button", props, { part: NativePart.ToolbarLink });
}

/** One toolbar input. It keeps the roving contract and its own element role. */
export function ToolbarInput(props: JSX.ToolbarInputProps): NativeNode {
  return createPartNode(props.element ?? "input", omit(props, "element"), {
    part: NativePart.ToolbarInput,
  });
}

/** A related run of toolbar items, projected with the core's Group role. */
export function ToolbarGroup(props: JSX.NativeScopedProps): NativeNode {
  return createPartNode("view", props, { part: NativePart.ToolbarGroup });
}

/** A toolbar separator, whose orientation the core takes from the toolbar's cross axis. */
export function ToolbarSeparator(props: JSX.NativeScopedProps): NativeNode {
  return createPartNode("view", props, { part: NativePart.ToolbarSeparator });
}

/** Base-UI-shaped compound parts for a toolbar. */
export const Toolbar = Object.assign(ToolbarRoot, {
  Root: ToolbarRoot,
  Item: ToolbarItem,
  Button: ToolbarButton,
  Link: ToolbarLink,
  Input: ToolbarInput,
  Group: ToolbarGroup,
  Separator: ToolbarSeparator,
});

/**
 * Toggle-group root with single or multiple selection.
 *
 * `items` declares the ordered navigation model and `value` the pressed values. The core owns the
 * selection policy, the roving Tab stop, and disabled-item skipping.
 */
export function ToggleGroupRoot(props: JSX.ToggleGroupProps): NativeNode {
  const [uncontrolled, setUncontrolled] = createSignal<readonly string[]>(
    untrack(() => props.defaultValue ?? []),
  );
  const pressed = () => props.value ?? uncontrolled();
  return createPartNode("view", omit(props, "value", "defaultValue", "onValueChange"), {
    part: NativePart.ToggleGroup,
    get values() {
      return pressed().slice();
    },
    onComponentChange: componentChangeListener(
      (details) => details.pressed,
      (next, event) => {
        if (props.value === undefined) setUncontrolled(next);
        props.onValueChange?.(next, event);
      },
    ),
  });
}

/** Application-owned toggle-group item carrying pressed-button semantics from the core. */
export function ToggleGroupItem(props: JSX.ComponentItemProps): NativeNode {
  return createPartNode("button", props, { part: NativePart.ToggleGroupItem });
}

/** Base-UI-shaped compound parts for a toggle group. */
export const ToggleGroup = Object.assign(ToggleGroupRoot, {
  Root: ToggleGroupRoot,
  Item: ToggleGroupItem,
});

// ---------------------------------------------------------------------------
// Declared input
//
// Every listener below is declared ahead of the core's decision. The Rust core owns targeting,
// capture, bubbling, multi-click counting, accelerator parsing, and drag promotion; JavaScript
// receives the outcome as a bounded asynchronous payload.
// ---------------------------------------------------------------------------

const inputTextEncoder = new TextEncoder();

/** Modifier flags shared by every declared input payload. */
export interface InputModifiers {
  shift: boolean;
  control: boolean;
  alt: boolean;
  meta: boolean;
}

export interface KeyEventDetails extends InputModifiers {
  /** DOM-shaped normalized key name, such as `"a"`, `"ArrowUp"`, or `"Escape"`. */
  key: string;
  /** Composed text the platform reported for this press, when it produced any. */
  text?: string;
  repeat?: boolean;
}

export interface MouseEventDetails extends InputModifiers {
  x: number;
  y: number;
  button?: string;
  pressedButton?: string | null;
  /** Exact native multi-click count. The first press is `1`. */
  clickCount?: number;
  firstMouse?: boolean;
}

export interface WheelEventDetails extends InputModifiers {
  x: number;
  y: number;
  deltaX: number;
  deltaY: number;
  /** `true` for a trackpad or precise wheel. */
  precise: boolean;
  phase: "started" | "moved" | "ended" | "cancelled";
}

export interface GestureEventDetails extends InputModifiers {
  x: number;
  y: number;
  delta?: number;
  phase?: "started" | "moved" | "ended" | "cancelled";
  pressure?: number;
  stage?: string;
}

export interface DropEventDetails extends InputModifiers {
  x: number;
  y: number;
  /** Declared identifier of an application-local payload. */
  id?: string;
  /** Node id that started the drag. */
  source?: number;
  /** Absolute paths of a native file drop. */
  paths?: string[];
  origin?: "internal" | "cross-window" | "external";
}

/** Accelerator-to-binding-id pairs the Rust core resolves for a focused element. */
export type Keymap = Readonly<Record<string, string>>;

export interface DragSource {
  /** Stable identifier delivered to an application-local drop target. */
  id?: string;
  /** Plain text promoted to other applications. */
  text?: string;
  /** Absolute URL promoted to other applications. */
  url?: string;
  /** Existing files or directories promoted to other applications. */
  files?: readonly { path: string; directory?: boolean }[];
}

export type DropKind = "local" | "files";

function bounded(value: string, limit: number, what: string): string {
  if (inputTextEncoder.encode(value).length > limit) {
    throw new RangeError(`QuickGUI ${what} are bounded to ${limit} bytes`);
  }
  return value;
}

function encodeKeymap(value: unknown): string | null {
  if (value === null || value === undefined || value === false) return null;
  if (!isRecord(value)) {
    throw new TypeError("QuickGUI keymap must map accelerators to binding ids");
  }
  for (const entry of Object.values(value)) {
    if (typeof entry !== "string") {
      throw new TypeError("QuickGUI keymap binding ids must be strings");
    }
  }
  return bounded(JSON.stringify(value), MAX_KEYMAP_JSON_BYTES, "keymaps");
}

function encodeDragSource(value: unknown): string | null {
  if (value === null || value === undefined || value === false) return null;
  const declaration = value === true ? {} : value;
  if (!isRecord(declaration)) {
    throw new TypeError("QuickGUI draggable must declare its payload");
  }
  return bounded(JSON.stringify(declaration), MAX_DRAG_JSON_BYTES, "drag declarations");
}

function encodeDropKinds(value: unknown): string | null {
  if (value === null || value === undefined || value === false) return null;
  const kinds = Array.isArray(value) ? value : [value];
  for (const kind of kinds) {
    if (kind !== "local" && kind !== "files") {
      throw new TypeError(
        `QuickGUI accepts the drop kinds \`local\` and \`files\`, not \`${String(kind)}\``,
      );
    }
  }
  return JSON.stringify(kinds);
}

function parseInputPayload<T>(event: QuickGuiEvent): T | undefined {
  if (!event.value) return undefined;
  try {
    return JSON.parse(event.value) as T;
  } catch {
    return undefined;
  }
}

/** Decode a `keydown` or `keyup` payload. */
export function keyEventFromEvent(event: QuickGuiEvent): KeyEventDetails | undefined {
  return parseInputPayload<KeyEventDetails>(event);
}

/** Decode a `mousedown`, `mouseup`, `mousemove`, `dblclick`, or `contextmenu` payload. */
export function mouseEventFromEvent(event: QuickGuiEvent): MouseEventDetails | undefined {
  return parseInputPayload<MouseEventDetails>(event);
}

/** Decode a `wheel` payload. */
export function wheelEventFromEvent(event: QuickGuiEvent): WheelEventDetails | undefined {
  return parseInputPayload<WheelEventDetails>(event);
}

/** Decode a `pinch`, `rotate`, `smartmagnify`, or `pressure` payload. */
export function gestureEventFromEvent(event: QuickGuiEvent): GestureEventDetails | undefined {
  return parseInputPayload<GestureEventDetails>(event);
}

/** Decode a `drop` or `filesdropped` payload. */
export function dropEventFromEvent(event: QuickGuiEvent): DropEventDetails | undefined {
  return parseInputPayload<DropEventDetails>(event);
}

/** The binding id an `action` event carries, or `undefined` when the payload is missing. */
export function actionFromEvent(event: QuickGuiEvent): string | undefined {
  return event.value || undefined;
}

// ---------------------------------------------------------------------------
// Declared menus
//
// A menu is declared ahead of time as one bounded JSON model. The Rust core owns validation,
// highlighting, typeahead, checkbox/radio policy, submenu models, accessibility semantics, and the
// cursor-point native surface; JavaScript never answers a synchronous question while a menu is
// open and never reimplements menu behavior.
// ---------------------------------------------------------------------------

export type MenuItemKind = "action" | "checkbox" | "radio" | "submenu" | "separator" | "group";

export interface MenuItem {
  /** Defaults to `submenu` when `items` is present, otherwise `action`. */
  type?: MenuItemKind;
  /** Stable identifier reported by `onSelect`. Required for every interactive entry. */
  id?: string;
  label?: string;
  /** Display-only accelerator hint, such as `"⌘O"`. */
  shortcut?: string;
  /** Radio group key. Defaults to the item's own id. */
  group?: string;
  checked?: boolean;
  disabled?: boolean;
  /** Override the core's default close policy for this entry. */
  closeOnSelect?: boolean;
  /** Alphanumeric navigation label when it differs from the visible one. */
  typeaheadLabel?: string;
  items?: readonly MenuItem[];
}

/** Structural geometry and appearance for a declared menu surface. */
export interface MenuAppearance {
  width?: number;
  itemHeight?: number;
  separatorHeight?: number;
  groupLabelHeight?: number;
  verticalPadding?: number;
  fontSize?: number;
  radius?: number;
  padding?: number;
  background?: ColorValue;
  color?: ColorValue;
  highlightBackground?: ColorValue;
  highlightColor?: ColorValue;
  mutedColor?: ColorValue;
  /** Wrap arrow navigation at the ends of a menu level. Defaults to `true`. */
  loop?: boolean;
}

export interface MenuSelectDetails {
  /** Stable id of the activated entry. */
  id: string;
  /** New checkbox value, or `true` for a radio item. */
  checked?: boolean;
  /** `true` when the entry opens a submenu instead of dispatching a command. */
  submenu?: boolean;
}

const menuAppearanceColors: readonly string[] = [
  "background",
  "color",
  "highlightBackground",
  "highlightColor",
  "mutedColor",
];

const menuTextEncoder = new TextEncoder();

function encodeMenuItems(items: readonly MenuItem[]): unknown[] {
  return items.map((item) => {
    const encoded: Record<string, unknown> = {};
    for (const [name, value] of Object.entries(item)) {
      if (value === undefined || name === "items") continue;
      encoded[name] = value;
    }
    if (item.items && item.items.length > 0) {
      encoded.items = encodeMenuItems(item.items);
    }
    return encoded;
  });
}

/**
 * Encode one bounded menu declaration for the Rust binding.
 *
 * Colors are packed here so the core never parses CSS, and the byte bound is enforced before the
 * declaration can cross N-API.
 */
export function encodeMenu(
  items: readonly MenuItem[] | undefined,
  appearance: MenuAppearance = {},
): string {
  const declaration: Record<string, unknown> = {
    items: encodeMenuItems(items ?? []),
  };
  for (const [name, value] of Object.entries(appearance)) {
    if (value === undefined || value === null) continue;
    if (name === "loop") {
      declaration.loopFocus = value === true;
      continue;
    }
    declaration[name] = menuAppearanceColors.includes(name)
      ? parseColor(value as ColorValue)
      : value;
  }
  const encoded = JSON.stringify(declaration);
  if (menuTextEncoder.encode(encoded).length > MAX_MENU_JSON_BYTES) {
    throw new RangeError(`QuickGUI menu declarations are bounded to ${MAX_MENU_JSON_BYTES} bytes`);
  }
  return encoded;
}

/** Decode the payload of a native `menuselect` event. */
export function menuSelectionFromEvent(event: QuickGuiEvent): MenuSelectDetails | undefined {
  if (!event.value) return undefined;
  try {
    const parsed = JSON.parse(event.value) as MenuSelectDetails;
    return typeof parsed?.id === "string" ? parsed : undefined;
  } catch {
    return undefined;
  }
}

interface MenuContextValue {
  items: () => readonly MenuItem[];
  appearance: () => MenuAppearance;
  select: (details: MenuSelectDetails, event: QuickGuiEvent) => void;
}

interface PopoverMenuContextValue extends MenuContextValue {
  open: () => boolean;
  setOpen: (open: boolean, reason: PopoverOpenChangeReason, event: QuickGuiEvent) => void;
  trigger: () => NativeNode | undefined;
  registerTrigger: (node: NativeNode) => void;
  popup: () => NativeNode | undefined;
  registerPopup: (node: NativeNode | undefined) => void;
  placement: () => PopoverPlacement;
  gap: () => number;
  viewportMargin: () => number;
  dismissOnEscape: () => boolean;
  dismissOnPointerOutside: () => boolean;
}

const PopoverMenuContext = createContext<PopoverMenuContextValue | null>(null);
const ContextMenuContext = createContext<MenuContextValue | null>(null);

function requirePopoverMenu(component: string): PopoverMenuContextValue {
  const context = useContext(PopoverMenuContext);
  if (!context) {
    throw new TypeError(`${component} must be used inside <PopoverMenu.Root>`);
  }
  return context;
}

function requireContextMenu(component: string): MenuContextValue {
  const context = useContext(ContextMenuContext);
  if (!context) {
    throw new TypeError(`${component} must be used inside <ContextMenu.Root>`);
  }
  return context;
}

function menuSelectListener(
  context: MenuContextValue,
  handler: ((event: QuickGuiEvent) => void) | undefined,
): (event: QuickGuiEvent) => void {
  return (event) => {
    handler?.(event);
    const details = menuSelectionFromEvent(event);
    if (details) context.select(details, event);
  };
}

/** Logical root of a declared popover menu. It creates no native element. */
export function PopoverMenuRoot(props: JSX.PopoverMenuRootProps): NativeNode {
  const [uncontrolledOpen, setUncontrolledOpen] = createSignal(
    untrack(() => props.defaultOpen ?? false),
  );
  // Triggers and popups register themselves while their own part renders, which Solid 2 treats as
  // an owned-scope write; the registration is intentional.
  const [trigger, setTrigger] = createSignal<NativeNode | undefined>(undefined, {
    ownedWrite: true,
  });
  const [popup, setPopup] = createSignal<NativeNode | undefined>(undefined, {
    ownedWrite: true,
  });
  const open = () => props.open ?? uncontrolledOpen();
  const context: PopoverMenuContextValue = {
    open,
    setOpen(nextOpen, reason, event) {
      if (props.open === undefined) setUncontrolledOpen(nextOpen);
      props.onOpenChange?.(nextOpen, { reason, event });
    },
    items: () => props.items ?? [],
    appearance: () => props.appearance ?? {},
    select(details, event) {
      props.onSelect?.(details, event);
      if (details.submenu) return;
      if (props.open === undefined) setUncontrolledOpen(false);
      props.onOpenChange?.(false, { reason: "dismiss", event });
    },
    trigger,
    registerTrigger: (node) => setTrigger(() => node),
    popup,
    registerPopup: (node) => setPopup(() => node),
    placement: () => props.placement ?? "bottom-start",
    gap: () => props.gap ?? 4,
    viewportMargin: () => props.viewportMargin ?? 8,
    dismissOnEscape: () => props.dismissOnEscape ?? true,
    dismissOnPointerOutside: () => props.dismissOnPointerOutside ?? true,
  };
  return PopoverMenuContext({
    value: context,
    get children() {
      return props.children as SolidElement;
    },
  }) as unknown as NativeNode;
}

/** Menu trigger. The core supplies its `has-popup`, expansion, and controls relationship. */
export function PopoverMenuTrigger(props: JSX.NativeProps): NativeNode {
  const context = requirePopoverMenu("PopoverMenu.Trigger");
  let trigger: NativeNode | undefined;
  return createPartNode("button", omit(props, "onClick", "ref"), {
    part: NativePart.PopoverMenuTrigger,
    get open() {
      return context.open();
    },
    get controls() {
      // The relationship exists only while the surface is mounted, so a closed menu declares no
      // dangling target and no signal is written while the surface disposes.
      return context.open() ? context.popup() : undefined;
    },
    ref: (node: NativeNode) => {
      trigger = node;
      context.registerTrigger(node);
      if (typeof props.ref === "function") props.ref(node);
    },
    onClick: forwardClick(props.onClick, (event) => {
      if (trigger) context.registerTrigger(trigger);
      context.setOpen(!context.open(), "trigger-press", event);
    }),
  });
}

/**
 * Menu surface anchored to the trigger.
 *
 * Rows come from the declared model, so the surface owns only its own paint. It is mounted only
 * while the menu is open and while a trigger exists to anchor it.
 */
export function PopoverMenuPopup(props: JSX.PopoverMenuPopupProps): NativeNode {
  const context = requirePopoverMenu("PopoverMenu.Popup");
  return Show({
    keyed: true,
    get when() {
      return context.open() ? context.trigger() : undefined;
    },
    children: (anchor: NativeNode) => {
      const node = createPartNode("view", omit(props, "style", "width", "onDismiss", "onSelect"), {
        part: NativePart.PopoverMenuPopup,
        anchor,
        get menu() {
          return encodeMenu(context.items(), context.appearance());
        },
        get style() {
          return [
            props.style,
            { width: props.width ?? context.appearance().width ?? 224 },
          ] as JSX.StyleProp;
        },
        get anchorPlacement() {
          return context.placement();
        },
        get anchorGap() {
          return context.gap();
        },
        get viewportMargin() {
          return context.viewportMargin();
        },
        get dismissOnEscape() {
          return context.dismissOnEscape();
        },
        get dismissOnPointerOutside() {
          return context.dismissOnPointerOutside();
        },
        onSelect: menuSelectListener(context, props.onSelect),
        onDismiss(event: QuickGuiEvent) {
          props.onDismiss?.(event);
          if (!event.defaultPrevented) context.setOpen(false, "dismiss", event);
        },
      });
      context.registerPopup(node);
      return node;
    },
  }) as unknown as NativeNode;
}

/** Base-UI-shaped compound parts for a declared popover menu. */
export const PopoverMenu = Object.assign(PopoverMenuRoot, {
  Root: PopoverMenuRoot,
  Trigger: PopoverMenuTrigger,
  Popup: PopoverMenuPopup,
});

/** Logical root of a declared cursor-point context menu. It creates no native element. */
export function ContextMenuRoot(props: JSX.ContextMenuRootProps): NativeNode {
  const context: MenuContextValue = {
    items: () => props.items ?? [],
    appearance: () => props.appearance ?? {},
    select(details, event) {
      props.onSelect?.(details, event);
    },
  };
  // The Base UI-shaped item parts are the same components here, so the context-menu root also
  // supplies the menu compound context they read; the Rust binding gathers the rows from the
  // trigger's own subtree and the core paints them in its cursor-point surface.
  const scope = props.scope ?? createComponentScope("qg-context-menu");
  const menu: MenuCompoundContextValue = {
    scope,
    state: () => settledMenu,
    reportState: () => {},
    open: () => false,
    setOpen: () => {},
    modal: () => undefined,
    orientation: () => undefined,
    loopFocus: () => props.loop,
    closeParentOnEsc: () => undefined,
    disabled: () => undefined,
    openOnHover: () => undefined,
    delay: () => undefined,
    closeDelay: () => undefined,
    positioning: () => ({}),
    declarePositioning: () => {},
    submenu: false,
  };
  return ContextMenuContext({
    value: context,
    get children() {
      return MenuCompoundContext({
        value: menu,
        get children() {
          return props.children as SolidElement;
        },
      });
    },
  }) as unknown as NativeNode;
}

/**
 * Secondary-click target for a declared context menu.
 *
 * The core opens its own cursor-point surface, keeps the menu inside the work area, owns submenu
 * hover intent and safe corridors, and tears the chain down child-first.
 */
export function ContextMenuTrigger(props: JSX.ContextMenuTriggerProps): NativeNode {
  const context = requireContextMenu("ContextMenu.Trigger");
  return createPartNode("view", omit(props, "onSelect"), {
    part: NativePart.ContextMenuTrigger,
    get menu() {
      return encodeMenu(context.items(), context.appearance());
    },
    onSelect: menuSelectListener(context, props.onSelect),
  });
}

/** Base-UI-shaped compound parts for a declared context menu. */
export const ContextMenu = Object.assign(ContextMenuRoot, {
  Root: ContextMenuRoot,
  Trigger: ContextMenuTrigger,
});

// ---------------------------------------------------------------------------
// Base UI-aligned Menu compound
//
// `Menu.Root` is a logical coordinator; the trigger is the one part the core keeps mounted whether
// the menu is open or closed, so it carries the whole declaration and every other part repeats the
// compound scope. Rows are ordinary child nodes: the application owns every pixel, and the core
// owns their identity, `menuitem` semantics, roving highlight, typeahead, toggle policy, radio
// groups, activation, and mount policy.
// ---------------------------------------------------------------------------

/** Everything the core decided about one menu surface, matching Base UI's popup `data-*`. */
export interface MenuSurfaceState {
  open: boolean;
  side: "top" | "bottom" | "left" | "right";
  align: "start" | "center" | "end";
  anchorHidden: boolean;
}

const settledMenu: MenuSurfaceState = {
  open: false,
  side: "bottom",
  align: "start",
  anchorHidden: false,
};

/** Everything the core decided about one menu row, matching Base UI's item `data-*`. */
export interface MenuItemState {
  highlighted: boolean;
  disabled: boolean;
  /** `null` for a row that is not checkable. */
  checked: boolean | null;
  /** Whether this row's submenu is open. */
  open: boolean;
}

const settledMenuItem: MenuItemState = {
  highlighted: false,
  disabled: false,
  checked: null,
  open: false,
};

interface MenuCompoundContextValue {
  scope: string;
  state: () => MenuSurfaceState;
  reportState: (next: MenuSurfaceState) => void;
  open: () => boolean;
  setOpen: (open: boolean, event: QuickGuiEvent) => void;
  modal: () => boolean | undefined;
  orientation: () => "horizontal" | "vertical" | undefined;
  loopFocus: () => boolean | undefined;
  closeParentOnEsc: () => boolean | undefined;
  disabled: () => boolean | undefined;
  openOnHover: () => boolean | undefined;
  delay: () => number | undefined;
  closeDelay: () => number | undefined;
  positioning: () => AnchorPositioning;
  declarePositioning: (positioning: AnchorPositioning) => void;
  submenu: boolean;
}

interface MenuRadioGroupContextValue {
  value: () => string | undefined;
  setValue: (value: string, event: QuickGuiEvent) => void;
}

const MenuCompoundContext = createContext<MenuCompoundContextValue | null>(null);
const MenuRadioGroupContext = createContext<MenuRadioGroupContextValue | null>(null);
const MenuItemStateContext = createContext<(() => MenuItemState) | null>(null);

function requireMenu(component: string): MenuCompoundContextValue {
  const context = useContext(MenuCompoundContext);
  if (!context) {
    throw new TypeError(`${component} must be used inside <Menu.Root>`);
  }
  return context;
}

function createMenuRoot(submenu: boolean, props: JSX.MenuRootProps): NativeNode {
  const [uncontrolled, setUncontrolled] = createSignal(untrack(() => props.defaultOpen ?? false));
  const [state, setState] = createSignal<MenuSurfaceState>(settledMenu);
  const [declared, setDeclared] = createSignal<AnchorPositioning>({}, { ownedWrite: true });
  const open = () => props.open ?? uncontrolled();
  const context: MenuCompoundContextValue = {
    scope: props.scope ?? createComponentScope("qg-menu"),
    state,
    reportState: setState,
    open,
    setOpen(next, event) {
      if (props.open === undefined) setUncontrolled(next);
      props.onOpenChange?.(next, event);
    },
    modal: () => props.modal,
    orientation: () => props.orientation,
    loopFocus: () => props.loopFocus,
    closeParentOnEsc: () => props.closeParentOnEsc,
    disabled: () => props.disabled,
    openOnHover: () => props.openOnHover,
    delay: () => props.delay,
    closeDelay: () => props.closeDelay,
    // The positioner is unmounted while the menu is closed, so its declaration is routed to the
    // trigger, which the core keeps mounted either way.
    positioning: () => {
      const override = declared();
      return {
        side: override.side ?? props.side,
        align: override.align ?? props.align,
        sideOffset: override.sideOffset ?? props.sideOffset,
        alignOffset: override.alignOffset ?? props.alignOffset,
        collisionPadding: override.collisionPadding ?? props.collisionPadding,
        sticky: override.sticky ?? props.sticky,
      };
    },
    declarePositioning: setDeclared,
    submenu,
  };
  return MenuCompoundContext({
    value: context,
    get children() {
      return props.children as SolidElement;
    },
  }) as unknown as NativeNode;
}

/** Logical root of a Base UI-shaped menu. It creates no native element. */
export function MenuRoot(props: JSX.MenuRootProps): NativeNode {
  return createMenuRoot(false, props);
}

/** Logical root of one nested menu level. */
export function MenuSubmenuRoot(props: JSX.MenuRootProps): NativeNode {
  return createMenuRoot(true, props);
}

/** Read what the core decided about the enclosing menu surface. */
export function useMenuState(): () => MenuSurfaceState {
  const context = optionalContext(MenuCompoundContext);
  return context ? context.state : () => settledMenu;
}

/** Read what the core decided about the enclosing menu row. */
export function useMenuItemState(): () => MenuItemState {
  const context = optionalContext(MenuItemStateContext);
  return context ?? (() => settledMenuItem);
}

/** Everything a menu trigger repeats so the Rust binding rebuilds the core descriptor. */
function menuTriggerProps(context: MenuCompoundContextValue, props: JSX.MenuTriggerProps): object {
  const positioning = () => context.positioning();
  return {
    scope: context.scope,
    get open() {
      return context.open();
    },
    get modal() {
      return context.modal();
    },
    get orientation() {
      return context.orientation();
    },
    get loopFocus() {
      return context.loopFocus();
    },
    get closeParentOnEsc() {
      return context.closeParentOnEsc();
    },
    get disabled() {
      return props.disabled ?? context.disabled();
    },
    get openOnHover() {
      return props.openOnHover ?? context.openOnHover();
    },
    get delay() {
      return props.delay ?? context.delay();
    },
    get closeDelay() {
      return props.closeDelay ?? context.closeDelay();
    },
    get side() {
      return positioning().side;
    },
    get align() {
      return positioning().align;
    },
    get sideOffset() {
      return positioning().sideOffset;
    },
    get alignOffset() {
      return positioning().alignOffset;
    },
    get collisionPadding() {
      return positioning().collisionPadding;
    },
    get sticky() {
      return positioning().sticky;
    },
    onComponentChange: componentChangeReader((details, event) => {
      const next: MenuSurfaceState = {
        open: details.open === true,
        side: details.side ?? context.state().side,
        align: details.align ?? context.state().align,
        anchorHidden: details.anchorHidden ?? false,
      };
      context.reportState(next);
      if (typeof details.open === "boolean" && details.open !== context.open()) {
        context.setOpen(details.open, event);
      }
    }),
  };
}

/** Menu trigger. It carries the whole `Menu.Root` declaration the core reads. */
export function MenuTrigger(props: JSX.MenuTriggerProps): NativeNode {
  const context = requireMenu("Menu.Trigger");
  return createPartNode(
    "button",
    omit(props, "openOnHover", "delay", "closeDelay"),
    universal.mergeProps(menuTriggerProps(context, props), {
      part: NativePart.MenuTrigger,
    }),
  );
}

/**
 * Submenu trigger.
 *
 * It is both a row of its parent level and the trigger of its own, so the core owns its
 * `menuitem` semantics, its `has-popup` relationship, and the expanded state it publishes.
 */
export function MenuSubmenuTrigger(props: JSX.MenuSubmenuTriggerProps): NativeNode {
  const context = requireMenu("Menu.SubmenuTrigger");
  const [state, setState] = createSignal<MenuItemState>(settledMenuItem);
  return createPartNode(
    "view",
    omit(props, "openOnHover", "delay", "closeDelay", "value", "label", "closeOnClick", "children"),
    universal.mergeProps(
      menuTriggerProps(context, props),
      { part: NativePart.MenuSubmenuTrigger },
      {
        get partValue() {
          return props.value ?? context.scope;
        },
        get ariaLabel() {
          return props.label;
        },
        get closeOnClick() {
          return props.closeOnClick;
        },
        // One node is both a row of its parent level and the trigger of its own, so it reports
        // both the row state the core derived and the surface state its own level resolved to.
        onComponentChange: componentChangeReader((details, event) => {
          if (details.highlighted !== undefined) {
            setState({
              highlighted: details.highlighted === true,
              disabled: details.disabled === true,
              checked: details.checked ?? null,
              open: details.open === true,
            });
          }
          if (details.side !== undefined || details.align !== undefined) {
            context.reportState({
              open: details.open === true,
              side: details.side ?? context.state().side,
              align: details.align ?? context.state().align,
              anchorHidden: details.anchorHidden ?? false,
            });
          }
          if (typeof details.open === "boolean" && details.open !== context.open()) {
            context.setOpen(details.open, event);
          }
        }),
        get children() {
          return MenuItemStateContext({
            value: state,
            get children() {
              return props.children as SolidElement;
            },
          });
        },
      },
    ),
  );
}

/** One declared part of a menu that only repeats the compound scope. */
function menuPart(
  component: string,
  element: NativeElementName,
  part: NativePartName,
  props: JSX.NativeProps,
): NativeNode {
  const context = requireMenu(component);
  return createPartNode(element, props, { part, scope: context.scope });
}

/** Portal boundary. QuickGUI's retained overlay node is itself the portal. */
export function MenuPortal(props: JSX.NativeProps): NativeNode {
  return menuPart("Menu.Portal", "view", NativePart.MenuPortal, props);
}

/** Application-owned positioner. Base UI declares the placement props here. */
export function MenuPositioner(props: JSX.MenuPositionerProps): NativeNode {
  const context = requireMenu("Menu.Positioner");
  context.declarePositioning({
    get side() {
      return props.side;
    },
    get align() {
      return props.align;
    },
    get sideOffset() {
      return props.sideOffset;
    },
    get alignOffset() {
      return props.alignOffset;
    },
    get collisionPadding() {
      return props.collisionPadding;
    },
    get sticky() {
      return props.sticky;
    },
  });
  return createPartNode(
    "view",
    omit(props, "side", "align", "sideOffset", "alignOffset", "collisionPadding", "sticky"),
    { part: NativePart.MenuPositioner, scope: context.scope },
  );
}

/** Full-viewport pointer layer for a modal menu. The core hides it from assistive technology. */
export function MenuBackdrop(props: JSX.NativeProps): NativeNode {
  return menuPart("Menu.Backdrop", "view", NativePart.MenuBackdrop, props);
}

/** The menu surface. The core owns its role, dismissal, focus restoration, and key bindings. */
export function MenuPopup(props: JSX.NativeProps): NativeNode {
  return menuPart("Menu.Popup", "view", NativePart.MenuPopup, props);
}

/** Caller-owned arrow pinned to the edge the popup really opened against. */
export function MenuArrow(props: JSX.NativeProps): NativeNode {
  return menuPart("Menu.Arrow", "view", NativePart.MenuArrow, props);
}

/** A related group of rows. */
export function MenuGroup(props: JSX.NativeProps): NativeNode {
  return menuPart("Menu.Group", "view", NativePart.MenuGroup, props);
}

/** A group's visible label row. */
export function MenuGroupLabel(props: JSX.MenuGroupLabelProps): NativeNode {
  const context = requireMenu("Menu.GroupLabel");
  return createPartNode("view", omit(props, "value", "label"), {
    part: NativePart.MenuGroupLabel,
    scope: context.scope,
    get partValue() {
      return props.value;
    },
    get ariaLabel() {
      return props.label;
    },
  });
}

/** A non-interactive divider row. */
export function MenuSeparator(props: JSX.NativeProps): NativeNode {
  return menuPart("Menu.Separator", "view", NativePart.MenuSeparator, props);
}

/** Shared declaration for every interactive row. */
function menuItemPart(
  component: string,
  part: NativePartName,
  props: JSX.MenuItemProps,
  extra: object = {},
  report?: (details: ComponentChangeDetails, event: QuickGuiEvent) => void,
): NativeNode {
  const context = requireMenu(component);
  const [state, setState] = createSignal<MenuItemState>(settledMenuItem);
  return createPartNode(
    "view",
    omit(props, "value", "label", "closeOnClick", "children"),
    universal.mergeProps(
      {
        part,
        scope: context.scope,
        get partValue() {
          return props.value;
        },
        get ariaLabel() {
          return props.label;
        },
        get closeOnClick() {
          return props.closeOnClick;
        },
        onComponentChange: componentChangeReader((details, event) => {
          setState({
            highlighted: details.highlighted === true,
            disabled: details.disabled === true,
            checked: details.checked ?? null,
            open: details.open === true,
          });
          report?.(details, event);
        }),
        get children() {
          return MenuItemStateContext({
            value: state,
            get children() {
              return props.children as SolidElement;
            },
          });
        },
      },
      extra,
    ),
  );
}

/**
 * One command row. Activation, closing policy, and semantics come from the core.
 *
 * This is the component; the same name also types one entry of the JSON `items` model a
 * `PopoverMenu` or `ContextMenu` declares, which is the other way to declare a menu's rows.
 */
export function MenuItem(props: JSX.MenuItemProps): NativeNode {
  return menuItemPart("Menu.Item", NativePart.MenuItem, props, {});
}

/**
 * One link row.
 *
 * QuickGUI has no document to navigate, so activation reaches the platform through the core's own
 * open-URL path and the destination is reported back here.
 */
export function MenuLinkItem(props: JSX.MenuLinkItemProps): NativeNode {
  return menuItemPart(
    "Menu.LinkItem",
    NativePart.MenuLinkItem,
    props,
    {
      get href() {
        return props.href;
      },
    },
    (details, event) => {
      if (details.href !== undefined) props.onNavigate?.(details.href, event);
    },
  );
}

/** One checkbox row. The core owns the toggle and reports the value it committed. */
export function MenuCheckboxItem(props: JSX.MenuCheckboxItemProps): NativeNode {
  return menuItemPart(
    "Menu.CheckboxItem",
    NativePart.MenuCheckboxItem,
    props,
    {
      get checked() {
        return props.checked;
      },
    },
    (details, event) => {
      if (typeof details.checked === "boolean") {
        props.onCheckedChange?.(details.checked, event);
      }
    },
  );
}

/** A checkbox row's mark. Mount it only while the row is checked, exactly as Base UI does. */
export function MenuCheckboxItemIndicator(props: JSX.NativeProps): NativeNode {
  return menuPart(
    "Menu.CheckboxItemIndicator",
    "view",
    NativePart.MenuCheckboxItemIndicator,
    props,
  );
}

/** A radio group. The core keeps exactly one of its rows checked. */
export function MenuRadioGroup(props: JSX.MenuRadioGroupProps): NativeNode {
  const context = requireMenu("Menu.RadioGroup");
  const [uncontrolled, setUncontrolled] = createSignal(untrack(() => props.defaultValue));
  const value = () => props.value ?? uncontrolled();
  const group: MenuRadioGroupContextValue = {
    value,
    setValue(next, event) {
      if (props.value === undefined) setUncontrolled(next);
      props.onValueChange?.(next, event);
    },
  };
  return createPartNode("view", omit(props, "value", "defaultValue", "onValueChange", "children"), {
    part: NativePart.MenuRadioGroup,
    scope: context.scope,
    get partValue() {
      return props.name;
    },
    get activeValue() {
      return value();
    },
    onComponentChange: componentChangeReader((details, event) => {
      if (typeof details.value === "string") group.setValue(details.value, event);
    }),
    get children() {
      return MenuRadioGroupContext({
        value: group,
        get children() {
          return props.children as SolidElement;
        },
      });
    },
  });
}

/** One radio row. */
export function MenuRadioItem(props: JSX.MenuRadioItemProps): NativeNode {
  const group = optionalContext(MenuRadioGroupContext);
  return menuItemPart("Menu.RadioItem", NativePart.MenuRadioItem, props, {
    get checked() {
      return props.checked ?? (group ? group.value() === props.value : undefined);
    },
  });
}

/** A radio row's mark. */
export function MenuRadioItemIndicator(props: JSX.NativeProps): NativeNode {
  return menuPart("Menu.RadioItemIndicator", "view", NativePart.MenuRadioItemIndicator, props);
}

/** Base UI-shaped compound parts for an in-window menu. */
export const Menu = Object.assign(MenuRoot, {
  Root: MenuRoot,
  Trigger: MenuTrigger,
  Portal: MenuPortal,
  Backdrop: MenuBackdrop,
  Positioner: MenuPositioner,
  Popup: MenuPopup,
  Arrow: MenuArrow,
  Item: MenuItem,
  LinkItem: MenuLinkItem,
  SubmenuRoot: MenuSubmenuRoot,
  SubmenuTrigger: MenuSubmenuTrigger,
  Group: MenuGroup,
  GroupLabel: MenuGroupLabel,
  RadioGroup: MenuRadioGroup,
  RadioItem: MenuRadioItem,
  RadioItemIndicator: MenuRadioItemIndicator,
  CheckboxItem: MenuCheckboxItem,
  CheckboxItemIndicator: MenuCheckboxItemIndicator,
  Separator: MenuSeparator,
});

export function createRenderer(code: () => JSX.Element): WindowRenderer {
  return (window) => {
    const nativeDispose = nativeRender(() => code() as NativeNode, window.root);
    let disposed = false;
    window.flush();
    return () => {
      if (disposed) return;
      disposed = true;
      nativeDispose();
      window.flush();
    };
  };
}

export const effect = universal.effect;
export const memo = universal.memo;
export const createComponent = universal.createComponent;
export const createElement = universal.createElement;
export const createTextNode = universal.createTextNode;
export const insertNode = universal.insertNode;
export const insert = universal.insert;
export const spread = universal.spread;
export const setProp = universal.setProp;
export const mergeProps = universal.mergeProps;
export const applyRef = universal.applyRef;
export const ref = universal.ref;

/** Easing curves the Rust core exposes. `ease` is an alias for `ease-in-out`. */
export type TransitionEasing = "linear" | "ease" | "ease-in" | "ease-out" | "ease-in-out";

/** Transition properties the Rust core can interpolate without a layout pass. */
export type TransitionPropertyName =
  | "all"
  | "bg"
  | "border-color"
  | "border-width"
  | "border-radius"
  | "color"
  | "box-shadow"
  | "opacity";

export interface TransitionDeclaration {
  property?: TransitionPropertyName | readonly TransitionPropertyName[];
  properties?: TransitionDeclaration["property"];
  duration?: number | string;
  easing?: TransitionEasing;
  timingFunction?: TransitionEasing;
  maxFps?: number;
}

// ---------------------------------------------------------------------------
// Declared option sources, virtual collections, and the remaining stateful fields
//
// Every value below is declared ahead of the core's decision. The Rust core owns filtering,
// highlight movement, typeahead, native popover placement and lifetime, virtual windowing,
// column widths and display order, selection ranges, expansion, lazy children, inline-edit
// lifetime, numeric parsing and clamping, civil-value segment arithmetic, month arithmetic,
// menubar roving focus, and toast auto-dismiss deadlines. JavaScript declares the data and
// receives whatever the core decided as one asynchronous `componentchange` or `commit` payload.
// ---------------------------------------------------------------------------

/** The payload of a native `commit` event. */
export interface CommitDetails {
  /** Committed option value, tree node id, or number-field value. */
  value?: string | number | null;
  /** Committed free-form input text. */
  inputValue?: string;
  /** Activated table row. */
  row?: number;
  /** Activated table column, by declaration index. */
  column?: number;
}

/** Decode the payload of a native `commit` event. */
export function commitFromEvent(event: QuickGuiEvent): CommitDetails | undefined {
  if (!event.value) return undefined;
  try {
    const parsed = JSON.parse(event.value) as CommitDetails;
    return typeof parsed === "object" && parsed !== null ? parsed : undefined;
  } catch {
    return undefined;
  }
}

function commitListener(
  handler: ((details: CommitDetails, event: QuickGuiEvent) => void) | undefined,
): ((event: QuickGuiEvent) => void) | undefined {
  if (!handler) return undefined;
  return (event) => {
    const details = commitFromEvent(event);
    if (details) handler(details, event);
  };
}

/** Read every key one declared component reports, applying only the ones the core moved. */
function componentChangeReader(
  apply: (details: ComponentChangeDetails, event: QuickGuiEvent) => void,
): (event: QuickGuiEvent) => void {
  return (event) => {
    const details = componentChangeFromEvent(event);
    if (details) apply(details, event);
  };
}

/** One entry in a declared select, combobox, or autocomplete option source. */
export interface OptionDeclaration {
  value: string;
  label?: string;
  /** Trailing hint text the core paints on the option row. */
  detail?: string;
  /** Searchable group name. The core's picker has no group rows, so this joins the keywords. */
  group?: string;
  keywords?: string;
  disabled?: boolean;
}

/** Structural geometry and paint for the rows the core renders in its own popover window. */
export interface PickerAppearance {
  width?: number;
  rowHeight?: number;
  maxVisibleRows?: number;
  anchorGap?: number;
  fontSize?: number;
  radius?: number;
  padding?: number;
  verticalPadding?: number;
  background?: ColorValue;
  color?: ColorValue;
  highlightBackground?: ColorValue;
  highlightColor?: ColorValue;
  selectedBackground?: ColorValue;
  mutedColor?: ColorValue;
}

/**
 * Pass one declared option source through in either of Base UI's two shapes.
 *
 * `items` is an array of option objects, or the map form — one entry per value and its label. The
 * Rust binding decodes both, so JavaScript reshapes nothing.
 */
function pickerOptionSource(
  items: readonly OptionDeclaration[] | Readonly<Record<string, string>> | undefined,
): unknown {
  if (items === undefined) return undefined;
  return Array.isArray(items) ? items.slice() : { ...items };
}

function encodePickerAppearance(
  appearance: PickerAppearance | undefined,
): Record<string, unknown> | undefined {
  if (!appearance) return undefined;
  const encoded: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(appearance)) {
    if (value === undefined || value === null) continue;
    encoded[key] =
      key === "background" ||
      key === "color" ||
      key === "highlightBackground" ||
      key === "highlightColor" ||
      key === "selectedBackground" ||
      key === "mutedColor"
        ? parseColor(value as ColorValue)
        : value;
  }
  return encoded;
}

/**
 * Controlled select trigger.
 *
 * The trigger is the only element JavaScript declares: the core opens its own native popover
 * window and paints every option row from `appearance`, so no row can ever wait on the hosted
 * runtime while the core is deciding what a keystroke means.
 */
export function SelectRoot(props: JSX.SelectProps): NativeNode {
  const [uncontrolled, setUncontrolled] = createSignal<string | undefined>(
    untrack(() => props.defaultValue),
  );
  const [uncontrolledValues, setUncontrolledValues] = createSignal<readonly string[]>(
    props.defaultValues ?? [],
  );
  const [state, setState] = createSignal<SelectPartState>(settledSelectState);
  const [valueText, setValueText] = createSignal<string | null>(null);
  const scope = props.scope ?? createComponentScope("qg-select");
  const value = () => props.value ?? uncontrolled();
  const values = () => props.values ?? uncontrolledValues();
  const context: PickerContextValue = {
    scope,
    selectState: state,
    comboboxState: () => settledComboboxState,
    valueText,
    chips: () => [],
  };
  return createPartNode(
    "button",
    omit(
      props,
      "value",
      "defaultValue",
      "values",
      "defaultValues",
      "onValueChange",
      "onValuesChange",
      "onOpenChange",
      "onCommit",
      "items",
      "appearance",
      "children",
    ),
    {
      part: NativePart.Select,
      scope,
      get options() {
        return pickerOptionSource(props.items);
      },
      get appearance() {
        return encodePickerAppearance(props.appearance);
      },
      get activeValue() {
        return props.multiple ? undefined : value();
      },
      get values() {
        return props.multiple ? values().slice() : undefined;
      },
      onComponentChange: componentChangeReader((details, event) => {
        if (details.state) setState(details.state as unknown as SelectPartState);
        if (details.valueText !== undefined) setValueText(details.valueText ?? null);
        if (props.multiple) {
          if (details.selectedValues !== undefined) {
            const next = details.selectedValues.slice();
            if (props.values === undefined) setUncontrolledValues(next);
            props.onValuesChange?.(next, event);
          }
        } else if (details.value !== undefined) {
          const next = typeof details.value === "string" ? details.value : undefined;
          if (props.value === undefined) setUncontrolled(next);
          props.onValueChange?.(next, event);
        }
        if (typeof details.open === "boolean") {
          props.onOpenChange?.(details.open, event);
        }
      }),
      onCommit: commitListener(props.onCommit),
      get children() {
        return PickerContext({
          value: context,
          get children() {
            return props.children as SolidElement;
          },
        });
      },
    },
  );
}

/** One declared option. The core paints the row itself, so this node mounts nothing. */
export function SelectOption(props: JSX.OptionProps): NativeNode {
  const scope = props.scope ?? optionalContext(PickerContext)?.scope;
  return createPartNode("view", optionPartProps(props), {
    part: NativePart.Option,
    scope,
  });
}

// ---------------------------------------------------------------------------
// Base UI-aligned select and combobox parts
//
// The option and result lists live in a separate native child window the core paints from the
// bounded `appearance` declaration, so the popup-side parts are declarations rather than
// owner-window elements: they name the surface, its placement, its scroll affordances, and the
// options it holds. Every owner-window part — label, value, icon, backdrop, input group, chips,
// clear, trigger, status, empty — is a real element decorated by the core's own part descriptor.
// ---------------------------------------------------------------------------

/** Everything the core decided about one select, matching Base UI's trigger `data-*`. */
export interface SelectPartState {
  popupOpen: boolean;
  popupSide: "top" | "bottom" | "left" | "right";
  pressed: boolean;
  placeholder: boolean;
  valid: boolean;
  invalid: boolean;
  dirty: boolean;
  touched: boolean;
  filled: boolean;
  focused: boolean;
  readOnly: boolean;
  required: boolean;
}

const settledSelectState: SelectPartState = {
  popupOpen: false,
  popupSide: "bottom",
  pressed: false,
  placeholder: true,
  valid: true,
  invalid: false,
  dirty: false,
  touched: false,
  filled: false,
  focused: false,
  readOnly: false,
  required: false,
};

/** Everything the core decided about one combobox, matching Base UI's input `data-*`. */
export interface ComboboxPartState {
  popupOpen: boolean;
  pressed: boolean;
  placeholder: boolean;
  valid: boolean;
  invalid: boolean;
  dirty: boolean;
  touched: boolean;
  filled: boolean;
  focused: boolean;
  readOnly: boolean;
  required: boolean;
  /** The polite live-region text the core derived for `Combobox.Status`. */
  status: string;
  /** Whether the query really matched nothing, which is when `Combobox.Empty` mounts. */
  empty: boolean;
  resultCount: number;
}

const settledComboboxState: ComboboxPartState = {
  popupOpen: false,
  pressed: false,
  placeholder: true,
  valid: true,
  invalid: false,
  dirty: false,
  touched: false,
  filled: false,
  focused: false,
  readOnly: false,
  required: false,
  status: "",
  empty: false,
  resultCount: 0,
};

interface PickerContextValue {
  scope: string;
  selectState: () => SelectPartState;
  comboboxState: () => ComboboxPartState;
  /** The joined label text a select's `Value` part renders, or `null` for the placeholder. */
  valueText: () => string | null;
  chips: () => readonly { value: string; label: string }[];
}

const PickerContext = createContext<PickerContextValue | null>(null);

function pickerScope(component: string, declared: string | undefined): string {
  const context = optionalContext(PickerContext);
  const scope = declared ?? context?.scope;
  if (!scope) {
    throw new TypeError(`${component} must be used inside its picker root`);
  }
  return scope;
}

/** Read what the core decided about the enclosing select. */
export function useSelectState(): () => SelectPartState {
  const context = optionalContext(PickerContext);
  return context ? context.selectState : () => settledSelectState;
}

/** Read what the core decided about the enclosing combobox or autocomplete. */
export function useComboboxState(): () => ComboboxPartState {
  const context = optionalContext(PickerContext);
  return context ? context.comboboxState : () => settledComboboxState;
}

/** Read the chips a multiple combobox holds, in chip order. */
export function useComboboxChips(): () => readonly { value: string; label: string }[] {
  const context = optionalContext(PickerContext);
  return context ? context.chips : () => [];
}

/** One declared part that only repeats its picker's scope. */
function pickerPart(
  component: string,
  element: NativeElementName,
  part: NativePartName,
  props: JSX.NativeScopedProps,
): NativeNode {
  const scope = pickerScope(component, props.scope);
  return createPartNode(element, props, { part, scope });
}

/** The select's visible label. The trigger points its accessible name at this identity. */
export function SelectLabel(props: JSX.NativeScopedProps): NativeNode {
  return pickerPart("Select.Label", "view", NativePart.SelectLabel, props);
}

/** The select's value text. Render `useSelectState()`'s value, or the placeholder. */
export function SelectValue(props: JSX.NativeScopedProps): NativeNode {
  return pickerPart("Select.Value", "view", NativePart.SelectValue, props);
}

/** The select's trigger affordance. */
export function SelectIcon(props: JSX.NativeScopedProps): NativeNode {
  return pickerPart("Select.Icon", "view", NativePart.SelectIcon, props);
}

/** An owner-window dimming layer mounted only while the core holds the surface open. */
export function SelectBackdrop(props: JSX.NativeScopedProps): NativeNode {
  return pickerPart("Select.Backdrop", "view", NativePart.SelectBackdrop, props);
}

/** The option-surface boundary. It declares the placement; the core paints the window. */
export function SelectPortal(props: JSX.SelectPositionerProps): NativeNode {
  return pickerPart("Select.Portal", "view", NativePart.SelectPortal, props);
}

/** The option-surface positioner. `side`, `align`, and `sideOffset` are declared here. */
export function SelectPositioner(props: JSX.SelectPositionerProps): NativeNode {
  return pickerPart("Select.Positioner", "view", NativePart.SelectPositioner, props);
}

/** The option surface itself. The core paints it in its own native window. */
export function SelectPopup(props: JSX.NativeScopedProps): NativeNode {
  return pickerPart("Select.Popup", "view", NativePart.SelectPopup, props);
}

/** A decorative arrow on the option surface. */
export function SelectArrow(props: JSX.NativeScopedProps): NativeNode {
  return pickerPart("Select.Arrow", "view", NativePart.SelectArrow, props);
}

/** The scrolling option list. */
export function SelectList(props: JSX.NativeScopedProps): NativeNode {
  return pickerPart("Select.List", "view", NativePart.SelectList, props);
}

/** One declared option. Its `ItemText` supplies the label when none is declared. */
export function SelectItem(props: JSX.OptionProps): NativeNode {
  const scope = pickerScope("Select.Item", props.scope);
  return createPartNode("view", optionPartProps(props), {
    part: NativePart.SelectItem,
    scope,
  });
}

/** One option's visible text. It is the option's label when `label` is omitted. */
export function SelectItemText(props: JSX.NativeScopedProps): NativeNode {
  return pickerPart("Select.ItemText", "view", NativePart.SelectItemText, props);
}

/** One option's selected mark. */
export function SelectItemIndicator(props: JSX.NativeScopedProps): NativeNode {
  return pickerPart("Select.ItemIndicator", "view", NativePart.SelectItemIndicator, props);
}

/** An option group. Its label becomes the searchable group name of the options inside it. */
export function SelectGroup(props: JSX.OptionProps): NativeNode {
  const scope = pickerScope("Select.Group", props.scope);
  return createPartNode("view", optionPartProps(props), {
    part: NativePart.SelectGroup,
    scope,
  });
}

/** An option group's label. */
export function SelectGroupLabel(props: JSX.NativeScopedProps): NativeNode {
  return pickerPart("Select.GroupLabel", "view", NativePart.SelectGroupLabel, props);
}

/** A divider between option groups. */
export function SelectSeparator(props: JSX.NativeScopedProps): NativeNode {
  return pickerPart("Select.Separator", "view", NativePart.SelectSeparator, props);
}

/**
 * Declares the upward scroll affordance the core mounts inside its option surface.
 *
 * While the pointer rests on it the option window advances one row every 50 ms, each step an exact
 * one-shot deadline armed by the previous one.
 */
export function SelectScrollUpArrow(props: JSX.NativeScopedProps): NativeNode {
  return pickerPart("Select.ScrollUpArrow", "view", NativePart.SelectScrollUpArrow, props);
}

/** Declares the downward scroll affordance the core mounts inside its option surface. */
export function SelectScrollDownArrow(props: JSX.NativeScopedProps): NativeNode {
  return pickerPart("Select.ScrollDownArrow", "view", NativePart.SelectScrollDownArrow, props);
}

/** The combobox's visible label. */
export function ComboboxLabel(props: JSX.NativeScopedProps): NativeNode {
  return pickerPart("Combobox.Label", "view", NativePart.ComboboxLabel, props);
}

/** The combobox's committed-value text. */
export function ComboboxValue(props: JSX.NativeScopedProps): NativeNode {
  return pickerPart("Combobox.Value", "view", NativePart.ComboboxValue, props);
}

/** The combobox's affordance glyph. */
export function ComboboxIcon(props: JSX.NativeScopedProps): NativeNode {
  return pickerPart("Combobox.Icon", "view", NativePart.ComboboxIcon, props);
}

/** The wrapper holding the input, its chips, and its affordances. */
export function ComboboxInputGroup(props: JSX.NativeScopedProps): NativeNode {
  return pickerPart("Combobox.InputGroup", "view", NativePart.ComboboxInputGroup, props);
}

/** The clear control. The core clears the committed value and every chip. */
export function ComboboxClear(props: JSX.NativeScopedProps): NativeNode {
  return pickerPart("Combobox.Clear", "button", NativePart.ComboboxClear, props);
}

/**
 * The surface trigger.
 *
 * QuickGUI's combobox opens from its own input, so this part carries Base UI's button semantics
 * and the `controls` relationship while the input keeps the opening behavior.
 */
export function ComboboxTrigger(props: JSX.NativeScopedProps): NativeNode {
  return pickerPart("Combobox.Trigger", "button", NativePart.ComboboxTrigger, props);
}

/** The chip container of a multiple combobox. */
export function ComboboxChips(props: JSX.NativeScopedProps): NativeNode {
  return pickerPart("Combobox.Chips", "view", NativePart.ComboboxChips, props);
}

/** One chip. `index` names the chip position the core retained. */
export function ComboboxChip(props: JSX.ComboboxChipProps): NativeNode {
  const scope = pickerScope("Combobox.Chip", props.scope);
  return createPartNode("view", omit(props, "index"), {
    part: NativePart.ComboboxChip,
    scope,
    get itemIndex() {
      return props.index ?? 0;
    },
  });
}

/** One chip's remove control. The core removes the chip and reports the new set. */
export function ComboboxChipRemove(props: JSX.ComboboxChipProps): NativeNode {
  const scope = pickerScope("Combobox.ChipRemove", props.scope);
  return createPartNode("button", omit(props, "index"), {
    part: NativePart.ComboboxChipRemove,
    scope,
    get itemIndex() {
      return props.index ?? 0;
    },
  });
}

/** An owner-window dimming layer mounted only while the core holds the surface open. */
export function ComboboxBackdrop(props: JSX.NativeScopedProps): NativeNode {
  return pickerPart("Combobox.Backdrop", "view", NativePart.ComboboxBackdrop, props);
}

/** The suggestion-surface boundary. */
export function ComboboxPortal(props: JSX.SelectPositionerProps): NativeNode {
  return pickerPart("Combobox.Portal", "view", NativePart.ComboboxPortal, props);
}

/** The suggestion-surface positioner. */
export function ComboboxPositioner(props: JSX.SelectPositionerProps): NativeNode {
  return pickerPart("Combobox.Positioner", "view", NativePart.ComboboxPositioner, props);
}

/** The suggestion surface itself. The core paints it in its own native window. */
export function ComboboxPopup(props: JSX.NativeScopedProps): NativeNode {
  return pickerPart("Combobox.Popup", "view", NativePart.ComboboxPopup, props);
}

/** A decorative arrow on the suggestion surface. */
export function ComboboxArrow(props: JSX.NativeScopedProps): NativeNode {
  return pickerPart("Combobox.Arrow", "view", NativePart.ComboboxArrow, props);
}

/** The polite live region. Render `useComboboxState()`'s `status` inside it. */
export function ComboboxStatus(props: JSX.NativeScopedProps): NativeNode {
  return pickerPart("Combobox.Status", "view", NativePart.ComboboxStatus, props);
}

/** The no-results part. The core mounts it only while the query really matched nothing. */
export function ComboboxEmpty(props: JSX.NativeScopedProps): NativeNode {
  return pickerPart("Combobox.Empty", "view", NativePart.ComboboxEmpty, props);
}

/** The scrolling result list. */
export function ComboboxList(props: JSX.NativeScopedProps): NativeNode {
  return pickerPart("Combobox.List", "view", NativePart.ComboboxList, props);
}

/** A grid-shaped result row. */
export function ComboboxRow(props: JSX.NativeScopedProps): NativeNode {
  return pickerPart("Combobox.Row", "view", NativePart.ComboboxRow, props);
}

/** One declared result. */
export function ComboboxItem(props: JSX.OptionProps): NativeNode {
  const scope = pickerScope("Combobox.Item", props.scope);
  return createPartNode("view", optionPartProps(props), { part: NativePart.ComboboxItem, scope });
}

/** One result's selected mark. */
export function ComboboxItemIndicator(props: JSX.NativeScopedProps): NativeNode {
  return pickerPart("Combobox.ItemIndicator", "view", NativePart.ComboboxItemIndicator, props);
}

/** A result group. Its label becomes the searchable group name of the results inside it. */
export function ComboboxGroup(props: JSX.OptionProps): NativeNode {
  const scope = pickerScope("Combobox.Group", props.scope);
  return createPartNode("view", optionPartProps(props), {
    part: NativePart.ComboboxGroup,
    scope,
  });
}

/** A result group's label. */
export function ComboboxGroupLabel(props: JSX.NativeScopedProps): NativeNode {
  return pickerPart("Combobox.GroupLabel", "view", NativePart.ComboboxGroupLabel, props);
}

/** A wrapper around the mounted rows. */
export function ComboboxCollection(props: JSX.NativeScopedProps): NativeNode {
  return pickerPart("Combobox.Collection", "view", NativePart.ComboboxCollection, props);
}

/** A divider between result groups. */
export function ComboboxSeparator(props: JSX.NativeScopedProps): NativeNode {
  return pickerPart("Combobox.Separator", "view", NativePart.ComboboxSeparator, props);
}

/** Base-UI-shaped compound parts for a select. */
export const Select = Object.assign(SelectRoot, {
  Root: SelectRoot,
  /** Base UI's name for the declaration-carrying trigger; `Select.Root` is the same node. */
  Trigger: SelectRoot,
  Option: SelectOption,
  Label: SelectLabel,
  Value: SelectValue,
  Icon: SelectIcon,
  Backdrop: SelectBackdrop,
  Portal: SelectPortal,
  Positioner: SelectPositioner,
  Popup: SelectPopup,
  Arrow: SelectArrow,
  List: SelectList,
  Item: SelectItem,
  ItemText: SelectItemText,
  ItemIndicator: SelectItemIndicator,
  Group: SelectGroup,
  GroupLabel: SelectGroupLabel,
  Separator: SelectSeparator,
  ScrollUpArrow: SelectScrollUpArrow,
  ScrollDownArrow: SelectScrollDownArrow,
});

/**
 * Controlled constrained combobox.
 *
 * Arbitrary text is an editing query, not a committable value: the core restores the last
 * committed label on dismissal and reports the constrained value it committed.
 */
export function ComboboxRoot(props: JSX.ComboboxProps): NativeNode {
  const [uncontrolled, setUncontrolled] = createSignal<string | undefined>(
    untrack(() => props.defaultValue),
  );
  const [uncontrolledValues, setUncontrolledValues] = createSignal<readonly string[]>(
    props.defaultValues ?? [],
  );
  const [state, setState] = createSignal<ComboboxPartState>(settledComboboxState);
  const [chips, setChips] = createSignal<readonly { value: string; label: string }[]>([]);
  const scope = props.scope ?? createComponentScope("qg-combobox");
  const value = () => props.value ?? uncontrolled();
  const values = () => props.values ?? uncontrolledValues();
  const context: PickerContextValue = {
    scope,
    selectState: () => settledSelectState,
    comboboxState: state,
    valueText: () => null,
    chips,
  };
  const input = createPartNode(
    "input",
    omit(
      props,
      "value",
      "defaultValue",
      "values",
      "defaultValues",
      "onValueChange",
      "onValuesChange",
      "onInputValueChange",
      "onOpenChange",
      "onCommit",
      "items",
      "appearance",
      "children",
    ),
    {
      part: NativePart.Combobox,
      scope,
      get options() {
        return pickerOptionSource(props.items);
      },
      get appearance() {
        return encodePickerAppearance(props.appearance);
      },
      get activeValue() {
        return value();
      },
      get values() {
        return props.multiple ? values().slice() : undefined;
      },
      onComponentChange: componentChangeReader((details, event) => {
        if (details.state) setState(details.state as unknown as ComboboxPartState);
        if (details.chipValues !== undefined) {
          const labels = details.chipLabels ?? [];
          setChips(
            details.chipValues.map((chip, index) => ({
              value: chip,
              label: labels[index] ?? chip,
            })),
          );
          if (props.multiple) {
            const next = details.chipValues.slice();
            if (props.values === undefined) setUncontrolledValues(next);
            props.onValuesChange?.(next, event);
          }
        }
        if (details.value !== undefined) {
          const next = typeof details.value === "string" ? details.value : undefined;
          if (props.value === undefined) setUncontrolled(next);
          props.onValueChange?.(next, event);
        }
        if (details.inputValue !== undefined) {
          props.onInputValueChange?.(details.inputValue, event);
        }
        if (typeof details.open === "boolean") {
          props.onOpenChange?.(details.open, event);
        }
      }),
      onCommit: commitListener(props.onCommit),
    },
  );
  // The input is a leaf: a text field paints its own content, so the compound's other parts are
  // siblings of it rather than children. Declared `Item` nodes are gathered from the whole
  // compound, so they keep working wherever the application puts them.
  return PickerContext({
    value: context,
    get children() {
      return [input, props.children] as unknown as SolidElement;
    },
  }) as unknown as NativeNode;
}

/** Base-UI-shaped compound parts for a constrained combobox. */
export const Combobox = Object.assign(ComboboxRoot, {
  Root: ComboboxRoot,
  /** Base UI's name for the declaration-carrying input; `Combobox.Root` is the same node. */
  Input: ComboboxRoot,
  Option: SelectOption,
  Label: ComboboxLabel,
  Value: ComboboxValue,
  Icon: ComboboxIcon,
  InputGroup: ComboboxInputGroup,
  Clear: ComboboxClear,
  Trigger: ComboboxTrigger,
  Chips: ComboboxChips,
  Chip: ComboboxChip,
  ChipRemove: ComboboxChipRemove,
  Backdrop: ComboboxBackdrop,
  Portal: ComboboxPortal,
  Positioner: ComboboxPositioner,
  Popup: ComboboxPopup,
  Arrow: ComboboxArrow,
  Status: ComboboxStatus,
  Empty: ComboboxEmpty,
  List: ComboboxList,
  Row: ComboboxRow,
  Item: ComboboxItem,
  ItemIndicator: ComboboxItemIndicator,
  Group: ComboboxGroup,
  GroupLabel: ComboboxGroupLabel,
  Collection: ComboboxCollection,
  Separator: ComboboxSeparator,
});

/**
 * Free-form autocomplete.
 *
 * `inputValue` seeds the core's retained text; every edit after that belongs to the core, which
 * reports the exact value it holds through `onInputValueChange`.
 */
export function AutocompleteRoot(props: JSX.AutocompleteProps): NativeNode {
  const scope = props.scope ?? createComponentScope("qg-autocomplete");
  const context: PickerContextValue = {
    scope,
    selectState: () => settledSelectState,
    comboboxState: () => settledComboboxState,
    valueText: () => null,
    chips: () => [],
  };
  const input = createPartNode(
    "input",
    omit(
      props,
      "onInputValueChange",
      "onOpenChange",
      "onCommit",
      "items",
      "appearance",
      "children",
    ),
    {
      part: NativePart.Autocomplete,
      scope,
      get options() {
        return pickerOptionSource(props.items);
      },
      get appearance() {
        return encodePickerAppearance(props.appearance);
      },
      onComponentChange: componentChangeReader((details, event) => {
        if (details.inputValue !== undefined) {
          props.onInputValueChange?.(details.inputValue, event);
        }
        if (typeof details.open === "boolean") {
          props.onOpenChange?.(details.open, event);
        }
      }),
      onCommit: commitListener(props.onCommit),
    },
  );
  return PickerContext({
    value: context,
    get children() {
      return [input, props.children] as unknown as SolidElement;
    },
  }) as unknown as NativeNode;
}

/**
 * Base-UI-shaped compound parts for a free-form autocomplete.
 *
 * The core's `AutocompleteState` shares the combobox's owner-window parts, so the same `Label`,
 * `Value`, `Icon`, `InputGroup`, `Clear`, `Status`, and `Empty` components mount here. Chips,
 * `multiple`, `readOnly`, and `required` belong to the constrained combobox only.
 */
export const Autocomplete = Object.assign(AutocompleteRoot, {
  Root: AutocompleteRoot,
  /** Base UI's name for the declaration-carrying input; `Autocomplete.Root` is the same node. */
  Input: AutocompleteRoot,
  Option: SelectOption,
  Label: ComboboxLabel,
  Value: ComboboxValue,
  Icon: ComboboxIcon,
  InputGroup: ComboboxInputGroup,
  Clear: ComboboxClear,
  Portal: ComboboxPortal,
  Positioner: ComboboxPositioner,
  Popup: ComboboxPopup,
  Arrow: ComboboxArrow,
  Status: ComboboxStatus,
  Empty: ComboboxEmpty,
  List: ComboboxList,
  Item: ComboboxItem,
  ItemIndicator: ComboboxItemIndicator,
  Group: ComboboxGroup,
  GroupLabel: ComboboxGroupLabel,
  Collection: ComboboxCollection,
  Separator: ComboboxSeparator,
});

/** One declared table column. */
export interface TableColumnDeclaration {
  id: string;
  label?: string;
  /** Fixed starting width in logical pixels. A column with a width is user-resizable. */
  width?: number;
  minWidth?: number;
  align?: "start" | "center" | "end";
  sortable?: boolean;
  /** Project this column's cells as the row's accessible name. */
  rowHeader?: boolean;
  /** CSS grid track for a column that is not user-resizable, such as `1fr` or `auto`. */
  track?: string;
}

/** The range of rows the core is currently virtualizing. */
export interface VisibleRange {
  start: number;
  end: number;
}

/** The sort state the core retains for one table. */
export interface TableSortState {
  column: string;
  direction: "ascending" | "descending";
}

/** One inline-edit position. */
export interface TableCell {
  row: number;
  column: number;
}

/** The end of one inline edit, reported with the core's own commit decision. */
export interface TableEditEndDetails extends TableCell {
  committed: boolean;
}

function collectionChange(props: {
  onVisibleRangeChange?: (range: VisibleRange, event: QuickGuiEvent) => void;
}): (details: ComponentChangeDetails, event: QuickGuiEvent) => void {
  return (details, event) => {
    if (details.visibleRange) {
      props.onVisibleRangeChange?.(details.visibleRange, event);
    }
  };
}

/**
 * Virtual table root.
 *
 * The rows JavaScript declares are the rows the core last reported as visible, so a million-row
 * table declares only the window on screen. Selection, sort, widths, display order, and the
 * inline-edit lifetime are all decided by the core and reported asynchronously.
 */
export function TableRoot(props: JSX.TableProps): NativeNode {
  return createPartNode(
    "view",
    omit(
      props,
      "columns",
      "sort",
      "selection",
      "editing",
      "onVisibleRangeChange",
      "onSelectionChange",
      "onSortChange",
      "onActiveCellChange",
      "onColumnResize",
      "onColumnReorder",
      "onEditEnd",
      "onActivate",
    ),
    {
      part: NativePart.Table,
      get columns() {
        return props.columns ? props.columns.slice() : undefined;
      },
      get sortColumn() {
        return props.sort?.column;
      },
      get sortDirection() {
        return props.sort?.direction;
      },
      get selection() {
        return props.selection ? props.selection.map((range) => range.slice()) : undefined;
      },
      get editing() {
        return props.editing ? { ...props.editing } : undefined;
      },
      onComponentChange: componentChangeReader((details, event) => {
        collectionChange(props)(details, event);
        if (details.selectedRanges) {
          props.onSelectionChange?.(details.selectedRanges, event);
        }
        if (details.sort !== undefined) {
          props.onSortChange?.(details.sort ?? undefined, event);
        }
        if (details.activeCell !== undefined) {
          props.onActiveCellChange?.(details.activeCell ?? undefined, event);
        }
        if (details.columnWidths) {
          props.onColumnResize?.(details.columnWidths, event);
        }
        if (details.columnOrder) {
          props.onColumnReorder?.(details.columnOrder, event);
        }
        if (details.editEnded) props.onEditEnd?.(details.editEnded, event);
      }),
      onCommit: commitListener((details, event) => {
        if (details.row !== undefined && details.column !== undefined) {
          props.onActivate?.({ row: details.row, column: details.column }, event);
        }
      }),
    },
  );
}

/**
 * One declared column header.
 *
 * The core assigns the header's grid identity and appends its own resize handle, so this node
 * declares content and paint only.
 */
export function TableHeader(props: JSX.TableHeaderProps): NativeNode {
  return createPartNode("view", omit(props, "column"), {
    part: NativePart.TableHeader,
    get partValue() {
      return props.column;
    },
  });
}

/** One declared row inside the range the core reported as visible. */
export function TableRow(props: JSX.TableRowProps): NativeNode {
  return createPartNode("view", omit(props, "index"), {
    part: NativePart.TableRow,
    get rowIndex() {
      return props.index;
    },
  });
}

/** One declared cell. The core owns its identity, selection state, and editor key context. */
export function TableCellPart(props: JSX.TableCellProps): NativeNode {
  return createPartNode("view", omit(props, "column", "index"), {
    part: NativePart.TableCell,
    get partValue() {
      return props.column;
    },
    get columnIndex() {
      return props.index;
    },
  });
}

/** Base-UI-shaped compound parts for a virtual table. */
export const Table = Object.assign(TableRoot, {
  Root: TableRoot,
  Header: TableHeader,
  Row: TableRow,
  Cell: TableCellPart,
});

/** One declared tree node, which may declare its own children or be lazily pending. */
export interface TreeNodeDeclaration {
  id: string;
  label?: string;
  disabled?: boolean;
  /** A branch whose children are fetched the first time it is expanded. */
  pending?: boolean;
  children?: readonly TreeNodeDeclaration[];
}

/**
 * Virtual tree root.
 *
 * Expansion and selection are controlled declarations; the core owns arrow navigation, the
 * virtual window, and the lazy-children request a pending branch makes exactly once.
 */
export function TreeRoot(props: JSX.TreeProps): NativeNode {
  const [uncontrolled, setUncontrolled] = createSignal<readonly string[]>(
    untrack(() => props.defaultExpanded ?? []),
  );
  const expanded = () => props.expanded ?? uncontrolled();
  return createPartNode(
    "view",
    omit(
      props,
      "nodes",
      "expanded",
      "defaultExpanded",
      "value",
      "setChildren",
      "onExpandedChange",
      "onValueChange",
      "onLoadChildren",
      "onVisibleRangeChange",
      "onActivate",
    ),
    {
      part: NativePart.Tree,
      get nodes() {
        return props.nodes ? props.nodes.slice() : undefined;
      },
      get expanded() {
        return expanded().slice();
      },
      get selectedValue() {
        return props.value;
      },
      get setChildren() {
        return props.setChildren ? { ...props.setChildren } : undefined;
      },
      onComponentChange: componentChangeReader((details, event) => {
        collectionChange(props)(details, event);
        if (details.expanded) {
          if (props.expanded === undefined) setUncontrolled(details.expanded);
          props.onExpandedChange?.(details.expanded, event);
        }
        if (details.value !== undefined) {
          props.onValueChange?.(
            typeof details.value === "string" ? details.value : undefined,
            event,
          );
        }
        if (details.loadChildren) {
          props.onLoadChildren?.(details.loadChildren, event);
        }
      }),
      onCommit: commitListener((details, event) => {
        if (typeof details.value === "string") {
          props.onActivate?.(details.value, event);
        }
      }),
    },
  );
}

/**
 * One declared tree row inside the range the core reported as visible.
 *
 * The core hands the binding a behavior-only disclosure control for a branch; `disclosure` on the
 * root decides whether it is mounted before or after this content.
 */
export function TreeRow(props: JSX.TreeRowProps): NativeNode {
  return createPartNode("view", omit(props, "nodeId"), {
    part: NativePart.TreeRow,
    get partValue() {
      return props.nodeId;
    },
  });
}

/** Base-UI-shaped compound parts for a virtual tree. */
export const Tree = Object.assign(TreeRoot, {
  Root: TreeRoot,
  Row: TreeRow,
});

/**
 * Controlled number-field root.
 *
 * The core parses, clamps, formats, and steps; `value` seeds its retained editing text and the
 * result travels back through `onValueChange`.
 */
export function NumberFieldRoot(props: JSX.NumberFieldProps): NativeNode {
  const [uncontrolled, setUncontrolled] = createSignal<number | undefined>(
    untrack(() => props.defaultValue),
  );
  const [scrub, setScrubState] = createSignal<NumberFieldState>(settledNumberField);
  const value = () => props.value ?? uncontrolled();
  const context: NumberFieldContextValue = { state: scrub };
  return createPartNode(
    "view",
    omit(
      props,
      "value",
      "defaultValue",
      "onValueChange",
      "onValueCommitted",
      "scrubDirection",
      "scrubSensitivity",
      "children",
    ),
    {
      part: NativePart.NumberField,
      get values() {
        const current = value();
        return current === undefined ? [] : [current];
      },
      get smallStep() {
        return props.smallStep;
      },
      get largeStep() {
        return props.largeStep;
      },
      get snapOnStep() {
        return props.snapOnStep;
      },
      get allowWheelScrub() {
        return props.allowWheelScrub;
      },
      get readOnly() {
        return props.readOnly;
      },
      get required() {
        return props.required;
      },
      get orientation() {
        return props.scrubDirection;
      },
      get pitch() {
        return props.scrubSensitivity;
      },
      onComponentChange: componentChangeReader((details, event) => {
        if (details.value === undefined) return;
        const next = typeof details.value === "number" ? details.value : undefined;
        setScrubState({
          scrubbing: details.scrubbing === true,
          readOnly: details.readOnly === true,
          required: details.required === true,
        });
        if (props.value === undefined) setUncontrolled(next);
        props.onValueChange?.(next, details.valid !== false, event);
        // The core's own commit boundary: the value it clamped and reformatted.
        if (details.committed === true) props.onValueCommitted?.(next, event);
      }),
      get children() {
        return NumberFieldContext({
          value: context,
          get children() {
            return props.children as SolidElement;
          },
        });
      },
    },
  );
}

/** Everything the core decided about one number field. */
export interface NumberFieldState {
  /** Whether a scrub gesture is in flight, matching Base UI's `data-scrubbing`. */
  scrubbing: boolean;
  readOnly: boolean;
  required: boolean;
}

const settledNumberField: NumberFieldState = {
  scrubbing: false,
  readOnly: false,
  required: false,
};

interface NumberFieldContextValue {
  state: () => NumberFieldState;
}

const NumberFieldContext = createContext<NumberFieldContextValue | null>(null);

/** Read the live number-field state inside a `NumberField.Root` subtree. */
export function useNumberFieldState(): () => NumberFieldState {
  const context = optionalContext(NumberFieldContext);
  return context ? context.state : () => settledNumberField;
}

/** Structural group the input and steppers are laid out inside. */
export function NumberFieldGroup(props: JSX.NativeScopedProps): NativeNode {
  return createPartNode("view", props, { part: NativePart.NumberFieldGroup });
}

/**
 * Scrub area.
 *
 * The core turns the captured drag into whole steps at the declared sensitivity and keeps the
 * unconverted remainder for the gesture, so a slow drag moves one step at a time.
 */
export function NumberFieldScrubArea(props: JSX.NativeScopedProps): NativeNode {
  return createPartNode("view", props, {
    part: NativePart.NumberFieldScrubArea,
  });
}

/** Caller-drawn scrub cursor. Style it from `useNumberFieldState().scrubbing`. */
export function NumberFieldScrubAreaCursor(props: JSX.NativeScopedProps): NativeNode {
  return createPartNode("view", props, {
    part: NativePart.NumberFieldScrubAreaCursor,
  });
}

/** Controlled number-field input. The core owns its editing text, parsing, and commit. */
export function NumberFieldInput(props: JSX.NumberFieldInputProps): NativeNode {
  return createPartNode("input", omit(props, "onCommit"), {
    part: NativePart.NumberFieldInput,
    onCommit: commitListener(props.onCommit),
  });
}

/** Application-owned increment stepper carrying the core's bounded press-and-hold repeat. */
export function NumberFieldIncrement(props: JSX.NativeScopedProps): NativeNode {
  return createPartNode("button", props, {
    part: NativePart.NumberFieldIncrement,
  });
}

/** Application-owned decrement stepper carrying the core's bounded press-and-hold repeat. */
export function NumberFieldDecrement(props: JSX.NativeScopedProps): NativeNode {
  return createPartNode("button", props, {
    part: NativePart.NumberFieldDecrement,
  });
}

/** Base-UI-shaped compound parts for a number field. */
export const NumberField = Object.assign(NumberFieldRoot, {
  Root: NumberFieldRoot,
  Group: NumberFieldGroup,
  Input: NumberFieldInput,
  Increment: NumberFieldIncrement,
  Decrement: NumberFieldDecrement,
  ScrubArea: NumberFieldScrubArea,
  ScrubAreaCursor: NumberFieldScrubAreaCursor,
});

/** One queued toast. Pushing a toast is adding an entry to this declared list. */
export interface ToastDeclaration {
  id: string;
  title: string;
  description?: string;
  action?: string;
  /** Base UI's own name for the toast kind. */
  type?: ToastType;
  /** QuickGUI's original name for `type`. Either one reaches the same core kind. */
  kind?: ToastType;
  /** Auto-dismiss duration in milliseconds. Omit for a toast that stays until dismissed. */
  duration?: number;
}

/** A Base UI-shaped manager over the declared toast list. */
export interface ToastManager {
  /** The declared queue, which is the source of truth for what is pushed. */
  toasts: () => readonly ToastDeclaration[];
  /** What the core decided about the queue: stack index, offset, limited and expanded flags. */
  stack: () => readonly ToastStackEntry[];
  /** Push one toast and return its identifier. */
  add: (toast: Omit<ToastDeclaration, "id"> & { id?: string }) => string;
  /** Replace one queued toast in place, keeping its identity and stack position. */
  update: (id: string, toast: Partial<Omit<ToastDeclaration, "id">>) => void;
  /** Drop one toast from the declaration, which dismisses it. */
  close: (id: string) => void;
  /** Drop every toast. */
  closeAll: () => void;
  /**
   * Queue a persistent loading toast and turn it into its result.
   *
   * QuickGUI owns no future, so the application drives both halves from the task it already
   * spawned; the toast keeps the same identity and stack position across the transition.
   */
  promise: <T>(
    work: Promise<T>,
    messages: {
      loading: string;
      success: string | ((value: T) => string);
      error: string | ((reason: unknown) => string);
    },
  ) => Promise<T>;
}

interface ToastContextValue extends ToastManager {
  scope: string;
  timeout: () => number | undefined;
  limit: () => number | undefined;
  expanded: () => boolean | undefined;
  swipeDirection: () => "left" | "right" | "up" | "down" | undefined;
  pitch: () => number | undefined;
  reportStack: (stack: readonly ToastStackEntry[]) => void;
}

const ToastContext = createContext<ToastContextValue | null>(null);

let nextToastId = 1;

/**
 * Toast provider.
 *
 * It owns the declared queue and the provider-level props Base UI puts here — the inherited
 * auto-dismiss `timeout`, the visible stack `limit`, the `expanded` stack, and the swipe
 * contract — and creates no native element of its own.
 */
export function ToastProvider(props: JSX.ToastProviderProps): NativeNode {
  const [queue, setQueue] = createSignal<readonly ToastDeclaration[]>([]);
  const [stack, setStack] = createSignal<readonly ToastStackEntry[]>([]);
  const context: ToastContextValue = {
    scope: createComponentScope("qg-toast"),
    toasts: queue,
    stack,
    reportStack: setStack,
    timeout: () => props.timeout,
    limit: () => props.limit,
    expanded: () => props.expanded,
    swipeDirection: () => props.swipeDirection,
    pitch: () => props.pitch,
    add(toast) {
      const id = toast.id ?? `qg-toast-${nextToastId++}`;
      setQueue((current) => [...current, { ...toast, id }]);
      return id;
    },
    update(id, toast) {
      setQueue((current) =>
        current.map((entry) => (entry.id === id ? { ...entry, ...toast, id } : entry)),
      );
    },
    close(id) {
      setQueue((current) => current.filter((entry) => entry.id !== id));
    },
    closeAll() {
      setQueue([]);
    },
    async promise(work, messages) {
      const id = context.add({
        title: messages.loading,
        type: "loading",
      });
      try {
        const value = await work;
        context.update(id, {
          title:
            typeof messages.success === "function" ? messages.success(value) : messages.success,
          type: "success",
        });
        return value;
      } catch (reason) {
        context.update(id, {
          title: typeof messages.error === "function" ? messages.error(reason) : messages.error,
          type: "error",
        });
        throw reason;
      } finally {
        flushSolid();
      }
    },
  };
  return ToastContext({
    value: context,
    get children() {
      return props.children as SolidElement;
    },
  }) as unknown as NativeNode;
}

/**
 * Read the toast manager inside a `Toast.Provider` subtree.
 *
 * `add`, `update`, `close`, `closeAll`, and `promise` all change the declared list; the core owns
 * the queue bound, the live-region politeness, the exact auto-dismiss deadline, the stack index
 * each toast is at, and the swipe arithmetic, and reports all of it back through `stack()`.
 */
export function useToastManager(): ToastManager {
  const context = optionalContext(ToastContext);
  if (!context) {
    throw new TypeError("useToastManager must be called inside <Toast.Provider>");
  }
  return context;
}

/** Portal boundary above the window's own content. */
export function ToastPortal(props: JSX.NativeScopedProps): NativeNode {
  const context = optionalContext(ToastContext);
  return createPartNode("view", props, {
    part: NativePart.ToastPortal,
    get scope() {
      return props.scope ?? context?.scope;
    },
  });
}

/** One toast positioner, offset by the core's own `index * pitch`. */
export function ToastPositioner(props: JSX.ToastProps): NativeNode {
  const context = optionalContext(ToastContext);
  return createPartNode("view", omit(props, "toastId"), {
    part: NativePart.ToastPositioner,
    get scope() {
      return props.scope ?? context?.scope;
    },
    get partValue() {
      return props.toastId;
    },
  });
}

/** One toast content box. */
export function ToastContent(props: JSX.ToastProps): NativeNode {
  const context = optionalContext(ToastContext);
  return createPartNode("view", omit(props, "toastId"), {
    part: NativePart.ToastContent,
    get scope() {
      return props.scope ?? context?.scope;
    },
    get partValue() {
      return props.toastId;
    },
  });
}

/**
 * Toast viewport.
 *
 * The queue is a declaration: adding an identifier pushes a toast, dropping one dismisses it, and
 * the core's own bounded queue reports every dismissal — including the timed ones — through
 * `onDismiss`.
 */
export function ToastViewport(props: JSX.ToastViewportProps): NativeNode {
  const context = optionalContext(ToastContext);
  return createPartNode("view", omit(props, "toasts", "onDismiss", "onStackChange"), {
    part: NativePart.ToastViewport,
    get scope() {
      return props.scope ?? context?.scope;
    },
    get toasts() {
      const declared = props.toasts ?? context?.toasts();
      return declared ? declared.slice() : undefined;
    },
    get timeout() {
      return props.timeout ?? context?.timeout();
    },
    get limit() {
      return props.limit ?? context?.limit();
    },
    get stackExpanded() {
      return props.expanded ?? context?.expanded();
    },
    get swipeDirection() {
      return props.swipeDirection ?? context?.swipeDirection();
    },
    get pitch() {
      return props.pitch ?? context?.pitch();
    },
    onComponentChange: componentChangeReader((details, event) => {
      if (details.dismissed) {
        for (const id of details.dismissed) context?.close(id);
        props.onDismiss?.(details.dismissed, event);
      }
      if (details.toasts) {
        context?.reportStack(details.toasts);
        props.onStackChange?.(details.toasts, event);
      }
    }),
  });
}

/** One queued toast root, projecting the live-region politeness its kind selects. */
export function ToastRoot(props: JSX.ToastProps): NativeNode {
  const context = optionalContext(ToastContext);
  return createPartNode("view", omit(props, "toastId"), {
    part: NativePart.Toast,
    get scope() {
      return props.scope ?? context?.scope;
    },
    get partValue() {
      return props.toastId;
    },
  });
}

/** One toast title, which names the toast for assistive technology. */
export function ToastTitle(props: JSX.ToastProps): NativeNode {
  const context = optionalContext(ToastContext);
  return createPartNode("view", omit(props, "toastId"), {
    part: NativePart.ToastTitle,
    get scope() {
      return props.scope ?? context?.scope;
    },
    get partValue() {
      return props.toastId;
    },
  });
}

/** One toast description, which describes the toast for assistive technology. */
export function ToastDescription(props: JSX.ToastProps): NativeNode {
  const context = optionalContext(ToastContext);
  return createPartNode("view", omit(props, "toastId"), {
    part: NativePart.ToastDescription,
    get scope() {
      return props.scope ?? context?.scope;
    },
    get partValue() {
      return props.toastId;
    },
  });
}

/** One toast action control. */
export function ToastAction(props: JSX.ToastProps): NativeNode {
  const context = optionalContext(ToastContext);
  return createPartNode("button", omit(props, "toastId"), {
    part: NativePart.ToastAction,
    get scope() {
      return props.scope ?? context?.scope;
    },
    get partValue() {
      return props.toastId;
    },
  });
}

/** One toast close control. Pressing it dismisses the toast through the core's own queue. */
export function ToastClose(props: JSX.ToastProps): NativeNode {
  const context = optionalContext(ToastContext);
  return createPartNode("button", omit(props, "toastId"), {
    part: NativePart.ToastClose,
    get scope() {
      return props.scope ?? context?.scope;
    },
    get partValue() {
      return props.toastId;
    },
  });
}

// ---------------------------------------------------------------------------
// Tooltip
//
// The compound tooltip sits on ordinary caller-owned elements. `Element.tooltip` — the `tooltip`
// prop every native node already accepts — stays the shortest path to a native-style hint; this
// is the composable one, with a shared warm provider, cursor tracking, and a resolved-placement
// arrow. Every deadline belongs to the core.
// ---------------------------------------------------------------------------

export type TooltipCursorAxis = "none" | "x" | "y" | "both";

interface TooltipProviderContextValue {
  scope: string;
}

const TooltipProviderContext = createContext<TooltipProviderContextValue | null>(null);

interface TooltipContextValue {
  scope: string;
  provider: string | undefined;
  open: () => boolean;
  placement: () => AnchorPlacementDetails;
  reportPlacement: (next: AnchorPlacementDetails, event: QuickGuiEvent) => void;
  adoptOpen: (open: boolean, event: QuickGuiEvent) => void;
  disabled: () => boolean | undefined;
  hoverable: () => boolean | undefined;
  trackCursorAxis: () => TooltipCursorAxis | undefined;
  positioning: () => AnchorPositioning;
  declarePositioning: (positioning: AnchorPositioning) => void;
}

const TooltipContext = createContext<TooltipContextValue | null>(null);

function requireTooltip(component: string): TooltipContextValue {
  const context = optionalContext(TooltipContext);
  if (!context) {
    throw new TypeError(`<${component}> must be rendered inside <Tooltip.Root>`);
  }
  return context;
}

/**
 * Shared warm group.
 *
 * Once one tooltip in the group has opened, an adjacent trigger opens instantly while the group
 * stays warm; that warm window is itself one exact core deadline, so a settled group owns no task
 * or timer. Unlike Base UI's DOM-less provider this is one ordinary element, which is also where
 * the group's deadlines are declared.
 */
export function TooltipProvider(props: JSX.TooltipProviderProps): NativeNode {
  const scope = createComponentScope("qg-tooltip-provider");
  const context: TooltipProviderContextValue = { scope };
  return createPartNode("view", omit(props, "children"), {
    part: NativePart.TooltipProvider,
    scope,
    get delay() {
      return props.delay;
    },
    get closeDelay() {
      return props.closeDelay;
    },
    get timeout() {
      return props.timeout;
    },
    get children() {
      return TooltipProviderContext({
        value: context,
        get children() {
          return props.children as SolidElement;
        },
      });
    },
  });
}

/** Logical tooltip root. It creates no native element of its own. */
export function TooltipRoot(props: JSX.TooltipRootProps): NativeNode {
  const provider = optionalContext(TooltipProviderContext);
  const [uncontrolled, setUncontrolled] = createSignal(untrack(() => props.defaultOpen ?? false));
  const [placement, setPlacement] = createSignal<AnchorPlacementDetails>(unresolvedPlacement);
  const [declared, setDeclared] = createSignal<AnchorPositioning>({}, { ownedWrite: true });
  const open = () => props.open ?? uncontrolled();
  const context: TooltipContextValue = {
    scope: createComponentScope("qg-tooltip"),
    provider: provider?.scope,
    open,
    placement,
    reportPlacement(next, event) {
      setPlacement(next);
      props.onPlacementChange?.(next, event);
    },
    adoptOpen(next, event) {
      if (next === open()) return;
      if (props.open === undefined) setUncontrolled(next);
      props.onOpenChange?.(next, event);
    },
    disabled: () => props.disabled,
    hoverable: () => props.hoverable,
    trackCursorAxis: () => props.trackCursorAxis,
    positioning: () => {
      const override = declared();
      return {
        side: override.side ?? props.side,
        align: override.align ?? props.align,
        sideOffset: override.sideOffset ?? props.sideOffset,
        collisionPadding: override.collisionPadding ?? props.collisionPadding,
      };
    },
    declarePositioning: setDeclared,
  };
  return TooltipContext({
    value: context,
    get children() {
      return props.children as SolidElement;
    },
  }) as unknown as NativeNode;
}

/** Read the placement the core really resolved to inside a `Tooltip.Root` subtree. */
export function useTooltipPlacement(): () => AnchorPlacementDetails {
  return requireTooltip("useTooltipPlacement").placement;
}

/**
 * Tooltip trigger.
 *
 * This is the one part the core keeps mounted whether the tooltip is open or closed, so it
 * carries the whole declaration and reports back what the core decided.
 */
export function TooltipTrigger(props: JSX.TooltipTriggerProps): NativeNode {
  const context = requireTooltip("Tooltip.Trigger");
  const positioning = () => context.positioning();
  return createPartNode(
    props.element ?? "button",
    omit(props, "element", "delay", "closeDelay", "closeOnClick"),
    {
      part: NativePart.TooltipTrigger,
      scope: context.scope,
      get provider() {
        return context.provider;
      },
      get open() {
        return context.open();
      },
      get disabled() {
        return props.disabled ?? context.disabled();
      },
      get hoverable() {
        return context.hoverable();
      },
      get trackCursorAxis() {
        return context.trackCursorAxis();
      },
      get delay() {
        return props.delay;
      },
      get closeDelay() {
        return props.closeDelay;
      },
      get closeOnClick() {
        return props.closeOnClick;
      },
      get side() {
        return positioning().side;
      },
      get align() {
        return positioning().align;
      },
      get sideOffset() {
        return positioning().sideOffset;
      },
      get collisionPadding() {
        return positioning().collisionPadding;
      },
      onComponentChange: componentChangeReader((details, event) => {
        const placement = placementFromDetails(details);
        if (placement) context.reportPlacement(placement, event);
        if (typeof details.open === "boolean") context.adoptOpen(details.open, event);
      }),
    },
  );
}

/** Tooltip positioner. Base UI declares the placement props here. */
export function TooltipPositioner(props: JSX.TooltipPositionerProps): NativeNode {
  const context = requireTooltip("Tooltip.Positioner");
  // The positioner is unmounted while the tooltip is closed, so its declaration is routed to the
  // trigger, which the core keeps mounted either way.
  context.declarePositioning({
    get side() {
      return props.side;
    },
    get align() {
      return props.align;
    },
    get sideOffset() {
      return props.sideOffset;
    },
    get collisionPadding() {
      return props.collisionPadding;
    },
  } as AnchorPositioning);
  onCleanup(() => context.declarePositioning({}));
  return createPartNode("view", omit(props, "side", "align", "sideOffset", "collisionPadding"), {
    part: NativePart.TooltipPositioner,
    scope: context.scope,
  });
}

/** Portal boundary. QuickGUI's retained overlay node is the portal, so it is the positioner. */
export function TooltipPortal(props: JSX.NativeProps): NativeNode {
  const context = requireTooltip("Tooltip.Portal");
  return createPartNode("view", props, {
    part: NativePart.TooltipPortal,
    scope: context.scope,
  });
}

/** The tooltip surface. Escape dismissal belongs to the core, so it declares no `onDismiss`. */
export function TooltipPopup(props: JSX.NativeProps): NativeNode {
  const context = requireTooltip("Tooltip.Popup");
  return createPartNode("view", props, {
    part: NativePart.TooltipPopup,
    scope: context.scope,
  });
}

/** Arrow pinned to the popup edge that really faces the trigger. */
export function TooltipArrow(props: JSX.NativeProps): NativeNode {
  const context = requireTooltip("Tooltip.Arrow");
  return createPartNode("view", props, {
    part: NativePart.TooltipArrow,
    scope: context.scope,
  });
}

/** Base-UI-shaped compound parts for a tooltip. */
export const Tooltip = Object.assign(TooltipRoot, {
  Provider: TooltipProvider,
  Root: TooltipRoot,
  Trigger: TooltipTrigger,
  Portal: TooltipPortal,
  Positioner: TooltipPositioner,
  Popup: TooltipPopup,
  Arrow: TooltipArrow,
});

/** Base-UI-shaped compound parts for a toast viewport. */
export const Toast = Object.assign(ToastRoot, {
  Provider: ToastProvider,
  Portal: ToastPortal,
  Viewport: ToastViewport,
  Positioner: ToastPositioner,
  Root: ToastRoot,
  Content: ToastContent,
  Title: ToastTitle,
  Description: ToastDescription,
  Action: ToastAction,
  Close: ToastClose,
});

/**
 * Controlled date field.
 *
 * `value`, `min`, and `max` are ISO `YYYY-MM-DD` civil values with no time zone. The core owns
 * segment arithmetic, digit entry, leap years, and the field's validity.
 */
export function DateFieldRoot(props: JSX.DateFieldProps): NativeNode {
  const [uncontrolled, setUncontrolled] = createSignal<string | undefined>(
    untrack(() => props.defaultValue),
  );
  const value = () => props.value ?? uncontrolled();
  return createPartNode(
    "view",
    omit(props, "value", "defaultValue", "min", "max", "format", "onValueChange"),
    {
      part: NativePart.DateField,
      get civilValue() {
        return value();
      },
      get civilMinimum() {
        return props.min;
      },
      get civilMaximum() {
        return props.max;
      },
      get segmentOrder() {
        return props.format;
      },
      onComponentChange: componentChangeReader((details, event) => {
        if (details.value === undefined) return;
        const next = typeof details.value === "string" ? details.value : undefined;
        if (props.value === undefined) setUncontrolled(next);
        props.onValueChange?.(next, event);
      }),
    },
  );
}

/** One date-field segment carrying the core's spin-button semantics and typed actions. */
export function DateFieldSegment(props: JSX.DateFieldSegmentProps): NativeNode {
  return createPartNode("view", props, { part: NativePart.DateFieldSegment });
}

/** Base-UI-shaped compound parts for a date field. */
export const DateField = Object.assign(DateFieldRoot, {
  Root: DateFieldRoot,
  Segment: DateFieldSegment,
});

/**
 * Controlled time field.
 *
 * `value`, `min`, and `max` are `HH:MM` or `HH:MM:SS` civil times. A twelve-hour field gains an
 * AM/PM segment, and a field without `showSeconds` mounts no seconds segment at all.
 */
export function TimeFieldRoot(props: JSX.TimeFieldProps): NativeNode {
  const [uncontrolled, setUncontrolled] = createSignal<string | undefined>(
    untrack(() => props.defaultValue),
  );
  const value = () => props.value ?? uncontrolled();
  return createPartNode(
    "view",
    omit(props, "value", "defaultValue", "min", "max", "hour12", "showSeconds", "onValueChange"),
    {
      part: NativePart.TimeField,
      get civilValue() {
        return value();
      },
      get civilMinimum() {
        return props.min;
      },
      get civilMaximum() {
        return props.max;
      },
      get segmentOrder() {
        return props.hour12 ? "h12" : "h23";
      },
      get variant() {
        return props.showSeconds ? "seconds" : undefined;
      },
      onComponentChange: componentChangeReader((details, event) => {
        if (details.value === undefined) return;
        const next = typeof details.value === "string" ? details.value : undefined;
        if (props.value === undefined) setUncontrolled(next);
        props.onValueChange?.(next, event);
      }),
    },
  );
}

/** One time-field segment carrying the core's spin-button semantics and typed actions. */
export function TimeFieldSegment(props: JSX.TimeFieldSegmentProps): NativeNode {
  return createPartNode("view", props, { part: NativePart.TimeFieldSegment });
}

/** Base-UI-shaped compound parts for a time field. */
export const TimeField = Object.assign(TimeFieldRoot, {
  Root: TimeFieldRoot,
  Segment: TimeFieldSegment,
});

/**
 * Controlled month grid.
 *
 * The core owns day, week, month, and year movement, the single Tab stop, and selectability
 * inside the declared civil bounds. `onFocusChange` reports the day the grid moved to.
 */
export function CalendarRoot(props: JSX.CalendarProps): NativeNode {
  const [uncontrolled, setUncontrolled] = createSignal<string | undefined>(
    untrack(() => props.defaultValue),
  );
  const value = () => props.value ?? uncontrolled();
  return createPartNode(
    "view",
    omit(
      props,
      "value",
      "defaultValue",
      "min",
      "max",
      "onValueChange",
      "onFocusChange",
      "onMonthChange",
    ),
    {
      part: NativePart.Calendar,
      get civilValue() {
        return value();
      },
      get civilMinimum() {
        return props.min;
      },
      get civilMaximum() {
        return props.max;
      },
      onComponentChange: componentChangeReader((details, event) => {
        if (details.value !== undefined) {
          const next = typeof details.value === "string" ? details.value : undefined;
          if (props.value === undefined) setUncontrolled(next);
          props.onValueChange?.(next, event);
        }
        if (typeof details.focused === "string") {
          props.onFocusChange?.(details.focused, event);
        }
        if (details.month) props.onMonthChange?.(details.month, event);
      }),
    },
  );
}

/** One calendar week row. */
export function CalendarWeek(props: JSX.CalendarWeekProps): NativeNode {
  return createPartNode("view", props, { part: NativePart.CalendarWeek });
}

/** One calendar day cell, named by its ISO `YYYY-MM-DD` civil date. */
export function CalendarDay(props: JSX.CalendarDayProps): NativeNode {
  return createPartNode("view", omit(props, "day"), {
    part: NativePart.CalendarDay,
    get civilValue() {
      return props.day;
    },
  });
}

/** Base-UI-shaped compound parts for a month grid. */
export const Calendar = Object.assign(CalendarRoot, {
  Root: CalendarRoot,
  Week: CalendarWeek,
  Day: CalendarDay,
});

/**
 * In-window menubar.
 *
 * The bar owns which menu is open and which one holds its single Tab stop; each menu's surface is
 * an ordinary declared `PopoverMenu` anchored to the matching `Menubar.Item`.
 */
export function MenubarRoot(props: JSX.MenubarProps): NativeNode {
  const [uncontrolled, setUncontrolled] = createSignal<number | undefined>(
    untrack(() => props.defaultOpen),
  );
  const open = () => (props.open === undefined ? uncontrolled() : (props.open ?? undefined));
  return createPartNode(
    "view",
    omit(props, "open", "defaultOpen", "count", "onOpenChange", "onActiveChange"),
    {
      part: NativePart.Menubar,
      get menuCount() {
        return props.count;
      },
      get open() {
        return open() !== undefined;
      },
      get itemIndex() {
        return open();
      },
      onComponentChange: componentChangeReader((details, event) => {
        if (details.open !== undefined) {
          const next = typeof details.open === "number" ? details.open : undefined;
          if (props.open === undefined) setUncontrolled(next);
          props.onOpenChange?.(next, event);
        }
        if (details.focused !== undefined && typeof details.focused === "number") {
          props.onActiveChange?.(details.focused, event);
        }
      }),
    },
  );
}

/** One menubar trigger. Exactly one trigger carries the bar's Tab stop. */
export function MenubarItem(props: JSX.MenubarItemProps): NativeNode {
  return createPartNode("button", props, { part: NativePart.MenubarItem });
}

/** Base-UI-shaped compound parts for an in-window menubar. */
export const Menubar = Object.assign(MenubarRoot, {
  Root: MenubarRoot,
  Item: MenubarItem,
});

// ---------------------------------------------------------------------------
// Base UI parity components
//
// Separators, avatars, checkbox groups, preview cards, scroll areas, OTP fields, drawers, and
// navigation menus. Every one of these declares the Rust core's own compound parts and reads the
// result back from the asynchronous `componentchange` event; none of them reimplements a delay, a
// deadline, a focus rule, a mount policy, or a keyboard contract in JavaScript.
// ---------------------------------------------------------------------------

/** Unstyled semantic separator. The caller still declares the rule's extent and colour. */
export function SeparatorRoot(props: JSX.SeparatorProps): NativeNode {
  return createPartNode("view", props, { part: NativePart.Separator });
}

/** Base-UI-shaped compound parts for a separator. */
export const Separator = Object.assign(SeparatorRoot, { Root: SeparatorRoot });

/** The load state the Rust core retains for one avatar, matching Base UI's own values. */
export type AvatarLoadingStatus = "idle" | "loading" | "loaded" | "error";

interface AvatarContextValue {
  scope: string;
}

const AvatarContext = createContext<AvatarContextValue | null>(null);

function requireAvatar(component: string): AvatarContextValue {
  const context = optionalContext(AvatarContext);
  if (!context) {
    throw new Error(`<${component}> must be rendered inside <Avatar.Root>`);
  }
  return context;
}

/**
 * Avatar root carrying the whole avatar's Image role and accessible name.
 *
 * The core decides which of the image and the fallback is mounted, so swapping between them never
 * changes what assistive technology announces. The binding drives the retained status from the
 * declared image source's own load outcome and reports every transition through
 * `onLoadingStatusChange`.
 */
export function AvatarRoot(props: JSX.AvatarRootProps): NativeNode {
  const scope = createComponentScope("qg-avatar");
  const context: AvatarContextValue = { scope };
  return createPartNode("view", omit(props, "onLoadingStatusChange", "children"), {
    part: NativePart.Avatar,
    scope,
    onComponentChange: componentChangeListener(
      (details) => details.loadingStatus,
      (next, event) => props.onLoadingStatusChange?.(next, event),
    ),
    get children() {
      return AvatarContext({
        value: context,
        get children() {
          return props.children as SolidElement;
        },
      });
    },
  });
}

/** Avatar image, mounted by the core only once its declared source has loaded. */
export function AvatarImage(props: JSX.AvatarImageProps): NativeNode {
  const context = requireAvatar("Avatar.Image");
  return createPartNode("image", omit(props, "src"), {
    part: NativePart.AvatarImage,
    scope: context.scope,
    get source() {
      return props.src;
    },
  });
}

/** Avatar fallback, held back for `delay` so a fast decode never flashes initials. */
export function AvatarFallback(props: JSX.AvatarFallbackProps): NativeNode {
  const context = requireAvatar("Avatar.Fallback");
  return createPartNode("view", props, {
    part: NativePart.AvatarFallback,
    scope: context.scope,
  });
}

/** Base-UI-shaped compound parts for an avatar. */
export const Avatar = Object.assign(AvatarRoot, {
  Root: AvatarRoot,
  Image: AvatarImage,
  Fallback: AvatarFallback,
});

interface CheckboxGroupContextValue {
  scope: string;
}

const CheckboxGroupContext = createContext<CheckboxGroupContextValue | null>(null);

/**
 * Controlled checkbox group.
 *
 * `allValues` declares the complete, ordered universe the parent checkbox derives its on/mixed/off
 * state from; the core keeps checked values in that declared order regardless of click order and
 * refuses every mutation while the group is disabled.
 */
export function CheckboxGroupRoot(props: JSX.CheckboxGroupProps): NativeNode {
  const scope = createComponentScope("qg-checkbox-group");
  const [uncontrolled, setUncontrolled] = createSignal<readonly string[]>(
    untrack(() => props.defaultValue ?? []),
  );
  const values = () => props.value ?? uncontrolled();
  const context: CheckboxGroupContextValue = { scope };
  return createPartNode(
    "view",
    omit(props, "value", "defaultValue", "onValueChange", "allValues", "children"),
    {
      part: NativePart.CheckboxGroup,
      scope,
      get values() {
        return values().slice();
      },
      get items() {
        return props.allValues ? props.allValues.slice() : undefined;
      },
      onComponentChange: componentChangeListener(
        (details) => details.checkedValues,
        (next, event) => {
          if (props.value === undefined) setUncontrolled(next);
          props.onValueChange?.(next, event);
        },
      ),
      get children() {
        return CheckboxGroupContext({
          value: context,
          get children() {
            return props.children as SolidElement;
          },
        });
      },
    },
  );
}

/** Base-UI-shaped compound parts for a checkbox group. */
export const CheckboxGroup = Object.assign(CheckboxGroupRoot, {
  Root: CheckboxGroupRoot,
});

interface PreviewCardContextValue {
  scope: string;
  open: () => boolean;
  setOpen: (next: boolean, event: QuickGuiEvent) => void;
}

const PreviewCardContext = createContext<PreviewCardContextValue | null>(null);

function requirePreviewCard(component: string): PreviewCardContextValue {
  const context = optionalContext(PreviewCardContext);
  if (!context) {
    throw new Error(`<${component}> must be rendered inside <PreviewCard.Root>`);
  }
  return context;
}

/**
 * Preview-card root owning the two exact deadlines a hover card needs.
 *
 * The core opens after `delay` once the pointer rests on the trigger, closes after `closeDelay`
 * once it has left both the trigger and the popup, and opens immediately on focus.
 */
export function PreviewCardRoot(props: JSX.PreviewCardRootProps): NativeNode {
  const scope = createComponentScope("qg-preview-card");
  const [uncontrolled, setUncontrolled] = createSignal(untrack(() => props.defaultOpen ?? false));
  const open = () => props.open ?? uncontrolled();
  const context: PreviewCardContextValue = {
    scope,
    open,
    setOpen(next, event) {
      if (props.open === undefined) setUncontrolled(next);
      props.onOpenChange?.(next, event);
    },
  };
  return createPartNode(
    "view",
    omit(
      props,
      "open",
      "defaultOpen",
      "onOpenChange",
      "placement",
      "gap",
      "viewportMargin",
      "children",
    ),
    {
      part: NativePart.PreviewCard,
      scope,
      get open() {
        return open();
      },
      get anchorPlacement() {
        return props.placement;
      },
      get anchorGap() {
        return props.gap;
      },
      get viewportMargin() {
        return props.viewportMargin;
      },
      onComponentChange: componentChangeListener(
        (details) => details.open,
        (next, event) => {
          if (typeof next !== "boolean") return;
          context.setOpen(next, event);
        },
      ),
      get children() {
        return PreviewCardContext({
          value: context,
          get children() {
            return props.children as SolidElement;
          },
        });
      },
    },
  );
}

/** Link-like trigger. The core owns the hover deadlines declared here. */
export function PreviewCardTrigger(props: JSX.PreviewCardTriggerProps): NativeNode {
  const context = requirePreviewCard("PreviewCard.Trigger");
  return createPartNode("button", props, {
    part: NativePart.PreviewCardTrigger,
    scope: context.scope,
  });
}

/** Portal boundary. QuickGUI's retained overlay node is itself the portal. */
export function PreviewCardPortal(props: JSX.NativeProps): NativeNode {
  const context = requirePreviewCard("PreviewCard.Portal");
  return createPartNode("view", props, {
    part: NativePart.PreviewCardPortal,
    scope: context.scope,
  });
}

/** Positioner. Mount either this or the portal, never both. */
export function PreviewCardPositioner(props: JSX.NativeProps): NativeNode {
  const context = requirePreviewCard("PreviewCard.Positioner");
  return createPartNode("view", props, {
    part: NativePart.PreviewCardPositioner,
    scope: context.scope,
  });
}

/** Popup surface. Escape and outside presses dismiss it through the core. */
export function PreviewCardPopup(props: JSX.NativeProps): NativeNode {
  const context = requirePreviewCard("PreviewCard.Popup");
  return createPartNode("view", props, {
    part: NativePart.PreviewCardPopup,
    scope: context.scope,
  });
}

/** Decorative arrow, hidden from assistive technology by the core. */
export function PreviewCardArrow(props: JSX.NativeProps): NativeNode {
  const context = requirePreviewCard("PreviewCard.Arrow");
  return createPartNode("view", props, {
    part: NativePart.PreviewCardArrow,
    scope: context.scope,
  });
}

/** Optional caller-painted viewport backdrop. */
export function PreviewCardBackdrop(props: JSX.NativeProps): NativeNode {
  const context = requirePreviewCard("PreviewCard.Backdrop");
  return createPartNode("view", props, {
    part: NativePart.PreviewCardBackdrop,
    scope: context.scope,
  });
}

/** Base-UI-shaped compound parts for a preview card. */
export const PreviewCard = Object.assign(PreviewCardRoot, {
  Root: PreviewCardRoot,
  Trigger: PreviewCardTrigger,
  Portal: PreviewCardPortal,
  Backdrop: PreviewCardBackdrop,
  Positioner: PreviewCardPositioner,
  Popup: PreviewCardPopup,
  Arrow: PreviewCardArrow,
});

/** The `data-`-like render state one scroll area reports for the application to style from. */
export interface ScrollAreaState {
  /** Offset the core clamped into the scrollable range. */
  offset: { x: number; y: number };
  scrolling: boolean;
  hovering: boolean;
  hasOverflowX: boolean;
  hasOverflowY: boolean;
  overflowXStart: boolean;
  overflowXEnd: boolean;
  overflowYStart: boolean;
  overflowYEnd: boolean;
}

interface ScrollAreaContextValue {
  scope: string;
  state: () => ScrollAreaState;
}

const ScrollAreaContext = createContext<ScrollAreaContextValue | null>(null);

function requireScrollArea(component: string): ScrollAreaContextValue {
  const context = optionalContext(ScrollAreaContext);
  if (!context) {
    throw new Error(`<${component}> must be rendered inside <ScrollArea.Root>`);
  }
  return context;
}

const idleScrollAreaState: ScrollAreaState = {
  offset: { x: 0, y: 0 },
  scrolling: false,
  hovering: false,
  hasOverflowX: false,
  hasOverflowY: false,
  overflowXStart: false,
  overflowXEnd: false,
  overflowYStart: false,
  overflowYEnd: false,
};

/**
 * Scroll-area root with caller-drawn scrollbars.
 *
 * The core owns the clamped offsets, the derived overflow flags, the thumb arithmetic, and the
 * captured pointer contract. QuickGUI has no layout observer at the hosted boundary, so the
 * application declares the extents it laid out through `viewportSize` and `contentSize`; every
 * result comes back through `onScrollStateChange` and `useScrollAreaState`.
 */
export function ScrollAreaRoot(props: JSX.ScrollAreaRootProps): NativeNode {
  const scope = createComponentScope("qg-scroll-area");
  const [state, setState] = createSignal<ScrollAreaState>(idleScrollAreaState);
  const context: ScrollAreaContextValue = { scope, state };
  return createPartNode(
    "view",
    omit(
      props,
      "viewportSize",
      "contentSize",
      "overflowEdgeThreshold",
      "onScrollStateChange",
      "children",
    ),
    {
      part: NativePart.ScrollArea,
      scope,
      get viewportSize() {
        return props.viewportSize;
      },
      get contentSize() {
        return props.contentSize;
      },
      get overflowEdgeThreshold() {
        return props.overflowEdgeThreshold;
      },
      onComponentChange: componentChangeListener(
        (details) => scrollAreaStateFromDetails(details),
        (next, event) => {
          setState(next);
          props.onScrollStateChange?.(next, event);
        },
      ),
      get children() {
        return ScrollAreaContext({
          value: context,
          get children() {
            return props.children as SolidElement;
          },
        });
      },
    },
  );
}

function scrollAreaStateFromDetails(details: ComponentChangeDetails): ScrollAreaState | undefined {
  if (!details.offset || typeof details.hasOverflowY !== "boolean") {
    return undefined;
  }
  return {
    offset: { x: details.offset.x ?? 0, y: details.offset.y ?? 0 },
    scrolling: details.scrolling === true,
    hovering: details.hovering === true,
    hasOverflowX: details.hasOverflowX === true,
    hasOverflowY: details.hasOverflowY === true,
    overflowXStart: details.overflowXStart === true,
    overflowXEnd: details.overflowXEnd === true,
    overflowYStart: details.overflowYStart === true,
    overflowYEnd: details.overflowYEnd === true,
  };
}

/**
 * Read the live scroll state inside a `ScrollArea.Root` subtree.
 *
 * Every flag is the core's own derived render state, so styling a fade, shadow, or scrollbar
 * visibility from it never needs an observer, a timer, or a measurement in JavaScript.
 */
export function useScrollAreaState(): () => ScrollAreaState {
  return requireScrollArea("useScrollAreaState").state;
}

/** Clipped viewport. The core answers the wheel and clamps the resulting offset. */
export function ScrollAreaViewport(props: JSX.NativeProps): NativeNode {
  const context = requireScrollArea("ScrollArea.Viewport");
  return createPartNode("view", omit(props, "onWheel"), {
    part: NativePart.ScrollAreaViewport,
    scope: context.scope,
  });
}

/** Scrolled content. Translate it by the negated reported offset. */
export function ScrollAreaContent(props: JSX.NativeProps): NativeNode {
  const context = requireScrollArea("ScrollArea.Content");
  return createPartNode("view", props, {
    part: NativePart.ScrollAreaContent,
    scope: context.scope,
  });
}

interface ScrollbarContextValue {
  orientation: () => "horizontal" | "vertical";
}

const ScrollbarContext = createContext<ScrollbarContextValue | null>(null);

/** One caller-drawn scrollbar track, mounted only while its axis can scroll. */
export function ScrollAreaScrollbar(props: JSX.ScrollAreaScrollbarProps): NativeNode {
  const context = requireScrollArea("ScrollArea.Scrollbar");
  const orientation = () => props.orientation ?? "vertical";
  return createPartNode("view", omit(props, "onPointer", "children"), {
    part: NativePart.ScrollAreaScrollbar,
    scope: context.scope,
    get orientation() {
      return orientation();
    },
    get keepMounted() {
      return props.keepMounted;
    },
    get children() {
      return ScrollbarContext({
        value: { orientation },
        get children() {
          return props.children as SolidElement;
        },
      });
    },
  });
}

/** One caller-drawn thumb carrying the core's captured drag. */
export function ScrollAreaThumb(props: JSX.ScrollAreaThumbProps): NativeNode {
  const context = requireScrollArea("ScrollArea.Thumb");
  const inherited = optionalContext(ScrollbarContext);
  return createPartNode("view", omit(props, "orientation", "onPointer"), {
    part: NativePart.ScrollAreaThumb,
    scope: context.scope,
    get orientation() {
      return props.orientation ?? inherited?.orientation() ?? "vertical";
    },
  });
}

/** The corner between a horizontal and a vertical scrollbar. */
export function ScrollAreaCorner(props: JSX.NativeProps): NativeNode {
  const context = requireScrollArea("ScrollArea.Corner");
  return createPartNode("view", props, {
    part: NativePart.ScrollAreaCorner,
    scope: context.scope,
  });
}

/** Base-UI-shaped compound parts for a scroll area. */
export const ScrollArea = Object.assign(ScrollAreaRoot, {
  Root: ScrollAreaRoot,
  Viewport: ScrollAreaViewport,
  Content: ScrollAreaContent,
  Scrollbar: ScrollAreaScrollbar,
  Thumb: ScrollAreaThumb,
  Corner: ScrollAreaCorner,
});

interface OtpFieldContextValue {
  scope: string;
}

const OtpFieldContext = createContext<OtpFieldContextValue | null>(null);

function requireOtpField(component: string): OtpFieldContextValue {
  const context = optionalContext(OtpFieldContext);
  if (!context) {
    throw new Error(`<${component}> must be rendered inside <OtpField.Root>`);
  }
  return context;
}

/**
 * Controlled OTP field.
 *
 * Each slot composes the core's own text input. Accepted characters fill and advance, a paste
 * distributes across consecutive slots, Backspace clears in place and then walks back, and the
 * arrows plus Home and End move between slots — all inside the core.
 */
export function OtpFieldRoot(props: JSX.OtpFieldRootProps): NativeNode {
  const scope = createComponentScope("qg-otp-field");
  const [uncontrolled, setUncontrolled] = createSignal(untrack(() => props.defaultValue ?? ""));
  const value = () => props.value ?? uncontrolled();
  const context: OtpFieldContextValue = { scope };
  return createPartNode(
    "view",
    omit(
      props,
      "value",
      "defaultValue",
      "onValueChange",
      "onComplete",
      "validationType",
      "children",
    ),
    {
      part: NativePart.OtpField,
      scope,
      get value() {
        return value();
      },
      get variant() {
        return props.validationType;
      },
      onComponentChange: (event: QuickGuiEvent) => {
        const details = componentChangeFromEvent(event);
        if (!details || typeof details.value !== "string") return;
        const next = details.value;
        if (props.value === undefined) setUncontrolled(next);
        props.onValueChange?.(next, event);
        if (typeof details.complete === "string") {
          props.onComplete?.(details.complete, event);
        }
      },
      get children() {
        return OtpFieldContext({
          value: context,
          get children() {
            return props.children as SolidElement;
          },
        });
      },
    },
  );
}

/** One slot input. The core owns its character, its mask, and every editing key. */
export function OtpFieldInput(props: JSX.OtpFieldInputProps): NativeNode {
  const context = requireOtpField("OtpField.Input");
  return createPartNode("input", omit(props, "index"), {
    part: NativePart.OtpFieldInput,
    scope: context.scope,
    get itemIndex() {
      return props.index;
    },
  });
}

/** Decorative separator between two slots, hidden from the announced code. */
export function OtpFieldSeparator(props: JSX.OtpFieldSeparatorProps): NativeNode {
  const context = requireOtpField("OtpField.Separator");
  return createPartNode("view", omit(props, "index"), {
    part: NativePart.OtpFieldSeparator,
    scope: context.scope,
    get itemIndex() {
      return props.index;
    },
  });
}

/** Base-UI-shaped compound parts for an OTP field. */
export const OtpField = Object.assign(OtpFieldRoot, {
  Root: OtpFieldRoot,
  Input: OtpFieldInput,
  Separator: OtpFieldSeparator,
});

/** The live swipe a drawer reports while a gesture is in flight. */
export interface DrawerSwipeState {
  swiping: boolean;
  /** The dismissing displacement in logical pixels. It is never negative. */
  swipeOffset: number;
}

interface DrawerContextValue {
  scope: string;
  open: () => boolean;
  setOpen: (next: boolean, event: QuickGuiEvent) => void;
  swipe: () => DrawerSwipeState;
}

const DrawerContext = createContext<DrawerContextValue | null>(null);

function requireDrawer(component: string): DrawerContextValue {
  const context = optionalContext(DrawerContext);
  if (!context) {
    throw new Error(`<${component}> must be rendered inside <Drawer.Root>`);
  }
  return context;
}

const settledDrawerSwipe: DrawerSwipeState = { swiping: false, swipeOffset: 0 };

/**
 * Controlled drawer.
 *
 * Focus containment, Escape, backdrop dismissal, and focus restoration reuse the core's dialog
 * machinery; snap points, the flick velocity that decides between snapping and dismissing, and
 * the live swipe offset are the core's own. The offset is a paint-only transform the application
 * applies, so a drag never relayouts.
 */
export function DrawerRoot(props: JSX.DrawerRootProps): NativeNode {
  const scope = createComponentScope("qg-drawer");
  const [uncontrolledOpen, setUncontrolledOpen] = createSignal(
    untrack(() => props.defaultOpen ?? false),
  );
  const [swipe, setSwipe] = createSignal<DrawerSwipeState>(settledDrawerSwipe);
  const open = () => props.open ?? uncontrolledOpen();
  const context: DrawerContextValue = {
    scope,
    open,
    setOpen(next, event) {
      if (props.open === undefined) setUncontrolledOpen(next);
      props.onOpenChange?.(next, event);
    },
    swipe,
  };
  return createPartNode(
    "view",
    omit(
      props,
      "open",
      "defaultOpen",
      "onOpenChange",
      "modal",
      "swipeDirection",
      "snapPoints",
      "snapPoint",
      "onSnapPointChange",
      "onSwipeChange",
      "disablePointerDismissal",
      "children",
    ),
    {
      part: NativePart.Drawer,
      scope,
      get open() {
        return open();
      },
      get variant() {
        return drawerModality(props.modal);
      },
      get swipeDirection() {
        return props.swipeDirection;
      },
      get values() {
        return props.snapPoints ? props.snapPoints.slice() : undefined;
      },
      get itemIndex() {
        return props.snapPoint;
      },
      get disablePointerDismissal() {
        return props.disablePointerDismissal;
      },
      onComponentChange: (event: QuickGuiEvent) => {
        const details = componentChangeFromEvent(event);
        if (!details) return;
        if (typeof details.swiping === "boolean") {
          const next: DrawerSwipeState = {
            swiping: details.swiping,
            swipeOffset: details.swipeOffset ?? 0,
          };
          setSwipe(next);
          props.onSwipeChange?.(next, event);
        }
        if (typeof details.snapPoint === "number") {
          props.onSnapPointChange?.(details.snapPoint, event);
        }
        if (typeof details.open === "boolean" && details.open !== open()) {
          context.setOpen(details.open, event);
        }
      },
      get children() {
        return DrawerContext({
          value: context,
          get children() {
            return props.children as SolidElement;
          },
        });
      },
    },
  );
}

function drawerModality(modal: boolean | "trap-focus" | undefined): string | undefined {
  if (modal === undefined) return undefined;
  if (modal === "trap-focus") return "trap-focus";
  return modal ? "modal" : "non-modal";
}

/**
 * Read the live swipe inside a `Drawer.Root` subtree.
 *
 * Apply `swipeOffset` as a paint-only transform on the popup; the core never animates the sheet.
 */
export function useDrawerSwipe(): () => DrawerSwipeState {
  return requireDrawer("useDrawerSwipe").swipe;
}

/** Trigger that opens the drawer. */
export function DrawerTrigger(props: JSX.NativeProps): NativeNode {
  const context = requireDrawer("Drawer.Trigger");
  return createPartNode("button", props, {
    part: NativePart.DrawerTrigger,
    scope: context.scope,
    onClick: forwardClick(props.onClick, (event) => context.setOpen(true, event)),
  });
}

/** Full-window portal and, for a containing modality, the focus boundary. */
export function DrawerPortal(props: JSX.NativeProps): NativeNode {
  const context = requireDrawer("Drawer.Portal");
  return createPartNode("view", props, {
    part: NativePart.DrawerPortal,
    scope: context.scope,
  });
}

/** Caller-painted backdrop. It dismisses unless `disablePointerDismissal` is declared. */
export function DrawerBackdrop(props: JSX.NativeProps): NativeNode {
  const context = requireDrawer("Drawer.Backdrop");
  return createPartNode("view", props, {
    part: NativePart.DrawerBackdrop,
    scope: context.scope,
  });
}

/** Container that aligns the sheet against its edge. The application declares the alignment. */
export function DrawerViewport(props: JSX.NativeProps): NativeNode {
  const context = requireDrawer("Drawer.Viewport");
  return createPartNode("view", props, {
    part: NativePart.DrawerViewport,
    scope: context.scope,
  });
}

/** The sheet itself. Escape and the backdrop dismiss it through the core. */
export function DrawerPopup(props: JSX.NativeProps): NativeNode {
  const context = requireDrawer("Drawer.Popup");
  return createPartNode("view", props, {
    part: NativePart.DrawerPopup,
    scope: context.scope,
  });
}

/** The scrollable body of the sheet. */
export function DrawerContent(props: JSX.NativeProps): NativeNode {
  const context = requireDrawer("Drawer.Content");
  return createPartNode("view", props, {
    part: NativePart.DrawerContent,
    scope: context.scope,
  });
}

/** The sheet's visible label target. */
export function DrawerTitle(props: JSX.NativeProps): NativeNode {
  const context = requireDrawer("Drawer.Title");
  return createPartNode("view", props, {
    part: NativePart.DrawerTitle,
    scope: context.scope,
  });
}

/** The sheet's visible description target. */
export function DrawerDescription(props: JSX.NativeProps): NativeNode {
  const context = requireDrawer("Drawer.Description");
  return createPartNode("view", props, {
    part: NativePart.DrawerDescription,
    scope: context.scope,
  });
}

/** Close control. Focus returns to the declared control on every dismissal path. */
export function DrawerClose(props: JSX.NativeProps): NativeNode {
  const context = requireDrawer("Drawer.Close");
  return createPartNode("button", props, {
    part: NativePart.DrawerClose,
    scope: context.scope,
    onClick: forwardClick(props.onClick, (event) => context.setOpen(false, event)),
  });
}

/** Grab handle carrying the core's captured swipe gesture. */
export function DrawerSwipeArea(props: JSX.NativeProps): NativeNode {
  const context = requireDrawer("Drawer.SwipeArea");
  return createPartNode("view", omit(props, "onPointer"), {
    part: NativePart.DrawerSwipeArea,
    scope: context.scope,
  });
}

/** Base-UI-shaped compound parts for a drawer. */
export const Drawer = Object.assign(DrawerRoot, {
  Root: DrawerRoot,
  Trigger: DrawerTrigger,
  Portal: DrawerPortal,
  Backdrop: DrawerBackdrop,
  Viewport: DrawerViewport,
  Popup: DrawerPopup,
  Content: DrawerContent,
  Title: DrawerTitle,
  Description: DrawerDescription,
  Close: DrawerClose,
  SwipeArea: DrawerSwipeArea,
});

/** The direction the user's attention travelled when a navigation panel changed. */
export type NavigationMenuActivationDirection = "left" | "right" | "up" | "down" | null;

interface NavigationMenuContextValue {
  scope: string;
}

const NavigationMenuContext = createContext<NavigationMenuContextValue | null>(null);
const NavigationMenuItemContext = createContext<(() => string) | null>(null);

function requireNavigationMenu(component: string): NavigationMenuContextValue {
  const context = optionalContext(NavigationMenuContext);
  if (!context) {
    throw new Error(`<${component}> must be rendered inside <NavigationMenu.Root>`);
  }
  return context;
}

function navigationMenuItemValue(component: string, declared?: string): string {
  const inherited = optionalContext(NavigationMenuItemContext);
  const value = declared ?? inherited?.();
  if (value === undefined) {
    throw new Error(`<${component}> needs a \`value\`, or a <NavigationMenu.Item value> ancestor`);
  }
  return value;
}

/**
 * Controlled navigation menu.
 *
 * The core owns the Navigation landmark, the bar's single Tab stop, arrow/Home/End movement with
 * disabled-item skipping, the exact hover open and close deadlines, Escape, and each panel's
 * anchored placement and dismissal.
 */
export function NavigationMenuRoot(props: JSX.NavigationMenuRootProps): NativeNode {
  const scope = createComponentScope("qg-navigation-menu");
  const [uncontrolled, setUncontrolled] = createSignal<string | undefined>(
    untrack(() => props.defaultValue),
  );
  const value = () => props.value ?? uncontrolled();
  const context: NavigationMenuContextValue = { scope };
  return createPartNode(
    "view",
    omit(
      props,
      "value",
      "defaultValue",
      "onValueChange",
      "onActivationDirectionChange",
      "placement",
      "children",
    ),
    {
      part: NativePart.NavigationMenu,
      scope,
      get activeValue() {
        return value();
      },
      get anchorPlacement() {
        return props.placement;
      },
      onComponentChange: (event: QuickGuiEvent) => {
        const details = componentChangeFromEvent(event);
        if (!details || !("value" in details)) return;
        const next = (details.value ?? undefined) as string | undefined;
        if (next !== value()) {
          if (props.value === undefined) setUncontrolled(next);
          props.onValueChange?.(next, event);
        }
        props.onActivationDirectionChange?.(
          (details.activationDirection ?? null) as NavigationMenuActivationDirection,
          event,
        );
      },
      get children() {
        return NavigationMenuContext({
          value: context,
          get children() {
            return props.children as SolidElement;
          },
        });
      },
    },
  );
}

/** The list of items, carrying the List role and the menu's orientation. */
export function NavigationMenuList(props: JSX.NativeProps): NativeNode {
  const context = requireNavigationMenu("NavigationMenu.List");
  return createPartNode("view", props, {
    part: NativePart.NavigationMenuList,
    scope: context.scope,
  });
}

/** One item. Its `value` flows to every part inside it. */
export function NavigationMenuItem(props: JSX.NavigationMenuItemProps): NativeNode {
  const context = requireNavigationMenu("NavigationMenu.Item");
  const value = () => props.value;
  return createPartNode("view", omit(props, "value", "children"), {
    part: NativePart.NavigationMenuItem,
    scope: context.scope,
    get partValue() {
      return props.value;
    },
    get children() {
      return NavigationMenuItemContext({
        value,
        get children() {
          return props.children as SolidElement;
        },
      });
    },
  });
}

function navigationMenuPart(
  component: string,
  part: NativePartName,
  element: NativeElementName,
  props: JSX.NavigationMenuPartProps,
): NativeNode {
  const context = requireNavigationMenu(component);
  const value = navigationMenuItemValue(component, props.value);
  return createPartNode(element, omit(props, "value"), {
    part,
    scope: context.scope,
    partValue: value,
  });
}

/** One trigger. Exactly one enabled trigger stays in the window's Tab sequence. */
export function NavigationMenuTrigger(props: JSX.NavigationMenuTriggerProps): NativeNode {
  return navigationMenuPart(
    "NavigationMenu.Trigger",
    NativePart.NavigationMenuTrigger,
    "button",
    props,
  );
}

/** Decorative trigger icon, hidden from the accessible name. */
export function NavigationMenuIcon(props: JSX.NavigationMenuPartProps): NativeNode {
  return navigationMenuPart("NavigationMenu.Icon", NativePart.NavigationMenuIcon, "view", props);
}

/** Portal boundary for one item's panel. */
export function NavigationMenuPortal(props: JSX.NavigationMenuPartProps): NativeNode {
  return navigationMenuPart(
    "NavigationMenu.Portal",
    NativePart.NavigationMenuPortal,
    "view",
    props,
  );
}

/** Positioner for one item's panel. Mount either this or the portal. */
export function NavigationMenuPositioner(props: JSX.NavigationMenuPartProps): NativeNode {
  return navigationMenuPart(
    "NavigationMenu.Positioner",
    NativePart.NavigationMenuPositioner,
    "view",
    props,
  );
}

/** One item's popup surface. */
export function NavigationMenuPopup(props: JSX.NavigationMenuPartProps): NativeNode {
  return navigationMenuPart("NavigationMenu.Popup", NativePart.NavigationMenuPopup, "view", props);
}

/** The clipping viewport an application animates a resizing panel inside. */
export function NavigationMenuViewport(props: JSX.NavigationMenuPartProps): NativeNode {
  return navigationMenuPart(
    "NavigationMenu.Viewport",
    NativePart.NavigationMenuViewport,
    "view",
    props,
  );
}

/** One item's panel content, labelled by its trigger. */
export function NavigationMenuContent(props: JSX.NavigationMenuPartProps): NativeNode {
  return navigationMenuPart(
    "NavigationMenu.Content",
    NativePart.NavigationMenuContent,
    "view",
    props,
  );
}

/** One item's decorative arrow. */
export function NavigationMenuArrow(props: JSX.NavigationMenuPartProps): NativeNode {
  return navigationMenuPart("NavigationMenu.Arrow", NativePart.NavigationMenuArrow, "view", props);
}

/** One item's optional viewport backdrop. */
export function NavigationMenuBackdrop(props: JSX.NavigationMenuPartProps): NativeNode {
  return navigationMenuPart(
    "NavigationMenu.Backdrop",
    NativePart.NavigationMenuBackdrop,
    "view",
    props,
  );
}

/** A navigation link. `active` projects as the native selected state, not a visual class. */
export function NavigationMenuLink(props: JSX.NavigationMenuLinkProps): NativeNode {
  const context = requireNavigationMenu("NavigationMenu.Link");
  return createPartNode("button", omit(props, "value", "active"), {
    part: NativePart.NavigationMenuLink,
    scope: context.scope,
    get partValue() {
      return props.value;
    },
    get checked() {
      return props.active === true;
    },
  });
}

/** Base-UI-shaped compound parts for a navigation menu. */
export const NavigationMenu = Object.assign(NavigationMenuRoot, {
  Root: NavigationMenuRoot,
  List: NavigationMenuList,
  Item: NavigationMenuItem,
  Trigger: NavigationMenuTrigger,
  Icon: NavigationMenuIcon,
  Content: NavigationMenuContent,
  Link: NavigationMenuLink,
  Portal: NavigationMenuPortal,
  Positioner: NavigationMenuPositioner,
  Popup: NavigationMenuPopup,
  Viewport: NavigationMenuViewport,
  Arrow: NavigationMenuArrow,
  Backdrop: NavigationMenuBackdrop,
});

export namespace JSX {
  export type Element = SolidElement;
  export type Child = SolidElement;
  export type EventHandler = NativeEventListener;

  export interface ElementChildrenAttribute {
    children: {};
  }

  export interface IntrinsicAttributes {
    key?: string | number;
  }

  export interface Style extends Omit<StyleHelpers, "flex" | "flexWrap"> {
    /** CSS flex shorthand, or the boolean display helper. */
    flex?: number | string | boolean | null | undefined;
    textColor?: ColorValue;
    display?: "none" | "block" | "flex" | "grid";
    flexDirection?: "row" | "row-reverse" | "column" | "column-reverse";
    flexWrap?: "nowrap" | "wrap" | "wrap-reverse" | boolean | null | undefined;
    flexGrow?: number;
    flexShrink?: number;
    flexBasis?: number | string;
    alignItems?: "start" | "flex-start" | "center" | "end" | "flex-end" | "baseline" | "stretch";
    alignSelf?: Style["alignItems"];
    justifyContent?:
      | "start"
      | "flex-start"
      | "center"
      | "end"
      | "flex-end"
      | "space-between"
      | "space-around"
      | "space-evenly";
    alignContent?: Style["justifyContent"] | "normal" | "stretch";
    gap?: number | string;
    columnGap?: number | string;
    rowGap?: number | string;
    width?: number | string;
    height?: number | string;
    minWidth?: number | string;
    minHeight?: number | string;
    maxWidth?: number | string;
    maxHeight?: number | string;
    padding?: number | string;
    paddingTop?: number | string;
    paddingRight?: number | string;
    paddingBottom?: number | string;
    paddingLeft?: number | string;
    margin?: number | string;
    marginTop?: number | string;
    marginRight?: number | string;
    marginBottom?: number | string;
    marginLeft?: number | string;
    /** A color, a CSS gradient function string, or the declared gradient object form. */
    bg?: number | string | GradientDeclaration;
    color?: number | string;
    /** @deprecated Declare `hover: { color }` instead. */
    hoverColor?: number | string;
    /** @deprecated Declare `active: { color }` instead. */
    activeColor?: number | string;
    transition?: number | string | TransitionDeclaration;
    opacity?: number;
    borderWidth?: number | string;
    borderTopWidth?: number | string;
    borderRightWidth?: number | string;
    borderBottomWidth?: number | string;
    borderLeftWidth?: number | string;
    borderColor?: number | string;
    borderRadius?: number | string;
    boxShadow?: string;
    fontSize?: number | string;
    fontFamily?: string;
    fontWeight?: number | string;
    lineHeight?: number | string;
    textAlign?: "left" | "center" | "right" | "justify" | "start" | "end";
    whiteSpace?: "normal" | "nowrap";
    textOverflow?: "clip" | "ellipsis";
    lineClamp?: number;
    overflow?: "visible" | "hidden" | "auto" | "scroll";
    overflowX?: Style["overflow"];
    overflowY?: Style["overflow"];
    cursor?: string;
    appRegion?: "drag" | "no-drag";
    position?: "relative" | "absolute" | "sticky";
    top?: number | string;
    right?: number | string;
    bottom?: number | string;
    left?: number | string;
    userSelect?: "auto" | "text" | "none";
    visibility?: "visible" | "hidden";
    aspectRatio?: number;
    /** CSS grid track list, an array of tracks, or a count of equal `1fr` tracks. */
    gridTemplateColumns?: number | string | readonly (number | string)[];
    gridTemplateRows?: Style["gridTemplateColumns"];
    gridAutoFlow?: "row" | "column" | "row dense" | "column dense";
    /** CSS `grid-column` shorthand such as `2`, `2 / 4`, or `span 3`. */
    gridColumn?: number | string;
    gridRow?: number | string;
    gridColumnStart?: number;
    gridColumnEnd?: number;
    gridColumnSpan?: number;
    gridRowStart?: number;
    gridRowEnd?: number;
    gridRowSpan?: number;
    transitionProperty?: string;
    transitionDuration?: number | string;
    transitionTimingFunction?: TransitionEasing;
    transitionEasing?: TransitionEasing;
    /** Repaint cadence ceiling while the transition runs. */
    transitionMaxFps?: number;
    objectFit?: "fill" | "contain" | "cover" | "scale-down" | "none";
    scrollToEndRevision?: number;

    /** Extra advance after every glyph cluster, clamped by the core to +/-256 logical pixels. */
    letterSpacing?: number | string;
    /** Extra advance after every space character, clamped by the core to +/-256 logical pixels. */
    wordSpacing?: number | string;
    /** Case mapping applied to non-editable text before shaping. Inputs are never transformed. */
    textTransform?: "none" | "uppercase" | "lowercase" | "capitalize";
    /** `"x y blur color"`, the object form, or `"none"`. Blur is a bounded approximation. */
    textShadow?: string | TextShadowDeclaration;
    /** Space-separated `underline`, `line-through`, and `overline`, or `none`. */
    textDecoration?: TextDecorationLine;
    textDecorationLine?: TextDecorationLine;
    textDecorationColor?: number | string;
    textDecorationStyle?: "solid" | "double" | "wavy";
    /** Adopts the closest native underline thickness: 0, 1, 2, 4, or 8 logical pixels. */
    textDecorationThickness?: number | string;
    wordBreak?: "normal" | "break-all" | "keep-all";
    overflowWrap?: "normal" | "anywhere" | "break-word";
    /** Author-placed soft hyphens only; the core never hyphenates from a dictionary. */
    hyphens?: "none" | "manual" | "auto";
    /** Base paragraph direction used while shaping, without mirroring layout. */
    textDirection?: "auto" | "ltr" | "rtl";

    /** Inline layout direction inherited by this whole subtree. */
    direction?: "ltr" | "rtl";
    paddingStart?: number | string;
    paddingEnd?: number | string;
    marginStart?: number | string;
    marginEnd?: number | string;
    borderStartWidth?: number | string;
    borderEndWidth?: number | string;

    /** A color, a CSS gradient function, or the declared gradient object form. */
    bgGradient?: number | string | GradientDeclaration;
    borderTopLeftRadius?: number | string;
    borderTopRightRadius?: number | string;
    borderBottomRightRadius?: number | string;
    borderBottomLeftRadius?: number | string;
    borderStyle?: "solid" | "dashed" | "dotted";
    /** CSS `outline` shorthand, a plain width, or `none`. Outlines never affect layout. */
    outline?: number | string;
    outlineWidth?: number | string;
    outlineColor?: number | string;
    outlineOffset?: number | string;
    outlineStyle?: "solid" | "dashed" | "dotted" | "none";
    /** Path, `file://`, or base64 `data:` URL decoded once and painted inside the rounded box. */
    bgImage?: string;
    bgSize?: "auto" | "cover" | "contain" | (string & {});
    bgRepeat?: "no-repeat" | "repeat" | "repeat-x" | "repeat-y";
    bgPosition?: string;
    /** CSS filter-function list. `blur()` and `drop-shadow()` promote a compositing group. */
    filter?: string | readonly string[];
    /** Colour filters and one blur applied to whatever is already painted behind this element. */
    backdropFilter?: string | readonly string[];
    /** CSS transform-function list, or the `matrix()` object form. Paint only; layout never moves. */
    transform?: string | readonly string[] | TransformMatrix;
    /** Fraction of the border box a transform acts around. Defaults to the centre. */
    transformOrigin?: string;
    mixBlendMode?: BlendMode;

    /** @deprecated Declare `hover: { bg }` instead. */
    hoverBg?: number | string | GradientDeclaration;
    /** @deprecated Declare `hover: { outline }` instead. */
    hoverOutline?: string;
    /** @deprecated Declare `hover: { transform }` instead. */
    hoverTransform?: string | readonly string[] | TransformMatrix;
    /** @deprecated Declare `active: { bg }` instead. */
    activeBg?: number | string | GradientDeclaration;
    /** @deprecated Declare `active: { outline }` instead. */
    activeOutline?: string;
    /** @deprecated Declare `active: { transform }` instead. */
    activeTransform?: string | readonly string[] | TransformMatrix;
    /** @deprecated Declare `focus: { bg }` instead. */
    focusBg?: number | string | GradientDeclaration;
    /** @deprecated Declare `focus: { color }` instead. */
    focusColor?: number | string;
    /** @deprecated Declare `focus: { outline }` instead. */
    focusOutline?: string;
    /** @deprecated Declare `focus: { transform }` instead. */
    focusTransform?: string | readonly string[] | TransformMatrix;

    /** Snap axis and strictness, such as `"x mandatory"` or `"y proximity"`. */
    scrollSnapType?: string;
    scrollSnapAlign?: "start" | "center" | "end";
    scrollSnapStop?: "normal" | "always";

    /**
     * Interaction states, each a paint-only override the Rust core swaps in on its own: no
     * JavaScript round trip decides what is hovered, pressed, or focused. Declare `transition`
     * on the element to animate between them.
     */
    /** While the pointer rests on this element. */
    hover?: StateStyle | null;
    /** While a pointer press on this element is held. */
    active?: StateStyle | null;
    /**
     * While this element owns visible keyboard focus, like CSS `:focus-visible`: focus a pointer
     * press lands paints nothing, focus a key lands paints it all, and text inputs paint theirs
     * whenever focused.
     */
    focus?: StateStyle | null;
    /** While the `disabled` prop is set. */
    disabled?: StateStyle | null;
    /** While the `invalid` prop is set. */
    invalid?: StateStyle | null;
    /**
     * While the `selected` prop is set, or while the core marks the element selected, such as a
     * `Table.Row` inside the table's selection. Like a native list row, a selected element keeps
     * this paint while hovered or pressed: it sits above the pointer states and beneath `disabled`.
     */
    selected?: StateStyle | null;
    /** While this `draggable` element is the source of an active drag. */
    dragging?: StateStyle | null;
    /** While a payload one of this element's `dropKinds` accepts is over it. */
    dragOver?: StateStyle | null;
    /**
     * While the nearest ancestor declared `group` — or the ancestor an entry's `group` names — is
     * hovered, like Tailwind's `group-hover`. A list follows several groups at once; entries layer
     * in order, later ones winning. Group states sit beneath this element's own states, so a
     * revealed button the pointer reaches keeps every group value its own `hover` does not
     * override.
     */
    groupHover?: GroupStateStyle | readonly GroupStateStyle[] | null;
    /**
     * While a press inside the nearest ancestor `group` — or the one an entry names — is held,
     * like Tailwind's `group-active`. It layers over `groupHover`.
     */
    groupActive?: GroupStateStyle | readonly GroupStateStyle[] | null;
    /**
     * While this element or any descendant owns keyboard focus, like CSS `:focus-within`. Unlike
     * `focus`, it follows the focus itself rather than focus visibility.
     */
    focusWithin?: Omit<StateStyle, "cursor"> | null;
  }

  /** One `groupHover` or `groupActive` entry; it may name the group it follows, never a cursor. */
  export interface GroupStateStyle extends Omit<StateStyle, "cursor"> {
    /**
     * The `group="name"` ancestor to follow, past any nearer group, like Tailwind's
     * `group-hover/name`. Omitted, the nearest ancestor group is followed.
     */
    group?: string;
  }

  /**
   * The paint-only overrides one interaction state swaps in: exactly what the core's own
   * `ElementStateStyle` carries. Layout never changes with a state.
   */
  export interface StateStyle {
    textColor?: number | string;
    /** A color, a CSS gradient function, or the declared gradient object form. */
    bg?: number | string | GradientDeclaration;
    /** A color, a CSS gradient function, or the declared gradient object form. */
    bgGradient?: number | string | GradientDeclaration;
    color?: number | string;
    borderColor?: number | string;
    /** Uniform border width in logical pixels. */
    borderWidth?: number | string;
    /** Uniform corner radius in logical pixels. */
    borderRadius?: number | string;
    /** CSS `outline` shorthand, a plain width, or `none`; uses the element's own `outlineOffset`. */
    outline?: number | string;
    /** CSS `box-shadow` list replacing the element's shadows, or `none`. */
    boxShadow?: string;
    opacity?: number;
    cursor?: string;
    transform?: string | readonly string[] | TransformMatrix;
    transformOrigin?: string;
  }

  /** Combinable decoration lines. `none` clears an inherited decoration. */
  export type TextDecorationLine =
    | "none"
    | "underline"
    | "overline"
    | "line-through"
    | (string & {});

  export type BlendMode =
    | "normal"
    | "multiply"
    | "screen"
    | "darken"
    | "lighten"
    | "overlay"
    | "difference"
    | "exclusion"
    | "hard-light"
    | "color-dodge"
    | "color-burn";

  export interface TextShadowDeclaration {
    offsetX: number;
    offsetY: number;
    blur?: number;
    /** Omitted, the shadow adopts the element's own text color. */
    color?: string;
  }

  /** A CSS `matrix(a, b, c, d, tx, ty)` in the core's own component order. */
  export interface TransformMatrix {
    a: number;
    b: number;
    c: number;
    d: number;
    tx: number;
    ty: number;
  }

  /** One gradient stop. A bare color string spaces evenly with its neighbours. */
  export type GradientStop = string | { color: string; position?: number };

  /**
   * The declared object form of a gradient.
   *
   * At most eight stops are retained by the core; extra stops are dropped in source order.
   */
  export interface GradientDeclaration {
    type: "linear" | "radial" | "conic";
    /** Linear gradient angle in CSS degrees; `0` points to the top. */
    angle?: number;
    /** Conic gradient start angle in CSS degrees. */
    fromAngle?: number;
    shape?: "circle" | "ellipse";
    extent?: "closest-side" | "farthest-side" | "farthest-corner";
    center?: { x: number; y: number };
    interpolation?: "linear-srgb" | "srgb" | "oklab";
    stops: readonly GradientStop[];
  }

  /** The interaction states `Style` nests; they live inside `style`, never as props of their own. */
  export type StateName =
    | "hover"
    | "active"
    | "focus"
    | "disabled"
    | "invalid"
    | "dragging"
    | "dragOver"
    | "groupHover"
    | "groupActive"
    | "focusWithin"
    | "selected";

  /**
   * What `style` accepts: one style, or an array of styles and falsy entries nested to any depth,
   * merged left to right by `flattenStyle`.
   */
  export type StyleProp = Style | false | null | undefined | ReadonlyArray<StyleProp>;

  export interface NativeProps {
    accessibilityLabel?: string;
    children?: unknown;
    style?: StyleProp;
    disabled?: boolean;
    /** Expose web-style invalid state; the `style.invalid` variant paints while it is set. */
    invalid?: boolean;
    /** Expose web-style selected state; the `style.selected` variant paints while it is set. */
    selected?: boolean;
    /**
     * Makes this element the group its descendants' `groupHover` and `groupActive` styles follow,
     * like Tailwind's `group`: `true` opens an unnamed group, and a string names it so a
     * descendant can follow it past a nearer group with `groupHover: { group: "name" }`.
     */
    group?: boolean | string;
    role?: string;
    tabIndex?: number;
    /** Keep keyboard focus where it is when this element is activated with a pointer. */
    focusOnPointer?: boolean;
    /** Paint this subtree in the viewport overlay plane above embedded native views. */
    overlay?: boolean;
    /** Contain keyboard focus within this subtree while it is the topmost trap. */
    focusTrap?: boolean;
    /** Restore the previously focused mounted control when this surface unmounts. */
    restorePreviousFocus?: boolean;
    /** Prefer this control when its containing focus trap takes focus. */
    autoFocus?: boolean;
    /** Expose modal semantics to assistive technology. */
    "aria-modal"?: boolean;
    ariaModal?: boolean;
    dismissOnEscape?: boolean;
    dismissOnPointerOutside?: boolean;
    /** Delayed, pointer-passive native tooltip text shown while this element is hovered. */
    tooltip?: string;
    tooltipPlacement?: PopoverPlacement;
    /** Hover delay in milliseconds, clamped by the Rust core to at most ten seconds. */
    tooltipDelay?: number;
    tooltipGap?: number;
    tooltipViewportMargin?: number;
    hitSlop?: number | string;
    hitSlopTop?: number | string;
    hitSlopRight?: number | string;
    hitSlopBottom?: number | string;
    hitSlopLeft?: number | string;
    "aria-label"?: string;
    ariaLabel?: string;
    ref?: ((node: NativeNode) => void) | NativeNode;
    onClick?: EventHandler;
    onMouseEnter?: EventHandler;
    onMouseLeave?: EventHandler;
    onPointerEnter?: EventHandler;
    onPointerLeave?: EventHandler;
    /** Captured pointer stream from press through release/cancel, including outside the element. */
    onPointer?: EventHandler;
    onInput?: EventHandler;
    onChange?: EventHandler;
    onSubmit?: EventHandler;
    onDismiss?: EventHandler;
    /** Focused key press. Declare a `tabIndex` to make an ordinary container focusable. */
    onKeyDown?: EventHandler;
    onKeyUp?: EventHandler;
    onMouseDown?: EventHandler;
    onMouseUp?: EventHandler;
    onMouseMove?: EventHandler;
    /** Second press of one exact native multi-click sequence. */
    onDoubleClick?: EventHandler;
    onWheel?: EventHandler;
    /** Secondary-button press. Use `ContextMenu` for a declared native menu. */
    onContextMenu?: EventHandler;
    onPinch?: EventHandler;
    onRotate?: EventHandler;
    onSmartMagnify?: EventHandler;
    onPressure?: EventHandler;
    onFocus?: EventHandler;
    onBlur?: EventHandler;
    /** Bounded accelerator table resolved by the core while this element is focused. */
    keymap?: Keymap;
    /** Typed binding id dispatched by `keymap`. */
    onAction?: EventHandler;
    /** Declared drag payload promoted when this element starts a drag. */
    draggable?: boolean | DragSource;
    onDragStart?: EventHandler;
    onDragEnd?: EventHandler;
    /** Payload kinds this element accepts, declared ahead of the native drag. */
    dropKinds?: DropKind | readonly DropKind[];
    onDrop?: EventHandler;
    onFilesDropped?: EventHandler;
  }

  export interface InputProps extends NativeProps {
    type?: "text" | "password";
    value?: string;
    placeholder?: string;
    multiline?: boolean;
  }

  export interface VirtualListProps extends NativeProps {
    estimatedItemHeight?: number;
    overscan?: number;
    overscanPixels?: number;
    itemHeights?: readonly number[];
    listAlignment?: "top" | "bottom";
    followMode?: "normal" | "tail";
  }


  export interface SvgProps extends NativeProps {
    /** Complete inline SVG document. External resources are ignored by the Rust core. */
    source: string;
  }

  export interface PopoverRootProps {
    children?: unknown;
    open?: boolean;
    defaultOpen?: boolean;
    onOpenChange?: (open: boolean, details: PopoverOpenChangeDetails) => void;
    dismissOnEscape?: boolean;
    dismissOnPointerOutside?: boolean;
    /** Trap focus in the popup and block pointer input behind it. */
    modal?: boolean;
    /** Open the popup while the trigger is hovered, on the core's own exact deadline. */
    openOnHover?: boolean;
    /** Hover open deadline in milliseconds. Defaults to the core's 300 ms. */
    delay?: number;
    /** Hover close deadline in milliseconds, so the pointer can cross the side offset. */
    closeDelay?: number;
    /**
     * Where the retained tree really placed the popup.
     *
     * The declared side and alignment are only a preference; style from this the way Base UI
     * styles from `data-side` and `data-align`.
     */
    onPlacementChange?: (placement: AnchorPlacementDetails, event: QuickGuiEvent) => void;
    /** Default positioning, overridden by whatever `Popover.Positioner` declares. */
    side?: "top" | "bottom" | "left" | "right";
    align?: "start" | "center" | "end";
    sideOffset?: number;
    alignOffset?: number;
    collisionPadding?: number;
    sticky?: boolean;
    anchor?: NativeNode | { x: number; y: number };
  }

  export interface PopoverTriggerProps extends NativeProps {
    /** Open the popup on hover instead of on press. */
    openOnHover?: boolean;
    delay?: number;
    closeDelay?: number;
  }

  export interface PopoverPositionerProps extends NativeProps {
    /** Preferred side. The core flips it when the popup does not fit. */
    side?: "top" | "bottom" | "left" | "right";
    /** Preferred cross-axis alignment. The core re-aligns it when it does not fit. */
    align?: "start" | "center" | "end";
    /** Distance from the anchor on the placement side, in logical pixels. */
    sideOffset?: number;
    /** Shift along the cross axis, applied before collision handling. */
    alignOffset?: number;
    /** Minimum distance from the viewport edge, in logical pixels. */
    collisionPadding?: number;
    /** Keep the popup inside the viewport. `false` lets it travel with a scrolling anchor. */
    sticky?: boolean;
    /** Anchor to another node, or to one logical point. Defaults to the trigger. */
    anchor?: NativeNode | { x: number; y: number };
  }

  export interface PopoverContentProps extends NativeProps {
    width: number;
    height: number;
    placement?: PopoverPlacement;
    gap?: number;
    viewportMargin?: number;
  }

  export interface CheckboxProps extends NativeProps {
    /** Inside a `CheckboxGroup.Root`, the declared value this checkbox toggles. */
    value?: string;
    /** Inside a `CheckboxGroup.Root`, make this the group's derived parent checkbox. */
    parent?: boolean;
    /**
     * A standalone parent checkbox's children, as checked booleans.
     *
     * The core folds them into on, mixed, or off with no registry, so the mixed state is derived
     * rather than retained anywhere.
     */
    childrenChecked?: readonly boolean[];
    /** Refuse changes while keeping the control focusable and its value announced. */
    readOnly?: boolean;
    /** Controlled `true`, `false`, or `"indeterminate"` toggle state. */
    checked?: CheckedState;
    defaultChecked?: CheckedState;
    onCheckedChange?: (checked: boolean, event: QuickGuiEvent) => void;
  }

  export interface RadioGroupProps extends NativeProps {
    value?: string;
    defaultValue?: string;
    onValueChange?: (value: string, event: QuickGuiEvent) => void;
    /** Refuse changes while keeping the group focusable. */
    readOnly?: boolean;
    required?: boolean;
  }

  export interface RadioProps extends NativeProps {
    /** Refuse changes while keeping the control focusable. */
    readOnly?: boolean;
    /** Value this radio selects in its `RadioGroup`. */
    value?: string;
    /** Controlled selection for a radio used without a `RadioGroup`. */
    checked?: boolean;
    defaultChecked?: boolean;
    onCheckedChange?: (checked: boolean, event: QuickGuiEvent) => void;
  }

  export interface SwitchProps extends NativeProps {
    checked?: boolean;
    defaultChecked?: boolean;
    onCheckedChange?: (checked: boolean, event: QuickGuiEvent) => void;
    /** Refuse changes while keeping the control focusable. */
    readOnly?: boolean;
  }

  export interface TabsRootProps extends NativeProps {
    value?: string;
    defaultValue?: string;
    onValueChange?: (value: string, event: QuickGuiEvent) => void;
    orientation?: "horizontal" | "vertical";
    /** `"manual"` activates on Enter or Space; `"automatic"` activates on arrow focus. */
    activation?: "manual" | "automatic";
    /** Wrap arrow navigation at the ends of the tab list. Defaults to `true`. */
    loop?: boolean;
    /** Retain inactive panels as `display: none` instead of omitting them. */
    keepMounted?: boolean;
    /**
     * Everything the core decided about the tab set.
     *
     * `activationDirection` is Base UI's `data-activation-direction`, and `indicator` is the
     * active tab's laid-out box published during the paint QuickGUI was already performing.
     */
    onTabsStateChange?: (state: TabsState, event: QuickGuiEvent) => void;
  }

  export interface TabsTabProps extends NativeProps {
    value: string;
    /** This tab's position, which is how the core knows which way the selection travelled. */
    index?: number;
  }

  export interface TabsIndicatorProps extends NativeProps {
    /** Tab this indicator belongs to. Defaults to the enclosing tab, then the active tab. */
    value?: string;
    /**
     * Anchor the indicator to the active tab's edge, and publish that tab's box back.
     *
     * `"bottom"` draws the familiar underline. Without a placement the indicator is a plain part
     * on the active tab and no geometry is reported.
     */
    placement?: PopoverPlacement;
  }

  export interface TabsPanelProps extends NativeProps {
    value: string;
  }

  export interface CollapsibleRootProps extends NativeProps {
    open?: boolean;
    defaultOpen?: boolean;
    onOpenChange?: (open: boolean, event: QuickGuiEvent) => void;
    keepMounted?: boolean;
  }

  export interface AccordionRootProps extends NativeProps {
    /** Open item value, or values when `multiple` is declared. */
    value?: string | readonly string[] | null;
    defaultValue?: string | readonly string[] | null;
    onValueChange?: (value: string | string[] | null, event: QuickGuiEvent) => void;
    multiple?: boolean;
    keepMounted?: boolean;
    /** Heading level for each item header, clamped by the core to 1 through 6. */
    headingLevel?: number;
  }

  export interface AccordionItemProps extends NativeProps {
    value: string;
    /** Caller-visible position projected across this item's parts. */
    index?: number;
  }

  export interface FieldRootProps extends NativeProps {
    invalid?: boolean;
    required?: boolean;
    touched?: boolean;
    dirty?: boolean;
    filled?: boolean;
    /** Bounded message retained for form reports and native accessibility. */
    validationMessage?: string;
    /** Which triggers validate. `"onSubmit"` by default, as in Base UI. */
    validationMode?: "onSubmit" | "onBlur" | "onChange";
    /** How long the core waits after a change before validating, in milliseconds. */
    validationDebounceTime?: number;
    /** The core's own answers for the declared validation mode. */
    onValidationChange?: (
      validation: {
        triggers: FieldValidationTriggers;
        delay: FieldValidationDelays;
      },
      event: QuickGuiEvent,
    ) => void;
  }

  export interface FieldValidityProps extends NativeProps {
    /** Whether the readout is shown. Defaults to `true`. */
    visible?: boolean;
  }

  export interface FieldLabelProps extends NativeProps {
    /** Name the control without forwarding pointer activation to it. */
    passive?: boolean;
  }

  export interface FieldControlProps extends InputProps {
    /** Native element this control renders. Defaults to `input`. */
    element?: "input" | "textarea" | "button" | "view" | "text";
  }

  export interface FieldsetRootProps extends NativeProps {}

  export interface ImageProps extends NativeProps {
    /** Filesystem path, `file://` URL, or a base64 `data:` URL. */
    source: string;
    /** `fill`, `contain` (default), `cover`, `scale-down`, or `none`. */
    fit?: "fill" | "contain" | "cover" | "scale-down" | "none";
    objectFit?: NonNullable<ImageProps["fit"]>;
  }

  export interface ShaderProps extends NativeProps {
    /** Complete WGSL bounded and validated by the Rust core. */
    source: string;
    /** Up to sixteen floats packed into the core's four fixed parameter vectors. */
    shaderParameters?: readonly number[] | readonly (readonly number[])[];
  }

  export interface GaugeFormatProps {
    /** Bounded value formatter the core applies: `"percent"` or `"fraction"`. */
    format?: "percent" | "fraction";
    /** Everything the core derived from the declared value. */
    onStatusChange?: (state: GaugeState, event: QuickGuiEvent) => void;
  }

  export interface ProgressProps extends NativeScopedProps, GaugeFormatProps {
    /** Completed amount. Omit, or declare `indeterminate`, for unknown progress. */
    value?: number;
    /** Completion maximum. Defaults to `1`. */
    max?: number;
    indeterminate?: boolean;
    /** Human-readable value such as `"3 of 12 files"`, preferred by assistive technology. */
    valueText?: string;
  }

  export interface MeterProps extends NativeScopedProps, GaugeFormatProps {
    value?: number;
    min?: number;
    max?: number;
    low?: number;
    high?: number;
    optimum?: number;
  }

  /** A declared component part that names the instance it belongs to. */
  export interface NativeScopedProps extends NativeProps {
    /** Stable key shared by every part of one component instance. */
    scope?: string;
  }

  /** One entry in a declared toolbar or toggle-group navigation model. */
  export interface ComponentItemDeclaration {
    value: string;
    disabled?: boolean;
  }

  /** One pane constraint in a declared splitter. */
  export interface SplitterPaneDeclaration {
    min?: number;
    collapsible?: boolean;
  }

  export interface SliderProps extends NativeScopedProps {
    /** The core's own pointer boundary, matching Base UI's `onValueCommitted`. */
    onValueCommitted?: (values: readonly number[], event: QuickGuiEvent) => void;
    /** Whole steps the core holds open between adjacent thumbs. */
    minStepsBetweenValues?: number;
    /** `"center"` (default) or `"edge"`, matching Base UI's `thumbAlignment`. */
    thumbAlignment?: "center" | "edge";
    /** Bounded value formatter the core applies: `"percent"` or `"fraction"`. */
    format?: "percent" | "fraction";
    /** Controlled thumb values, one entry per thumb. */
    value?: readonly number[];
    defaultValue?: readonly number[];
    min?: number;
    max?: number;
    step?: number;
    largeStep?: number;
    orientation?: "horizontal" | "vertical";
    onValueChange?: (values: readonly number[], event: QuickGuiEvent) => void;
  }

  export interface SliderThumbProps extends NativeScopedProps {
    /** Base UI's `data-index`. An alias for `itemIndex`. */
    index?: number;
    /** Which thumb this part paints, matching the index in `value`. */
    itemIndex?: number;
  }

  export interface SplitterProps extends NativeScopedProps {
    /** Controlled pane sizes in logical pixels. */
    value?: readonly number[];
    defaultValue?: readonly number[];
    panes?: readonly SplitterPaneDeclaration[];
    step?: number;
    orientation?: "horizontal" | "vertical";
    onSizesChange?: (sizes: readonly number[], event: QuickGuiEvent) => void;
  }

  export interface SplitterPaneProps extends NativeScopedProps {
    /** Which pane or handle this part paints. */
    itemIndex?: number;
  }

  export interface ToolbarInputProps extends NativeScopedProps {
    /** Value this input owns in its toolbar's `items`. */
    partValue?: string;
    /** Native element this toolbar input renders. Defaults to `input`. */
    element?: "input" | "button" | "view" | "text";
  }

  export interface ToolbarProps extends NativeScopedProps {
    items?: readonly ComponentItemDeclaration[];
    active?: string;
    defaultActive?: string;
    orientation?: "horizontal" | "vertical";
    loopFocus?: boolean;
    onActiveChange?: (active: string | undefined, event: QuickGuiEvent) => void;
  }

  export interface ToggleGroupProps extends NativeScopedProps {
    items?: readonly ComponentItemDeclaration[];
    /** Controlled pressed values. */
    value?: readonly string[];
    defaultValue?: readonly string[];
    /** `"single"` presses at most one item; `"multiple"` presses any number. */
    variant?: "single" | "multiple";
    active?: string;
    orientation?: "horizontal" | "vertical";
    loopFocus?: boolean;
    onValueChange?: (values: readonly string[], event: QuickGuiEvent) => void;
  }

  /** One declared toolbar or toggle-group item part. */
  export interface ComponentItemProps extends NativeScopedProps {
    /** The item's stable value, matching an entry in the group's `items`. */
    partValue?: string;
  }

  export interface ToggleProps extends NativeProps {
    pressed?: boolean;
    defaultPressed?: boolean;
    onPressedChange?: (pressed: boolean, event: QuickGuiEvent) => void;
  }

  export interface PopoverMenuRootProps {
    children?: unknown;
    /** Bounded menu model rebuilt by the Rust core on every declaration change. */
    items?: readonly MenuItem[];
    /** Structural geometry and paint for the declared rows. */
    appearance?: MenuAppearance;
    open?: boolean;
    defaultOpen?: boolean;
    onOpenChange?: (open: boolean, details: PopoverOpenChangeDetails) => void;
    onSelect?: (details: MenuSelectDetails, event: QuickGuiEvent) => void;
    placement?: PopoverPlacement;
    gap?: number;
    viewportMargin?: number;
    dismissOnEscape?: boolean;
    dismissOnPointerOutside?: boolean;
  }

  export interface PopoverMenuPopupProps extends NativeProps {
    /** Surface width. Defaults to the declared appearance width. */
    width?: number;
    onSelect?: EventHandler;
  }

  export interface ContextMenuRootProps {
    children?: unknown;
    items?: readonly MenuItem[];
    appearance?: MenuAppearance;
    onSelect?: (details: MenuSelectDetails, event: QuickGuiEvent) => void;
    /** Stable key shared by the compound's parts. One is generated when it is omitted. */
    scope?: string;
    /** Wrap the highlight at the ends of a level. Defaults to `true`. */
    loop?: boolean;
  }

  export interface ContextMenuTriggerProps extends NativeProps {
    onSelect?: EventHandler;
  }

  /** Base UI's `Menu.Root` props. The trigger carries them across the hosted boundary. */
  export interface MenuRootProps {
    children?: unknown;
    /** Stable key shared by every part of one menu level. One is generated when it is omitted. */
    scope?: string;
    open?: boolean;
    defaultOpen?: boolean;
    onOpenChange?: (open: boolean, event: QuickGuiEvent) => void;
    /** Contain Tab focus inside the popup and expect a mounted `Menu.Backdrop`. */
    modal?: boolean;
    /** `"horizontal"` installs the core's horizontal menu key context. */
    orientation?: "horizontal" | "vertical";
    /** Wrap the highlight at the ends of the level. Defaults to `true`. */
    loopFocus?: boolean;
    /** Dismissing this level with Escape closes the level above it too. */
    closeParentOnEsc?: boolean;
    /** Refuse to open at all. */
    disabled?: boolean;
    /** Trigger `openOnHover` default for the whole level. */
    openOnHover?: boolean;
    delay?: number;
    /** Grace period before a hover-opened level closes. Defaults to 100 ms. */
    closeDelay?: number;
    side?: "top" | "bottom" | "left" | "right";
    align?: "start" | "center" | "end";
    sideOffset?: number;
    alignOffset?: number;
    collisionPadding?: number;
    sticky?: boolean;
  }

  export interface MenuTriggerProps extends NativeProps {
    /** Open after `delay` while the pointer rests on the trigger. */
    openOnHover?: boolean;
    delay?: number;
    /** Grace period before a hover-opened level closes. Defaults to 100 ms. */
    closeDelay?: number;
  }

  export interface MenuPositionerProps extends NativeProps {
    side?: "top" | "bottom" | "left" | "right";
    align?: "start" | "center" | "end";
    sideOffset?: number;
    alignOffset?: number;
    collisionPadding?: number;
    sticky?: boolean;
  }

  /** One declared menu row. */
  export interface MenuItemProps extends NativeProps {
    /** The row's stable identifier. Required for every interactive row. */
    value?: string;
    /** Accessible name and typeahead label. Defaults to the row's own declared text. */
    label?: string;
    /** Override the core's default close policy for this row. */
    closeOnClick?: boolean;
    disabled?: boolean;
  }

  export interface MenuLinkItemProps extends MenuItemProps {
    /** The destination the core hands to its own open-URL path. */
    href?: string;
    /** Reported once the core has opened the destination. */
    onNavigate?: (href: string, event: QuickGuiEvent) => void;
  }

  export interface MenuCheckboxItemProps extends MenuItemProps {
    checked?: boolean;
    onCheckedChange?: (checked: boolean, event: QuickGuiEvent) => void;
  }

  export interface MenuRadioItemProps extends MenuItemProps {
    checked?: boolean;
  }

  export interface MenuRadioGroupProps extends NativeProps {
    /** The group's own name. Defaults to one derived from the declaring node. */
    name?: string;
    value?: string;
    defaultValue?: string;
    onValueChange?: (value: string, event: QuickGuiEvent) => void;
  }

  export interface MenuGroupLabelProps extends NativeProps {
    /** A stable identifier for the label row. */
    value?: string;
    label?: string;
  }

  export interface MenuSubmenuTriggerProps extends MenuTriggerProps, MenuItemProps {}

  export interface TooltipProviderProps extends NativeProps {
    /** Group open deadline in milliseconds. Defaults to the core's 600 ms. */
    delay?: number;
    /** Group close deadline in milliseconds. */
    closeDelay?: number;
    /** How long the group stays warm after the last tooltip closed, in milliseconds. */
    timeout?: number;
  }

  export interface TooltipRootProps {
    children?: unknown;
    open?: boolean;
    defaultOpen?: boolean;
    onOpenChange?: (open: boolean, event: QuickGuiEvent) => void;
    /** Cancel any pending deadline and close. The trigger stays focusable. */
    disabled?: boolean;
    /**
     * Let the pointer cross into the popup without closing. Defaults to `false`: a tooltip is a
     * passive help tag like AppKit's, and reaching its popup closes it. Base UI defaults to
     * `true`; opt in for a popup with content worth hovering.
     */
    hoverable?: boolean;
    /** Follow the cursor on one axis, both, or neither. */
    trackCursorAxis?: TooltipCursorAxis;
    /** Where the core really placed the popup. */
    onPlacementChange?: (placement: AnchorPlacementDetails, event: QuickGuiEvent) => void;
    /** Default positioning, overridden by whatever `Tooltip.Positioner` declares. */
    side?: "top" | "bottom" | "left" | "right";
    align?: "start" | "center" | "end";
    sideOffset?: number;
    collisionPadding?: number;
  }

  export interface TooltipTriggerProps extends NativeProps {
    /** Native element this trigger renders. Defaults to `button`. */
    element?: "button" | "view" | "text" | "input";
    /** Open deadline in milliseconds, overriding the provider's. */
    delay?: number;
    /** Close deadline in milliseconds, overriding the provider's. */
    closeDelay?: number;
    /** Close on press. Defaults to `true`. */
    closeOnClick?: boolean;
  }

  export interface TooltipPositionerProps extends NativeProps {
    side?: "top" | "bottom" | "left" | "right";
    align?: "start" | "center" | "end";
    sideOffset?: number;
    collisionPadding?: number;
  }

  export interface ToastProviderProps {
    children?: unknown;
    /** Inherited auto-dismiss duration in milliseconds. Defaults to the core's five seconds. */
    timeout?: number;
    /** How many toasts stay unlimited. Older ones are flagged, never silenced. Defaults to 3. */
    limit?: number;
    /** Whether the stack is expanded. */
    expanded?: boolean;
    /** Which way a swipe dismisses a toast. */
    swipeDirection?: "left" | "right" | "up" | "down";
    /** Stack pitch in logical pixels, which the core turns into each toast's own offset. */
    pitch?: number;
  }

  export interface DialogRootProps {
    children?: unknown;
    open?: boolean;
    defaultOpen?: boolean;
    onOpenChange?: (open: boolean, details: DialogOpenChangeDetails) => void;
    /** Dismiss on Escape. Defaults to `true` for both dialog kinds. */
    dismissOnEscape?: boolean;
    /** Dismiss on a backdrop press. Defaults to `true`, or `false` for an alert dialog. */
    dismissOnBackdrop?: boolean;
    /** How long the opening transition runs, in milliseconds. */
    enterDuration?: number;
    /**
     * How long the closing transition runs, in milliseconds.
     *
     * The core holds the dialog mounted for exactly this long so the application's own exit
     * transition can finish, then reports the completion.
     */
    exitDuration?: number;
    /** Base UI's `onOpenChangeComplete`: the transition the core just finished. */
    onOpenChangeComplete?: (open: boolean, event: QuickGuiEvent) => void;
  }

  /** A declared option source shared by the select, combobox, and autocomplete. */
  export interface PickerSourceProps extends NativeScopedProps {
    /**
     * Bounded option source, in either of Base UI's two shapes.
     *
     * An array of option objects, or the map form — one entry per value and its label. Omit it to
     * declare the options as child `Item` nodes instead.
     */
    items?: readonly OptionDeclaration[] | Readonly<Record<string, string>>;
    /** Structural geometry and paint for the rows the core renders in its own window. */
    appearance?: PickerAppearance;
    /**
     * Base UI's `filter`, answered by the core.
     *
     * `"contains"` and `"startsWith"` keep the source order, `"fuzzy"` ranks with the core's own
     * matcher and is the only mode that reports label highlight ranges, and `"none"` keeps a
     * source an application or service already filtered.
     */
    filterMode?: "fuzzy" | "contains" | "startsWith" | "none";
    /** Accessible name for the control the core decorates. */
    ariaLabel?: string;
    /** Reported when the core opens or closes its own native popover window. */
    onOpenChange?: (open: boolean, event: QuickGuiEvent) => void;
  }

  export interface SelectProps extends PickerSourceProps {
    /** Controlled selected option value. */
    value?: string;
    defaultValue?: string;
    onValueChange?: (value: string | undefined, event: QuickGuiEvent) => void;
    /** Accept more than one value, Base UI's `multiple`. */
    multiple?: boolean;
    /** Controlled value set of a multiple select, bounded by the core's own 256 values. */
    values?: readonly string[];
    defaultValues?: readonly string[];
    onValuesChange?: (values: readonly string[], event: QuickGuiEvent) => void;
    /** Require a value before form submission. */
    required?: boolean;
    /** Refuse every value change while staying focusable, unlike `disabled`. */
    readOnly?: boolean;
    /** Expect a mounted `Select.Backdrop`; QuickGUI adds no dimming of its own. */
    modal?: boolean;
    /** Line the selected row up with the trigger, Base UI's `alignItemWithTrigger`. */
    alignItemWithTrigger?: boolean;
    /** One edge per commit, even when the committed value did not move. */
    onCommit?: (details: CommitDetails, event: QuickGuiEvent) => void;
  }

  export interface ComboboxProps extends PickerSourceProps {
    value?: string;
    defaultValue?: string;
    placeholder?: string;
    /** Seeds the core's retained editing text. The core owns every edit after that. */
    inputValue?: string;
    onValueChange?: (value: string | undefined, event: QuickGuiEvent) => void;
    onInputValueChange?: (value: string, event: QuickGuiEvent) => void;
    /** Accept more than one value as chips, Base UI's `multiple`. */
    multiple?: boolean;
    /** Controlled chip set, bounded by the core's own 64 values. */
    values?: readonly string[];
    defaultValues?: readonly string[];
    onValuesChange?: (values: readonly string[], event: QuickGuiEvent) => void;
    /** Highlight the first result as soon as the query changes. */
    autoHighlight?: boolean;
    /** Open the suggestion surface when the input itself is pressed. Defaults to `true`. */
    openOnInputClick?: boolean;
    /** Move the highlight onto a hovered row. Defaults to `true`. */
    highlightItemOnHover?: boolean;
    /** Wrap the highlight at the ends of the result list. Defaults to `true`. */
    loopFocus?: boolean;
    /** Refuse every value change while staying focusable. */
    readOnly?: boolean;
    required?: boolean;
    onCommit?: (details: CommitDetails, event: QuickGuiEvent) => void;
  }

  export interface AutocompleteProps extends PickerSourceProps {
    placeholder?: string;
    /** Seeds the core's retained free-form text. The core owns every edit after that. */
    inputValue?: string;
    onInputValueChange?: (value: string, event: QuickGuiEvent) => void;
    onCommit?: (details: CommitDetails, event: QuickGuiEvent) => void;
  }

  /** A declared picker surface part. The core resolves the real placement in its own window. */
  export interface SelectPositionerProps extends NativeScopedProps {
    side?: "top" | "bottom" | "left" | "right";
    align?: "start" | "center" | "end";
    sideOffset?: number;
  }

  /** One declared chip of a multiple combobox. */
  export interface ComboboxChipProps extends NativeScopedProps {
    /** The chip's position in the set the core retained. */
    index?: number;
  }

  /** One declared option node. It contributes no element: the core paints every row itself. */
  export interface OptionProps extends NativeScopedProps {
    /** The option's stable value. */
    partValue?: string;
    /** Row label. Defaults to the value. */
    label?: string;
    /** Trailing hint text. */
    valueText?: string;
    /** Searchable group name. */
    group?: string;
  }

  export interface TableProps extends NativeScopedProps {
    columns?: readonly TableColumnDeclaration[];
    /** Total logical row count, up to the core's own million-row bound. */
    rowCount?: number;
    rowHeight?: number;
    headerHeight?: number;
    selectionMode?: "single" | "multiple";
    /** Controlled selection as inclusive `[start, end]` row ranges. */
    selection?: readonly (readonly number[])[];
    sort?: TableSortState;
    /** Controlled inline-edit position. Declaring it opens the editor over that cell. */
    editing?: TableCell;
    /** The range of rows the core is virtualizing. Declare exactly these `Row` children. */
    onVisibleRangeChange?: (range: VisibleRange, event: QuickGuiEvent) => void;
    onSelectionChange?: (ranges: readonly (readonly number[])[], event: QuickGuiEvent) => void;
    onSortChange?: (sort: TableSortState | undefined, event: QuickGuiEvent) => void;
    onActiveCellChange?: (cell: TableCell | undefined, event: QuickGuiEvent) => void;
    onColumnResize?: (widths: Readonly<Record<string, number>>, event: QuickGuiEvent) => void;
    onColumnReorder?: (order: readonly string[], event: QuickGuiEvent) => void;
    onEditEnd?: (details: TableEditEndDetails, event: QuickGuiEvent) => void;
    onActivate?: (cell: TableCell, event: QuickGuiEvent) => void;
  }

  export interface TableHeaderProps extends NativeProps {
    /** Declared column identifier this header paints. */
    column: string;
  }

  export interface TableRowProps extends NativeProps {
    /** Logical row index, inside the range the core reported as visible. */
    index: number;
  }

  export interface TableCellProps extends NativeProps {
    /** Declared column identifier, or use `index` for a positional cell. */
    column?: string;
    index?: number;
  }

  export interface TreeProps extends NativeScopedProps {
    nodes?: readonly TreeNodeDeclaration[];
    /** Controlled expanded node identifiers. */
    expanded?: readonly string[];
    defaultExpanded?: readonly string[];
    /** Controlled selected node identifier. */
    value?: string;
    rowHeight?: number;
    /** Row text the core paints while a pending branch is loading. */
    loadingLabel?: string;
    /** Where the core's behavior-only disclosure control is mounted inside a declared row. */
    disclosure?: "leading" | "trailing" | "none";
    /** One bounded, atomically validated lazy-children splice. */
    setChildren?: { id: string; children: readonly TreeNodeDeclaration[] };
    onExpandedChange?: (expanded: readonly string[], event: QuickGuiEvent) => void;
    onValueChange?: (value: string | undefined, event: QuickGuiEvent) => void;
    /** A pending branch asks for its children exactly once, when it is first expanded. */
    onLoadChildren?: (id: string, event: QuickGuiEvent) => void;
    onVisibleRangeChange?: (range: VisibleRange, event: QuickGuiEvent) => void;
    onActivate?: (id: string, event: QuickGuiEvent) => void;
  }

  export interface TreeRowProps extends NativeProps {
    /** Declared node identifier this row paints. */
    nodeId: string;
  }

  export interface NumberFieldProps extends NativeScopedProps {
    /** Alt-modified step. Defaults to a tenth of `step`. */
    smallStep?: number;
    /** Shift-modified step. Defaults to ten times `step`. */
    largeStep?: number;
    /** Land a stepped value on the step grid. */
    snapOnStep?: boolean;
    /** Step on the wheel while focused. Defaults to `true`. */
    allowWheelScrub?: boolean;
    /** Refuse every change while staying focusable, unlike `disabled`. */
    readOnly?: boolean;
    /** Required for form submission. */
    required?: boolean;
    /** Which axis a scrub gesture reads. Defaults to `"horizontal"`. */
    scrubDirection?: "horizontal" | "vertical" | "both";
    /** Logical pixels per step during a scrub. Defaults to the core's own two. */
    scrubSensitivity?: number;
    /** The core's own commit boundary, matching Base UI's `onValueCommitted`. */
    onValueCommitted?: (value: number | undefined, event: QuickGuiEvent) => void;
    /** Controlled numeric value. Omit for an empty field. */
    value?: number;
    defaultValue?: number;
    min?: number;
    max?: number;
    step?: number;
    /** Fractional digits used when the core formats a committed value. */
    precision?: number;
    onValueChange?: (value: number | undefined, valid: boolean, event: QuickGuiEvent) => void;
  }

  export interface NumberFieldInputProps extends InputProps {
    /** Stable key shared by every part of one number field. */
    scope?: string;
    /** Return commits: the core clamps into range and reformats before reporting. */
    onCommit?: (details: CommitDetails, event: QuickGuiEvent) => void;
  }

  export interface ToastProviderDeclarations {
    timeout?: number;
    limit?: number;
    expanded?: boolean;
    swipeDirection?: "left" | "right" | "up" | "down";
    pitch?: number;
    /** Everything the core decided about the queue: index, offset, limited and expanded flags. */
    onStackChange?: (stack: readonly ToastStackEntry[], event: QuickGuiEvent) => void;
  }

  export interface ToastViewportProps
    extends Omit<NativeScopedProps, "onDismiss">, ToastProviderDeclarations {
    /** The bounded queue. Adding an entry pushes a toast; dropping one dismisses it. */
    toasts?: readonly ToastDeclaration[];
    /** Every dismissal the core decided, including timed auto-dismissals. */
    onDismiss?: (ids: readonly string[], event: QuickGuiEvent) => void;
  }

  export interface ToastProps extends NativeScopedProps {
    /** Declared identifier of the queued toast this part belongs to. */
    toastId: string;
  }

  export interface DateFieldProps extends NativeScopedProps {
    /** Controlled ISO `YYYY-MM-DD` civil date. */
    value?: string;
    defaultValue?: string;
    min?: string;
    max?: string;
    /** Segment order. Defaults to ISO `ymd`. */
    format?: "ymd" | "dmy" | "mdy";
    onValueChange?: (value: string | undefined, event: QuickGuiEvent) => void;
  }

  export interface DateFieldSegmentProps extends NativeScopedProps {
    segment: "year" | "month" | "day";
  }

  export interface TimeFieldProps extends NativeScopedProps {
    /** Controlled `HH:MM` or `HH:MM:SS` civil time. */
    value?: string;
    defaultValue?: string;
    min?: string;
    max?: string;
    /** Add an AM/PM segment and display hours on a twelve-hour clock. */
    hour12?: boolean;
    /** Mount a seconds segment. Without it the core owns no seconds segment at all. */
    showSeconds?: boolean;
    onValueChange?: (value: string | undefined, event: QuickGuiEvent) => void;
  }

  export interface TimeFieldSegmentProps extends NativeScopedProps {
    segment: "hour" | "minute" | "second" | "period";
  }

  export interface CalendarProps extends NativeScopedProps {
    /** Controlled selected ISO `YYYY-MM-DD` civil date. */
    value?: string;
    defaultValue?: string;
    min?: string;
    max?: string;
    /** First weekday column, where Monday is `0` and Sunday is `6`. */
    firstWeekday?: number;
    onValueChange?: (value: string | undefined, event: QuickGuiEvent) => void;
    /** The day the grid's single Tab stop moved to. */
    onFocusChange?: (day: string, event: QuickGuiEvent) => void;
    /** The displayed month, as `YYYY-MM`. */
    onMonthChange?: (month: string, event: QuickGuiEvent) => void;
  }

  export interface CalendarWeekProps extends NativeScopedProps {
    /** Week row index inside the displayed month. */
    itemIndex?: number;
  }

  export interface CalendarDayProps extends NativeScopedProps {
    /** ISO `YYYY-MM-DD` civil date this cell paints. */
    day: string;
  }

  export interface MenubarProps extends NativeScopedProps {
    /** Number of declared menus. The core clamps it to its own bound. */
    count?: number;
    /** Controlled open menu index, or `null` for a closed bar. */
    open?: number | null;
    defaultOpen?: number;
    onOpenChange?: (open: number | undefined, event: QuickGuiEvent) => void;
    /** The menu that now owns the bar's single Tab stop. */
    onActiveChange?: (index: number, event: QuickGuiEvent) => void;
  }

  export interface MenubarItemProps extends NativeScopedProps {
    /** Position of this menu on the bar. */
    itemIndex?: number;
  }

  export interface SeparatorProps extends NativeProps {
    /** A horizontal rule divides stacked content; a vertical one divides a row. */
    orientation?: "horizontal" | "vertical";
  }

  export interface AvatarRootProps extends NativeProps {
    /** Accessible name for the whole avatar, announced exactly once however it renders. */
    ariaLabel?: string;
    /** Every load-status transition the core decided. */
    onLoadingStatusChange?: (status: AvatarLoadingStatus, event: QuickGuiEvent) => void;
  }

  export interface AvatarImageProps extends NativeProps {
    /** Filesystem path, `file://` URL, or a base64 `data:` URL. */
    src: string;
    fit?: "fill" | "contain" | "cover" | "scale-down" | "none";
    objectFit?: NonNullable<AvatarImageProps["fit"]>;
  }

  export interface AvatarFallbackProps extends NativeProps {
    /** Hold the fallback back for this many milliseconds after loading starts. */
    delay?: number;
  }

  export interface CheckboxGroupProps extends NativeProps {
    /** The complete, ordered universe the parent checkbox derives its state from. */
    allValues?: readonly string[];
    /** Controlled checked values. The core keeps them in the declared order. */
    value?: readonly string[];
    defaultValue?: readonly string[];
    disabled?: boolean;
    onValueChange?: (values: readonly string[], event: QuickGuiEvent) => void;
  }

  export interface PreviewCardRootProps extends NativeProps {
    open?: boolean;
    defaultOpen?: boolean;
    onOpenChange?: (open: boolean, event: QuickGuiEvent) => void;
    placement?: PopoverPlacement;
    gap?: number;
    viewportMargin?: number;
  }

  export interface PreviewCardTriggerProps extends NativeProps {
    /** Milliseconds the pointer must rest on the trigger before the card opens. */
    delay?: number;
    /** Milliseconds after the pointer leaves both the trigger and the popup. */
    closeDelay?: number;
  }

  export interface ScrollAreaRootProps extends NativeProps {
    /**
     * The viewport extent the application laid out.
     *
     * QuickGUI has no layout observer at the hosted boundary, so the extents the core's
     * arithmetic needs are declared ahead of its decision like every other bounded property.
     */
    viewportSize?: { width: number; height: number };
    /** The content extent the application laid out. */
    contentSize?: { width: number; height: number };
    /** Distance from an edge that still counts as being at that edge. Defaults to 1 pixel. */
    overflowEdgeThreshold?: number;
    /** Everything the core decided: the clamped offset and every derived overflow flag. */
    onScrollStateChange?: (state: ScrollAreaState, event: QuickGuiEvent) => void;
  }

  export interface ScrollAreaScrollbarProps extends NativeProps {
    orientation?: "horizontal" | "vertical";
    /**
     * Keep the scrollbar mounted while its axis cannot scroll.
     *
     * The core keeps this per scroll area, so one kept scrollbar keeps the whole area's
     * scrollbars and corner mounted. A mounted-but-useless scrollbar is hidden from assistive
     * technology, so it is never announced.
     */
    keepMounted?: boolean;
  }

  export interface ScrollAreaThumbProps extends NativeProps {
    /** Defaults to the enclosing `ScrollArea.Scrollbar`'s orientation. */
    orientation?: "horizontal" | "vertical";
  }

  export interface OtpFieldRootProps extends NativeProps {
    /** Controlled code. Characters outside the accepted class are dropped by the core. */
    value?: string;
    defaultValue?: string;
    /** Slots retained by the field, bounded by the core's own maximum of twelve. */
    length?: number;
    /** Accepted character class. Defaults to `"numeric"`. */
    validationType?: "numeric" | "alpha" | "alphanumeric" | "none";
    /** Present the code the way a password input is presented. */
    mask?: boolean;
    disabled?: boolean;
    readOnly?: boolean;
    required?: boolean;
    /** Submit this form through the core as soon as the final slot is filled. */
    autoSubmit?: string;
    onValueChange?: (value: string, event: QuickGuiEvent) => void;
    /** The transition into a full code, which is an edge rather than a value. */
    onComplete?: (value: string, event: QuickGuiEvent) => void;
  }

  export interface OtpFieldInputProps extends Omit<NativeProps, "onInput" | "value"> {
    /** The slot this input paints. */
    index: number;
  }

  export interface OtpFieldSeparatorProps extends NativeProps {
    /** Position between two slots, used only to keep the separator's identity stable. */
    index?: number;
  }

  export interface DrawerRootProps extends NativeProps {
    open?: boolean;
    defaultOpen?: boolean;
    onOpenChange?: (open: boolean, event: QuickGuiEvent) => void;
    /** `true` contains focus and projects modal semantics; `"trap-focus"` only contains focus. */
    modal?: boolean | "trap-focus";
    /** The edge a swipe dismisses toward. Defaults to `"down"`. */
    swipeDirection?: "up" | "down" | "left" | "right";
    /** A point at or below `1` is a fraction of the viewport extent; a larger one is pixels. */
    snapPoints?: readonly number[];
    /** Which declared snap point an opening drawer lands on. */
    snapPoint?: number;
    onSnapPointChange?: (index: number, event: QuickGuiEvent) => void;
    /** Refuse a backdrop or swipe dismissal, leaving Escape and the close control. */
    disablePointerDismissal?: boolean;
    /** The live swipe the core decided, for the paint-only transform the application applies. */
    onSwipeChange?: (swipe: DrawerSwipeState, event: QuickGuiEvent) => void;
  }

  export interface NavigationMenuRootProps extends NativeProps {
    /** The open item, or `undefined` while every panel is closed. */
    value?: string;
    defaultValue?: string;
    onValueChange?: (value: string | undefined, event: QuickGuiEvent) => void;
    orientation?: "horizontal" | "vertical";
    /** Milliseconds a pointer rests on a trigger before its panel opens. Defaults to 50. */
    delay?: number;
    /** Milliseconds after the pointer leaves both a trigger and its popup. Defaults to 50. */
    closeDelay?: number;
    loopFocus?: boolean;
    placement?: PopoverPlacement;
    /**
     * The ordered navigation model.
     *
     * Omit it and the core reads the mounted `NavigationMenu.Item` children in declaration order;
     * declare it to name disabled items or to keep a model the children do not spell out.
     */
    items?: readonly ComponentItemDeclaration[];
    /** The direction the user's attention travelled, so the application can slide its panel. */
    onActivationDirectionChange?: (
      direction: NavigationMenuActivationDirection,
      event: QuickGuiEvent,
    ) => void;
  }

  export interface NavigationMenuItemProps extends NativeProps {
    /** The item's stable value, inherited by every part inside it. */
    value: string;
  }

  export interface NavigationMenuPartProps extends NativeProps {
    /** Defaults to the enclosing `NavigationMenu.Item`'s value. */
    value?: string;
  }

  /** A trigger's activation belongs to the core, so it declares no click listener of its own. */
  export interface NavigationMenuTriggerProps extends Omit<NavigationMenuPartProps, "onClick"> {}

  export interface NavigationMenuLinkProps extends NativeProps {
    /** Stable identity for the link. */
    value: string;
    /** Marks the link for the current destination as the native selected state. */
    active?: boolean;
  }

  export interface IntrinsicElements {
    view: NativeProps;
    /** Native View alias; no component import is required. */
    div: NativeProps;
    text: NativeProps;
    /** Native Text alias; no component import is required. */
    span: NativeProps;
    button: NativeProps;
    input: InputProps;
    textarea: InputProps;
    "virtual-list": VirtualListProps;
    svg: SvgProps;
    image: ImageProps;
    shader: ShaderProps;
  }
}

export * from "./router.ts";

export const TextInput = Input;
export type NativeProps = JSX.NativeProps;
export type NativeStyle = JSX.Style;
