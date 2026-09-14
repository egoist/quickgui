package main

import (
	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/ui"
	"strconv"
)

func menuAppearance() *ui.MenuAppearance {
	return &ui.MenuAppearance{
		Width:               ptr(220.0),
		ItemHeight:          ptr(28.0),
		FontSize:            ptr(12.0),
		Radius:              ptr(10.0),
		Background:          p().Popup,
		Color:               p().Ink,
		HighlightBackground: p().Accent,
		HighlightColor:      p().OnAccent,
		MutedColor:          p().Muted,
	}
}
func menuRow() ui.PartProps {
	return ui.PartProps{Style: ui.Style().
		Merge(controlStyle()).
		JustifyContent("space-between").
		BackgroundColor("transparent").
		BorderWidth(0).
		Height(28).
		BorderRadius(5).
		PaddingLeft(10).
		PaddingRight(10).
		Hover(func(s ui.StyleBuilder) ui.StyleBuilder {
			return s.BackgroundColor(color(func(p palette) string {
				return p.Selection
			}))
		})}
}
func menuLabel(caption string) *ui.Element {
	state := ui.UseMenuItemState()
	return ui.Text(caption).TextColor(choose(state().Highlighted, p().Accent, p().Ink)).Flex(1)
}
func ContextMenuDemo() *ui.Element {
	command, setCommand := ui.CreateSignal("nothing yet")
	parts, setParts := ui.CreateSignal("nothing yet")
	target := func() ui.ContextMenuTriggerProps {
		return ui.ContextMenuTriggerProps{PartProps: ui.PartProps{Style: ui.Style().
			Display("flex").
			Height(64).
			BorderRadius(10).
			BorderWidth(1).
			BorderColor(color(func(p palette) string {
				return p.Border
			})).
			BorderStyle("dashed").
			AlignItems("center").
			JustifyContent("center").
			BackgroundColor(color(func(p palette) string {
				return p.PanelAlt
			}))}}
	}
	return panel("Context menu", "Secondary-click menus from a declared row model or from Menu.Item parts.", func() *native.Node {
		return ui.Fragment([]*native.Node{
			func() *native.Node {
				contextMenu15 := ui.NewContextMenu(ui.ContextMenuRootProps{
					Items: func() []ui.MenuItemDeclaration {
						return []ui.MenuItemDeclaration{
							{ID: "cut", Label: "Cut"},
							{ID: "copy", Label: "Copy"},
							{Type: "separator"},
							{ID: "paste", Label: "Paste"},
						}
					},
					Appearance: menuAppearance(),
					OnSelect: func(d ui.MenuSelectDetails, _ *native.Event) {
						setCommand(d.ID)
					},
				})
				return contextMenu15.Root().
					Children(func() *native.Node {
						return contextMenu15.Trigger(target()).
							Children("Right-click: declared rows").
							NativeNode()
					}).
					NativeNode()
			}(),
			func() *native.Node {
				contextMenu16 := ui.NewContextMenu(ui.ContextMenuRootProps{
					Appearance: menuAppearance(),
					OnSelect: func(d ui.MenuSelectDetails, _ *native.Event) {
						setParts(d.ID)
					},
				})
				return contextMenu16.Root().
					Children(func() *native.Node {
						return contextMenu16.Trigger(target()).
							Children(func() *native.Node {
								return ui.Fragment([]*native.Node{
									contextMenu16.Item(ui.MenuItemProps{
										Value: "rename",
										Label: "Rename",
									}).
										NativeNode(),
									contextMenu16.Item(ui.MenuItemProps{
										Value: "duplicate",
										Label: "Duplicate",
									}).
										NativeNode(),
									contextMenu16.Separator(ui.PartProps{}).NativeNode(),
									contextMenu16.Item(ui.MenuItemProps{
										Value: "delete",
										Label: "Delete",
									}).
										NativeNode(),
									muted("Right-click: Menu.Item parts").Node,
								})
							}).
							NativeNode()
					}).
					NativeNode()
			}(),
			note(func() string {
				return "rows → " + command() + " · parts → " + parts()
			}).Node,
		})
	})
}
func SystemContextMenuDemo() *ui.Element {
	action, setAction := ui.CreateSignal("nothing yet")
	status, setStatus := ui.CreateSignal("closed")
	showHidden, setShowHidden := ui.CreateSignal(false)
	sortBy, setSortBy := ui.CreateSignal("Name")
	open := func() {
		window := native.CurrentWindow()
		sortItems := make([]native.MenuItem, 0, 3)
		for _, name := range []string{"Name", "Date modified", "Size"} {
			sortItems = append(sortItems, native.MenuItem{
				Label:   name,
				Checked: sortBy() == name,
				Click: func() {
					setSortBy(name)
					setAction("Sort by " + name)
				},
			})
		}
		setStatus("open")
		window.PopupMenu([]native.MenuItem{
			{Label: "Open", Click: func() {
				setAction("Open")
			}},
			{Label: "Rename", Click: func() {
				setAction("Rename")
			}},
			{Type: "separator"},
			{Label: "Show hidden files", Checked: showHidden(), Click: func() {
				setShowHidden(!showHidden())
				setAction("Show hidden files " + choose(showHidden(), "on", "off"))
			}},
			{Label: "Sort by", Items: sortItems},
			{Type: "separator"},
			{Label: "Unavailable action", Enabled: ptr(false)},
		}, nil, func(err error) {
			if err != nil {
				setStatus("error: " + err.Error())
				return
			}
			setStatus("closed")
		})
	}
	return panel("System context menu", "The operating system draws this menu. Actions update the readout below; reopen it to see the current checkmarks.", func() *native.Node {
		return ui.Fragment([]*native.Node{
			ui.View().
				Child("Right-click here for the system menu").
				Display("flex").
				Height(96).
				AlignItems("center").
				JustifyContent("center").
				BorderRadius(10).
				BorderWidth(1).
				BorderStyle("dashed").
				BorderColor(color(func(p palette) string {
					return p.Border
				})).
				BackgroundColor(color(func(p palette) string {
					return p.PanelAlt
				})).
				TextColor(color(func(p palette) string {
					return p.Muted
				})).
				AppRegion("no-drag").
				UserSelect("none").
				OnContextMenu(func(event *native.Event) {
					event.PreventDefault()
					open()
				}).Node,
			row(func() *ui.Element {
				return button("Open system menu", open)
			}).Node,
			note(func() string {
				return "menu " + status() + " · last action " + action()
			}).Node,
			note(func() string {
				return "hidden files " + strconv.FormatBool(showHidden()) + " · sort by " + sortBy()
			}).Node,
		})
	})
}
func MenuDemo() *ui.Element {
	open, setOpen := ui.CreateSignal(false)
	wrap, setWrap := ui.CreateSignal(false)
	density, setDensity := ui.CreateSignal("cozy")
	activated, setActivated := ui.CreateSignal("nothing yet")
	item := func(menu *ui.MenuComponent, value, caption string) *native.Node {
		props := menuRow()
		props.OnClick = func(*native.Event) {
			setActivated(value)
		}
		return menu.Item(ui.MenuItemProps{Value: value, Label: caption, PartProps: props}).
			Child(func() *ui.Element {
				return menuLabel(caption)
			}).
			NativeNode()
	}
	return panel("Menu", "Styled rows with keyboard navigation, checkbox state, radio exclusivity, and a nested submenu.", func() *native.Node {
		return ui.Fragment([]*native.Node{
			func() *native.Node {
				menu17 := ui.NewMenu(ui.MenuRootProps{
					Open:         open,
					OnOpenChange: change(setOpen),
					Side:         "bottom",
					Align:        "start",
					SideOffset:   ptr(6.0),
				})
				return menu17.Root().
					Children(func() *native.Node {
						return ui.Fragment([]*native.Node{
							menu17.Trigger(ui.MenuTriggerProps{PartProps: control()}).
								Children("Edit ▾").
								NativeNode(),
							menu17.Positioner(ui.MenuPositionerProps{}).
								Children(func() *native.Node {
									return menu17.Popup(popup(250)).
										Children(func() *native.Node {
											var children []*native.Node
											children = append(children, menu17.GroupLabel(ui.MenuGroupLabelProps{
												Value: "clipboard",
												Label: "Clipboard",
											}).
												Children("Clipboard").
												NativeNode())
											children = append(children, item(menu17, "copy", "Copy"), item(menu17, "cut", "Cut"))
											children = append(children, menu17.LinkItem(ui.MenuLinkItemProps{
												MenuItemProps: ui.MenuItemProps{
													Value:     "docs",
													Label:     "Documentation",
													PartProps: menuRow(),
												},
												Href: "https://quickgui.dev",
												OnNavigate: func(href string, _ *native.Event) {
													setActivated("link " + href)
												},
											}).
												Children(func() *ui.Element {
													return menuLabel("Documentation")
												}).
												NativeNode())
											children = append(children, menu17.Separator(ui.PartProps{Style: ui.Style().
												Height(1).
												BackgroundColor(color(func(p palette) string {
													return p.Border
												}))}).
												NativeNode())
											children = append(children, menu17.CheckboxItem(ui.MenuCheckboxItemProps{
												MenuItemProps: ui.MenuItemProps{
													Value:     "wrap",
													Label:     "Wrap lines",
													PartProps: menuRow(),
												},
												Checked:         wrap,
												OnCheckedChange: change(setWrap),
											}).
												Children(func() *native.Node {
													return ui.Fragment([]*native.Node{
														menuLabel("Wrap lines").Node,
														menu17.CheckboxItemIndicator(ui.PartProps{}).
															Children("✓").
															NativeNode(),
													})
												}).
												NativeNode())
											children = append(children, menu17.RadioGroup(ui.MenuRadioGroupProps{
												Name: "density",
												Value: func() *string {
													return ptr(density())
												},
												OnValueChange: change(setDensity),
												PartProps:     columnPart(),
											}).
												Children(func() *native.Node {
													var children []*native.Node
													for _, value := range []string{"compact", "cozy", "comfortable"} {
														children = append(children, menu17.RadioItem(ui.MenuRadioItemProps{MenuItemProps: ui.MenuItemProps{
															Value:     value,
															Label:     value,
															PartProps: menuRow(),
														}}).
															Children(func() *native.Node {
																return ui.Fragment([]*native.Node{
																	menuLabel(value).Node,
																	menu17.RadioItemIndicator(ui.PartProps{}).
																		Children("●").
																		NativeNode(),
																})
															}).
															NativeNode())
													}
													return ui.Fragment(children)
												}).
												NativeNode())
											children = append(children, menu17.SubmenuRoot(ui.MenuRootProps{CloseDelay: 100}).
												Children(func() *native.Node {
													return ui.Fragment([]*native.Node{
														menu17.SubmenuTrigger(ui.MenuSubmenuTriggerProps{
															Value: "recent",
															Label: "Open recent",
															MenuTriggerProps: ui.MenuTriggerProps{
																PartProps:   menuRow(),
																OpenOnHover: ptr(true),
															},
														}).
															Children(func() *native.Node {
																return ui.Fragment([]*native.Node{
																	menuLabel("Open recent").Node,
																	label("›").Node,
																})
															}).
															NativeNode(),
														menu17.Positioner(ui.MenuPositionerProps{
															Side:       "right",
															Align:      "start",
															SideOffset: ptr(4.0),
														}).
															Children(func() *native.Node {
																return menu17.Popup(popup(200)).
																	Children(func() *native.Node {
																		return ui.Fragment(
																			item(menu17, "notes", "notes.md"),
																			item(menu17, "readme", "README.md"),
																		)
																	}).
																	NativeNode()
															}).
															NativeNode(),
													})
												}).
												NativeNode())
											return ui.Fragment(children)
										}).
										NativeNode()
								}).
								NativeNode(),
						})
					}).
					NativeNode()
			}(),
			note(func() string {
				return "open " + strconv.FormatBool(open()) + " · activated " + activated() + " · wrap " + strconv.FormatBool(wrap()) + " · density " + density()
			}).Node,
		})
	})
}
func MenubarDemo() *ui.Element {
	open, setOpen := ui.CreateSignal[*int](nil)
	active, setActive := ui.CreateSignal(0)
	command, setCommand := ui.CreateSignal("nothing yet")
	menus := []struct {
		label string
		items []ui.MenuItemDeclaration
	}{{"File", []ui.MenuItemDeclaration{
		{ID: "new", Label: "New window", Shortcut: "⌘N"},
		{ID: "open", Label: "Open…", Shortcut: "⌘O"},
		{Type: "separator"},
		{ID: "close", Label: "Close", Shortcut: "⌘W"},
	}}, {"Edit", []ui.MenuItemDeclaration{
		{ID: "undo", Label: "Undo", Shortcut: "⌘Z"},
		{ID: "redo", Label: "Redo", Shortcut: "⇧⌘Z"},
		{Type: "separator"},
		{ID: "paste", Label: "Paste", Shortcut: "⌘V"},
	}}, {"View", []ui.MenuItemDeclaration{
		{ID: "zoom-in", Label: "Zoom in"},
		{ID: "zoom-out", Label: "Zoom out"},
	}}}
	return panel("Menubar", "One roving Tab stop. Arrow keys switch menus while open; Escape closes without leaving the bar.", func() *native.Node {
		return ui.Fragment([]*native.Node{
			func() *native.Node {
				menubar18 := ui.NewMenubar(ui.MenubarRootProps{
					Count:          len(menus),
					Open:           open,
					OnOpenChange:   change(setOpen),
					OnActiveChange: change(setActive),
					PartProps:      rowPart(),
				})
				return menubar18.Root().
					Children(func() *native.Node {
						var children []*native.Node
						for index, menu := range menus {
							children = append(children, func() *native.Node {
								popoverMenu19 := ui.NewPopoverMenu(ui.PopoverMenuRootProps{
									Items: func() []ui.MenuItemDeclaration {
										return menu.items
									},
									Appearance: menuAppearance(),
									Open: func() bool {
										return open() != nil && *open() == index
									},
									OnOpenChange: func(v bool, _ ui.PopoverMenuOpenChangeDetails) {
										if v {
											setOpen(ptr(index))
										} else {
											setOpen(nil)
										}
									},
									OnSelect: func(d ui.MenuSelectDetails, _ *native.Event) {
										setCommand(d.ID)
									},
								})
								return popoverMenu19.Root().
									Children(func() *native.Node {
										return ui.Fragment([]*native.Node{
											popoverMenu19.Trigger(ui.PartProps{}).
												Children(func() *native.Node {
													return menubar18.Item(ui.MenubarItemProps{
														Index: ptr(index),
														PartProps: togglePart(func() bool {
															return open() != nil && *open() == index
														}),
													}).
														Children(menu.label).
														NativeNode()
												}).
												NativeNode(),
											popoverMenu19.Popup(ui.PopoverMenuPopupProps{PartProps: popup(220)}).
												NativeNode(),
										})
									}).
									NativeNode()
							}())
						}
						return ui.Fragment(children)
					}).
					NativeNode()
			}(),
			note(func() string {
				return "open " + textValue(open()) + " · tab stop " + strconv.Itoa(active()) + " · command " + command()
			}).Node,
		})
	})
}
func NavigationMenuDemo() *ui.Element {
	value, setValue := ui.CreateSignal[*string](nil)
	direction, setDirection := ui.CreateSignal[*string](nil)
	items := []ui.ComponentItem{
		{Value: "products"},
		{Value: "solutions"},
		{Value: "support", Disabled: true},
	}
	return panel("Navigation menu", "Hover opens a panel after the declared delay. The navigation landmark has one Tab stop and reports activation direction.", func() *native.Node {
		return ui.Fragment([]*native.Node{
			func() *native.Node {
				navigationMenu20 := ui.NewNavigationMenu(ui.NavigationMenuRootProps{
					Value:                       value,
					OnValueChange:               change(setValue),
					OnActivationDirectionChange: change(setDirection),
					Delay:                       80,
					CloseDelay:                  80,
					Items:                       items,
				})
				return navigationMenu20.Root().
					Children(func() *native.Node {
						return navigationMenu20.List(rowPart()).
							Children(func() *native.Node {
								var children []*native.Node
								for _, item := range items {
									children = append(children, navigationMenu20.Item(ui.NavigationMenuItemProps{Value: item.Value}).
										Children(func() *native.Node {
											var children []*native.Node
											props := togglePart(func() bool {
												return value() != nil && *value() == item.Value
											})
											props.Disabled = item.Disabled
											children = append(children, navigationMenu20.Trigger(ui.NavigationMenuPartProps{PartProps: props}).
												Children(item.Value).
												NativeNode())
											children = append(children, navigationMenu20.Positioner(ui.NavigationMenuPartProps{}).
												Children(func() *native.Node {
													return navigationMenu20.Popup(ui.NavigationMenuPartProps{PartProps: popup(230)}).
														Children(func() *native.Node {
															return navigationMenu20.Viewport(ui.NavigationMenuPartProps{}).
																Children(func() *native.Node {
																	return navigationMenu20.Content(ui.NavigationMenuPartProps{PartProps: columnPart()}).
																		Children(func() *native.Node {
																			return ui.Fragment([]*native.Node{
																				navigationMenu20.Link(ui.NavigationMenuLinkProps{
																					Value:  item.Value + "-overview",
																					Active: true,
																				}).
																					Children(item.Value + " overview").
																					NativeNode(),
																				navigationMenu20.Link(ui.NavigationMenuLinkProps{Value: item.Value + "-pricing"}).
																					Children(item.Value + " pricing").
																					NativeNode(),
																			})
																		}).
																		NativeNode()
																}).
																NativeNode()
														}).
														NativeNode()
												}).
												NativeNode())
											return ui.Fragment(children)
										}).
										NativeNode())
								}
								return ui.Fragment(children)
							}).
							NativeNode()
					}).
					NativeNode()
			}(),
			note(func() string {
				return "open panel " + textValue(value()) + " · direction " + textValue(direction())
			}).Node,
		})
	})
}
func PopoverDemo() *ui.Element {
	open, setOpen := ui.CreateSignal(false)
	hover, setHover := ui.CreateSignal(false)
	return panel("Popover", "Preferred placement adapts to the available space. Compare click and delayed hover triggers.", func() *native.Node {
		return ui.Fragment([]*native.Node{
			func() *native.Node {
				popover21 := ui.NewPopover(ui.PopoverRootProps{
					Open: open,
					OnOpenChange: func(v bool, _ ui.PopoverOpenChangeDetails) {
						setOpen(v)
					},
					Side:             "bottom",
					Align:            "start",
					SideOffset:       ptr(8.0),
					CollisionPadding: ptr(12.0),
				})
				return popover21.Root().
					Children(func() *native.Node {
						var children []*native.Node
						children = append(children, popover21.Trigger(ui.PopoverTriggerProps{PartProps: control()}).
							Children("Account").
							NativeNode())
						placement := ui.UsePopoverPlacement()
						children = append(children, note(func() string {
							return "resolved " + placement().Side + "/" + placement().Align
						}).Node)
						children = append(children, popover21.Positioner(ui.PopoverPositionerProps{}).
							Children(func() *native.Node {
								return popover21.Popup(ui.PopoverPopupProps{PartProps: popup(240)}).
									Children(func() *native.Node {
										return ui.Fragment([]*native.Node{
											popover21.Arrow(ui.PartProps{Style: ui.Style().
												Width(10).
												Height(10).
												BackgroundColor(color(func(p palette) string {
													return p.Popup
												}))}).
												NativeNode(),
											popover21.Title(ui.PartProps{}).
												Children("Signed in").
												NativeNode(),
											popover21.Viewport(ui.PartProps{}).
												Children("ada@example.com").
												NativeNode(),
											popover21.Close(control()).Children("Done").NativeNode(),
										})
									}).
									NativeNode()
							}).
							NativeNode())
						return ui.Fragment(children)
					}).
					NativeNode()
			}(),
			func() *native.Node {
				popover22 := ui.NewPopover(ui.PopoverRootProps{
					OpenOnHover: ptr(true),
					Delay:       200,
					CloseDelay:  120,
					Side:        "right",
					Align:       "center",
					SideOffset:  ptr(8.0),
					OnOpenChange: func(v bool, _ ui.PopoverOpenChangeDetails) {
						setHover(v)
					},
				})
				return popover22.Root().
					Children(func() *native.Node {
						return ui.Fragment([]*native.Node{
							popover22.Trigger(ui.PopoverTriggerProps{PartProps: control()}).
								Children("Hover to open").
								NativeNode(),
							popover22.Positioner(ui.PopoverPositionerProps{}).
								Children(func() *native.Node {
									return popover22.Popup(ui.PopoverPopupProps{PartProps: popup(200)}).
										Children("Opened after the hover delay.").
										NativeNode()
								}).
								NativeNode(),
						})
					}).
					NativeNode()
			}(),
			note(func() string {
				return "click " + strconv.FormatBool(open()) + " · hover " + strconv.FormatBool(hover())
			}).Node,
		})
	})
}
func PreviewCardDemo() *ui.Element {
	open, setOpen := ui.CreateSignal(false)
	return panel("Preview card", "Rest the pointer to preview. Keyboard focus opens immediately.", func() *native.Node {
		return ui.Fragment([]*native.Node{
			row(func() *native.Node {
				return ui.Fragment([]*native.Node{
					muted("Written by").Node,
					func() *native.Node {
						previewCard23 := ui.NewPreviewCard(ui.PreviewCardRootProps{
							Open:         open,
							OnOpenChange: change(setOpen),
							Placement:    "bottom-start",
							Gap:          8,
						})
						return previewCard23.Root().
							Children(func() *native.Node {
								return ui.Fragment([]*native.Node{
									previewCard23.Trigger(ui.PreviewCardTriggerProps{
										Delay:      350,
										CloseDelay: 200,
										PartProps: ui.PartProps{Style: ui.Style().
											Padding(2).
											TextColor(color(func(p palette) string {
												return p.Accent
											})).
											TextDecorationLine("underline")},
									}).
										Children("@ada").
										NativeNode(),
									previewCard23.Positioner(ui.PartProps{}).
										Children(func() *native.Node {
											return previewCard23.Popup(popup(240)).
												Children(func() *native.Node {
													return ui.Fragment([]*native.Node{
														label("Ada Lovelace").Node,
														muted("Wrote the first algorithm intended for a machine.").Node,
													})
												}).
												NativeNode()
										}).
										NativeNode(),
								})
							}).
							NativeNode()
					}(),
				})
			}).Node,
			note(func() string {
				return "open " + strconv.FormatBool(open())
			}).Node,
		})
	})
}
func ToastDemo() *native.Node {
	toast24 := ui.NewToast(ui.ToastProviderProps{
		Timeout:        5000,
		Limit:          3,
		SwipeDirection: "right",
		Pitch:          8,
	})
	return toast24.Provider().Child(func() *ui.Element { return toastDemoBody(toast24) }).NativeNode()
}
func toastDemoBody(notifications *ui.ToastComponent) *ui.Element {
	manager := ui.UseToastManager()
	counter := 0
	last := ""
	return panel("Toast", "Info, success, and error notifications with update, close, swipe, and automatic dismissal. Stack geometry comes from the core.", func() *native.Node {
		return ui.Fragment([]*native.Node{
			row(func() *native.Node {
				var children []*native.Node
				for _, kind := range []ui.ToastType{"info", "success", "error"} {
					children = append(children, button(string(kind), func() {
						counter++
						last = manager.Add(ui.ToastRequest{
							Title:       "Build " + strconv.Itoa(counter) + " · " + string(kind),
							Description: "Auto-dismisses in 5 seconds",
							Type:        kind,
						})
					}).Node)
				}
				children = append(children, button("Update last", func() {
					if last != "" {
						manager.Update(last, ui.ToastUpdate{Title: ptr("Updated in place")})
					}
				}).Node)
				children = append(children, button("Clear", manager.CloseAll).Node)
				return ui.Fragment(children)
			}).Node,
			notifications.Viewport(ui.ToastViewportProps{PartProps: ui.PartProps{Style: ui.Style().Display("flex").FlexDirection("column").Gap(8).MinHeight(40)}}).
				Child(func() *native.Node {
					return ui.KeyedFor(
						manager.Stack,
						func(entry ui.ToastStackEntry) any {
							return entry.ID
						},
						func(entry func() ui.ToastStackEntry, _ func() int) *native.Node {
							id := entry().ID
							current := func() ui.ToastDeclaration {
								for _, toast := range manager.Toasts() {
									if toast.ID == id {
										return toast
									}
								}
								return ui.ToastDeclaration{ID: id}
							}
							return notifications.Positioner(ui.ToastPartProps{ToastID: id}).
								Child(func() *native.Node {
									return notifications.Root(ui.ToastPartProps{
										ToastID: id,
										PartProps: ui.PartProps{Style: func() ui.StyleBuilder {
											s := popupStyle()
											s = s.Padding(10)
											s = s.Opacity(choose(entry().Limited, 0.55, 1.0))
											s = s.Transform("translateX(" + strconv.FormatFloat(entry().SwipeMovement, 'g', -1, 64) + "px)")
											s = s.BorderColor(choose(entry().Type == "error", p().Danger, choose(entry().Type == "success", p().Accent, p().Border)))
											return s
										}},
									}).
										Child(func() *native.Node {
											return notifications.Content(ui.ToastPartProps{ToastID: id}).
												Child(func() *native.Node {
													return ui.Fragment([]*native.Node{
														row(func() *native.Node {
															return ui.Fragment([]*native.Node{
																notifications.Title(ui.ToastPartProps{ToastID: id}).
																	Child(func() *ui.Element {
																		return label(func() string {
																			return current().Title
																		})
																	}).
																	NativeNode(),
																muted(func() string {
																	return "#" + strconv.Itoa(entry().Index) + " · +" + strconv.FormatFloat(entry().Offset, 'g', -1, 64) + "px"
																}).Node,
																notifications.Close(ui.ToastPartProps{
																	ToastID:   id,
																	PartProps: ui.PartProps{AriaLabel: "Dismiss notification"},
																}).
																	Child("×").
																	NativeNode(),
															})
														}).Node,
														notifications.Description(ui.ToastPartProps{ToastID: id}).
															Child(func() *ui.Element {
																return muted(func() string {
																	return current().Description
																})
															}).
															NativeNode(),
													})
												}).
												NativeNode()
										}).
										NativeNode()
								}).
								NativeNode()
						},
						nil,
					)
				}).
				NativeNode(),
			note(func() string {
				limited := 0
				for _, entry := range manager.Stack() {
					if entry.Limited {
						limited++
					}
				}
				return "queued " + strconv.Itoa(len(manager.Toasts())) + " · stack " + strconv.Itoa(len(manager.Stack())) + " · limited " + strconv.Itoa(limited)
			}).Node,
		})
	})
}
func TooltipDemo() *ui.Element {
	open, setOpen := ui.CreateSignal(false)
	placement, setPlacement := ui.CreateSignal("—")
	return panel("Tooltip", "A provider shares a warm delay across triggers. The second tooltip tracks the horizontal cursor position.", func() *native.Node {
		return ui.Fragment([]*native.Node{
			ui.NewTooltipProvider(ui.TooltipProviderProps{
				Delay:      500,
				CloseDelay: 120,
				Timeout:    400,
			}).
				Root().
				Child(func() *ui.Element {
					return row(func() *native.Node {
						var children []*native.Node
						for i, caption := range []string{"Hover me", "Tracks the cursor"} {
							props := ui.TooltipRootProps{Side: "top", SideOffset: ptr(8.0)}
							if i == 0 {
								props.OnOpenChange = change(setOpen)
								props.OnPlacementChange = func(d ui.AnchorPlacementDetails, _ *native.Event) {
									setPlacement(d.Side + "/" + d.Align)
								}
							} else {
								props.Side = "bottom"
								props.TrackCursorAxis = "x"
							}
							children = append(children, func() *native.Node {
								tooltip25 := ui.NewTooltip(props)
								return tooltip25.Root().
									Children(func() *native.Node {
										return ui.Fragment([]*native.Node{
											tooltip25.Trigger(ui.TooltipTriggerProps{PartProps: control()}).
												Children(caption).
												NativeNode(),
											tooltip25.Positioner(ui.TooltipPositionerProps{}).
												Children(func() *native.Node {
													s := popupStyle()
													s = s.BackgroundColor(color(func(p palette) string {
														return p.Ink
													}))
													s = s.TextColor(color(func(p palette) string {
														return p.Panel
													}))
													s = s.Padding(8)
													s = s.FontSize(11)
													return tooltip25.Popup(ui.PartProps{Style: s}).
														Children(func() *native.Node {
															var children []*native.Node
															children = append(children, ui.Text(choose(i == 0, "Shared hover delay", "Tracks the horizontal cursor")).Node)
															if i == 0 {
																children = append(children, tooltip25.Arrow(ui.PartProps{Style: ui.Style().
																	Width(8).
																	Height(8).
																	BackgroundColor(color(func(p palette) string {
																		return p.Ink
																	}))}).
																	NativeNode())
															}
															return ui.Fragment(children)
														}).
														NativeNode()
												}).
												NativeNode(),
										})
									}).
									NativeNode()
							}())
						}
						return ui.Fragment(children)
					})
				}).
				NativeNode(),
			note(func() string {
				return "open " + strconv.FormatBool(open()) + " · resolved " + placement()
			}).Node,
		})
	})
}
