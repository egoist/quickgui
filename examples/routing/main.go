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
				Title:                "QuickGUI Routing",
				Width:                820,
				Height:               560,
				MinimumWidth:         680,
				MinimumHeight:        440,
				Background:           background,
				TitleBarStyle:        "hiddenInset",
				TrafficLightPosition: &native.Point{X: 16, Y: 18},
				Component:            Routes,
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

func Routes() *native.Node {
	return ui.Router(ui.RouterProps{
		InitialPath: "/",
		Routes: []*ui.RouteDeclaration{ui.Route(
			"/",
			Navigation,
			ui.Route("/", Home),
			ui.Route("/projects", Projects),
			ui.Route("/projects/:projectId", Project),
			ui.Route(
				"/settings",
				SettingsLayout,
				ui.Route("", GeneralSettings),
				ui.Route("appearance", AppearanceSettings),
			),
			ui.Route("*", NotFound),
		)},
	})
}

func Navigation() *ui.Element {
	router, location := ui.UseRouter(), ui.UseLocation()
	return ui.View().
		Children(

			ui.View().
				Children(

					ui.View().
						Child(
							ui.Text("Router").FontWeight(700),
						).
						Display("flex").
						Height("100%").
						AlignItems("center").
						PaddingRight(10).
						AppRegion("drag"),
					link("Home", "/", true),
					link("Projects", "/projects", false),
					link("Settings", "/settings", false),
					ui.View().Flex(1).Height("100%").AppRegion("drag"),
					historyButton("Go back", "M19 12H5m6-6-6 6 6 6", router.Back, func() bool { return !router.State().CanGoBack }),
					historyButton("Go forward", "M5 12h14m-6-6 6 6-6 6", router.Forward, func() bool { return !router.State().CanGoForward }),
				).
				Display("flex").
				Height(54).
				FlexShrink(0).
				AlignItems("center").
				PaddingLeft(78).
				PaddingRight(16).
				Gap(8).
				BorderBottomWidth(1).
				BorderColor(border),
			ui.View().
				Child(

					ui.Text(
						location().Href,
					).
						FontFamily("monospace").
						FontSize(12).
						TextColor(muted),
				).
				Display("flex").
				Height(34).
				FlexShrink(0).
				AlignItems("center").
				PaddingLeft(20).
				PaddingRight(20).
				BackgroundColor("#0e1526").
				BorderBottomWidth(1).
				BorderColor(border),
			ui.View().
				Child(
					ui.Outlet(),
				).
				Flex(1).
				MinHeight(0),
		).
		Display("flex").
		FlexDirection("column").
		Width("100%").
		Height("100%").
		BackgroundColor(background).
		TextColor(textColor)
}

func Home() *ui.Element {
	return page("Core-owned routing", "The native core owns matching, decoded parameters, query parsing, active paths, and bounded memory history. The application renders the returned route chain.", func() *ui.Element {
		return ui.View().
			Children(

				card("Dynamic parameters", "Open /projects/quickgui and read :projectId from the matched core route.", "/projects/quickgui?tab=overview"),
				card("Nested layouts", "Settings keeps its local navigation mounted while its Outlet changes.", "/settings"),
				card("Fallback routes", "A final wildcard catches destinations that no specific pattern matched.", "/this-route-does-not-exist"),
			).
			Display("flex").
			FlexWrap("wrap").
			Gap(12)
	})
}

func Projects() *ui.Element {
	return page("Projects", "These links push memory-history entries. Use the title-bar arrows to traverse them.", func() *ui.Element {
		return ui.View().
			Children(

				card("QuickGUI", "A native retained UI framework.", "/projects/quickgui?tab=overview"),
				card("Screenflare", "A polished native screen recorder.", "/projects/screenflare?tab=activity"),
			).
			Display("flex").
			FlexWrap("wrap").
			Gap(12)
	})
}

func Project() *ui.Element {
	projectID, search, navigate := ui.UseParam("projectId"), ui.UseSearchParams(), ui.UseNavigate()
	return page([]any{"Project: ", projectID}, "The page stays mounted when only the query changes; its reactive core snapshot updates in place.", func() *ui.Element {
		return ui.View().
			Children(

				ui.Text("Decoded :projectId").TextColor(muted),
				ui.Text(projectID).FontFamily("monospace").TextColor(blue),
				ui.Text("Decoded ?tab").MarginTop(8).TextColor(muted),
				ui.Text(
					func() string {
						if value, ok := search()["tab"]; ok {
							return value
						}
						return "overview"
					},
				).
					FontFamily("monospace").
					TextColor(blue),
				ui.View().
					Children(

						button("Overview", func() { navigate("?tab=overview") }),
						button("Activity", func() { navigate("?tab=activity") }),
						button("Replace", func() { navigate("?tab=activity", ui.NavigateOptions{Replace: true}) }).
							Style(ui.Style().BackgroundColor("#322847")),
					).
					Display("flex").
					Gap(8).
					MarginTop(8),
			).
			Display("flex").
			FlexDirection("column").
			Width(440).
			MaxWidth("100%").
			Padding(18).
			Gap(12).
			BackgroundColor(panel).
			BorderWidth(1).
			BorderColor(border).
			BorderRadius(12)
	})
}

func SettingsLayout() *ui.Element {
	return ui.View().
		Children(

			ui.View().
				Children(

					ui.Text("Settings").MarginBottom(6).FontWeight(700),
					link("General", "/settings", true),
					link("Appearance", "/settings/appearance", false),
				).
				Display("flex").
				FlexDirection("column").
				Width(190).
				FlexShrink(0).
				Padding(16).
				Gap(8).
				BackgroundColor(panel).
				BorderRightWidth(1).
				BorderColor(border),
			ui.View().
				Child(
					ui.Outlet(),
				).
				Flex(1).
				MinWidth(0),
		).
		Display("flex").
		Width("100%").
		Height("100%")
}

func GeneralSettings() *ui.Element {
	return page("General", "This is the index child at the same /settings path.")
}

func AppearanceSettings() *ui.Element {
	return page("Appearance", "The settings layout is shared; only this nested Outlet branch changes.")
}

func NotFound() *ui.Element {
	wildcard, navigate := ui.UseParam("*"), ui.UseNavigate()
	return page("Route not found", []any{"The core wildcard captured: ", wildcard}, func() *ui.Element {
		return button("Back home", func() { navigate("/", ui.NavigateOptions{Replace: true}) }).
			Style(ui.Style().
				Width(140).
				Height(38).
				BackgroundColor(blueSurface))
	})
}
