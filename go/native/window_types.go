package native

import "encoding/json"

type Size struct {
	Width  float64 `json:"width"`
	Height float64 `json:"height"`
}
type WindowBounds struct {
	X      float64 `json:"x"`
	Y      float64 `json:"y"`
	Width  float64 `json:"width"`
	Height float64 `json:"height"`
	State  string  `json:"state,omitempty"`
}
type WindowResizePolicy struct {
	AspectRatio *float64 `json:"aspectRatio,omitempty"`
	Minimum     *Size    `json:"minimum,omitempty"`
	Maximum     *Size    `json:"maximum,omitempty"`
	Snap        *Size    `json:"snap,omitempty"`
}
type WindowMovePolicy struct {
	KeepOnScreen *bool `json:"keepOnScreen,omitempty"`
}
type TaskbarProgress struct {
	State    string  `json:"state"`
	Progress float64 `json:"progress"`
}
type TaskbarOverlay struct {
	Icon        ImageSource
	Description string
}
type WindowNativeTabs struct {
	Count           uint32
	SelectedIndex   *uint32
	TabBarVisible   bool
	OverviewVisible bool
	Truncated       bool
}

// FrameMetrics is the latest CPU-side frame timing retained for one window.
type FrameMetrics struct {
	FrameNumber               uint64  `json:"frameNumber"`
	CPUMilliseconds           float64 `json:"cpuMilliseconds"`
	SmoothedCPUMilliseconds   float64 `json:"smoothedCpuMilliseconds"`
	FrameMilliseconds         float64 `json:"frameMilliseconds"`
	SmoothedFrameMilliseconds float64 `json:"smoothedFrameMilliseconds"`
}

// WindowState is the native window snapshot, in logical pixels.
type WindowState struct {
	DisplayID                string           `json:"displayId"`
	Kind                     string           `json:"kind"`
	X                        float64          `json:"x"`
	Y                        float64          `json:"y"`
	Width                    float64          `json:"width"`
	Height                   float64          `json:"height"`
	ViewportWidth            float64          `json:"viewportWidth"`
	ViewportHeight           float64          `json:"viewportHeight"`
	MinimumWidth             *float64         `json:"minimumWidth"`
	MinimumHeight            *float64         `json:"minimumHeight"`
	MaximumWidth             *float64         `json:"maximumWidth"`
	MaximumHeight            *float64         `json:"maximumHeight"`
	ScaleFactor              float64          `json:"scaleFactor"`
	Appearance               string           `json:"appearance"`
	BackgroundAppearance     string           `json:"backgroundAppearance"`
	Vibrancy                 string           `json:"vibrancy"`
	VisualEffectState        string           `json:"visualEffectState"`
	Focused                  bool             `json:"focused"`
	Focusable                bool             `json:"focusable"`
	Visible                  bool             `json:"visible"`
	Minimized                bool             `json:"minimized"`
	Maximized                bool             `json:"maximized"`
	Fullscreen               bool             `json:"fullscreen"`
	Occluded                 bool             `json:"occluded"`
	Movable                  bool             `json:"movable"`
	Resizable                bool             `json:"resizable"`
	Minimizable              bool             `json:"minimizable"`
	Maximizable              bool             `json:"maximizable"`
	Closable                 bool             `json:"closable"`
	Decorated                bool             `json:"decorated"`
	Shadow                   bool             `json:"shadow"`
	ContentProtected         bool             `json:"contentProtected"`
	WindowLevel              string           `json:"windowLevel"`
	SkipTaskbar              bool             `json:"skipTaskbar"`
	VisibleOnAllWorkspaces   bool             `json:"visibleOnAllWorkspaces"`
	Opacity                  float64          `json:"opacity"`
	HasIcon                  bool             `json:"hasIcon"`
	TaskbarProgressState     string           `json:"taskbarProgressState"`
	TaskbarProgress          float64          `json:"taskbarProgress"`
	HasTaskbarOverlayIcon    bool             `json:"hasTaskbarOverlayIcon"`
	CursorVisible            bool             `json:"cursorVisible"`
	CursorGrab               string           `json:"cursorGrab"`
	CursorHitTest            bool             `json:"cursorHitTest"`
	CursorX                  *float64         `json:"cursorX"`
	CursorY                  *float64         `json:"cursorY"`
	RepresentedFile          bool             `json:"representedFile"`
	DocumentEdited           bool             `json:"documentEdited"`
	NativeTabbing            bool             `json:"nativeTabbing"`
	NativeTabCount           uint32           `json:"nativeTabCount"`
	NativeSelectedTab        *uint32          `json:"nativeSelectedTab"`
	NativeTabBarVisible      bool             `json:"nativeTabBarVisible"`
	NativeTabOverviewVisible bool             `json:"nativeTabOverviewVisible"`
	NativeTabsTruncated      bool             `json:"nativeTabsTruncated"`
	Bounds                   Rectangle        `json:"-"`
	ViewportSize             Size             `json:"-"`
	MinimumSize              *Size            `json:"-"`
	MaximumSize              *Size            `json:"-"`
	CursorPosition           *Point           `json:"-"`
	NativeTabs               WindowNativeTabs `json:"-"`
}

func (state *WindowState) UnmarshalJSON(data []byte) error {
	type snapshot WindowState
	var decoded snapshot
	if err := json.Unmarshal(data, &decoded); err != nil {
		return err
	}
	*state = WindowState(decoded)
	state.Bounds = Rectangle{X: state.X, Y: state.Y, Width: state.Width, Height: state.Height}
	state.ViewportSize = Size{Width: state.ViewportWidth, Height: state.ViewportHeight}
	if state.MinimumWidth != nil && state.MinimumHeight != nil {
		state.MinimumSize = &Size{Width: *state.MinimumWidth, Height: *state.MinimumHeight}
	}
	if state.MaximumWidth != nil && state.MaximumHeight != nil {
		state.MaximumSize = &Size{Width: *state.MaximumWidth, Height: *state.MaximumHeight}
	}
	if state.CursorX != nil && state.CursorY != nil {
		state.CursorPosition = &Point{X: *state.CursorX, Y: *state.CursorY}
	}
	state.NativeTabs = WindowNativeTabs{Count: state.NativeTabCount, SelectedIndex: state.NativeSelectedTab, TabBarVisible: state.NativeTabBarVisible, OverviewVisible: state.NativeTabOverviewVisible, Truncated: state.NativeTabsTruncated}
	return nil
}
