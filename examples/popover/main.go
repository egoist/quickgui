package main

import (
	"log"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/ui"
)

func main() {
	if err := native.Run(func() {
		open := func() {
			native.NewWindow(native.WindowOptions{
				Title:         "QuickGUI Popovers",
				Width:         760,
				Height:        540,
				MinimumWidth:  620,
				MinimumHeight: 480,
				Background:    "#0b0f17",
				TitleBarStyle: "hiddenInset",
				TrafficLightPosition: &native.Point{
					X: 16,
					Y: 14,
				},
				Component: Popovers,
			})
		}
		native.App.OnReopen(func(event native.ReopenEvent) {
			if !event.HasVisibleWindows {
				open()
			}
		})
		open()
	}); err != nil {
		log.Fatal(err)
	}
}

func Popovers() *ui.Element {
	status, setStatus := ui.CreateSignal("Open either surface to compare its native behavior.")
	systemOpen, setSystemOpen := ui.CreateSignal(false)
	inWindowOpen, setInWindowOpen := ui.CreateSignal(false)
	system := ui.NewSystemPopover().
		Open(systemOpen).
		OnOpenChange(func(open bool, _ ui.PopoverOpenChangeDetails) {
			setSystemOpen(open)
			if open {
				setStatus("System popover opened.")
			} else {
				setStatus("System popover closed.")
			}
		})
	popover := ui.NewPopover().
		Open(inWindowOpen).
		OnOpenChange(func(open bool, _ ui.PopoverOpenChangeDetails) {
			setInWindowOpen(open)
			if open {
				setStatus("In-window popover opened.")
			} else {
				setStatus("In-window popover closed.")
			}
		})
	gap, margin := 8.0, 12.0
	placement := ui.PopoverContentProps{
		Width:          340,
		Height:         220,
		Placement:      "bottom-start",
		Gap:            &gap,
		ViewportMargin: &margin,
		PartProps:      ui.PartProps{Style: ui.Style().SizeFull()},
	}
	return ui.View().
		FlexCol().
		SizeFull().
		Bg("#0b0f17").
		TextColor("#f5f7fb").
		Children(
			ui.View().
				Flex().
				Height(52).
				FlexShrink(0).
				ItemsCenter().
				JustifyCenter().
				AppRegion("drag").
				BorderColor("#202838").
				BorderBottomWidth(1).
				FontSize(14).
				FontWeight(600).
				Child("Native popover surfaces"),
			ui.View().
				Flex().
				Flex1().
				MinHeight(0).
				ItemsCenter().
				JustifyCenter().
				Padding(36).
				Child(
					ui.View().
						FlexCol().
						Width("100%").
						MaxWidth(680).
						Gap(20).
						Children(
							ui.View().
								FlexCol().
								Gap(8).
								Children(
									ui.Text("System and in-window popovers").
										FontSize(28).
										LineHeight(34).
										FontWeight(700),
									ui.Text("Both use the same controlled Go API. SystemPopover opens a native child window; Popover stays in this window's retained overlay plane.").
										TextColor("#9ba8bc").
										FontSize(14).
										LineHeight(21),
								),
							ui.View().
								Flex().
								Gap(14).
								Children(
									card("SystemPopover", "A child window that may cross the owner's edge and stays within the display.", func() *ui.Element {
										return system.Root().
											Children(
												system.Trigger().
													Style(buttonStyle).
													Child(func() string {
														if systemOpen() {
															return "Close system"
														}
														return "Open system"
													}),
												system.Content(placement).
													Child(func() *ui.Element {
														return content("System popover", "It has its own retained tree on a native child surface and may cross the owner window's edge.", func() { setSystemOpen(false) })
													}),
											)
									}),
									card("In-window popover", "A retained overlay that flips and shifts but remains inside this window.", func() *ui.Element {
										return popover.Root().
											Children(
												popover.Trigger().
													Style(buttonStyle).
													Child(func() string {
														if inWindowOpen() {
															return "Close in-window"
														}
														return "Open in-window"
													}),
												popover.Content(placement).
													Child(func() *ui.Element {
														return content("In-window popover", "It shares this window's tree and renders above ordinary content without creating another native window.", func() { setInWindowOpen(false) })
													}),
											)
									}),
								),
							ui.View().
								Flex().
								MinHeight(50).
								ItemsCenter().
								JustifyCenter().
								Px4().
								Bg("#10151e").
								BorderColor("#293244").
								BorderWidth(1).
								RoundedLg().
								TextColor("#b8c4d6").
								FontSize(13).
								Child(status),
						),
				),
		)
}
