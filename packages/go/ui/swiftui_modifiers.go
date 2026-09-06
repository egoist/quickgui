package ui

// SwiftUIModifier is one bounded modifier record the Rust SwiftUI bridge interprets.
type SwiftUIModifier struct {
	Type         string   `json:"$type"`
	Style        string   `json:"style,omitempty"`
	Shape        string   `json:"shape,omitempty"`
	CornerRadius *float64 `json:"cornerRadius,omitempty"`
	Size         string   `json:"size,omitempty"`
	Color        string   `json:"color,omitempty"`
	Disabled     *bool    `json:"disabled,omitempty"`
}

func (swiftUIAPI) ButtonStyle(style string) SwiftUIModifier {
	return SwiftUIModifier{Type: "buttonStyle", Style: style}
}

func (swiftUIAPI) ButtonBorderShape(shape string, cornerRadius ...float64) SwiftUIModifier {
	modifier := SwiftUIModifier{Type: "buttonBorderShape", Shape: shape}
	if len(cornerRadius) > 0 {
		radius := cornerRadius[0]
		modifier.CornerRadius = &radius
	}
	return modifier
}

func (swiftUIAPI) ControlSize(size string) SwiftUIModifier {
	return SwiftUIModifier{Type: "controlSize", Size: size}
}

func (swiftUIAPI) LabelStyle(style string) SwiftUIModifier {
	return SwiftUIModifier{Type: "labelStyle", Style: style}
}

func (swiftUIAPI) Tint(color string) SwiftUIModifier {
	return SwiftUIModifier{Type: "tint", Color: color}
}

func (swiftUIAPI) Disabled(value ...bool) SwiftUIModifier {
	flag := true
	if len(value) > 0 {
		flag = value[0]
	}
	return SwiftUIModifier{Type: "disabled", Disabled: &flag}
}
