// macOS-only SwiftUI host tests. The Linux `test:typescript` suite ignores this file;
// CI runs it on the macOS quality gate via `bun run test:swift-ui`.
import { describe, expect, test } from "bun:test";
import {
  app,
  NativeNodeTag,
  PropertyCode,
  QuickGuiEvent,
  Window,
  type NativeNode,
} from "@quickgui/native";
import { createComponent, createRenderer, Text, View } from "../src/index.ts";
import {
  Button,
  ColorPicker,
  DatePicker,
  Gauge,
  Host,
  Picker,
  Popover,
  PopoverContent,
  PopoverTrigger,
  ProgressView,
  QuickGUIHostView,
  SecureField,
  SegmentedControl,
  Slider,
  Stepper,
  TextField,
  Toggle,
} from "../src/swift-ui.ts";
import {
  buttonBorderShape,
  buttonStyle,
  controlSize,
  disabled,
  labelStyle,
  tint,
} from "../src/swift-ui/modifiers.ts";

describe("SwiftUI Solid bridge", () => {
  test("builds a retained Host and native Button description", () => {
    let presses = 0;
    const button = createComponent(Button, {
      label: "Save changes",
      systemImage: "square.and.arrow.down",
      role: "default",
      target: "save",
      testID: "save-button",
      modifiers: [
        buttonStyle("glass"),
        controlSize("large"),
        buttonBorderShape("roundedRectangle", 14),
        labelStyle("titleAndIcon"),
        tint("#3366ffff"),
        disabled(false),
      ],
      onPress: () => presses++,
    });
    const host = createComponent(Host, {
      matchContents: true,
      children: button,
    });

    expect(host.tag).toBe(NativeNodeTag.SwiftUIHost);
    expect(host.properties.get(PropertyCode.SwiftUIMatchContentsHorizontal)).toBe(true);
    expect(host.properties.get(PropertyCode.SwiftUIMatchContentsVertical)).toBe(true);
    expect(host.children).toEqual([button]);
    expect(button.tag).toBe(NativeNodeTag.SwiftUIButton);
    expect(button.properties.get(PropertyCode.Value)).toBe("Save changes");
    expect(button.properties.get(PropertyCode.SwiftUISystemImage)).toBe("square.and.arrow.down");
    expect(button.properties.get(PropertyCode.SwiftUITarget)).toBe("save");
    expect(button.properties.get(PropertyCode.SwiftUITestId)).toBe("save-button");
    expect(JSON.parse(button.properties.get(PropertyCode.SwiftUIModifiers) as string)).toEqual([
      { $type: "buttonStyle", style: "glass" },
      { $type: "controlSize", size: "large" },
      {
        $type: "buttonBorderShape",
        shape: "roundedRectangle",
        cornerRadius: 14,
      },
      { $type: "labelStyle", style: "titleAndIcon" },
      { $type: "tint", color: "#3366ffff" },
      { $type: "disabled", disabled: false },
    ]);
    expect(button.properties.get(PropertyCode.ClickListener)).toBe(true);

    button.listeners.get("click")!(new QuickGuiEvent("click", button));
    expect(presses).toBe(1);
  });

  test("builds controlled native SwiftUI form controls and typed events", () => {
    const numbers: number[] = [];
    const toggles: boolean[] = [];
    const text: string[] = [];
    const selections: string[] = [];
    const colors: string[] = [];
    const dates: Date[] = [];
    let submits = 0;
    const slider = createComponent(Slider, {
      value: 0.4,
      min: 0,
      max: 1,
      step: 0.1,
      label: "Volume",
      testID: "volume-slider",
      modifiers: [controlSize("small")],
      onValueChange: (value) => numbers.push(value),
    });
    const toggle = createComponent(Toggle, {
      isOn: false,
      label: "Notifications",
      onIsOnChange: (isOn) => toggles.push(isOn),
    });
    const progress = createComponent(ProgressView, {
      value: 3,
      total: 10,
      label: "Uploading",
      currentValueLabel: "3 of 10",
    });
    const stepper = createComponent(Stepper, {
      value: 2,
      min: 1,
      max: 5,
      step: 1,
      label: "Copies",
      onValueChange: (value) => numbers.push(value),
    });
    const field = createComponent(TextField, {
      value: "Ada",
      placeholder: "Name",
      onValueChange: (value) => text.push(value),
      onSubmit: () => submits++,
    });
    const secure = createComponent(SecureField, {
      value: "secret",
      placeholder: "Password",
    });
    const picker = createComponent(Picker, {
      selection: "grid",
      options: [
        { value: "list", label: "List", systemImage: "list.bullet" },
        { value: "grid", label: "Grid" },
      ],
      label: "Layout",
      style: "menu",
      onSelectionChange: (selection) => selections.push(selection),
    });
    const segmented = createComponent(SegmentedControl, {
      selection: "day",
      options: [
        { value: "day", label: "Day" },
        { value: "week", label: "Week" },
      ],
      onSelectionChange: (selection) => selections.push(selection),
    });
    const segmentedTabs = createComponent(SegmentedControl, {
      role: "tabs",
      selection: "list",
      options: [
        { value: "list", label: "List" },
        { value: "grid", label: "Grid" },
      ],
    });
    const date = createComponent(DatePicker, {
      value: new Date("2026-09-04T12:30:00.250Z"),
      min: new Date("2026-01-01T00:00:00.000Z"),
      label: "When",
      displayedComponents: "dateAndTime",
      style: "field",
      onValueChange: (value) => dates.push(value),
    });
    const color = createComponent(ColorPicker, {
      selection: "#3366ffff",
      label: "Accent",
      supportsOpacity: false,
      onSelectionChange: (selection) => colors.push(selection),
    });
    const gauge = createComponent(Gauge, {
      value: 0.72,
      min: 0,
      max: 1,
      label: "Battery",
      currentValueLabel: "72%",
      minimumValueLabel: "0%",
      maximumValueLabel: "100%",
      style: "accessoryLinearCapacity",
    });

    expect(slider.tag).toBe(NativeNodeTag.SwiftUISlider);
    expect(slider.properties.get(PropertyCode.Value)).toBe(0.4);
    expect(slider.properties.get(PropertyCode.Minimum)).toBe(0);
    expect(slider.properties.get(PropertyCode.Maximum)).toBe(1);
    expect(slider.properties.get(PropertyCode.Step)).toBe(0.1);
    expect(slider.properties.get(PropertyCode.SwiftUITestId)).toBe("volume-slider");
    expect(slider.children[0]?.text).toBe("Volume");
    expect(slider.properties.get(PropertyCode.InputListener)).toBe(true);

    expect(toggle.tag).toBe(NativeNodeTag.SwiftUIToggle);
    expect(toggle.properties.get(PropertyCode.Checked)).toBe(false);
    expect(toggle.children[0]?.text).toBe("Notifications");
    expect(progress.tag).toBe(NativeNodeTag.SwiftUIProgressView);
    expect(progress.properties.get(PropertyCode.Value)).toBe(3);
    expect(progress.properties.get(PropertyCode.Maximum)).toBe(10);
    expect(progress.properties.get(PropertyCode.ValueText)).toBe("3 of 10");
    expect(stepper.tag).toBe(NativeNodeTag.SwiftUIStepper);
    expect(stepper.properties.get(PropertyCode.Value)).toBe(2);
    expect(field.tag).toBe(NativeNodeTag.SwiftUITextField);
    expect(field.properties.get(PropertyCode.Value)).toBe("Ada");
    expect(field.properties.get(PropertyCode.Placeholder)).toBe("Name");
    expect(field.properties.get(PropertyCode.Password)).toBe(false);
    expect(field.properties.get(PropertyCode.SubmitListener)).toBe(true);
    expect(secure.tag).toBe(NativeNodeTag.SwiftUITextField);
    expect(secure.properties.get(PropertyCode.Password)).toBe(true);
    expect(picker.tag).toBe(NativeNodeTag.SwiftUIPicker);
    expect(picker.properties.get(PropertyCode.Value)).toBe("grid");
    expect(JSON.parse(picker.properties.get(PropertyCode.Items) as string)).toEqual([
      { value: "list", label: "List", systemImage: "list.bullet" },
      { value: "grid", label: "Grid" },
    ]);
    expect(picker.properties.get(PropertyCode.SwiftUIPickerStyle)).toBe("menu");
    expect(segmented.tag).toBe(NativeNodeTag.SwiftUIPicker);
    expect(segmented.properties.get(PropertyCode.SwiftUIPickerStyle)).toBe("segmented");
    expect(segmentedTabs.properties.get(PropertyCode.SwiftUIPickerStyle)).toBe("segmented");
    expect(segmentedTabs.properties.get(PropertyCode.Role)).toBe("tabs");
    expect(date.tag).toBe(NativeNodeTag.SwiftUIDatePicker);
    expect(date.properties.get(PropertyCode.CivilValue)).toBe("1788525000.25");
    expect(date.properties.get(PropertyCode.CivilMinimum)).toBe("1767225600");
    expect(date.properties.get(PropertyCode.SwiftUIDatePickerComponents)).toBe("dateAndTime");
    expect(date.properties.get(PropertyCode.SwiftUIDatePickerStyle)).toBe("field");
    expect(color.tag).toBe(NativeNodeTag.SwiftUIColorPicker);
    expect(color.properties.get(PropertyCode.SwiftUIColorSupportsOpacity)).toBe(false);
    expect(gauge.tag).toBe(NativeNodeTag.SwiftUIGauge);
    expect(gauge.properties.get(PropertyCode.SwiftUIGaugeStyle)).toBe("accessoryLinearCapacity");
    expect(gauge.properties.get(PropertyCode.SwiftUIGaugeMinimumValueLabel)).toBe("0%");

    slider.listeners.get("input")!(new QuickGuiEvent("input", slider, "0.75"));
    toggle.listeners.get("input")!(new QuickGuiEvent("input", toggle, "true"));
    stepper.listeners.get("input")!(new QuickGuiEvent("input", stepper, "4"));
    field.listeners.get("input")!(new QuickGuiEvent("input", field, "Grace"));
    field.listeners.get("submit")!(new QuickGuiEvent("submit", field));
    picker.listeners.get("input")!(new QuickGuiEvent("input", picker, "list"));
    segmented.listeners.get("input")!(new QuickGuiEvent("input", segmented, "week"));
    date.listeners.get("input")!(new QuickGuiEvent("input", date, "1788528600.5"));
    color.listeners.get("input")!(new QuickGuiEvent("input", color, "#ff000080"));
    expect(numbers).toEqual([0.75, 4]);
    expect(toggles).toEqual([true]);
    expect(text).toEqual(["Grace"]);
    expect(submits).toBe(1);
    expect(selections).toEqual(["list", "week"]);
    expect(dates).toEqual([new Date("2026-09-04T13:30:00.500Z")]);
    expect(colors).toEqual(["#ff000080"]);
  });

  test("supports per-axis intrinsic sizing", () => {
    const host = createComponent(Host, {
      matchContents: { vertical: true },
      children: createComponent(Button, { label: "Save" }),
    });

    expect(host.properties.has(PropertyCode.SwiftUIMatchContentsHorizontal)).toBe(false);
    expect(host.properties.get(PropertyCode.SwiftUIMatchContentsVertical)).toBe(true);
  });

  test("composes a rendered trigger with controlled native Popover behavior", () => {
    let isPresented = false;
    const changes: boolean[] = [];
    let renderedTrigger: NativeNode | undefined;
    const popover = createComponent(Popover, {
      get isPresented() {
        return isPresented;
      },
      attachmentAnchor: "bottom",
      arrowEdge: "top",
      onIsPresentedChange(presented) {
        isPresented = presented;
        changes.push(presented);
      },
      get children() {
        const button = createComponent(Button, { label: "Open" });
        return [
          createComponent(PopoverTrigger, {
            render: button,
            ref: (node) => {
              renderedTrigger = node;
            },
          }),
          createComponent(PopoverContent, {
            children: createComponent(Button, { label: "Inside" }),
          }),
        ];
      },
    });

    expect(popover.tag).toBe(NativeNodeTag.SwiftUIPopover);
    expect(popover.properties.has(PropertyCode.SwiftUIIsPresented)).toBe(false);
    expect(popover.properties.get(PropertyCode.SwiftUIAttachmentAnchor)).toBe("bottom");
    expect(popover.properties.get(PropertyCode.SwiftUIArrowEdge)).toBe("top");
    expect(popover.listeners.has("presentationchange")).toBe(true);
    expect(popover.children.map((child) => child.tag)).toEqual([
      NativeNodeTag.SwiftUIPopoverTrigger,
      NativeNodeTag.SwiftUIPopoverContent,
    ]);
    const trigger = popover.children[0]!;
    const button = trigger.children[0]!;
    expect(renderedTrigger).toBe(button);
    expect(button.tag).toBe(NativeNodeTag.SwiftUIButton);
    expect(trigger.properties.get(PropertyCode.ClickListener)).toBe(true);

    const press = new QuickGuiEvent("click", button);
    let current: NativeNode | undefined = button;
    while (current) {
      press.currentTarget = current;
      current.listeners.get("click")?.(press);
      if (press.propagationStopped) break;
      current = current.parent;
    }
    expect(changes).toEqual([true]);

    popover.listeners.get("presentationchange")!(
      new QuickGuiEvent("presentationchange", popover, "false"),
    );
    expect(changes).toEqual([true, false]);
  });

  test("lets the rendered trigger prevent Popover presentation", () => {
    const changes: boolean[] = [];
    const popover = createComponent(Popover, {
      isPresented: false,
      onIsPresentedChange: (presented) => changes.push(presented),
      get children() {
        return [
          createComponent(PopoverTrigger, {
            render: createComponent(Button, {
              label: "Open",
              onPress: (event) => event.preventDefault(),
            }),
          }),
          createComponent(PopoverContent, {
            children: createComponent(Button, { label: "Inside" }),
          }),
        ];
      },
    });
    const button = popover.children[0]!.children[0]!;
    const press = new QuickGuiEvent("click", button);
    let current: NativeNode | undefined = button;
    while (current) {
      press.currentTarget = current;
      current.listeners.get("click")?.(press);
      if (press.propagationStopped) break;
      current = current.parent;
    }

    expect(press.defaultPrevented).toBe(true);
    expect(changes).toEqual([]);
  });

  test("owns an ordinary QuickGUI renderer for QuickGUIHostView", async () => {
    await app.whenReady();
    let reverseHost: NativeNode | undefined;
    const owner = new Window({
      title: "SwiftUI reverse host test",
      width: 320,
      height: 180,
      renderer: createRenderer(() =>
        createComponent(Host, {
          matchContents: true,
          children: createComponent(QuickGUIHostView, {
            width: 220,
            height: 96,
            ref: (node) => {
              reverseHost = node;
            },
            children: createComponent(View, {
              children: createComponent(Text, {
                children: "Ordinary QuickGUI",
              }),
            }),
          }),
        }),
      ),
    });

    expect(reverseHost?.tag).toBe(NativeNodeTag.SwiftUIQuickGUIHost);
    const embeddedId = reverseHost?.properties.get(PropertyCode.SwiftUIEmbeddedWindow);
    expect(typeof embeddedId).toBe("number");
    const embedded = app.windows.get(embeddedId as number);
    expect(embedded).toBeDefined();
    expect(embedded?.root.children[0]?.children[0]?.children[0]?.children[0]?.text).toBe(
      "Ordinary QuickGUI",
    );

    owner.close();
    await Promise.resolve();
    expect(embedded?.closed).toBe(true);
  });
});
