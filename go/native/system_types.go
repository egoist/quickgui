package native

// ImageSource supplies a file path, encoded image bytes, or RGBA8 bytes with dimensions.
//
// Template marks the artwork as a macOS template image. When omitted, paths whose stem
// ends in Template (optionally @2x) are inferred.
type ImageSource struct {
	Path     string `json:"path,omitempty"`
	Data     []byte `json:"data,omitempty"`
	Width    uint32 `json:"width,omitempty"`
	Height   uint32 `json:"height,omitempty"`
	Template *bool  `json:"template,omitempty"`
}

// TemplateImage is a file path marked as a macOS template image.
func TemplateImage(path string) ImageSource {
	template := true
	return ImageSource{Path: path, Template: &template}
}

// MenuIcon uses the native menu image encoding.
type MenuIcon struct {
	Path     string `json:"path,omitempty"`
	Data     []byte `json:"dataBase64,omitempty"`
	Width    uint32 `json:"width,omitempty"`
	Height   uint32 `json:"height,omitempty"`
	Template *bool  `json:"template,omitempty"`
}

type AboutPanelOptions struct {
	ApplicationName    string       `json:"applicationName,omitempty"`
	ApplicationVersion string       `json:"applicationVersion,omitempty"`
	Version            string       `json:"version,omitempty"`
	Copyright          string       `json:"copyright,omitempty"`
	Credits            string       `json:"credits,omitempty"`
	Icon               *ImageSource `json:"icon,omitempty"`
}

type WindowRegistry struct {
	Windows      []uint32 `json:"windows"`
	ActiveWindow *uint32  `json:"activeWindow,omitempty"`
	Truncated    bool     `json:"truncated"`
}

// Rectangle is the corresponding native platform declaration or snapshot.
type Rectangle struct {
	X      float64 `json:"x"`
	Y      float64 `json:"y"`
	Width  float64 `json:"width"`
	Height float64 `json:"height"`
}

// Display is the corresponding native platform declaration or snapshot.
type Display struct {
	ID          string    `json:"id"`
	UUID        string    `json:"uuid,omitempty"`
	Name        string    `json:"name"`
	Bounds      Rectangle `json:"bounds"`
	WorkArea    Rectangle `json:"workArea"`
	ScaleFactor float64   `json:"scaleFactor"`
	RefreshRate *float64  `json:"refreshRate,omitempty"`
	Primary     bool      `json:"primary"`
}

// DesktopIntegrationSupport is the corresponding native platform declaration or snapshot.
type DesktopIntegrationSupport struct {
	SystemNotifications         bool `json:"systemNotifications"`
	ScheduledNotifications      bool `json:"scheduledNotifications"`
	NotificationReplies         bool `json:"notificationReplies"`
	NativeApplicationMenus      bool `json:"nativeApplicationMenus"`
	NativePopupMenus            bool `json:"nativePopupMenus"`
	TrayIcons                   bool `json:"trayIcons"`
	ProgrammableTrayPopup       bool `json:"programmableTrayPopup"`
	GlobalShortcuts             bool `json:"globalShortcuts"`
	SingleInstance              bool `json:"singleInstance"`
	DynamicProtocolRegistration bool `json:"dynamicProtocolRegistration"`
	Autostart                   bool `json:"autostart"`
	WindowIcons                 bool `json:"windowIcons"`
	WindowFocusability          bool `json:"windowFocusability"`
	WindowOpacity               bool `json:"windowOpacity"`
	SkipTaskbar                 bool `json:"skipTaskbar"`
	VisibleOnAllWorkspaces      bool `json:"visibleOnAllWorkspaces"`
	CursorControl               bool `json:"cursorControl"`
	CursorScreenPosition        bool `json:"cursorScreenPosition"`
	TaskbarProgress             bool `json:"taskbarProgress"`
	TaskbarOverlayIcons         bool `json:"taskbarOverlayIcons"`
	DockBadges                  bool `json:"dockBadges"`
	DockIcons                   bool `json:"dockIcons"`
	DockMenus                   bool `json:"dockMenus"`
	RecentDocuments             bool `json:"recentDocuments"`
	FileIcons                   bool `json:"fileIcons"`
	NativeAboutPanel            bool `json:"nativeAboutPanel"`
	UserTasks                   bool `json:"userTasks"`
}

// UserTask is the corresponding native platform declaration or snapshot.
type UserTask struct {
	Title            string   `json:"title"`
	Arguments        string   `json:"arguments"`
	Program          string   `json:"program,omitempty"`
	Description      string   `json:"description,omitempty"`
	WorkingDirectory string   `json:"workingDirectory,omitempty"`
	IconPath         string   `json:"iconPath,omitempty"`
	IconIndex        *float64 `json:"iconIndex,omitempty"`
}

