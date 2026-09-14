package main

import (
	"fmt"
	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/ui"
	"slices"
	"strconv"
	"strings"
)

const gaugeWidth = 420

func gaugeReadout() *ui.Element {
	state := ui.UseGaugeState()
	return note(func() string {
		s := state()
		return "status " + s.Status + " · display " + textValue(s.DisplayValue) + " · completion " + textValue(s.Completion)
	})
}
func gaugeTrack() ui.PartProps {
	return ui.PartProps{Style: func() ui.StyleBuilder {
		return ui.Style().
			Display("flex").
			Width(gaugeWidth).
			Height(8).
			BorderRadius(4).
			BackgroundColor(p().Track).
			Overflow("hidden")
	}}
}
func sliderParts(slider *ui.SliderComponent, values func() []float64) *native.Node {
	return slider.Control(ui.PartProps{Style: ui.Style().
		Position("relative").
		Display("flex").
		AlignItems("center").
		Width(gaugeWidth).
		Height(24)}).
		Child(func() *native.Node {
			var children []*native.Node
			children = append(children, slider.Track(ui.PartProps{Style: func() ui.StyleBuilder {
				return ui.Style().
					Display("flex").
					Width(gaugeWidth).
					Height(4).
					BorderRadius(2).
					BackgroundColor(p().Track)
			}}).
				Child(func() *native.Node {
					var children []*native.Node
					style := func() ui.StyleBuilder {
						v := values()
						left, width := 0.0, v[0]
						if len(v) > 1 {
							left, width = v[0], v[1]-v[0]
						}
						return ui.Style().
							Height(4).
							BorderRadius(2).
							BackgroundColor(p().Accent).
							MarginLeft(left * gaugeWidth / 100).
							Width(strconv.FormatFloat(width, 'g', -1, 64) + "%")
					}
					if len(values()) > 1 {
						children = append(children, slider.Range(ui.PartProps{Style: style}).NativeNode())
					} else {
						children = append(children, slider.Indicator(ui.PartProps{Style: style}).NativeNode())
					}
					return ui.Fragment(children)
				}).
				NativeNode())
			for i := range values() {
				children = append(children, slider.Thumb(ui.SliderThumbProps{
					Index: ptr(i),
					PartProps: ui.PartProps{Style: func() ui.StyleBuilder {
						return ui.Style().
							Position("absolute").
							Left(values()[i]*gaugeWidth/100 - 10).
							Top(2).
							Width(20).
							Height(20).
							BorderRadius(10).
							BackgroundColor(p().Panel).
							BorderWidth(1).
							BorderColor(p().Border).
							BoxShadow("0 1px 2px #0000003d").
							FocusStyle(func(s ui.StyleBuilder) ui.StyleBuilder {
								return s.Outline("2px solid " + p().Accent)
							}).
							OutlineOffset(2)
					}},
				}).
					NativeNode())
			}
			return ui.Fragment(children)
		}).
		NativeNode()
}
func MeterDemo() *ui.Element {
	level, setLevel := ui.CreateSignal([]float64{62})
	return panel("Meter", "A known range, with low/high/optimum thresholds and the core's formatted gauge state.", func() *native.Node {
		return ui.Fragment([]*native.Node{
			func() *native.Node {
				meter35 := ui.NewMeter(ui.MeterProps{
					Value: func() *float64 {
						return ptr(level()[0])
					},
					Min:     0,
					Max:     100,
					Low:     25,
					High:    80,
					Optimum: 50,
					GaugeFormatProps: ui.GaugeFormatProps{
						Format:    "percent",
						PartProps: columnPart(),
					},
				})
				return meter35.Root().
					Children(func() *native.Node {
						return ui.Fragment([]*native.Node{
							row(func() *native.Node {
								return ui.Fragment([]*native.Node{
									meter35.Label(ui.PartProps{}).Children("Disk used").NativeNode(),
									meter35.Value(ui.PartProps{}).
										Children(func() *ui.Element {
											return label(func() string {
												return strconv.FormatFloat(level()[0], 'f', 0, 64) + "%"
											})
										}).
										NativeNode(),
								})
							}).Node,
							meter35.Track(gaugeTrack()).
								Children(func() *native.Node {
									return meter35.Indicator(ui.PartProps{Style: func() ui.StyleBuilder {
										value := level()[0]
										return ui.Style().
											Height(8).
											BorderRadius(4).
											Width(strconv.FormatFloat(value, 'g', -1, 64) + "%").
											BackgroundColor(choose(value < 25, p().Danger, choose(value > 80, "#c88a00", p().Accent)))
									}}).
										NativeNode()
								}).
								NativeNode(),
							gaugeReadout().Node,
						})
					}).
					NativeNode()
			}(),
			func() *native.Node {
				slider36 := ui.NewSlider(ui.SliderRootProps{
					Value:         level,
					Min:           0,
					Max:           100,
					Step:          1,
					OnValueChange: change(setLevel),
				})
				return slider36.Root().
					Children(func() *native.Node {
						return sliderParts(slider36, level)
					}).
					NativeNode()
			}(),
		})
	})
}
func ProgressDemo() *ui.Element {
	done, setDone := ui.CreateSignal(3)
	indeterminate, setIndeterminate := ui.CreateSignal(false)
	return panel("Progress", "Determinate and indeterminate task progress. No timer keeps the gallery awake.", func() *native.Node {
		return ui.Fragment([]*native.Node{
			func() *native.Node {
				progress37 := ui.NewProgress(ui.ProgressProps{
					Value: func() *float64 {
						if indeterminate() {
							return nil
						}
						return ptr(float64(done()))
					},
					Max:           12,
					Indeterminate: indeterminate,
					ValueText: func() *string {
						return ptr(strconv.Itoa(done()) + " of 12 files")
					},
					GaugeFormatProps: ui.GaugeFormatProps{
						Format:    "fraction",
						PartProps: columnPart(),
					},
				})
				return progress37.Root().
					Children(func() *native.Node {
						return ui.Fragment([]*native.Node{
							row(func() *native.Node {
								return ui.Fragment([]*native.Node{
									progress37.Label(ui.PartProps{}).
										Children("Uploading").
										NativeNode(),
									progress37.Value(ui.PartProps{}).
										Children(func() *ui.Element {
											return label(func() string {
												return choose(indeterminate(), "…", strconv.Itoa(done())+" / 12")
											})
										}).
										NativeNode(),
								})
							}).Node,
							progress37.Track(gaugeTrack()).
								Children(func() *native.Node {
									return progress37.Indicator(ui.PartProps{Style: func() ui.StyleBuilder {
										return ui.Style().
											Height(8).
											BorderRadius(4).
											BackgroundColor(p().Accent).
											Width(strconv.FormatFloat(choose(indeterminate(), 35.0, float64(done())/12*100), 'g', -1, 64) + "%")
									}}).
										NativeNode()
								}).
								NativeNode(),
							gaugeReadout().Node,
						})
					}).
					NativeNode()
			}(),
			row(func() *native.Node {
				return ui.Fragment([]*native.Node{
					button("−1", func() {
						setDone(max(0, done()-1))
					}).Node,
					button("+1", func() {
						setDone(min(12, done()+1))
					}).Node,
					button("Complete", func() {
						setDone(12)
					}).Node,
					button(func() string {
						return choose(indeterminate(), "Determinate", "Indeterminate")
					}, func() {
						setIndeterminate(!indeterminate())
					}).Node,
				})
			}).Node,
		})
	})
}
func RadioDemo() *ui.Element {
	theme, setTheme := ui.CreateSignal("system")
	radio := func(value string, selected func() bool, caption string) *native.Node {
		radio38 := ui.NewRadio(ui.RadioProps{
			Value:     value,
			PartProps: ui.PartProps{Style: checkboxStyle()},
		})
		return radio38.Root().
			Children(func() *native.Node {
				return ui.Fragment([]*native.Node{
					radio38.Indicator(ui.PartProps{Style: func() ui.StyleBuilder {
						return ui.Style().
							Width(15).
							Height(15).
							BorderRadius(8).
							BorderWidth(choose(selected(), 4, 1)).
							BorderColor(choose(selected(), p().Accent, p().Border)).
							BackgroundColor(p().Control)
					}}).
						NativeNode(),
					label(caption).Node,
				})
			}).
			NativeNode()
	}
	return panel("Radio", "Arrow keys select within a group; read-only groups retain their single Tab stop.", func() *native.Node {
		return ui.Fragment([]*native.Node{
			func() *native.Node {
				radioGroup39 := ui.NewRadioGroup(ui.RadioGroupProps{
					Value: func() *string {
						return ptr(theme())
					},
					OnValueChange: change(setTheme),
					Required:      true,
					PartProps:     columnPart(),
				})
				return radioGroup39.Root().
					Children(func() *native.Node {
						var choices []*native.Node
						for _, value := range []string{"light", "dark", "system"} {
							choices = append(choices, radio(value, func() bool {
								return theme() == value
							}, value))
						}
						return ui.Fragment(choices)
					}).
					NativeNode()
			}(),
			func() *native.Node {
				radioGroup40 := ui.NewRadioGroup(ui.RadioGroupProps{
					DefaultValue: "b",
					ReadOnly:     true,
					PartProps:    rowPart(),
				})
				return radioGroup40.Root().
					Children(func() *native.Node {
						var choices []*native.Node
						for _, value := range []string{"a", "b", "c"} {
							choices = append(choices, radio(value, func() bool {
								return value == "b"
							}, "read-only "+value))
						}
						return ui.Fragment(choices)
					}).
					NativeNode()
			}(),
			note(func() string {
				return "theme " + theme() + " · read-only group b"
			}).Node,
		})
	})
}
func SeparatorDemo() *ui.Element {
	return panel("Separator", "Orientation, with application-defined thickness and colour. An adjustable divider is Splitter.", func() *native.Node {
		return ui.Fragment([]*native.Node{
			label("Above the rule").Node,
			func() *native.Node {
				separator41 := ui.NewSeparator(ui.SeparatorProps{
					Orientation: "horizontal",
					PartProps: ui.PartProps{Style: ui.Style().
						Height(1).
						BackgroundColor(color(func(p palette) string {
							return p.Border
						}))},
				})
				return separator41.Root().NativeNode()
			}(),
			label("Below the rule").Node,
			row(func() *native.Node {
				var children []*native.Node
				for i, caption := range []string{"Cut", "Copy", "Paste"} {
					if i > 0 {
						children = append(children, func() *native.Node {
							separator42 := ui.NewSeparator(ui.SeparatorProps{
								Orientation: "vertical",
								PartProps: ui.PartProps{Style: ui.Style().
									Width(1).
									Height(18).
									BackgroundColor(color(func(p palette) string {
										return p.Border
									}))},
							})
							return separator42.Root().NativeNode()
						}())
					}
					children = append(children, label(caption).Node)
				}
				return ui.Fragment(children)
			}).Node,
		})
	})
}
func SliderDemo() *ui.Element {
	volume, setVolume := ui.CreateSignal([]float64{40})
	committed, setCommitted := ui.CreateSignal("—")
	interval, setInterval := ui.CreateSignal([]float64{20, 70})
	return panel("Slider", "Clamping, step snapping, thumb ordering, and captured drag arithmetic belong to the core.", func() *native.Node {
		return ui.Fragment([]*native.Node{
			func() *native.Node {
				slider43 := ui.NewSlider(ui.SliderRootProps{
					Value:         volume,
					Min:           0,
					Max:           100,
					Step:          5,
					LargeStep:     25,
					Format:        "percent",
					OnValueChange: change(setVolume),
					OnValueCommitted: func(v []float64, _ *native.Event) {
						setCommitted(fmt.Sprint(v))
					},
					PartProps: columnPart(),
				})
				return slider43.Root().
					Children(func() *native.Node {
						return ui.Fragment([]*native.Node{
							slider43.Label(ui.PartProps{}).Children("Volume").NativeNode(),
							slider43.Value(ui.PartProps{}).
								Children(func() *ui.Element {
									state := ui.UseSliderState()
									return label(func() string {
										return textValue(state().DisplayValue) + " · dragging " + strconv.FormatBool(state().Dragging)
									})
								}).
								NativeNode(),
							sliderParts(slider43, volume),
						})
					}).
					NativeNode()
			}(),
			note(func() string {
				return fmt.Sprintf("volume %v · committed %s", volume(), committed())
			}).Node,
			func() *native.Node {
				slider44 := ui.NewSlider(ui.SliderRootProps{
					Value:                 interval,
					Min:                   0,
					Max:                   100,
					Step:                  1,
					MinStepsBetweenValues: 5,
					OnValueChange:         change(setInterval),
					PartProps:             columnPart(),
				})
				return slider44.Root().
					Children(func() *native.Node {
						return ui.Fragment([]*native.Node{
							slider44.Label(ui.PartProps{}).
								Children("Range · five steps between thumbs").
								NativeNode(),
							sliderParts(slider44, interval),
						})
					}).
					NativeNode()
			}(),
			note(func() string {
				return fmt.Sprintf("range %v", interval())
			}).Node,
		})
	})
}
func SplitterDemo() *ui.Element {
	sizes, setSizes := ui.CreateSignal([]float64{200, 160, 160})
	return panel("Splitter", "Drag or use arrow keys. The core conserves the total, enforces minima, and can collapse the last pane.", func() *native.Node {
		return ui.Fragment([]*native.Node{
			func() *native.Node {
				splitter45 := ui.NewSplitter(ui.SplitterRootProps{
					Value:         sizes,
					Step:          8,
					OnSizesChange: change(setSizes),
					Panes: []ui.SplitterPaneDeclaration{
						{Min: ptr(80.0)},
						{Min: ptr(80.0)},
						{Min: ptr(60.0), Collapsible: true},
					},
					PartProps: ui.PartProps{Style: ui.Style().
						Display("flex").
						Height(140).
						BorderRadius(10).
						BorderWidth(1).
						BorderColor(color(func(p palette) string {
							return p.Border
						})).
						Overflow("hidden").
						BackgroundColor(color(func(p palette) string {
							return p.PanelAlt
						}))},
				})
				return splitter45.Root().
					Children(func() *native.Node {
						var children []*native.Node
						for i := range 3 {
							if i > 0 {
								children = append(children, splitter45.HandleWith(ui.SplitterPaneProps{
									Index: ptr(i - 1),
									PartProps: ui.PartProps{Style: ui.Style().
										Width(6).
										BackgroundColor(color(func(p palette) string {
											return p.Border
										})).
										Cursor("col-resize")},
								}).
									NativeNode())
							}
							children = append(children, splitter45.PaneWith(ui.SplitterPaneProps{
								Index: ptr(i),
								PartProps: ui.PartProps{Style: ui.Style().
									Display("flex").
									AlignItems("center").
									JustifyContent("center")},
							}).
								Children(func() *ui.Element {
									return muted(func() string {
										return "pane " + strconv.Itoa(i) + " · " + strconv.FormatFloat(sizes()[i], 'f', 0, 64) + "px"
									})
								}).
								NativeNode())
						}
						return ui.Fragment(children)
					}).
					NativeNode()
			}(),
			note(func() string {
				return fmt.Sprintf("sizes %v", sizes())
			}).Node,
		})
	})
}
func switchControl(checked func() bool, set func(bool), readOnly bool, caption string) *ui.Element {
	return row(func() *native.Node {
		return ui.Fragment([]*native.Node{
			func() *native.Node {
				switch46 := ui.NewSwitch(ui.SwitchProps{
					Checked:         checked,
					OnCheckedChange: change(set),
					ReadOnly:        readOnly,
					PartProps: ui.PartProps{
						AriaLabel: caption,
						Style: func() ui.StyleBuilder {
							return ui.Style().
								Width(44).
								Height(24).
								BorderRadius(12).
								Padding(2).
								Display("flex").
								AlignItems("center").
								BackgroundColor(choose(checked(), p().Accent, p().Track)).
								Transition("background-color 160ms ease-out")
						},
					},
				})
				return switch46.Root().
					Children(func() *native.Node {
						return switch46.Thumb(ui.PartProps{Style: func() ui.StyleBuilder {
							return ui.Style().
								Width(20).
								Height(20).
								BorderRadius(10).
								BackgroundColor("white").
								FlexShrink(0).
								Transform(choose(checked(), "translateX(20px)", "translateX(0px)")).
								Transition("transform 160ms ease-out")
						}}).
							NativeNode()
					}).
					NativeNode()
			}(),
			label(caption).Node,
		})
	})
}
func SwitchDemo() *ui.Element {
	wifi, setWifi := ui.CreateSignal(true)
	beta, setBeta := ui.CreateSignal(false)
	managed, setManaged := ui.CreateSignal(true)
	return panel("Switch", "Animated thumb and track color, including a focusable read-only example.", func() *native.Node {
		return ui.Fragment([]*native.Node{
			switchControl(wifi, setWifi, false, "Wi-Fi").Node,
			switchControl(beta, setBeta, false, "Beta updates").Node,
			switchControl(managed, setManaged, true, "Managed by policy · read-only").Node,
			note(func() string {
				return "wifi " + strconv.FormatBool(wifi()) + " · beta " + strconv.FormatBool(beta()) + " · managed " + strconv.FormatBool(managed())
			}).Node,
		})
	})
}
func TabsDemo() *ui.Element {
	tab, setTab := ui.CreateSignal("overview")
	return panel("Tabs", "Automatic activation, keyboard navigation, lazy panels, and measured indicator geometry.", func() *native.Node {
		return ui.Fragment([]*native.Node{
			func() *native.Node {
				tabs47 := ui.NewTabs(ui.TabsRootProps{
					Value: func() *string {
						return ptr(tab())
					},
					OnValueChange: change(setTab),
					Activation:    "automatic",
					Orientation:   "horizontal",
					PartProps:     columnPart(),
				})
				return tabs47.Root().
					Children(func() *native.Node {
						var children []*native.Node
						children = append(children, tabs47.List(rowPart()).
							Children(func() *native.Node {
								var children []*native.Node
								for i, value := range []string{"overview", "usage", "limits"} {
									children = append(children, tabs47.TabWith(ui.TabsTabProps{
										Value: value,
										Index: ptr(i),
										PartProps: ui.PartProps{Style: func() ui.StyleBuilder {
											s := controlStyle()
											s = s.BackgroundColor(choose(tab() == value, p().Selection, "transparent"))
											return s
										}},
									}).
										Children(value).
										NativeNode())
								}
								children = append(children, tabs47.Indicator(ui.TabsIndicatorProps{
									Placement: "bottom",
									PartProps: ui.PartProps{Style: ui.Style().
										Height(2).
										BackgroundColor(color(func(p palette) string {
											return p.Accent
										}))},
								}).
									NativeNode())
								return ui.Fragment(children)
							}).
							NativeNode())
						state := ui.UseTabsState()
						children = append(children, note(func() string {
							return fmt.Sprintf("activation %s · indicator %+v", state().ActivationDirection, state().Indicator)
						}).Node)
						children = append(children, tabs47.PanelWith(ui.TabsPanelProps{Value: "overview"}).
							Children("An inactive panel contributes no layout or paint.").
							NativeNode())
						children = append(children, tabs47.PanelWith(ui.TabsPanelProps{Value: "usage"}).
							Children("Arrow keys move focus and select automatically.").
							NativeNode())
						children = append(children, tabs47.PanelWith(ui.TabsPanelProps{Value: "limits"}).
							Children("KeepMounted retains inactive panels with display:none.").
							NativeNode())
						return ui.Fragment(children)
					}).
					NativeNode()
			}(),
			note(func() string {
				return "active " + tab()
			}).Node,
		})
	})
}
func togglePart(pressed func() bool) ui.PartProps {
	return ui.PartProps{Style: func() ui.StyleBuilder {
		s := controlStyle()
		s = s.BackgroundColor(choose(pressed(), p().Selection, p().Control))
		s = s.BorderColor(choose(pressed(), p().Accent, p().Border))
		return s
	}}
}
func ToggleDemo() *ui.Element {
	bold, setBold := ui.CreateSignal(false)
	pinned, setPinned := ui.CreateSignal(true)
	return panel("Toggle", "Buttons that stay pressed, with native toggle-button semantics.", func() *native.Node {
		return ui.Fragment([]*native.Node{
			row(func() *native.Node {
				return ui.Fragment([]*native.Node{
					func() *native.Node {
						toggle48 := ui.NewToggle(ui.ToggleProps{
							Pressed:         bold,
							OnPressedChange: change(setBold),
							PartProps:       togglePart(bold),
						})
						return toggle48.Root().Children("B").NativeNode()
					}(),
					func() *native.Node {
						toggle49 := ui.NewToggle(ui.ToggleProps{
							Pressed:         pinned,
							OnPressedChange: change(setPinned),
							PartProps:       togglePart(pinned),
						})
						return toggle49.Root().
							Children(func() *native.Node {
								return ui.Fragment([]*native.Node{
									toggle49.Indicator(ui.PartProps{Style: ui.Style().
										Width(6).
										Height(6).
										BorderRadius(3).
										BackgroundColor(color(func(p palette) string {
											return p.Accent
										}))}).
										NativeNode(),
									label("Pinned").Node,
								})
							}).
							NativeNode()
					}(),
				})
			}).Node,
			note(func() string {
				return "bold " + strconv.FormatBool(bold()) + " · pinned " + strconv.FormatBool(pinned())
			}).Node,
		})
	})
}
func ToggleGroupDemo() *ui.Element {
	align, setAlign := ui.CreateSignal([]string{"left"})
	formats, setFormats := ui.CreateSignal([]string{"bold"})
	return panel("Toggle group", "Single and multiple selection, roving focus, and a disabled item.", func() *native.Node {
		var children []*native.Node
		for _, group := range []struct {
			values   func() []string
			set      func([]string)
			items    []string
			multiple bool
		}{{align, setAlign, []string{"left", "center", "right"}, false}, {formats, setFormats, []string{"bold", "italic", "underline"}, true}} {
			items := []ui.ComponentItem{}
			for _, value := range group.items {
				items = append(items, ui.ComponentItem{Value: value, Disabled: value == "underline"})
			}
			children = append(children, func() *native.Node {
				toggleGroup50 := ui.NewToggleGroup(ui.ToggleGroupProps{
					Value:         group.values,
					OnValueChange: change(group.set),
					Multiple:      group.multiple,
					Items:         items,
					PartProps:     rowPart(),
				})
				return toggleGroup50.Root().
					Children(func() *native.Node {
						var children []*native.Node
						for _, value := range group.items {
							part := togglePart(func() bool {
								return slices.Contains(group.values(), value)
							})
							part.Disabled = value == "underline"
							children = append(children, toggleGroup50.ItemWith(ui.ToggleGroupItemProps{
								Value:     value,
								PartProps: part,
							}).
								Children(value).
								NativeNode())
						}
						return ui.Fragment(children)
					}).
					NativeNode()
			}())
		}
		children = append(children, note(func() string {
			return "align [" + strings.Join(align(), ", ") + "] · formats [" + strings.Join(formats(), ", ") + "]"
		}).Node)
		return ui.Fragment(children)
	})
}
func ToolbarDemo() *ui.Element {
	tool, setTool := ui.CreateSignal("select")
	query, setQuery := ui.CreateSignal("")
	items := []ui.ComponentItem{
		{Value: "select"},
		{Value: "pen"},
		{Value: "erase", Disabled: true},
		{Value: "share"},
		{Value: "docs"},
		{Value: "find"},
	}
	return panel("Toolbar", "One roving Tab stop across items, a button, a link, and an input. Arrow keys skip the disabled item.", func() *native.Node {
		return ui.Fragment([]*native.Node{
			func() *native.Node {
				toolbar51 := ui.NewToolbar(ui.ToolbarRootProps{
					Active: func() *string {
						return ptr(tool())
					},
					OnActiveChange: change(setTool),
					Items:          items,
					Orientation:    "horizontal",
					PartProps:      rowPart(),
				})
				return toolbar51.Root().
					Children(func() *native.Node {
						var children []*native.Node
						children = append(children, toolbar51.Group(rowPart()).
							Children(func() *native.Node {
								var children []*native.Node
								for _, value := range []string{"select", "pen", "erase"} {
									part := togglePart(func() bool {
										return tool() == value
									})
									part.Disabled = value == "erase"
									children = append(children, toolbar51.Item(ui.ToolbarItemProps{
										Value:     value,
										PartProps: part,
									}).
										Children(value).
										NativeNode())
								}
								return ui.Fragment(children)
							}).
							NativeNode())
						children = append(children, toolbar51.Separator(ui.PartProps{Style: ui.Style().
							Width(1).
							Height(20).
							BackgroundColor(color(func(p palette) string {
								return p.Border
							}))}).
							NativeNode())
						children = append(children, toolbar51.Button(ui.ToolbarItemProps{
							Value:     "share",
							PartProps: control(),
						}).
							Children("Share").
							NativeNode())
						children = append(children, toolbar51.Link(ui.ToolbarItemProps{
							Value:     "docs",
							PartProps: control(),
						}).
							Children("Docs").
							NativeNode())
						style := inputStyle()
						style = style.Width(140)
						children = append(children, toolbar51.Input(ui.ToolbarInputProps{
							PartValue: "find",
							InputPartProps: ui.InputPartProps{
								Value:       query,
								Placeholder: "Find",
								OnInput: func(e *native.Event) {
									setQuery(e.Value)
								},
								PartProps: ui.PartProps{Style: style},
							},
						}).
							NativeNode())
						return ui.Fragment(children)
					}).
					NativeNode()
			}(),
			note(func() string {
				return "active " + tool() + " · find " + query()
			}).Node,
		})
	})
}
