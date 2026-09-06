package ui

import (
	"encoding/json"
	"testing"

	"github.com/egoist/quickgui/packages/go/host"
	"github.com/egoist/quickgui/packages/go/native"
	"github.com/egoist/quickgui/packages/go/protocol"
	"github.com/egoist/quickgui/packages/go/reactive"
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
	if table.definitions[1].ID != "route-2" || table.definitions[1].Path != "" || table.definitions[1].ParentID != "route-1" {
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
	if table.definitions[0].Path != "" || table.definitions[1].ParentID != "route-1" {
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
	var projectID func() string
	var navigate func(string, ...NavigateOptions)

	reactive.CreateRoot(func(dispose func()) struct{} {
		parent := View(Props{})
		node := Router(RouterProps{
			Routes: []*RouteDeclaration{
				Route("/", func() *native.Node {
					return View(Props{Children: []any{
						Text(Props{Children: "shell"}),
						Outlet(),
					}})
				}, Route("", func() *native.Node {
					homeCreated++
					navigate = UseNavigate()
					return Text(Props{Children: "home"})
				}), Route("projects/:id", func() *native.Node {
					projectCreated++
					projectID = UseParam("id")
					navigate = UseNavigate()
					return Text(Props{Children: func() string { return "project " + projectID() }})
				})),
			},
			Fallback: func() *native.Node { return Text(Props{Children: "missing"}) },
		})
		native.InsertNode(parent, node, nil)
		if homeCreated != 1 || projectCreated != 0 {
			t.Fatalf("home=%d project=%d", homeCreated, projectCreated)
		}
		navigate("/projects/12")
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
			script.state.Location = native.RouteLocation{Href: body.Destination, Pathname: body.Destination}
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
		parent := View(Props{})
		node := Router(RouterProps{
			Routes: []*RouteDeclaration{
				Route("/", func() *native.Node {
					return Link(LinkProps{
						Href:      "/projects/12",
						PartProps: PartProps{Children: func() *native.Node { return Text(Props{Children: "Go"}) }},
					})
				}),
			},
		})
		native.InsertNode(parent, node, nil)
		link := findButton(parent)
		if link == nil {
			t.Fatal("missing link")
		}
		if len(link.Listeners) == 0 {
			t.Fatal("expected a click listener")
		}
		native.DispatchEvent(&native.NodeHost{Nodes: map[uint32]*native.Node{link.ID: link}}, protocol.EventClick, link.ID, "", false)
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