// NativeImage is the corresponding native platform declaration or snapshot.
type NativeImage struct {
	Data   []byte `json:"data"`
	Width  uint32 `json:"width"`
	Height uint32 `json:"height"`
}

// KeyboardLayout is the corresponding native platform declaration or snapshot.
type KeyboardLayout struct {
	ID   string `json:"id"`
	Name string `json:"name"`
}

// WindowRestoreState is the corresponding native platform declaration or snapshot.
type WindowRestoreState struct {
	X           float64 `json:"x"`
	Y           float64 `json:"y"`
	Width       float64 `json:"width"`
	Height      float64 `json:"height"`
	Maximized   bool    `json:"maximized"`
	Fullscreen  bool    `json:"fullscreen"`
	DisplayID   string  `json:"displayId,omitempty"`
	DisplayUUID string  `json:"displayUuid,omitempty"`
	ScaleFactor float64 `json:"scaleFactor"`
}

// NotificationAction is the corresponding native platform declaration or snapshot.
type NotificationAction struct {
	ID          string `json:"id"`
	Label       string `json:"label"`
	Type        string `json:"kind,omitempty"`
	Placeholder string `json:"placeholder,omitempty"`
}

// NotificationAttachment is the corresponding native platform declaration or snapshot.
type NotificationAttachment struct {
	ID   string `json:"id"`
	Path string `json:"path"`
}

// NotificationOptions is the corresponding native platform declaration or snapshot.
type NotificationOptions struct {
	Tag          string                   `json:"tag"`
	Title        string                   `json:"title"`
	Body         string                   `json:"body"`
	Subtitle     string                   `json:"subtitle,omitempty"`
	Actions      []NotificationAction     `json:"actions,omitempty"`
	Sound        string                   `json:"sound,omitempty"`
	IconPath     string                   `json:"iconPath,omitempty"`
	Attachments  []NotificationAttachment `json:"attachments,omitempty"`
	DeliveryAtMs *int64                   `json:"deliveryAtMs,omitempty"`
}

// PowerEvent is the corresponding native platform declaration or snapshot.
type PowerEvent struct {
	Type    string   `json:"type"`
	Source  string   `json:"source,omitempty"`
	State   string   `json:"state,omitempty"`
	Enabled *bool    `json:"enabled,omitempty"`
	Percent *float64 `json:"percent,omitempty"`
}

// BatteryState is the corresponding native platform declaration or snapshot.
type BatteryState struct {
	ChargePercent *float64 `json:"chargePercent,omitempty"`
	Status        string   `json:"status"`
}

// PowerState is the corresponding native platform declaration or snapshot.
type PowerState struct {
	Source               string        `json:"source"`
	Battery              *BatteryState `json:"battery,omitempty"`
	ThermalState         string        `json:"thermalState"`
	LowPowerMode         *bool         `json:"lowPowerMode,omitempty"`
	CPUSpeedLimitPercent *float64      `json:"cpuSpeedLimitPercent,omitempty"`
}

// SystemColor is the corresponding native platform declaration or snapshot.
type SystemColor struct {
	Red   float64 `json:"red"`
	Green float64 `json:"green"`
	Blue  float64 `json:"blue"`
	Alpha float64 `json:"alpha"`
}

// SystemPreferencesSnapshot is the corresponding native platform declaration or snapshot.
type SystemPreferencesSnapshot struct {
	ColorScheme               string       `json:"colorScheme"`
	ReduceMotion              *bool        `json:"reduceMotion,omitempty"`
	ReduceTransparency        *bool        `json:"reduceTransparency,omitempty"`
	IncreaseContrast          *bool        `json:"increaseContrast,omitempty"`
	DifferentiateWithoutColor *bool        `json:"differentiateWithoutColor,omitempty"`
	InvertColors              *bool        `json:"invertColors,omitempty"`
	ForcedColors              *bool        `json:"forcedColors,omitempty"`
	ScreenReader              *bool        `json:"screenReader,omitempty"`
	SwitchControl             *bool        `json:"switchControl,omitempty"`
	AccentColor               *SystemColor `json:"accentColor,omitempty"`
	HighlightColor            *SystemColor `json:"highlightColor,omitempty"`
	HighlightTextColor        *SystemColor `json:"highlightTextColor,omitempty"`
	WindowBackgroundColor     *SystemColor `json:"windowBackgroundColor,omitempty"`
	WindowTextColor           *SystemColor `json:"windowTextColor,omitempty"`
	ControlBackgroundColor    *SystemColor `json:"controlBackgroundColor,omitempty"`
	ControlTextColor          *SystemColor `json:"controlTextColor,omitempty"`
	LinkColor                 *SystemColor `json:"linkColor,omitempty"`
}
