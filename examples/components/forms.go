package main

import (
	"fmt"
	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/ui"
	"strconv"
	"strings"
	"time"
)

func fieldInput(value func() string, set func(string), placeholder string) ui.FieldControlProps {
	s := inputStyle()
	s = s.Width(280)
	return ui.FieldControlProps{InputPartProps: ui.InputPartProps{
		Value:       value,
		Placeholder: placeholder,
		OnInput: func(e *native.Event) {
			set(e.Value)
		},
		PartProps: ui.PartProps{Style: s},
	}}
}
func FieldDemo() *ui.Element {
	email, setEmail := ui.CreateSignal("")
	triggers, setTriggers := ui.CreateSignal("waiting for validation state")
	invalid := func() bool {
		return !strings.Contains(email(), "@")
	}
	return panel("Field", "A control, its label, description, error, and validity. Validation triggers and debounce are reported by the core.", func() *native.Node {
		return ui.Fragment([]*native.Node{
			func() *native.Node {
				field52 := ui.NewField(ui.FieldRootProps{
					Required: true,
					Invalid:  invalid,
					Filled: func() bool {
						return email() != ""
					},
					ValidationMode:         "onChange",
					ValidationDebounceTime: 200,
					ValidationMessage: func() string {
						return "Enter an address containing @"
					},
					OnValidationChange: func(v ui.FieldValidationDetails, _ *native.Event) {
						setTriggers(fmt.Sprintf("triggers %+v · delays %+v", v.Triggers, v.Delay))
					},
					PartProps: columnPart(),
				})
				return field52.Root().
					Children(func() *native.Node {
						return ui.Fragment([]*native.Node{
							field52.Label(ui.FieldLabelProps{}).
								Children("Email address").
								NativeNode(),
							field52.Control(fieldInput(email, setEmail, "you@example.com")).
								NativeNode(),
							field52.Description(ui.PartProps{}).
								Children("We only use this address for receipts.").
								NativeNode(),
							field52.Error(ui.PartProps{Style: ui.Style().
								FontSize(12).
								TextColor(color(func(p palette) string {
									return p.Danger
								}))}).
								Children("Enter an address containing @").
								NativeNode(),
							field52.Validity(ui.FieldValidityProps{}).
								Children(func() *ui.Element {
									return note(func() string {
										return "filled " + strconv.FormatBool(email() != "") + " · invalid " + strconv.FormatBool(invalid())
									})
								}).
								NativeNode(),
						})
					}).
					NativeNode()
			}(),
			note(triggers).Node,
		})
	})
}
func FieldsetDemo() *ui.Element {
	saving, setSaving := ui.CreateSignal(false)
	name, setName := ui.CreateSignal("Ada")
	org, setOrg := ui.CreateSignal("Analytical Engines")
	return panel("Fieldset", "A semantic group with a legend. A single flag disables every nested field.", func() *native.Node {
		var children []*native.Node
		children = append(children, switchControl(saving, setSaving, false, "Disable while saving").Node)
		props := columnPart()
		props.Disabled = saving
		children = append(children, func() *native.Node {
			fieldset53 := ui.NewFieldset(props)
			return fieldset53.Root().
				Children(func() *native.Node {
					var children []*native.Node
					children = append(children, fieldset53.Legend(ui.PartProps{}).Children("Profile").NativeNode())
					children = append(children, fieldset53.Description(ui.PartProps{}).
						Children("Your public identity").
						NativeNode())
					for _, field := range []struct {
						label string
						value func() string
						set   func(string)
					}{{"Name", name, setName}, {"Organization", org, setOrg}} {
						children = append(children, func() *native.Node {
							field54 := ui.NewField(ui.FieldRootProps{PartProps: columnPart()})
							return field54.Root().
								Children(func() *native.Node {
									return ui.Fragment([]*native.Node{
										field54.Label(ui.FieldLabelProps{}).
											Children(field.label).
											NativeNode(),
										field54.Control(fieldInput(field.value, field.set, field.label)).
											NativeNode(),
									})
								}).
								NativeNode()
						}())
					}
					return ui.Fragment(children)
				}).
				NativeNode()
		}())
		children = append(children, note(func() string {
			return "disabled " + strconv.FormatBool(saving()) + " · " + name() + " · " + org()
		}).Node)
		return ui.Fragment(children)
	})
}
func FormDemo() *ui.Element {
	email, setEmail := ui.CreateSignal("")
	plan, setPlan := ui.CreateSignal("pro")
	terms, setTerms := ui.CreateSignal(false)
	attempts, setAttempts := ui.CreateSignal(0)
	errors, setErrors := ui.CreateSignal(false)
	result, setResult := ui.CreateSignal("nothing submitted yet")
	invalid := func() bool {
		return !strings.Contains(email(), "@")
	}
	submit := func() {
		setAttempts(attempts() + 1)
		if invalid() || !terms() {
			setErrors(true)
			setResult("rejected: fix the fields below")
			return
		}
		setErrors(false)
		setResult("accepted: " + email() + " on the " + plan() + " plan")
	}
	return panel("Form", "Validation appears on submission. Return in the email field also submits; Reset clears all form state.", func() *native.Node {
		return ui.Fragment([]*native.Node{
			func() *native.Node {
				fieldset55 := ui.NewFieldset(columnPart())
				return fieldset55.Root().
					Children(func() *native.Node {
						return ui.Fragment([]*native.Node{
							fieldset55.Legend(ui.PartProps{}).Children("Sign up").NativeNode(),
							func() *native.Node {
								field56 := ui.NewField(ui.FieldRootProps{
									Required: true,
									Invalid: func() bool {
										return errors() && invalid()
									},
									Touched: func() bool {
										return attempts() > 0
									},
									Filled: func() bool {
										return email() != ""
									},
									ValidationMessage: func() string {
										return "An address containing @ is required"
									},
									PartProps: columnPart(),
								})
								return field56.Root().
									Children(func() *native.Node {
										var children []*native.Node
										children = append(children, field56.Label(ui.FieldLabelProps{}).
											Children("Email").
											NativeNode())
										control := fieldInput(email, setEmail, "you@example.com")
										control.OnSubmit = func(*native.Event) {
											submit()
										}
										children = append(children, field56.Control(control).NativeNode())
										children = append(children, field56.Error(ui.PartProps{Style: ui.Style().
											FontSize(12).
											TextColor(color(func(p palette) string {
												return p.Danger
											}))}).
											Children("An address containing @ is required").
											NativeNode())
										return ui.Fragment(children)
									}).
									NativeNode()
							}(),
							label("Plan").Node,
							func() *native.Node {
								radioGroup57 := ui.NewRadioGroup(ui.RadioGroupProps{
									Value: func() *string {
										return ptr(plan())
									},
									OnValueChange: change(setPlan),
									Required:      true,
									PartProps:     rowPart(),
								})
								return radioGroup57.Root().
									Children(func() *native.Node {
										var children []*native.Node
										for _, value := range []string{"free", "pro", "team"} {
											children = append(children, func() *native.Node {
												radio58 := ui.NewRadio(ui.RadioProps{
													Value: value,
													PartProps: togglePart(func() bool {
														return plan() == value
													}),
												})
												return radio58.Root().Children(value).NativeNode()
											}())
										}
										return ui.Fragment(children)
									}).
									NativeNode()
							}(),
							checkbox(ui.CheckboxProps{
								Checked: func() ui.CheckedState {
									return terms()
								},
								OnCheckedChange: change(setTerms),
							}, "I accept the terms"),
							ui.Show(
								errors() && !terms(),
								func() *ui.Element {
									return ui.Text("The terms must be accepted").
										FontSize(12).
										TextColor(color(func(p palette) string {
											return p.Danger
										}))
								},
							),
							row(func() *native.Node {
								return ui.Fragment([]*native.Node{
									primary("Submit", submit).Node,
									button("Reset", func() {
										ui.Batch(func() {
											setEmail("")
											setPlan("pro")
											setTerms(false)
											setErrors(false)
											setAttempts(0)
											setResult("nothing submitted yet")
										})
									}).Node,
								})
							}).Node,
						})
					}).
					NativeNode()
			}(),
			note(func() string {
				return "attempts " + strconv.Itoa(attempts()) + " · " + result()
			}).Node,
		})
	})
}
func InputDemo() *ui.Element {
	text, setText := ui.CreateSignal("")
	secret, setSecret := ui.CreateSignal("")
	notes, setNotes := ui.CreateSignal("Two\nlines")
	submits, setSubmits := ui.CreateSignal(0)
	return panel("Input", "Controlled native editors: single-line, password, and multiline. Return in the first field reports submission.", func() *native.Node {
		return ui.Fragment([]*native.Node{
			input(text, setText, "Type and press Return").
				Style(ui.Style().Width(300)).
				OnSubmitEvent(func(*native.Event) {
					setSubmits(submits() + 1)
				}).Node,
			input(secret, setSecret, "Password").Style(ui.Style().Width(300)).Password(true).Node,
			input(notes, setNotes, "Notes").Style(ui.Style().Width(420).Height(96)).Multiline(true).Node,
			note(func() string {
				return "text " + strconv.Quote(text()) + " · password length " + strconv.Itoa(len(secret())) + " · notes " + strconv.Quote(notes()) + " · submits " + strconv.Itoa(submits())
			}).Node,
		})
	})
}
func numberControls(field *ui.NumberFieldComponent) *native.Node {
	return field.Group(rowPart()).
		Child(func() *native.Node {
			var children []*native.Node
			children = append(children, field.Decrement(control()).Child("−").NativeNode())
			s := inputStyle()
			s = s.Width(110)
			children = append(children, field.Input(ui.NumberFieldInputProps{InputPartProps: ui.InputPartProps{PartProps: ui.PartProps{Style: s}}}).
				NativeNode())
			children = append(children, field.Increment(control()).Child("+").NativeNode())
			return ui.Fragment(children)
		}).
		NativeNode()
}
func NumberFieldDemo() *ui.Element {
	quantity, setQuantity := ui.CreateSignal(ptr(8.0))
	valid, setValid := ui.CreateSignal(true)
	committed, setCommitted := ui.CreateSignal[*float64](nil)
	return panel("Number field", "Parsing, clamping, step snapping, and scrub gestures. Alt uses the small step; Shift uses the large step; Return commits.", func() *native.Node {
		return ui.Fragment([]*native.Node{
			func() *native.Node {
				numberField59 := ui.NewNumberField(ui.NumberFieldRootProps{
					Value:           quantity,
					Min:             0,
					Max:             100,
					Step:            1,
					SmallStep:       0.1,
					LargeStep:       10,
					Precision:       1,
					AllowWheelScrub: ptr(true),
					Required:        true,
					OnValueChange: func(v *float64, ok bool, _ *native.Event) {
						setQuantity(v)
						setValid(ok)
					},
					OnValueCommitted: change(setCommitted),
					PartProps:        columnPart(),
				})
				return numberField59.Root().
					Children(func() *native.Node {
						var children []*native.Node
						state := ui.UseNumberFieldState()
						children = append(children, numberField59.ScrubArea(ui.PartProps{Style: ui.Style().Cursor("ew-resize").Height(24)}).
							Children("Drag here to change quantity").
							NativeNode())
						children = append(children, numberControls(numberField59))
						children = append(children, numberField59.ScrubAreaCursor(ui.PartProps{}).
							Children(func() *native.Node {
								return ui.Show(
									state().Scrubbing,
									func() *ui.Element {
										return label("Scrubbing")
									},
								)
							}).
							NativeNode())
						children = append(children, note(func() string {
							return "scrubbing " + strconv.FormatBool(state().Scrubbing) + " · required " + strconv.FormatBool(state().Required)
						}).Node)
						return ui.Fragment(children)
					}).
					NativeNode()
			}(),
			func() *native.Node {
				numberField60 := ui.NewNumberField(ui.NumberFieldRootProps{
					DefaultValue: ptr(42.0),
					ReadOnly:     true,
					PartProps:    columnPart(),
				})
				return numberField60.Root().
					Children(func() *native.Node {
						return ui.Fragment([]*native.Node{
							label("Read-only").Node,
							numberControls(numberField60),
						})
					}).
					NativeNode()
			}(),
			note(func() string {
				return "quantity " + textValue(quantity()) + " · valid " + strconv.FormatBool(valid()) + " · committed " + textValue(committed())
			}).Node,
		})
	})
}
func OtpFieldDemo() *ui.Element {
	code, setCode := ui.CreateSignal("")
	completed, setCompleted := ui.CreateSignal("not yet")
	return panel("OTP field", "Six slots: typing advances, paste distributes, and Backspace walks back. Completion is reported once per completed value.", func() *native.Node {
		return ui.Fragment([]*native.Node{
			func() *native.Node {
				otpField61 := ui.NewOtpField(ui.OtpFieldRootProps{
					Value:          code,
					Length:         6,
					ValidationType: "numeric",
					OnValueChange:  change(setCode),
					OnComplete:     change(setCompleted),
					PartProps:      rowPart(),
				})
				return otpField61.Root().
					Children(func() *native.Node {
						var children []*native.Node
						for i := range 6 {
							if i == 3 {
								children = append(children, otpField61.SeparatorWith(ui.OtpFieldSeparatorProps{Index: ptr(i)}).
									Children("−").
									NativeNode())
							}
							s := inputStyle()
							s = s.Width(36)
							s = s.Height(40)
							s = s.TextAlign("center")
							s = s.FontSize(18)
							children = append(children, otpField61.InputWith(ui.OtpFieldInputProps{
								Index:          i,
								InputPartProps: ui.InputPartProps{PartProps: ui.PartProps{Style: s}},
							}).
								NativeNode())
						}
						return ui.Fragment(children)
					}).
					NativeNode()
			}(),
			row(func() *native.Node {
				return ui.Fragment([]*native.Node{
					button("Fill 123456", func() {
						setCode("123456")
					}).Node,
					button("Clear", func() {
						setCode("")
					}).Node,
				})
			}).Node,
			note(func() string {
				return "code " + code() + " · completed " + completed()
			}).Node,
		})
	})
}
func segment(width int) ui.PartProps {
	s := inputStyle()
	s = s.Width(width)
	s = s.TextAlign("center")
	s = s.PaddingLeft(4)
	s = s.PaddingRight(4)
	return ui.PartProps{Style: s}
}
func DateFieldDemo() *ui.Element {
	due, setDue := ui.CreateSignal(ptr("2026-09-03"))
	iso, setISO := ui.CreateSignal(ptr("2026-01-15"))
	return panel("Date field", "Civil dates with no time zone. Up/Down steps a segment; typing advances to the next; min/max constrain the result.", func() *native.Node {
		var children []*native.Node
		for _, field := range []struct {
			value     func() *string
			set       func(*string)
			format    string
			segments  []string
			separator string
		}{{due, setDue, "mdy", []string{"month", "day", "year"}, "/"}, {iso, setISO, "ymd", []string{"year", "month", "day"}, "-"}} {
			children = append(children, func() *native.Node {
				dateField62 := ui.NewDateField(ui.DateFieldRootProps{
					Value:         field.value,
					OnValueChange: change(field.set),
					Min:           "2026-01-01",
					Max:           "2026-12-31",
					Format:        field.format,
					PartProps:     rowPart(),
				})
				return dateField62.Root().
					Children(func() *native.Node {
						var children []*native.Node
						for i, name := range field.segments {
							if i > 0 {
								children = append(children, label(field.separator).Node)
							}
							children = append(children, dateField62.SegmentWith(ui.DateFieldSegmentProps{
								Segment:   name,
								PartProps: segment(choose(name == "year", 64, 40)),
							}).
								NativeNode())
						}
						return ui.Fragment(children)
					}).
					NativeNode()
			}())
		}
		children = append(children, note(func() string {
			return "US " + textValue(due()) + " · ISO " + textValue(iso())
		}).Node)
		return ui.Fragment(children)
	})
}
func TimeFieldDemo() *ui.Element {
	at, setAt := ui.CreateSignal(ptr("09:30"))
	precise, setPrecise := ui.CreateSignal(ptr("14:05:30"))
	return panel("Time field", "12-hour and 24-hour civil times. A seconds segment exists only when ShowSeconds is enabled.", func() *native.Node {
		return ui.Fragment([]*native.Node{
			func() *native.Node {
				timeField63 := ui.NewTimeField(ui.TimeFieldRootProps{
					Value:         at,
					OnValueChange: change(setAt),
					Hour12:        true,
					PartProps:     rowPart(),
				})
				return timeField63.Root().
					Children(func() *native.Node {
						return ui.Fragment([]*native.Node{
							timeField63.SegmentWith(ui.TimeFieldSegmentProps{
								Segment:   "hour",
								PartProps: segment(40),
							}).
								NativeNode(),
							label(":").Node,
							timeField63.SegmentWith(ui.TimeFieldSegmentProps{
								Segment:   "minute",
								PartProps: segment(40),
							}).
								NativeNode(),
							timeField63.SegmentWith(ui.TimeFieldSegmentProps{
								Segment:   "period",
								PartProps: segment(48),
							}).
								NativeNode(),
						})
					}).
					NativeNode()
			}(),
			func() *native.Node {
				timeField64 := ui.NewTimeField(ui.TimeFieldRootProps{
					Value:         precise,
					OnValueChange: change(setPrecise),
					ShowSeconds:   true,
					PartProps:     rowPart(),
				})
				return timeField64.Root().
					Children(func() *native.Node {
						var children []*native.Node
						for i, name := range []string{"hour", "minute", "second"} {
							if i > 0 {
								children = append(children, label(":").Node)
							}
							children = append(children, timeField64.SegmentWith(ui.TimeFieldSegmentProps{
								Segment:   name,
								PartProps: segment(40),
							}).
								NativeNode())
						}
						return ui.Fragment(children)
					}).
					NativeNode()
			}(),
			note(func() string {
				return "12-hour " + textValue(at()) + " · precise " + textValue(precise())
			}).Node,
		})
	})
}
func monthGrid(month string) [][]string {
	first, err := time.Parse("2006-01", month)
	if err != nil {
		return nil
	}
	start := first.AddDate(0, 0, -int(first.Weekday()))
	weeks := make([][]string, 6)
	for week := range weeks {
		weeks[week] = make([]string, 7)
		for day := range weeks[week] {
			weeks[week][day] = start.AddDate(0, 0, week*7+day).Format("2006-01-02")
		}
	}
	return weeks
}
func CalendarDemo() *ui.Element {
	day, setDay := ui.CreateSignal(ptr("2026-09-03"))
	month, setMonth := ui.CreateSignal("2026-09")
	focused, setFocused := ui.CreateSignal("2026-09-03")
	return panel("Calendar", "An application-declared grid with native keyboard navigation, one Tab stop, and reported month changes.", func() *native.Node {
		return ui.Fragment([]*native.Node{
			row(func() *native.Node {
				return ui.Fragment([]*native.Node{
					button("Previous month", func() {
						t, _ := time.Parse("2006-01", month())
						setMonth(t.AddDate(0, -1, 0).Format("2006-01"))
					}).Node,
					label(month).Node,
					button("Next month", func() {
						t, _ := time.Parse("2006-01", month())
						setMonth(t.AddDate(0, 1, 0).Format("2006-01"))
					}).Node,
				})
			}).Node,
			func() *native.Node {
				calendar65 := ui.NewCalendar(ui.CalendarRootProps{
					Value:         day,
					OnValueChange: change(setDay),
					OnMonthChange: change(setMonth),
					OnFocusChange: change(setFocused),
					Min:           "2026-01-01",
					Max:           "2026-12-31",
					FirstWeekday:  0,
					PartProps:     columnPart(),
				})
				return calendar65.Root().
					Children(func() *native.Node {
						return ui.Fragment([]*native.Node{
							ui.View().
								Child(func() *native.Node {
									var children []*native.Node
									for _, name := range []string{"Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"} {
										children = append(children, ui.Text(name).Width(32).TextAlign("center").FontSize(10).Node)
									}
									return ui.Fragment(children)
								}).
								Display("flex").
								Gap(3).Node,
							ui.For(
								func() [][]string {
									return monthGrid(month())
								},
								func(week []string, index func() int) *native.Node {
									return calendar65.WeekWith(ui.CalendarWeekProps{
										Index:     ptr(index()),
										PartProps: ui.PartProps{Style: ui.Style().Display("flex").Gap(3)},
									}).
										Children(func() *native.Node {
											var children []*native.Node
											for _, date := range week {
												selected := func() bool {
													return day() != nil && *day() == date
												}
												children = append(children, calendar65.DayWith(ui.CalendarDayProps{
													Day: date,
													PartProps: ui.PartProps{Style: func() ui.StyleBuilder {
														return ui.Style().
															Display("flex").
															AlignItems("center").
															JustifyContent("center").
															Width(32).
															Height(28).
															BorderRadius(7).
															BackgroundColor(choose(selected(), p().Accent, p().PanelAlt)).
															TextColor(choose(selected(), p().OnAccent, choose(strings.HasPrefix(date, month()), p().Ink, p().Faint))).
															FontSize(11).
															Hover(func(s ui.StyleBuilder) ui.StyleBuilder {
																return s.BackgroundColor(choose(selected(), p().Accent, p().ControlHover))
															}).
															FocusStyle(func(s ui.StyleBuilder) ui.StyleBuilder {
																return s.Outline("2px solid " + p().Accent)
															})
													}},
												}).
													Children(date[8:]).
													NativeNode())
											}
											return ui.Fragment(children)
										}).
										NativeNode()
								},
								nil,
								nil,
							),
						})
					}).
					NativeNode()
			}(),
			note(func() string {
				return "selected " + textValue(day()) + " · showing " + month() + " · tab stop " + focused()
			}).Node,
		})
	})
}
