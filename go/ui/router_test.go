package ui

import (
	"encoding/json"
	"testing"

	"github.com/egoist/quickgui/go/host"
	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/protocol"
	"github.com/egoist/quickgui/go/reactive"
)

func serviceOK(value any) string {
	payload, err := json.Marshal(struct {
		OK    bool `json:"ok"`
		Value any  `json:"value"`
	}{OK: true, Value: value})
	if err != nil {
		panic(err)
	}
	return string(payload)
}

func TestBuildRouteTableAssignsIdsAndParents(t *testing.T) {
	table := buildRouteTable([]*RouteDeclaration{
		Route("/", nil, Route("", nil), Route("projects/:id", nil)),
	})
	if len(table.definitions) != 3 {
		t.Fatalf("definitions %d", len(table.definitions))
	}
	if table.definitions[0].ID != "route-1" || table.definitions[0].Path != "/" || table.definitions[0].ParentID != "" {
		t.Fatalf("%+v", table.definitions[0])
	}
	if table.definitions[1].ID != "route-2" || table.definitions[1].Path != "" || !table.definitions[1].Index || table.definitions[1].ParentID != "route-1" {
		t.Fatalf("%+v", table.definitions[1])
	}
	if table.definitions[2].ID != "route-3" || table.definitions[2].ParentID != "route-1" {
		t.Fatalf("%+v", table.definitions[2])
	}
	if len(table.chains["route-3"]) != 2 {
		t.Fatalf("chain %d", len(table.chains["route-3"]))
	}
}

func TestLayoutOmitsPath(t *testing.T) {
	table := buildRouteTable([]*RouteDeclaration{
		Layout(nil, Route("inbox", nil)),
	})
	if table.definitions[0].Path != "" || table.definitions[0].Index || table.definitions[1].ParentID != "route-1" {
		t.Fatalf("%+v", table.definitions)
	}
}

type scriptedRouter struct {
	state native.RouterState
}

func (s *scriptedRouter) install(t *testing.T) func() {
	t.Helper()
	return host.Install(&host.Fake{CallFn: func(method, params string) string {
		switch method {
		case "router-create":
			return serviceOK(1)
		case "router-state":
			return serviceOK(s.state)
		case "router-push", "router-replace":
			var body struct {
				Destination string `json:"destination"`
			}
			if err := json.Unmarshal([]byte(params), &body); err != nil {
				t.Fatal(err)
			}
			s.apply(body.Destination)
			return serviceOK(s.state)
		case "router-is-active":
			var body struct {
				Destination string `json:"destination"`
				End         bool   `json:"end"`
			}
			_ = json.Unmarshal([]byte(params), &body)
			active := s.state.Location.Pathname == body.Destination
			if !body.End {
				active = len(s.state.Location.Pathname) >= len(body.Destination) &&
					(body.Destination == "/" || s.state.Location.Pathname == body.Destination ||
						(len(s.state.Location.Pathname) > len(body.Destination) && s.state.Location.Pathname[:len(body.Destination)] == body.Destination))
			}
			return serviceOK(active)
		case "router-resolve":
			return serviceOK(s.state.Location)
		case "router-back", "router-forward", "router-go":
			return serviceOK(s.state)
		case "router-release":
			return serviceOK(nil)
		default:
			t.Fatalf("unexpected method %s", method)
			return serviceOK(nil)
		}
	}})
}

func (s *scriptedRouter) apply(destination string) {
	switch destination {
	case "/", "/home":
		s.state.Location = native.RouteLocation{Href: "/", Pathname: "/"}
		s.state.Matched = &native.RouteMatch{RouteIDs: []string{"route-1", "route-2"}}
	case "/projects/12":
		s.state.Location = native.RouteLocation{Href: "/projects/12", Pathname: "/projects/12"}
		s.state.Matched = &native.RouteMatch{
			RouteIDs: []string{"route-1", "route-3"},
			Params:   []native.RouteValue{{Name: "id", Value: "12"}},
		}
	case "/projects/13":
		s.state.Location = native.RouteLocation{Href: "/projects/13", Pathname: "/projects/13"}
		s.state.Matched = &native.RouteMatch{
			RouteIDs: []string{"route-1", "route-3"},
			Params:   []native.RouteValue{{Name: "id", Value: "13"}},
		}
	default:
		s.state.Location = native.RouteLocation{Href: destination, Pathname: destination}
		s.state.Matched = nil
	}
	s.state.HistoryLength++
	s.state.HistoryIndex++
	s.state.CanGoBack = s.state.HistoryIndex > 0
}

