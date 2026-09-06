package ui

import (
	"github.com/egoist/quickgui/packages/go/native"
	"github.com/egoist/quickgui/packages/go/protocol"
	"github.com/egoist/quickgui/packages/go/reactive"
)

// GaugeState is everything the core decided about one progress bar or meter.
type GaugeState struct {
	Status       string
	DisplayValue *string
	Completion   *float64
}

func settledGauge() GaugeState {
	return GaugeState{Status: "indeterminate"}
}

type GaugeFormatProps struct {
	PartProps
	Format         string
	OnStatusChange func(GaugeState, *native.Event)
}

type ProgressProps struct {
	GaugeFormatProps
	Value         func() *float64
	Max           any
	Indeterminate func() bool
	ValueText     func() *string
}

type MeterProps struct {
	GaugeFormatProps
	Value   func() *float64
	Min     any
	Max     any
	Low     any
	High    any
	Optimum any
}

type gaugeContextValue struct {
	state *reactive.Signal[GaugeState]
}

func (g *gaugeContextValue) live() GaugeState {
	return g.state.Read()
}

var gaugeContext = createPartContext[gaugeContextValue]()

// UseGaugeState reads the live status inside a Progress.Root or Meter.Root subtree.
func UseGaugeState() func() GaugeState {
	context := gaugeContext.Use()
	if context == nil {
		return settledGauge
	}
	return context.live
}

func createGaugeRoot(part string, props GaugeFormatProps, declare func(*native.Node)) *native.Node {
	context := &gaugeContextValue{state: reactive.NewSignal(settledGauge())}
	node := createViewPart(props.PartProps)
	setPart(node, part, createComponentScope("qg-gauge"), "")
	if props.Format != "" {
		setString(node, protocol.Format, props.Format)
	}
	declare(node)
	setListener(node, protocol.EventComponentChange, func(event *native.Event) {
		details := ComponentChangeFromEvent(event)
		if details == nil || details.Status == "" {
			return
		}
		next := GaugeState{
			Status:       details.Status,
			DisplayValue: details.DisplayValue.ptr(),
			Completion:   details.Completion.ptr(),
		}
		context.state.Write(next)
		if props.OnStatusChange != nil {
			props.OnStatusChange(next, event)
		}
	})
	return reactive.Provide(gaugeContext, context, func() *native.Node {
		return finishPart(node, props.PartProps)
	})
}

func createGaugePart(part string, props PartProps) *native.Node {
	node := createViewPart(props)
	setPart(node, part, "", "")
	return finishPart(node, props)
}

func bindGaugeValue(node *native.Node, value func() *float64) {
	if value == nil {
		return
	}
	reactive.CreateRenderEffect(func() {
		current := value()
		if current == nil {
			setJson(node, protocol.Values, 65536, nil)
			return
		}
		setJson(node, protocol.Values, 65536, []float64{*current})
	})
}

// Progress is a determinate or indeterminate progress indicator.
var Progress = progressAPI{}

type progressAPI struct{}

func (progressAPI) Root(props ProgressProps) *native.Node {
	return createGaugeRoot(protocol.PartProgress, props.GaugeFormatProps, func(node *native.Node) {
		bindGaugeValue(node, props.Value)
		max := props.Max
		if max == nil {
			max = 1
		}
		setNumber(node, protocol.Maximum, max)
		if props.Indeterminate != nil {
			reactive.CreateRenderEffect(func() {
				setExplicitBool(node, protocol.Indeterminate, props.Indeterminate())
			})
		}
		if props.ValueText != nil {
			reactive.CreateRenderEffect(func() {
				current := props.ValueText()
				if current == nil {
					setString(node, protocol.ValueText, "")
					return
				}
				setString(node, protocol.ValueText, *current)
			})
		}
	})
}

func (progressAPI) Track(props PartProps) *native.Node {
	return createGaugePart(protocol.PartProgressTrack, props)
}

func (progressAPI) Indicator(props PartProps) *native.Node {
	return createGaugePart(protocol.PartProgressIndicator, props)
}

func (progressAPI) Label(props PartProps) *native.Node {
	return createGaugePart(protocol.PartProgressLabel, props)
}

func (progressAPI) Value(props PartProps) *native.Node {
	return createGaugePart(protocol.PartProgressValue, props)
}

// Meter is a static measurement gauge.
var Meter = meterAPI{}

type meterAPI struct{}

func (meterAPI) Root(props MeterProps) *native.Node {
	return createGaugeRoot(protocol.PartMeter, props.GaugeFormatProps, func(node *native.Node) {
		bindGaugeValue(node, props.Value)
		if props.Min != nil {
			setNumber(node, protocol.Minimum, props.Min)
		}
		if props.Max != nil {
			setNumber(node, protocol.Maximum, props.Max)
		}
		if props.Low != nil {
			setNumber(node, protocol.Low, props.Low)
		}
		if props.High != nil {
			setNumber(node, protocol.High, props.High)
		}
		if props.Optimum != nil {
			setNumber(node, protocol.Optimum, props.Optimum)
		}
	})
}

func (meterAPI) Track(props PartProps) *native.Node {
	return createGaugePart(protocol.PartMeterTrack, props)
}

func (meterAPI) Indicator(props PartProps) *native.Node {
	return createGaugePart(protocol.PartMeterIndicator, props)
}

func (meterAPI) Label(props PartProps) *native.Node {
	return createGaugePart(protocol.PartMeterLabel, props)
}

func (meterAPI) Value(props PartProps) *native.Node {
	return createGaugePart(protocol.PartMeterValue, props)
}
