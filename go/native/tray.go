package native

import (
	"encoding/json"
	"fmt"

	"github.com/egoist/quickgui/go/reactive"
)

type TrayMenuItem struct {
	Type    string
	Label   string
	Enabled *bool
	Checked bool
	Items   []TrayMenuItem
	Click   func()
}

type TrayIconOptions struct {
	Icon    ImageSource
	Tooltip string
	Title   string
	// IconIsTemplate treats the artwork as a macOS template image. Prefer ImageSource.Template
	// or a *Template.png filename when the flag should be omitted so the path can be inferred.
	IconIsTemplate  bool
	MenuOnLeftClick *bool
	Visible         *bool
	Menu            []TrayMenuItem
}

type TrayEvent struct {
	Kind        string   `json:"kind"`
	MenuItemID  uint32   `json:"menuItemId,omitempty"`
	Button      string   `json:"button,omitempty"`
	Position    *Point   `json:"position,omitempty"`
	Pressed     *bool    `json:"pressed,omitempty"`
	ScrollDelta *float64 `json:"scrollDelta,omitempty"`
	Horizontal  *bool    `json:"horizontal,omitempty"`
}

type trayAPI struct{}

var Tray trayAPI
var nextTrayIcon uint32 = 1
var trayIcons = map[uint32]*TrayIcon{}

// TrayIcon serializes updates so each visible native menu retains the callbacks
// that produced it. Failed updates leave the previous menu and handlers intact.
type TrayIcon struct {
	ID                      uint32
	listeners               subscriptions[TrayEvent]
	callbacks               map[uint32]menuCallback
	nextAction              uint32
	removed, removing, busy bool
	operations              []trayOperation
	destroyWaiters          []func(error)
}

type trayOperation struct {
	payload string
	done    func(error)
}

func (trayAPI) Create(options TrayIconOptions, done func(*TrayIcon, error)) {
	if nextTrayIcon >= 0xffff_fff0 {
		panic("QuickGUI tray id space exhausted")
	}
	icon := &TrayIcon{ID: nextTrayIcon, nextAction: 1}
	nextTrayIcon++
	trayIcons[icon.ID] = icon
	icon.Update(options, func(err error) {
		if err != nil {
			delete(trayIcons, icon.ID)
			icon.removed = true
			if done != nil {
				done(nil, err)
			}
		} else if done != nil {
			done(icon, nil)
		}
	})
}

func (icon *TrayIcon) OnEvent(listener func(TrayEvent)) func() {
	if icon == nil || icon.removed {
		return func() {}
	}
	return icon.listeners.add(listener)
}

func (icon *TrayIcon) Update(options TrayIconOptions, done func(error)) {
	if icon.unavailable(done) {
		return
	}
	callbacks := map[uint32]menuCallback{}
	menu := icon.encodeMenu(options.Menu, callbacks)
	native := map[string]any{
		"id": icon.ID, "menu": mustString(menu), "tooltip": options.Tooltip, "title": options.Title,
		"menuOnLeftClick": enabledOrTrue(options.MenuOnLeftClick), "visible": enabledOrTrue(options.Visible),
	}
	if options.Icon.Path != "" {
		native["iconPath"] = options.Icon.Path
	} else {
		data := options.Icon.Data
		if data == nil {
			data = []byte{}
		}
		native["iconData"] = data
		if options.Icon.Width != 0 {
			native["width"] = options.Icon.Width
		}
		if options.Icon.Height != 0 {
			native["height"] = options.Icon.Height
		}
	}
	if options.Icon.Template != nil {
		native["iconIsTemplate"] = *options.Icon.Template
	} else if options.IconIsTemplate {
		native["iconIsTemplate"] = true
	}
	icon.enqueue(trayOperation{
		payload: mustString(map[string]any{"method": "set-tray-icon", "options": native}),
		done: func(err error) {
			if err == nil {
				icon.callbacks = callbacks
			}
			if done != nil {
				done(err)
			}
		},
	})
}

func (icon *TrayIcon) ShowMenu(done func(error)) {
	if icon.unavailable(done) {
		return
	}
	icon.enqueue(trayOperation{payload: mustString(map[string]any{"method": "show-tray-menu", "id": icon.ID}), done: done})
}

func (icon *TrayIcon) Destroy(done func(error)) {
	if icon == nil || icon.removed {
		if done != nil {
			done(nil)
		}
		return
	}
	icon.destroyWaiters = append(icon.destroyWaiters, done)
	if icon.removing {
		return
	}
	icon.removing = true
	icon.enqueue(trayOperation{
		payload: mustString(map[string]any{"method": "remove-tray-icon", "id": icon.ID}),
		done: func(err error) {
			icon.removing = false
			if err == nil {
				icon.removed = true
				delete(trayIcons, icon.ID)
				icon.callbacks = nil
				icon.listeners.clear()
			}
			waiters := icon.destroyWaiters
			icon.destroyWaiters = nil
			for _, waiter := range waiters {
				if waiter != nil {
					waiter(err)
				}
			}
		},
	})
}

func (icon *TrayIcon) unavailable(done func(error)) bool {
	if icon != nil && !icon.removed && !icon.removing {
		return false
	}
	if done != nil {
		done(fmt.Errorf("the tray icon was removed"))
	}
	return true
}

func (icon *TrayIcon) enqueue(operation trayOperation) {
	icon.operations = append(icon.operations, operation)
	icon.runNext()
}

func (icon *TrayIcon) runNext() {
	if icon.busy || len(icon.operations) == 0 {
		return
	}
	icon.busy = true
	operation := icon.operations[0]
	icon.operations[0] = trayOperation{}
	icon.operations = icon.operations[1:]
	SendCommand(operation.payload, func(_ string, err error) {
		defer func() { icon.busy = false; icon.runNext() }()
		if operation.done != nil {
			operation.done(err)
		}
	})
}

func (icon *TrayIcon) encodeMenu(items []TrayMenuItem, callbacks map[uint32]menuCallback) []nativeMenuItem {
	result := make([]nativeMenuItem, 0, len(items))
	for _, item := range items {
		kind := item.Type
		if kind == "" {
			kind = "action"
			if item.Items != nil {
				kind = "submenu"
			}
		}
		entry := nativeMenuItem{Type: kind, Label: item.Label, Enabled: enabledOrTrue(item.Enabled), Checked: item.Checked}
		switch kind {
		case "separator":
		case "submenu":
			children := icon.encodeMenu(item.Items, callbacks)
			entry.Items = &children
		default:
			if icon.nextAction >= 0xffff_fff0 {
				panic("QuickGUI tray menu id space exhausted")
			}
			entry.ID = icon.nextAction
			icon.nextAction++
			callbacks[entry.ID] = menuCallback{click: item.Click, owner: reactive.GetOwner(), window: contextWindow()}
		}
		result = append(result, entry)
	}
	return result
}

func dispatchTrayEvent(id uint32, value string) {
	icon := trayIcons[id]
	if icon == nil || icon.removed || icon.removing {
		return
	}
	var event TrayEvent
	if json.Unmarshal([]byte(value), &event) != nil {
		return
	}
	if event.Kind == "menu-item" {
		icon.callbacks[event.MenuItemID].run()
	}
	if !icon.removing && !icon.removed {
		icon.listeners.emit(event)
	}
}