func TestRouterMountsMatchedChainAndKeepsPageOnParamChange(t *testing.T) {
	native.ResetTreeStateForTests()
	script := &scriptedRouter{}
	script.apply("/")
	script.state.HistoryIndex = 0
	script.state.HistoryLength = 1
	script.state.CanGoBack = false
	restore := script.install(t)
	defer restore()

	homeCreated := 0
	projectCreated := 0
	shellCreated := 0
	var projectID func() string
	var navigate func(string, ...NavigateOptions)

	reactive.CreateRoot(func(dispose func()) struct{} {
		parent := View()
		node := Router(RouterProps{
			Routes: []*RouteDeclaration{
				Route("/", func() *Element {
					shellCreated++
					return View().
						Children(
							Text("shell"),
							Outlet(),
						)
				}, Route("", func() *Element {
					homeCreated++
					navigate = UseNavigate()
					return Text(Props{Children: "home"})
				}), Route("projects/:id", func() *Element {
					projectCreated++
					projectID = UseParam("id")
					navigate = UseNavigate()
					return Text(Props{Children: func() string { return "project " + projectID() }})
				})),
			},
			Fallback: func() *Element { return Text(Props{Children: "missing"}) },
		})
		native.InsertNode(parent.Node, node, nil)
		if homeCreated != 1 || projectCreated != 0 {
			t.Fatalf("home=%d project=%d", homeCreated, projectCreated)
		}
		navigate("/projects/12")
		if shellCreated != 1 {
			t.Fatalf("remounted shared shell when switching pages: %d", shellCreated)
		}
		if homeCreated != 1 || projectCreated != 1 {
			t.Fatalf("after first project home=%d project=%d", homeCreated, projectCreated)
		}
		if projectID() != "12" {
			t.Fatalf("id %s", projectID())
		}
		navigate("/projects/13")
		if projectCreated != 1 {
			t.Fatalf("recreated project page on param change: %d", projectCreated)
		}
		if projectID() != "13" {
			t.Fatalf("id %s", projectID())
		}
		navigate("/missing")
		if len(node.Group) != 1 || len(node.Group[0].Children) != 1 || node.Group[0].Children[0].Text != "missing" {
			t.Fatalf("fallback %+v", node.Group)
		}
		dispose()
		return struct{}{}
	})
}

func TestRouterRetainsNestedLayoutsAndDisposesOnlyReplacedBranches(t *testing.T) {
	native.ResetTreeStateForTests()
	script := &scriptedRouter{state: native.RouterState{
		Location: native.RouteLocation{Href: "/settings", Pathname: "/settings"},
		Matched:  &native.RouteMatch{RouteIDs: []string{"shell", "settings", "general"}},
	}}
	restore := script.install(t)
	defer restore()
	mounted, disposed := map[string]int{}, map[string]int{}
	nodes := map[string]*native.Node{}
	var controller *RouterController
	var count func() int
	var setCount func(int)
	record := func(name string, children Component) Component {
		return func() *Element {
			mounted[name]++
			reactive.OnCleanup(func() { disposed[name]++ })
			view := View().Children(Text(name), children)
			nodes[name] = view.Node
			return view
		}
	}
	declare := func(id, path string, component Component, children ...*RouteDeclaration) *RouteDeclaration {
		route := Route(path, component, children...)
		route.ID = id
		return route
	}
	reactive.CreateRoot(func(dispose func()) struct{} {
		Router(RouterProps{
			Routes: []*RouteDeclaration{declare("shell", "/", record("shell", func() *native.Node {
				controller = UseRouter()
				return Outlet()
			}),
				declare("settings", "settings", record("settings", func() *native.Node {
					var children_ []*native.Node
					read, write := reactive.CreateSignal(0)
					count = read
					setCount = func(value int) { write(value) }
					children_ = append(children_, Text(read).Node)
					children_ = append(children_, Outlet())
					return Fragment(children_)
				}),
					declare("general", "", record("general", nil)),
					declare("appearance", "appearance", record("appearance", nil)),
				),
				declare("home", "", record("home", nil)),
			)},
			Fallback: record("fallback", nil),
		})
		shell, settings := nodes["shell"], nodes["settings"]
		setCount(42)
		controller.commit(native.RouterState{
			Location: native.RouteLocation{
				Href:     "/settings/appearance",
				Pathname: "/settings/appearance",
			},
			Matched: &native.RouteMatch{RouteIDs: []string{"shell", "settings", "appearance"}},
		})
		if nodes["shell"] != shell || nodes["settings"] != settings || count() != 42 {
			t.Fatal("changing nested pages replaced the shared layout or its local state")
		}
		if mounted["appearance"] != 1 || disposed["general"] != 1 || disposed["settings"] != 0 || disposed["shell"] != 0 {
			t.Fatalf("nested transition: mounted=%v disposed=%v", mounted, disposed)
		}
		controller.commit(native.RouterState{
			Location: native.RouteLocation{
				Href:     "/settings/appearance?theme=dark",
				Pathname: "/settings/appearance",
				Query:    []native.RouteValue{{Name: "theme", Value: "dark"}},
			},
			Matched: &native.RouteMatch{RouteIDs: []string{"shell", "settings", "appearance"}},
		})
		if mounted["appearance"] != 1 || count() != 42 {
			t.Fatal("query-only navigation remounted the branch")
		}
		controller.commit(native.RouterState{
			Location: native.RouteLocation{Href: "/", Pathname: "/"},
			Matched:  &native.RouteMatch{RouteIDs: []string{"shell", "home"}},
		})
		if nodes["shell"] != shell || mounted["home"] != 1 || disposed["settings"] != 1 || disposed["appearance"] != 1 {
			t.Fatalf("leaving settings: mounted=%v disposed=%v", mounted, disposed)
		}
		controller.commit(native.RouterState{Location: native.RouteLocation{
			Href:     "/missing",
			Pathname: "/missing",
		}})
		if mounted["fallback"] != 1 || disposed["shell"] != 1 || disposed["home"] != 1 {
			t.Fatalf("unmatched route: mounted=%v disposed=%v", mounted, disposed)
		}
		dispose()
		for name, created := range mounted {
			if disposed[name] != created {
				t.Fatalf("%s cleanup: created=%d disposed=%d", name, created, disposed[name])
			}
		}
		return struct{}{}
	})
}

