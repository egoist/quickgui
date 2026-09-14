package main

import (
	"errors"
	"strconv"
	"strings"

	"github.com/egoist/quickgui/go/native"
)

func (s *systemState) actions() []systemAction {
	return []systemAction{
		{"App environment", func(done completion) {
			native.App.GetInfo(after(s, done, func(info *native.AppInfo) {
				native.App.GetSystemInfo(after(s, done, func(system native.SystemInfo) {
					native.App.GetPaths(after(s, done, func(paths *native.AppPaths) {
						report[any](done)(map[string]any{"application": info, "system": system, "paths": paths}, nil)
					}))
				}))
			}))
		}},
		{"Rich clipboard", func(done completion) {
			native.Clipboard.Write(
				native.ClipboardItem{Entries: []native.ClipboardEntry{
					{Type: "text", Text: "Hello from QuickGUI", Metadata: `{"source":"system-api"}`},
					{Type: "data", MIMEType: "text/html", Data: []byte("<strong>Hello from QuickGUI</strong>")},
					{Type: "bookmark", Title: "QuickGUI", URL: "https://github.com/egoist/quickgui"},
				}},
				func(err error) {
					if err != nil {
						done("", err)
						return
					}
					if !s.alive() {
						return
					}
					native.Clipboard.Read(after(s, done, func(item *native.ClipboardItem) {
						var kinds []string
						if item != nil {
							for _, entry := range item.Entries {
								kinds = append(kinds, entry.Type)
							}
						}
						if len(kinds) == 0 {
							kinds = []string{"none"}
						}
						done("Clipboard representations: "+strings.Join(kinds, ", "), nil)
					}))
				},
			)
		}},
		{"Displays", func(done completion) {
			native.Screen.GetAllDisplays(after(s, done, func(displays []native.Display) {
				native.Screen.GetPrimaryDisplay(after(s, done, func(primary *native.Display) {
					native.Screen.GetCursorScreenPoint(after(s, done, func(cursor native.Point) {
						report[any](done)(map[string]any{"displays": displays, "primary": primary, "cursor": cursor}, nil)
					}))
				}))
			}))
		}},
		{"Preferences", func(done completion) {
			native.SystemPreferences.GetCurrent(report[native.SystemPreferencesSnapshot](done))
		}},
		{"Keyboard layout", func(done completion) { native.Keyboard.GetLayout(report[native.KeyboardLayout](done)) }},
		{"Permissions", func(done completion) {
			values := map[native.PermissionKind]native.PermissionStatus{}
			for _, kind := range []native.PermissionKind{
				native.PermissionCamera,
				native.PermissionMicrophone,
				native.PermissionScreenRecording,
				native.PermissionAccessibility,
			} {
				status, err := native.Permissions.Status(kind)
				if err != nil {
					done("", err)
					return
				}
				values[kind] = status
			}
			report[any](done)(values, nil)
		}},
		{"Power snapshot", func(done completion) {
			power, err := native.PowerMonitor.GetState()
			if err != nil {
				done("", err)
				return
			}
			idle, err := native.PowerMonitor.GetSystemIdleTime()
			if err != nil {
				done("", err)
				return
			}
			state, err := native.PowerMonitor.GetSystemIdleState(60)
			if err != nil {
				done("", err)
				return
			}
			report[any](done)(map[string]any{"power": power, "idleSeconds": idle, "idleState": state}, nil)
		}},
		{"Sleep assertion", s.toggleAssertion},
		{"Window state", func(done completion) { s.window.GetState(report[native.WindowState](done)) }},
		{"Toggle opacity", func(done completion) {
			native.Desktop.GetSupport(after(s, done, func(support native.DesktopIntegrationSupport) {
				if !support.WindowOpacity {
					done("Window opacity is unsupported here.", nil)
					return
				}
				s.dimmed = !s.dimmed
				opacity := 1.0
				if s.dimmed {
					opacity = 0.82
				}
				s.window.SetOpacity(opacity)
				done("Window opacity is now "+strconv.FormatFloat(opacity*100, 'f', 0, 64)+"%.", nil)
			}))
		}},
		{"Desktop support", func(done completion) { native.Desktop.GetSupport(report[native.DesktopIntegrationSupport](done)) }},
		{"About panel", func(done completion) {
			native.Desktop.GetSupport(after(s, done, func(support native.DesktopIntegrationSupport) {
				if !support.NativeAboutPanel {
					done("A native About panel is unsupported here.", nil)
					return
				}
				native.App.GetInfo(after(s, done, func(info *native.AppInfo) {
					options := native.AboutPanelOptions{
						ApplicationName: appName,
						Credits:         "Native desktop integrations exposed through Go and purego.",
					}
					if info != nil {
						options.ApplicationVersion = info.Version
					}
					native.Desktop.ShowAboutPanel(options)
					done("Opened the native About panel.", nil)
				}))
			}))
		}},
		{"Executable icon", func(done completion) {
			native.Desktop.GetSupport(after(s, done, func(support native.DesktopIntegrationSupport) {
				if !support.FileIcons {
					done("Native file icons are unsupported here.", nil)
					return
				}
				native.App.GetPaths(after(s, done, func(paths *native.AppPaths) {
					if paths == nil || paths.Executable == "" {
						done("No executable path is available.", nil)
						return
					}
					native.Desktop.GetFileIcon(
						paths.Executable,
						"normal",
						after(s, done, func(icon native.NativeImage) {
							done("Loaded a "+strconv.FormatUint(uint64(icon.Width), 10)+"×"+strconv.FormatUint(uint64(icon.Height), 10)+" native file icon.", nil)
						}),
					)
				}))
			}))
		}},
		{"Dock badge", func(done completion) {
			native.Desktop.GetSupport(after(s, done, func(support native.DesktopIntegrationSupport) {
				if !support.DockBadges {
					done("Dock badges are unsupported here.", nil)
					return
				}
				s.badge = !s.badge
				if s.badge {
					native.Desktop.SetDockBadge("1")
					done("Dock badge set.", nil)
				} else {
					native.Desktop.SetDockBadge("")
					done("Dock badge cleared.", nil)
				}
			}))
		}},
		{"Notification", func(done completion) {
			native.Notifications.RequestPermission(after(s, done, func(permission native.NotificationPermissionStatus) {
				if permission != native.NotificationPermissionGranted && permission != native.NotificationPermissionUnsupported {
					done("Notification permission is "+string(permission)+".", nil)
					return
				}
				native.Notifications.Show(
					native.NotificationOptions{
						Tag:      "system-api-example",
						Title:    "QuickGUI",
						Body:     "Native notification delivery is working.",
						Subtitle: "Native system integration",
						Sound:    "default",
						Actions: []native.NotificationAction{
							{ID: "open", Label: "Open"},
							{ID: "reply", Label: "Reply", Type: "text-input", Placeholder: "Message"},
						},
					},
					finished(done, "Notification delivered."),
				)
			}))
		}},
		{"Open website", func(done completion) {
			native.Shell.OpenExternal(
				"https://github.com/egoist/quickgui",
				finished(done, "Opened the QuickGUI repository."),
			)
		}},
		{"Secure storage", func(done completion) {
			native.SecureStorage.SetText(
				identifier,
				"example-token",
				"native-secret",
				after(s, done, func(stored bool) {
					if !stored {
						done("", errors.New("the credential was not stored"))
						return
					}
					native.SecureStorage.GetText(
						identifier,
						"example-token",
						after(s, done, func(value *string) {
							if value == nil || *value != "native-secret" {
								done("", errors.New("credential round trip did not match"))
								return
							}
							done("Credential round trip verified.", nil)
						}),
					)
				}),
			)
		}},
		{"Toggle autostart", func(done completion) {
			options := native.AutoStartOptions{AppName: identifier, BundleIdentifier: identifier}
			native.AutoStart.IsEnabled(
				options,
				after(s, done, func(enabled bool) {
					if enabled {
						native.AutoStart.Disable(
							options,
							finished(done, "Autostart is now disabled."),
						)
					} else {
						native.AutoStart.Enable(
							options,
							finished(done, "Autostart is now enabled."),
						)
					}
				}),
			)
		}},
		{"Register deep link", func(done completion) {
			supported, err := native.DeepLink.SupportsDynamicRegistration()
			if err != nil {
				done("", err)
				return
			}
			if !supported {
				done("macOS deep links are declared by protocols in quickgui.config.ts.", nil)
				return
			}
			native.DeepLink.Register(
				native.ProtocolRegistrationOptions{
					Scheme:  "quickgui-system",
					AppName: appName,
					AppID:   identifier,
				},
				after(s, done, func(registered bool) {
					if !registered {
						done("", errors.New("protocol registration was not accepted"))
						return
					}
					done("Registered quickgui-system:// with the current user account.", nil)
				}),
			)
		}},
		{"Global shortcut", s.toggleShortcut},
		{"Tray icon", s.toggleTray},
	}
}

