package ui

import (
	"github.com/egoist/quickgui/packages/go/native"
	"github.com/egoist/quickgui/packages/go/protocol"
	"github.com/egoist/quickgui/packages/go/reactive"
)

type civilValueProps struct {
	PartProps
	Value         func() *string
	DefaultValue  string
	Min           string
	Max           string
	OnValueChange func(*string, *native.Event)
}

type DateFieldRootProps struct {
	PartProps
	Value         func() *string
	DefaultValue  string
	Min           string
	Max           string
	OnValueChange func(*string, *native.Event)
	Format        string
}

type DateFieldSegmentProps struct {
	PartProps
	Segment string
}

type TimeFieldRootProps struct {
	PartProps
	Value         func() *string
	DefaultValue  string
	Min           string
	Max           string
	OnValueChange func(*string, *native.Event)
	Hour12        bool
	ShowSeconds   bool
}

type TimeFieldSegmentProps struct {
	PartProps
	Segment string
}

type CalendarRootProps struct {
	PartProps
	Value         func() *string
	DefaultValue  string
	Min           string
	Max           string
	OnValueChange func(*string, *native.Event)
	FirstWeekday  any
	OnFocusChange func(string, *native.Event)
	OnMonthChange func(string, *native.Event)
}

type CalendarWeekProps struct {
	PartProps
	Index *int
}

type CalendarDayProps struct {
	PartProps
	Day string
}

var dateFieldContext = createPartContext[string]()
var timeFieldContext = createPartContext[string]()
var calendarContext = createPartContext[string]()

func createCivilRoot(
	part, scope string,
	context *reactive.Context[*string],
	props civilValueProps,
	declare func(*native.Node),
	report func(*native.Node, *native.Event),
) *native.Node {
	var initial *string
	if props.DefaultValue != "" {
		value := props.DefaultValue
		initial = &value
	}
	uncontrolled := reactive.NewSignal(initial)
	value := func() *string {
		if props.Value != nil {
			return props.Value()
		}
		return uncontrolled.Read()
	}
	node := createViewPart(props.PartProps)
	setPart(node, part, scope, "")
	reactive.CreateRenderEffect(func() {
		current := value()
		if current == nil {
			setString(node, protocol.CivilValue, "")
			return
		}
		setString(node, protocol.CivilValue, *current)
	})
	setString(node, protocol.CivilMinimum, props.Min)
	setString(node, protocol.CivilMaximum, props.Max)
	declare(node)
	setListener(node, protocol.EventComponentChange, func(event *native.Event) {
		details := ComponentChangeFromEvent(event)
		if details == nil {
			return
		}
		if details.Value.present() {
			next := details.Value.ptr()
			if props.Value == nil {
				uncontrolled.Write(next)
			}
			if props.OnValueChange != nil {
				props.OnValueChange(next, event)
			}
		}
		report(node, event)
	})
	return reactive.Provide(context, &scope, func() *native.Node {
		return finishPart(node, props.PartProps)
	})
}

func ignoreCivilReport(*native.Node, *native.Event) {}

func civilScope(context *reactive.Context[*string]) string {
	if current := context.Use(); current != nil {
		return *current
	}
	return ""
}

func civilFromDate(props DateFieldRootProps) civilValueProps {
	return civilValueProps{
		PartProps: props.PartProps, Value: props.Value, DefaultValue: props.DefaultValue,
		Min: props.Min, Max: props.Max, OnValueChange: props.OnValueChange,
	}
}

func civilFromTime(props TimeFieldRootProps) civilValueProps {
	return civilValueProps{
		PartProps: props.PartProps, Value: props.Value, DefaultValue: props.DefaultValue,
		Min: props.Min, Max: props.Max, OnValueChange: props.OnValueChange,
	}
}

func civilFromCalendar(props CalendarRootProps) civilValueProps {
	return civilValueProps{
		PartProps: props.PartProps, Value: props.Value, DefaultValue: props.DefaultValue,
		Min: props.Min, Max: props.Max, OnValueChange: props.OnValueChange,
	}
}

// DateField is a controlled date field. Values are ISO YYYY-MM-DD civil dates.
var DateField = dateFieldAPI{}

type dateFieldAPI struct{}

func (dateFieldAPI) Root(props DateFieldRootProps) *native.Node {
	return createCivilRoot(
		protocol.PartDateField,
		createComponentScope("qg-date-field"),
		dateFieldContext,
		civilFromDate(props),
		func(node *native.Node) {
			setString(node, protocol.SegmentOrder, props.Format)
		},
		ignoreCivilReport,
	)
}

func (dateFieldAPI) Segment(props DateFieldSegmentProps) *native.Node {
	node := createViewPart(props.PartProps)
	setPart(node, protocol.PartDateFieldSegment, civilScope(dateFieldContext), "")
	setString(node, protocol.Segment, props.Segment)
	return finishPart(node, props.PartProps)
}

// TimeField is a controlled time field. Values are HH:MM or HH:MM:SS civil times.
var TimeField = timeFieldAPI{}

type timeFieldAPI struct{}

func (timeFieldAPI) Root(props TimeFieldRootProps) *native.Node {
	return createCivilRoot(
		protocol.PartTimeField,
		createComponentScope("qg-time-field"),
		timeFieldContext,
		civilFromTime(props),
		func(node *native.Node) {
			order := "h23"
			if props.Hour12 {
				order = "h12"
			}
			setString(node, protocol.SegmentOrder, order)
			if props.ShowSeconds {
				setString(node, protocol.Variant, "seconds")
			}
		},
		ignoreCivilReport,
	)
}

func (timeFieldAPI) Segment(props TimeFieldSegmentProps) *native.Node {
	node := createViewPart(props.PartProps)
	setPart(node, protocol.PartTimeFieldSegment, civilScope(timeFieldContext), "")
	setString(node, protocol.Segment, props.Segment)
	return finishPart(node, props.PartProps)
}

// Calendar is a controlled month grid.
var Calendar = calendarAPI{}

type calendarAPI struct{}

func (calendarAPI) Root(props CalendarRootProps) *native.Node {
	return createCivilRoot(
		protocol.PartCalendar,
		createComponentScope("qg-calendar"),
		calendarContext,
		civilFromCalendar(props),
		func(node *native.Node) {
			if props.FirstWeekday != nil {
				setNumber(node, protocol.FirstWeekday, props.FirstWeekday)
			}
		},
		func(_ *native.Node, event *native.Event) {
			details := ComponentChangeFromEvent(event)
			if details == nil {
				return
			}
			if focused := details.Focused.ptr(); focused != nil && props.OnFocusChange != nil {
				props.OnFocusChange(*focused, event)
			}
			if details.Month != "" && props.OnMonthChange != nil {
				props.OnMonthChange(details.Month, event)
			}
		},
	)
}

func (calendarAPI) Week(props CalendarWeekProps) *native.Node {
	node := createViewPart(props.PartProps)
	setPart(node, protocol.PartCalendarWeek, civilScope(calendarContext), "")
	if props.Index != nil {
		setNumber(node, protocol.ItemIndex, *props.Index)
	}
	return finishPart(node, props.PartProps)
}

func (calendarAPI) Day(props CalendarDayProps) *native.Node {
	node := createViewPart(props.PartProps)
	setPart(node, protocol.PartCalendarDay, civilScope(calendarContext), "")
	setString(node, protocol.CivilValue, props.Day)
	return finishPart(node, props.PartProps)
}
