package native

import (
	"encoding/json"
	"fmt"
	"strconv"
	"strings"
)

// RouteDefinition is one core route table entry. Omit Path for a pathless layout.
type RouteDefinition struct {
	ID       string `json:"id"`
	Path     string `json:"path,omitempty"`
	ParentID string `json:"parentId,omitempty"`
}

// RouteValue is one decoded parameter or query pair.
type RouteValue struct {
	Name  string `json:"name"`
	Value string `json:"value"`
}

// RouteLocation is the normalized current location.
type RouteLocation struct {
	Href     string       `json:"href"`
	Pathname string       `json:"pathname"`
	Search   string       `json:"search"`
	Hash     string       `json:"hash"`
	Query    []RouteValue `json:"query"`
}

// RouteMatch is the winning route chain and its parameters.
type RouteMatch struct {
	RouteIDs []string     `json:"routeIds"`
	Params   []RouteValue `json:"params"`
}

// RouterState is one core router snapshot.
type RouterState struct {
	Location      RouteLocation `json:"location"`
	Matched       *RouteMatch   `json:"matched,omitempty"`
	HistoryIndex  uint32        `json:"historyIndex"`
	HistoryLength uint32        `json:"historyLength"`
	CanGoBack     bool          `json:"canGoBack"`
	CanGoForward  bool          `json:"canGoForward"`
}

type routerCreateParams struct {
	Routes             []RouteDefinition `json:"routes"`
	InitialDestination string            `json:"initialDestination"`
}

type routerIDParams struct {
	ID uint32 `json:"id"`
}

type routerDestinationParams struct {
	ID          uint32 `json:"id"`
	Destination string `json:"destination"`
}

type routerActiveParams struct {
	ID          uint32 `json:"id"`
	Destination string `json:"destination"`
	End         bool   `json:"end"`
}

type routerGoParams struct {
	ID    uint32 `json:"id"`
	Delta int    `json:"delta"`
}

// Router is a core-owned route table and bounded memory history.
//
// Every operation is synchronous and CPU-only: the router never reaches the application
// runtime, so it answers through the host's synchronous service channel.
type Router struct {
	ID       uint32
	released bool
}

// NewRouter creates a core router for routes, starting at initialDestination (default "/").
func NewRouter(routes []RouteDefinition, initialDestination string) *Router {
	if initialDestination == "" {
		initialDestination = "/"
	}
	raw := mustService("router-create", mustJSON(routerCreateParams{
		Routes:             routes,
		InitialDestination: initialDestination,
	}))
	id, err := strconv.ParseUint(strings.TrimSpace(raw), 10, 32)
	if err != nil {
		panic(fmt.Sprintf("QuickGUI router-create returned %s", raw))
	}
	return &Router{ID: uint32(id)}
}

func (r *Router) State() RouterState {
	var state RouterState
	mustUnmarshal(mustService("router-state", mustJSON(routerIDParams{ID: r.ID})), &state)
	return state
}

func (r *Router) Resolve(destination string) RouteLocation {
	var location RouteLocation
	mustUnmarshal(mustService("router-resolve", mustJSON(routerDestinationParams{
		ID: r.ID, Destination: destination,
	})), &location)
	return location
}

func (r *Router) IsActive(destination string, end bool) bool {
	return mustService("router-is-active", mustJSON(routerActiveParams{
		ID: r.ID, Destination: destination, End: end,
	})) == "true"
}

func (r *Router) Push(destination string) RouterState {
	return r.commit("router-push", mustJSON(routerDestinationParams{ID: r.ID, Destination: destination}))
}

func (r *Router) Replace(destination string) RouterState {
	return r.commit("router-replace", mustJSON(routerDestinationParams{ID: r.ID, Destination: destination}))
}

func (r *Router) Go(delta int) RouterState {
	return r.commit("router-go", mustJSON(routerGoParams{ID: r.ID, Delta: delta}))
}

func (r *Router) Back() RouterState {
	return r.commit("router-back", mustJSON(routerIDParams{ID: r.ID}))
}

func (r *Router) Forward() RouterState {
	return r.commit("router-forward", mustJSON(routerIDParams{ID: r.ID}))
}

// Release frees the core router. Later calls fail.
func (r *Router) Release() {
	if r.released {
		return
	}
	r.released = true
	_, _ = CallService("router-release", mustJSON(routerIDParams{ID: r.ID}))
}

func (r *Router) commit(method, params string) RouterState {
	var state RouterState
	mustUnmarshal(mustService(method, params), &state)
	return state
}

func mustService(method, params string) string {
	value, err := CallService(method, params)
	if err != nil {
		panic(err)
	}
	return value
}

func mustJSON(value any) string {
	payload, err := json.Marshal(value)
	if err != nil {
		panic(err)
	}
	return string(payload)
}

func mustUnmarshal(raw string, dest any) {
	if err := json.Unmarshal([]byte(raw), dest); err != nil {
		panic(fmt.Errorf("the native router reply was malformed: %w", err))
	}
}
