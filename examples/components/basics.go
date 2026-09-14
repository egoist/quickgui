package main

import (
	"fmt"
	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/ui"
	"slices"
	"strconv"
	"strings"
)

func AccordionDemo() *ui.Element {
	open, setOpen := ui.CreateSignal([]string{"declared"})
	entries := []struct{ value, title, body string }{{"declared", "Everything is declared", "Declare values and elements in Go. The native core owns roles, focus, and deadlines."}, {"panels", "Closed panels are not mounted", "A closed Accordion.Panel contributes no layout, paint, input, or accessibility node."}, {"heading", "Heading level belongs to the core", "HeadingLevel is clamped to 1 through 6 and projected on every header."}}
	return panel("Accordion", "Multiple open items, semantic headings, and lazy panels. Click a header or use the arrow keys.", func() *native.Node {
		return ui.Fragment([]*native.Node{
			func() *native.Node {
				accordion7 := ui.NewAccordion(ui.AccordionRootProps{
					Value:         open,
					OnValueChange: change(setOpen),
					Multiple:      true,
					HeadingLevel:  3,
					PartProps:     columnPart(),
				})
				return accordion7.Root().
					Children(func() *native.Node {
						var children_ []*native.Node
						for index, entry := range entries {
							children_ = append(children_, accordion7.ItemWith(ui.AccordionItemProps{
								Value: entry.value,
								Index: ptr(index),
								PartProps: ui.PartProps{Style: func() ui.StyleBuilder {
									return ui.Style().
										Display("flex").
										FlexDirection("column").
										BorderRadius(10).
										BorderWidth(1).
										BorderColor(p().Border).
										BackgroundColor(p().PanelAlt).
										Overflow("hidden")
								}},
							}).
								Children(func() *native.Node {
									return ui.Fragment([]*native.Node{
										accordion7.Header(ui.PartProps{Style: ui.Style().Display("flex").Width("100%")}).
											Children(func() *native.Node {
												props := control()
												s := controlStyle()
												s = s.Width("100%")
												s = s.JustifyContent("space-between")
												s = s.BorderWidth(0)
												s = s.Height(34)
												props.Style = s
												return accordion7.Trigger(props).
													Children(func() *native.Node {
														return ui.Fragment([]*native.Node{
															label(entry.title).Node,
															label(func() string {
																return choose(slices.Contains(open(), entry.value), "−", "+")
															}).Node,
														})
													}).
													NativeNode()
											}).
											NativeNode(),
										accordion7.Panel(ui.PartProps{Style: ui.Style().Padding(12).PaddingTop(4)}).
											Children(func() *ui.Element {
												return muted(entry.body)
											}).
											NativeNode(),
									})
								}).
								NativeNode())
						}
						return ui.Fragment(children_)
					}).
					NativeNode()
			}(),
			note(func() string {
				return "open: " + strings.Join(open(), ", ")
			}).Node,
		})
	})
}
func AlertDialogDemo() *ui.Element {
	open, setOpen := ui.CreateSignal(false)
	outcome, setOutcome := ui.CreateSignal("nothing yet")
	reason, setReason := ui.CreateSignal("—")
	return panel("Alert dialog", "The backdrop refuses dismissal. Choose an answer or press Escape; focus returns to the trigger.", func() *native.Node {
		return ui.Fragment([]*native.Node{
			func() *native.Node {
				alertDialog8 := ui.NewAlertDialog(ui.DialogRootProps{
					Open: open,
					OnOpenChange: func(next bool, details ui.DialogOpenChangeDetails) {
						setOpen(next)
						setReason(string(details.Reason))
						if !next && outcome() == "nothing yet" {
							setOutcome("dismissed")
						}
					},
				})
				return alertDialog8.Root().
					Children(func() *native.Node {
						return ui.Fragment([]*native.Node{
							alertDialog8.Trigger(control()).Children("Delete branch…").NativeNode(),
							alertDialog8.Portal(overlay()).
								Children(func() *native.Node {
									return ui.Fragment([]*native.Node{
										alertDialog8.Backdrop(backdrop()).NativeNode(),
										alertDialog8.Popup(ui.DialogPopupProps{PartProps: popup(340)}).
											Children(func() *native.Node {
												return ui.Fragment([]*native.Node{
													alertDialog8.Title(ui.PartProps{}).
														Children(func() *ui.Element {
															return ui.Text("Delete “electron-parity”?").
																FontSize(15).
																FontWeight(700)
														}).
														NativeNode(),
													alertDialog8.Description(ui.PartProps{}).
														Children(func() *ui.Element {
															return muted("This gallery records the answer; it does not delete a real branch.")
														}).
														NativeNode(),
													row(func() *native.Node {
														return ui.Fragment([]*native.Node{
															alertDialog8.Close(control()).
																Children("Cancel").
																NativeNode(),
															button("Delete", func() {
																setOutcome("deleted")
																setOpen(false)
															}).
																Style(ui.Style().
																	BackgroundColor(color(func(p palette) string {
																		return p.Danger
																	})).
																	TextColor("white")).Node,
														})
													}).Node,
												})
											}).
											NativeNode(),
									})
								}).
								NativeNode(),
						})
					}).
					NativeNode()
			}(),
			note(func() string {
				return "open " + strconv.FormatBool(open()) + " · reason " + reason() + " · outcome " + outcome()
			}).Node,
		})
	})
}
func AvatarDemo() *ui.Element {
	missing, setMissing := ui.CreateSignal("idle")
	fallback, setFallback := ui.CreateSignal("idle")
	shell := func() ui.StyleBuilder {
		return ui.Style().
			Width(44).
			Height(44).
			BorderRadius(22).
			Display("flex").
			AlignItems("center").
			JustifyContent("center").
			BackgroundColor(p().Selection).
			BorderColor(p().Border).
			BorderWidth(1).
			Overflow("hidden")
	}
	return panel("Avatar", "Images and fallbacks share one accessible name. Failed loads report their actual status.", func() *native.Node {
		return ui.Fragment([]*native.Node{
			row(func() *native.Node {
				return ui.Fragment([]*native.Node{
					func() *native.Node {
						avatar9 := ui.NewAvatar(ui.AvatarRootProps{
							OnLoadingStatusChange: change(setMissing),
							PartProps: ui.PartProps{
								AriaLabel: "Ada Lovelace",
								Style:     shell,
							},
						})
						return avatar9.Root().
							Children(func() *native.Node {
								return ui.Fragment([]*native.Node{
									avatar9.Image(ui.AvatarImageProps{
										Src:       "./avatar-does-not-exist.png",
										PartProps: ui.PartProps{Style: ui.Style().Width(44).Height(44)},
									}).
										NativeNode(),
									avatar9.Fallback(ui.AvatarFallbackProps{Delay: 120}).
										Children("AL").
										NativeNode(),
								})
							}).
							NativeNode()
					}(),
					func() *native.Node {
						avatar10 := ui.NewAvatar(ui.AvatarRootProps{
							OnLoadingStatusChange: change(setFallback),
							PartProps: ui.PartProps{
								AriaLabel: "Grace Hopper",
								Style:     shell,
							},
						})
						return avatar10.Root().
							Children(func() *native.Node {
								return avatar10.Fallback(ui.AvatarFallbackProps{}).
									Children("GH").
									NativeNode()
							}).
							NativeNode()
					}(),
				})
			}).Node,
			note(func() string {
				return "with image: " + missing() + " · fallback only: " + fallback()
			}).Node,
		})
	})
}
func ButtonDemo() *ui.Element {
	clicks, setClicks := ui.CreateSignal(0)
	last, setLast := ui.CreateSignal("nothing yet")
	return panel("Button", "Native press, focus, disabled state, and double-click handling.", func() *native.Node {
		return ui.Fragment([]*native.Node{
			row(func() *native.Node {
				return ui.Fragment([]*native.Node{
					primary("Primary", func() {
						setClicks(clicks() + 1)
						setLast("primary")
					}).Node,
					button("Secondary", func() {
						setClicks(clicks() + 1)
						setLast("secondary")
					}).Node,
					button("Disabled", func() {
						setLast("never")
					}).Disabled(true).Node,
					button("Double-click me", func() {
						setLast("single click")
					}).OnDoubleClick(func(*native.Event) {
						setLast("double click")
					}).Node,
				})
			}).Node,
			note(func() string {
				return "clicks " + strconv.Itoa(clicks()) + " · last " + last()
			}).Node,
		})
	})
}
func CheckboxDemo() *ui.Element {
	notify, setNotify := ui.CreateSignal[ui.CheckedState]("indeterminate")
	kids, setKids := ui.CreateSignal([]bool{true, false})
	return panel("Checkbox", "Tri-state, read-only, and a parent whose mixed state is derived from ChildrenChecked.", func() *native.Node {
		return ui.Fragment([]*native.Node{
			checkbox(ui.CheckboxProps{
				Checked: notify,
				OnCheckedChange: func(v bool, _ *native.Event) {
					setNotify(v)
				},
			}, "Email me about releases"),
			checkbox(ui.CheckboxProps{
				DefaultChecked: true,
				ReadOnly:       true,
			}, "Read-only: focusable, refuses changes"),
			checkbox(ui.CheckboxProps{
				Parent:          true,
				ChildrenChecked: kids,
				Checked: func() ui.CheckedState {
					checked := 0
					for _, value := range kids() {
						if value {
							checked++
						}
					}
					return checkboxSelectionState(checked, len(kids()))
				},
				OnCheckedChange: func(value bool, _ *native.Event) {
					next := make([]bool, len(kids()))
					for i := range next {
						next[i] = value
					}
					setKids(next)
				},
			}, "Parent, derived from its children"),
			row(func() *native.Node {
				var children_ []*native.Node
				for i, name := range []string{"Analytics", "Crash reports"} {
					children_ = append(children_, checkbox(ui.CheckboxProps{
						Checked: func() ui.CheckedState {
							return kids()[i]
						},
						OnCheckedChange: func(v bool, _ *native.Event) {
							next := slices.Clone(kids())
							next[i] = v
							setKids(next)
						},
					}, name))
				}
				return ui.Fragment(children_)
			}).Node,
			note(func() string {
				return fmt.Sprintf("notify %v · children %v", notify(), kids())
			}).Node,
		})
	})
}
func CheckboxGroupDemo() *ui.Element {
	colors, setColors := ui.CreateSignal([]string{"green"})
	all := []string{"red", "green", "blue", "violet"}
	return panel("Checkbox group", "The parent toggles all values and derives its mixed state from the group.", func() *native.Node {
		return ui.Fragment([]*native.Node{
			func() *native.Node {
				checkboxGroup11 := ui.NewCheckboxGroup(ui.CheckboxGroupProps{
					AllValues:     all,
					Value:         colors,
					OnValueChange: change(setColors),
					PartProps:     columnPart(),
				})
				return checkboxGroup11.Root().
					Children(func() *native.Node {
						var children_ []*native.Node
						children_ = append(children_, checkbox(ui.CheckboxProps{
							Parent: true,
							Checked: func() ui.CheckedState {
								return checkboxSelectionState(len(colors()), len(all))
							},
						}, "All colours"))
						for _, value := range all {
							children_ = append(children_, checkbox(ui.CheckboxProps{
								Value: value,
								Checked: func() ui.CheckedState {
									return slices.Contains(colors(), value)
								},
							}, value))
						}
						return ui.Fragment(children_)
					}).
					NativeNode()
			}(),
			note(func() string {
				return "checked: " + strings.Join(colors(), ", ")
			}).Node,
		})
	})
}
func CollapsibleDemo() *ui.Element {
	open, setOpen := ui.CreateSignal(false)
	kept, setKept := ui.CreateSignal(true)
	return panel("Collapsible", "Closed panels are omitted; KeepMounted instead retains the panel with display:none.", func() *native.Node {
		return ui.Fragment([]*native.Node{
			func() *native.Node {
				collapsible12 := ui.NewCollapsible(ui.CollapsibleRootProps{
					Open:         open,
					OnOpenChange: change(setOpen),
					PartProps:    columnPart(),
				})
				return collapsible12.Root().
					Children(func() *native.Node {
						return ui.Fragment([]*native.Node{
							collapsible12.Trigger(control()).
								Children(func() *ui.Element {
									return label(func() string {
										return choose(open(), "Hide advanced options", "Show advanced options")
									})
								}).
								NativeNode(),
							collapsible12.Panel(ui.PartProps{}).
								Children(func() *ui.Element {
									return muted("While closed, this panel contributes no layout, paint, input, or accessibility node.")
								}).
								NativeNode(),
						})
					}).
					NativeNode()
			}(),
			func() *native.Node {
				collapsible13 := ui.NewCollapsible(ui.CollapsibleRootProps{
					Open:         kept,
					OnOpenChange: change(setKept),
					KeepMounted:  true,
					PartProps:    columnPart(),
				})
				return collapsible13.Root().
					Children(func() *native.Node {
						return ui.Fragment([]*native.Node{
							collapsible13.Trigger(control()).
								Children("Toggle kept panel").
								NativeNode(),
							collapsible13.Panel(ui.PartProps{}).
								Children("Retained as display:none instead of omitted.").
								NativeNode(),
						})
					}).
					NativeNode()
			}(),
			note(func() string {
				return "plain " + strconv.FormatBool(open()) + " · keepMounted " + strconv.FormatBool(kept())
			}).Node,
		})
	})
}
func DialogDemo() *ui.Element {
	open, setOpen := ui.CreateSignal(false)
	reason, setReason := ui.CreateSignal("—")
	completed, setCompleted := ui.CreateSignal("—")
	notes, setNotes := ui.CreateSignal("")
	return panel("Dialog", "An in-window dialog with a focus trap, a scrollable body, and a 120ms exit deadline.", func() *native.Node {
		return ui.Fragment([]*native.Node{
			func() *native.Node {
				dialog14 := ui.NewDialog(ui.DialogRootProps{
					Open:         open,
					ExitDuration: 120,
					OnOpenChange: func(v bool, d ui.DialogOpenChangeDetails) {
						setOpen(v)
						setReason(string(d.Reason))
					},
					OnOpenChangeComplete: func(v bool, _ *native.Event) {
						setCompleted(choose(v, "opened", "closed"))
					},
				})
				return dialog14.Root().
					Children(func() *native.Node {
						return ui.Fragment([]*native.Node{
							dialog14.Trigger(control()).Children("Open dialog").NativeNode(),
							dialog14.Portal(overlay()).
								Children(func() *native.Node {
									return ui.Fragment([]*native.Node{
										dialog14.Backdrop(backdrop()).NativeNode(),
										dialog14.Popup(ui.DialogPopupProps{PartProps: popup(360)}).
											Children(func() *native.Node {
												return ui.Fragment([]*native.Node{
													dialog14.Title(ui.PartProps{}).
														Children(func() *ui.Element {
															return ui.Text("Publish this build?").
																FontSize(15).
																FontWeight(700)
														}).
														NativeNode(),
													dialog14.Description(ui.PartProps{}).
														Children(func() *ui.Element {
															return muted("Escape, the backdrop, and Cancel restore focus to the trigger.")
														}).
														NativeNode(),
													dialog14.Viewport(ui.PartProps{Style: ui.Style().
														Display("flex").
														FlexDirection("column").
														Gap(8).
														MaxHeight(160).
														Padding(4)}).
														Children(func() *ui.Element {
															return input(notes, setNotes, "Release notes").
																Multiline(true).
																Style(ui.Style().
																	Width("100%").
																	Height(64).
																	PaddingTop(6))
														}).
														NativeNode(),
													row(func() *native.Node {
														return ui.Fragment([]*native.Node{
															dialog14.Close(control()).
																Children("Cancel").
																NativeNode(),
															primary("Publish", func() {
																setOpen(false)
															}).Node,
														})
													}).Node,
												})
											}).
											NativeNode(),
									})
								}).
								NativeNode(),
						})
					}).
					NativeNode()
			}(),
			note(func() string {
				return "open " + strconv.FormatBool(open()) + " · reason " + reason() + " · transition " + completed() + " · notes " + strconv.Itoa(len(notes())) + " chars"
			}).Node,
		})
	})
}
