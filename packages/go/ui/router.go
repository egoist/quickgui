package ui

import (
	"fmt"

	"github.com/egoist/quickgui/packages/go/native"
	"github.com/egoist/quickgui/packages/go/protocol"
	"github.com/egoist/quickgui/packages/go/reactive"
)

// RouteParams are decoded pattern parameters of the matched route, by name.
type RouteParams map[string]string

// RouteSearchParams are decoded query values by name; a repeated name keeps its last value.
type RouteSearchParams map[string]string

// RouteComponent reads the router through UseParams, UseLocation, UseSearchParams, and UseRouter.
// A layout renders its matched child with Outlet.
type RouteComponent func() *native.Node

// RouteDeclaration is one declared route; build these with Route and Layout.
type RouteDeclaration struct {
	ID        string
	Path      *string
	Component RouteComponent
	Children  []*RouteDeclaration
}

// Route declares one route, with its nested routes as extra arguments.
func Route(path string, component RouteComponent, children ...*RouteDeclaration) *RouteDeclaration {
	return &RouteDeclaration{Path: &path, Component: component, Children: children}
}

// Layout is a pathless layout route around children.
func Layout(component RouteComponent, children ...*RouteDeclaration) *RouteDeclaration {
	return &RouteDeclaration{Component: component, Children: children}
}

type renderedRoute struct {
	id        string
	component RouteComponent
}

type routeTable struct {
	definitions []native.RouteDefinition
	chains      map[string][]renderedRoute
}

func buildRouteTable(routes []*RouteDeclaration) routeTable {
	table := routeTable{chains: map[string][]renderedRoute{}}
	nextID := 1
	var visit func(declaration *RouteDeclaration, parentID string, chain []renderedRoute)
	visit = func(declaration *RouteDeclaration, parentID string, chain []renderedRoute) {
		id := declaration.ID
		if id == "" {
			id = fmt.Sprintf("route-%d", nextID)
			nextID++
		}
		definition := native.RouteDefinition{ID: id, ParentID: parentID}
		if declaration.Path != nil {
			definition.Path = *declaration.Path
		}
		table.definitions = append(table.definitions, definition)
		rendered := append(append([]renderedRoute{}, chain...), renderedRoute{id: id, component: declaration.Component})
		table.chains[id] = rendered
		for _, child := range declaration.Children {
			visit(child, id, rendered)
		}
	}
	for _, declaration := range routes {
		visit(declaration, "", nil)
	}
	return table
}

// NavigateOptions control how a destination is written into history.
type NavigateOptions struct {
	Replace bool
}

// RouterController is imperative access to the router: the current snapshot and history navigation.
type RouterController struct {
	State    func() native.RouterState
	native   *native.Router
	setState reactive.Setter[native.RouterState]
}

func (c *RouterController) Navigate(destination string, options ...NavigateOptions) {
	replace := len(options) > 0 && options[0].Replace
	if replace {
		c.commit(c.native.Replace(destination))
		return
	}
	c.commit(c.native.Push(destination))
}

func (c *RouterController) Push(destination string) {
	c.commit(c.native.Push(destination))
}

func (c *RouterController) Replace(destination string) {
	c.commit(c.native.Replace(destination))
}

func (c *RouterController) Go(delta int) {
	c.commit(c.native.Go(delta))
}

func (c *RouterController) Back() {
	c.commit(c.native.Back())
}

func (c *RouterController) Forward() {
	c.commit(c.native.Forward())
}

func (c *RouterController) Resolve(destination string) native.RouteLocation {
	return c.native.Resolve(destination)
}

// IsActive reports whether destination is active. end requires an exact pathname. Reactive.
func (c *RouterController) IsActive(destination string, end bool) bool {
	c.State()
	return c.native.IsActive(destination, end)
}

func (c *RouterController) commit(next native.RouterState) {
	c.setState(next)
	reactive.Flush()
}

type routerContextValue struct {
	controller   *RouterController
	params       func() RouteParams
	searchParams func() RouteSearchParams
}

var (
	routerContext = reactive.CreateContext[*routerContextValue](nil)
	outletContext = reactive.CreateContext[func() *native.Node](nil)
)

// RouterProps configure one static route table.
type RouterProps struct {
	Routes      []*RouteDeclaration
	InitialPath string
	Fallback    func() *native.Node
}

func valuesMap(values []native.RouteValue) map[string]string {
	result := map[string]string{}
	for _, entry := range values {
		result[entry.Name] = entry.Value
	}
	return result
}