func (s *systemState) toggleAssertion(done completion) {
	if s.assertion != nil {
		_, err := s.assertion.Release()
		if err == nil {
			s.assertion = nil
		}
		done("Released the application-suspension assertion.", err)
		return
	}
	assertion, err := native.NewPowerAssertion(
		"prevent-application-suspension",
		"QuickGUI system API example",
	)
	if err == nil {
		s.assertion = assertion
	}
	done("Preventing application suspension until clicked again or the window closes.", err)
}

func (s *systemState) toggleShortcut(done completion) {
	if s.shortcut != nil {
		s.shortcut.Unregister(func(err error) {
			if err == nil {
				s.shortcut = nil
			}
			done("Unregistered shift+alt+KeyQ.", err)
		})
		return
	}
	native.GlobalShortcut.Register(
		"shift+alt+KeyQ",
		func() {
			s.showWindow()
			if s.alive() {
				s.status("The global shortcut was pressed.")
			}
		},
		func(shortcut *native.ShortcutRegistration, err error) {
			if !s.alive() {
				if shortcut != nil {
					shortcut.Unregister(logError)
				}
				return
			}
			if err == nil {
				s.shortcut = shortcut
			}
			done("Registered shift+alt+KeyQ.", err)
		},
	)
}

func (s *systemState) toggleTray(done completion) {
	if s.tray != nil {
		s.tray.Destroy(func(err error) {
			if err == nil {
				s.tray = nil
			}
			done("Removed the tray icon.", err)
		})
		return
	}
	native.Tray.Create(
		native.TrayIconOptions{
			Icon:           makeTrayIcon(),
			IconIsTemplate: true,
			Tooltip:        appName,
			Menu: []native.TrayMenuItem{
				{Label: "Show window", Click: s.showWindow},
				{Type: "separator"},
				{Label: "Quit", Click: func() { native.App.Quit(false, nil) }},
			},
		},
		func(tray *native.TrayIcon, err error) {
			if !s.alive() {
				if tray != nil {
					tray.Destroy(logError)
				}
				return
			}
			if err == nil {
				s.tray = tray
				tray.OnEvent(func(event native.TrayEvent) {
					if s.alive() && event.Kind == "click" {
						s.status("Tray icon clicked.")
					}
				})
			}
			done("Created the tray icon.", err)
		},
	)
}

func makeTrayIcon() native.ImageSource {
	const size = 20
	pixels := make([]byte, size*size*4)
	for y := 0; y < size; y++ {
		for x := 0; x < size; x++ {
			dx, dy := float64(x)-9.5, float64(y)-9.5
			if dx*dx+dy*dy < 78 {
				offset := (y*size + x) * 4
				copy(pixels[offset:offset+4], []byte{122, 162, 247, 255})
			}
		}
	}
	return native.ImageSource{Width: size, Height: size, Data: pixels}
}