func TestLinkNavigatesAndSetsRole(t *testing.T) {
	native.ResetTreeStateForTests()
	script := &scriptedRouter{state: native.RouterState{
		Location: native.RouteLocation{Href: "/", Pathname: "/"},
		Matched:  &native.RouteMatch{RouteIDs: []string{"route-1"}},
	}}
	var pushed string
	restore := host.Install(&host.Fake{CallFn: func(method, params string) string {
		switch method {
		case "router-create":
			return serviceOK(1)
		case "router-state":
			return serviceOK(script.state)
		case "router-push":
			var body struct {
				Destination string `json:"destination"`
			}
			_ = json.Unmarshal([]byte(params), &body)
			pushed = body.Destination
			script.state.Location = native.RouteLocation{
				Href:     body.Destination,
				Pathname: body.Destination,
			}
			return serviceOK(script.state)
		case "router-is-active":
			return serviceOK(true)
		case "router-release":
			return serviceOK(nil)
		default:
			return serviceOK(script.state)
		}
	}})
	defer restore()

	reactive.CreateRoot(func(dispose func()) struct{} {
		parent := View()
		node := Router(RouterProps{
			Routes: []*RouteDeclaration{
				Route("/", func() *native.Node {
					return Link(LinkProps{
						Href:      "/projects/12",
						PartProps: PartProps{Children: func() *Element { return Text(Props{Children: "Go"}) }},
					})
				}),
			},
		})
		native.InsertNode(parent.Node, node, nil)
		link := findButton(parent.Node)
		if link == nil {
			t.Fatal("missing link")
		}
		if len(link.Listeners) == 0 {
			t.Fatal("expected a click listener")
		}
		native.DispatchEvent(
			&native.NodeHost{Nodes: map[uint32]*native.Node{link.ID: link}},
			protocol.EventClick,
			link.ID,
			"",
			false,
		)
		if pushed != "/projects/12" {
			t.Fatalf("pushed %q", pushed)
		}
		dispose()
		return struct{}{}
	})
}

func findButton(node *native.Node) *native.Node {
	if node == nil {
		return nil
	}
	if node.Tag == protocol.TagButton {
		return node
	}
	if node.Group != nil {
		for _, child := range node.Group {
			if found := findButton(child); found != nil {
				return found
			}
		}
	}
	for _, child := range node.Children {
		if found := findButton(child); found != nil {
			return found
		}
	}
	return nil
}

func TestUseRouterRequiresRouter(t *testing.T) {
	defer func() {
		if recover() == nil {
			t.Fatal("expected panic")
		}
	}()
	reactive.CreateRoot(func(func()) struct{} {
		UseRouter()
		return struct{}{}
	})
}