// Router renders the route table's matched chain. Declarations never change after creation.
func Router(props RouterProps) *native.Node {
	table := buildRouteTable(props.Routes)
	initial := props.InitialPath
	if initial == "" {
		initial = "/"
	}
	core := native.NewRouter(table.definitions, initial)
	reactive.OnCleanup(core.Release)
	state, setState := reactive.CreateSignal(core.State())
	controller := &RouterController{State: state, native: core, setState: setState}
	params := reactive.CreateMemo(func() RouteParams {
		matched := state().Matched
		if matched == nil {
			return valuesMap(nil)
		}
		return valuesMap(matched.Params)
	})
	searchParams := reactive.CreateMemo(func() RouteSearchParams {
		return valuesMap(state().Location.Query)
	})
	context := &routerContextValue{controller: controller, params: params, searchParams: searchParams}
	// Equality on this scalar keeps a page mounted while only its params, query, or fragment change.
	leaf := reactive.CreateMemo(func() string {
		matched := state().Matched
		if matched == nil || len(matched.RouteIDs) == 0 {
			return ""
		}
		return matched.RouteIDs[len(matched.RouteIDs)-1]
	})
	return reactive.Provide(routerContext, context, func() *native.Node {
		return DynamicMaybe(func() *native.Node {
			id := leaf()
			if id == "" {
				if props.Fallback == nil {
					return nil
				}
				return reactive.Untrack(props.Fallback)
			}
			chain, ok := table.chains[id]
			if !ok {
				panic(fmt.Sprintf("QuickGUI core returned unknown route `%s`", id))
			}
			return reactive.Untrack(func() *native.Node {
				return renderRouteChain(chain, 0, context)
			})
		})
	})
}

func renderRouteChain(chain []renderedRoute, index int, context *routerContextValue) *native.Node {
	if index >= len(chain) {
		return Fragment(nil)
	}
	entry := chain[index]
	outlet := func() *native.Node { return renderRouteChain(chain, index+1, context) }
	return reactive.Provide(outletContext, outlet, func() *native.Node {
		if entry.component == nil {
			return outlet()
		}
		return entry.component()
	})
}

// Outlet renders the next matched child route inside a layout component.
func Outlet() *native.Node {
	outlet := outletContext.Use()
	if outlet == nil {
		return Fragment(nil)
	}
	return outlet()
}

// UseRouter returns the current router. Must be called below Router.
func UseRouter() *RouterController {
	return requireContext(routerContext, "UseRouter", "Router").controller
}

// UseLocation is a reactive accessor for the normalized current location.
func UseLocation() func() native.RouteLocation {
	controller := requireContext(routerContext, "UseLocation", "Router").controller
	return func() native.RouteLocation { return controller.State().Location }
}

// UseParams is a reactive accessor for decoded parameters from the winning route.
func UseParams() func() RouteParams {
	return requireContext(routerContext, "UseParams", "Router").params
}

// UseSearchParams is a reactive accessor for decoded query values.
func UseSearchParams() func() RouteSearchParams {
	return requireContext(routerContext, "UseSearchParams", "Router").searchParams
}

// UseParam is a reactive accessor for one decoded parameter, or "" while the winning route has none.
func UseParam(name string) func() string {
	params := UseParams()
	return func() string { return params()[name] }
}

// UseSearchParam is a reactive accessor for one decoded query value, or "" while the query has none.
func UseSearchParam(name string) func() string {
	searchParams := UseSearchParams()
	return func() string { return searchParams()[name] }
}

// UseNavigate is a stable imperative navigation function for the current router.
func UseNavigate() func(destination string, options ...NavigateOptions) {
	controller := requireContext(routerContext, "UseNavigate", "Router").controller
	return func(destination string, options ...NavigateOptions) {
		controller.Navigate(destination, options...)
	}
}

// LinkProps configure a native link button backed by the current router.
type LinkProps struct {
	PartProps
	Href          string
	Replace       bool
	End           bool
	ActiveStyle   *Style
	InactiveStyle *Style
}

// Link is a native link button backed by the current router. ActiveStyle layers over Style while active.
func Link(props LinkProps) *native.Node {
	router := UseRouter()
	node := native.CreateElement(protocol.TagButton)
	applyPartBehavior(node, props.PartProps)
	if props.Role == "" {
		setString(node, protocol.Role, "link")
	}
	bindStyleList(node, func() []Style {
		styles := []Style{}
		if base := resolveStyle(props.Style); base != nil {
			styles = append(styles, *base)
		}
		if router.IsActive(props.Href, props.End) {
			if props.ActiveStyle != nil {
				styles = append(styles, *props.ActiveStyle)
			}
		} else if props.InactiveStyle != nil {
			styles = append(styles, *props.InactiveStyle)
		}
		styles = append(styles, Style{AppRegion: "no-drag"})
		return styles
	})
	setListener(node, protocol.EventClick, forwardClick(props.OnClick, func(*native.Event) {
		if props.Replace {
			router.Navigate(props.Href, NavigateOptions{Replace: true})
			return
		}
		router.Navigate(props.Href)
	}))
	return finishPart(node, props.PartProps)
}
