// Package reactive is a Solid-style fine-grained graph: signals, memos, effects, ownership, and context.
//
// A signal write marks its observers stale and queues them. Memos update lazily in dependency
// order before the effects that read them run, and effects run once per batch. The graph is
// synchronous. Application UI work should stay on the native application goroutine that
// [github.com/egoist/quickgui/packages/go/native.Run] owns.
package reactive

type Accessor[T any] func() T
type Setter[T any] func(T)

const (
	clean   = 0
	stale   = 1
	pending = 2
)

// Source is anything a computation can depend on.
type Source struct {
	Observers     []*Computation
	ObserverSlots []int
}

func (s *Source) asSource() *Source           { return s }
func (s *Source) asComputation() *Computation { return nil }

type linkable interface {
	asSource() *Source
	asComputation() *Computation
}

// Owner is a disposal scope. Computations are owners too, so nested scopes die with their parent.
type Owner struct {
	Source
	Owner      *Owner
	Owned      []*Owner
	Cleanups   []func()
	Disposed   bool
	Controller *Computation
	self       linkable
}

func (o *Owner) asSource() *Source { return &o.Source }

func (o *Owner) asComputation() *Computation {
	if o.self == nil {
		return nil
	}
	return o.self.asComputation()
}

// NewOwner attaches a disposal scope to parent.
func NewOwner(parent *Owner) *Owner {
	owner := &Owner{Owner: parent}
	if parent != nil {
		parent.Owned = append(parent.Owned, owner)
	}
	return owner
}

// Computation is a tracked body. Memos are pure and run before effects in every batch.
type Computation struct {
	Owner
	Fn          func()
	State       int
	Pure        bool
	Sources     []linkable
	SourceSlots []int
}

func (c *Computation) asSource() *Source           { return &c.Source }
func (c *Computation) asComputation() *Computation { return c }

func newComputation(parent *Owner, fn func(), pure bool) *Computation {
	node := &Computation{Fn: fn, Pure: pure}
	node.Owner.Owner = parent
	node.Owner.self = node
	if parent != nil {
		parent.Owned = append(parent.Owned, &node.Owner)
	}
	return node
}

// Memo is a cached derived value that recomputes when its dependencies change.
type Memo[T any] struct {
	Computation
	Value   T
	compute func() T
	equals  bool
}

func (m *Memo[T]) asSource() *Source           { return &m.Source }
func (m *Memo[T]) asComputation() *Computation { return &m.Computation }

func (m *Memo[T]) Read() T {
	if m.State != clean {
		if m.State == pending {
			lookUpstream(&m.Computation)
		}
		if m.State == stale {
			updateComputation(&m.Computation)
		}
	}
	if Listener != nil {
		link(Listener, m)
	}
	return m.Value
}

// Signal is a reactive value. Reads inside computations subscribe; writes notify.
type Signal[T any] struct {
	Source
	Value  T
	Equals bool
}

func (s *Signal[T]) asSource() *Source           { return &s.Source }
func (s *Signal[T]) asComputation() *Computation { return nil }

func (s *Signal[T]) Read() T {
	if Listener != nil {
		link(Listener, s)
	}
	return s.Value
}

func (s *Signal[T]) Write(next T) {
	if s.Equals && identical(s.Value, next) {
		return
	}
	s.Value = next
	if len(s.Observers) > 0 {
		runUpdates(func() {
			markObservers(s)
		})
	}
}

func (s *Signal[T]) Peek() T { return s.Value }

func identical[T any](a, b T) (eq bool) {
	defer func() {
		if recover() != nil {
			eq = false
		}
	}()
	eq = any(a) == any(b)
	return
}

var (
	Listener     *Computation
	CurrentOwner *Owner
	Updates      []*Computation
	Effects      []*Computation
	updating     bool
)

func link(listener *Computation, source linkable) {
	src := source.asSource()
	sourceSlot := len(src.Observers)
	listenerSlot := len(listener.Sources)
	listener.Sources = append(listener.Sources, source)
	listener.SourceSlots = append(listener.SourceSlots, sourceSlot)
	src.Observers = append(src.Observers, listener)
	src.ObserverSlots = append(src.ObserverSlots, listenerSlot)
}

func markObservers(source linkable) {
	observers := source.asSource().Observers
	for _, observer := range observers {
		if observer.State == clean {
			queue(observer)
			if len(observer.Observers) > 0 {
				markDownstream(&observer.Source)
			}
		}
		observer.State = stale
	}
}

func markDownstream(node *Source) {
	for _, observer := range node.Observers {
		if observer.State == clean {
			observer.State = pending
			queue(observer)
			if len(observer.Observers) > 0 {
				markDownstream(&observer.Source)
			}
		}
	}
}

func queue(node *Computation) {
	if node.Pure {
		Updates = append(Updates, node)
		return
	}
	Effects = append(Effects, node)
}

func runUpdates(fn func()) {
	if updating {
		fn()
		return
	}
	updating = true
	Updates = Updates[:0]
	if Effects == nil {
		Effects = []*Computation{}
	}
	defer func() {
		updating = false
		Updates = nil
		Effects = nil
	}()
	fn()
	completeUpdates()
}

func completeUpdates() {
	for {
		if len(Updates) > 0 {
			batch := Updates
			Updates = nil
			for _, node := range batch {
				runTop(node)
			}
			continue
		}
		if len(Effects) > 0 {
			batch := Effects
			Effects = nil
			for _, node := range batch {
				runTop(node)
			}
			continue
		}
		return
	}
}

func runTop(node *Computation) {
	if node.State == clean || node.Disposed {
		return
	}
	var ancestors []*Computation
	current := node.Owner.Owner
	for current != nil {
		if c := current.asComputation(); c != nil && c.State != clean && !c.Disposed {
			ancestors = append(ancestors, c)
		}
		if controller := current.Controller; controller != nil && controller.State != clean && !controller.Disposed {
			ancestors = append(ancestors, controller)
		}
		current = current.Owner
	}
	for i := len(ancestors) - 1; i >= 0; i-- {
		ancestor := ancestors[i]
		if ancestor.State == pending {
			lookUpstream(ancestor)
		}
		if ancestor.State == stale {
			updateComputation(ancestor)
		}
	}
	if node.Disposed {
		return
	}
	if node.State == pending {
		lookUpstream(node)
	}
	if node.State == stale {
		refreshSources(node)
		updateComputation(node)
	}
}

func refreshSources(node *Computation) {
	snapshot := append([]linkable(nil), node.Sources...)
	for _, source := range snapshot {
		if c := source.asComputation(); c != nil && !c.Disposed {
			if c.State == pending {
				lookUpstream(c)
			}
			if c.State == stale {
				updateComputation(c)
			}
		}
	}
}

func lookUpstream(node *Computation) {
	node.State = clean
	for _, source := range node.Sources {
		if c := source.asComputation(); c != nil {
			if c.State == stale {
				updateComputation(c)
			} else if c.State == pending {
				lookUpstream(c)
			}
			if node.State == stale {
				return
			}
		}
	}
}

func updateComputation(node *Computation) {
	if node.Disposed {
		return
	}
	cleanNode(node)
	previousListener := Listener
	previousOwner := CurrentOwner
	Listener = node
	CurrentOwner = &node.Owner
	node.State = clean
	defer func() {
		Listener = previousListener
		CurrentOwner = previousOwner
	}()
	node.Fn()
}

func initialComputation[T any](node *Computation, compute func() T) T {
	previousListener := Listener
	previousOwner := CurrentOwner
	Listener = node
	CurrentOwner = &node.Owner
	node.State = clean
	defer func() {
		Listener = previousListener
		CurrentOwner = previousOwner
	}()
	return compute()
}

func cleanNode(node *Computation) {
	for len(node.Sources) > 0 {
		last := len(node.Sources) - 1
		source := node.Sources[last]
		slot := node.SourceSlots[last]
		node.Sources = node.Sources[:last]
		node.SourceSlots = node.SourceSlots[:last]
		src := source.asSource()
		observers := src.Observers
		if len(observers) == 0 {
			continue
		}
		moved := observers[len(observers)-1]
		movedSlot := src.ObserverSlots[len(src.ObserverSlots)-1]
		src.Observers = observers[:len(observers)-1]
		src.ObserverSlots = src.ObserverSlots[:len(src.ObserverSlots)-1]
		if slot < len(src.Observers) {
			moved.SourceSlots[movedSlot] = slot
			src.Observers[slot] = moved
			src.ObserverSlots[slot] = movedSlot
		}
	}
	disposeOwned(&node.Owner)
	runCleanups(&node.Owner)
}

func disposeOwned(owner *Owner) {
	owned := owner.Owned
	owner.Owned = nil
	for _, child := range owned {
		if c := child.asComputation(); c != nil {
			disposeComputation(c)
		} else {
			DisposeOwner(child)
		}
	}
}

func disposeComputation(node *Computation) {
	if node.Disposed {
		return
	}
	detachOwner(&node.Owner)
	node.Disposed = true
	node.State = clean
	cleanNode(node)
	node.Observers = nil
	node.ObserverSlots = nil
}

func detachOwner(owner *Owner) {
	parent := owner.Owner
	owner.Owner = nil
	if parent == nil {
		return
	}
	for i, child := range parent.Owned {
		if child == owner {
			parent.Owned = append(parent.Owned[:i], parent.Owned[i+1:]...)
			return
		}
	}
}

func runCleanups(owner *Owner) {
	cleanups := owner.Cleanups
	owner.Cleanups = nil
	for i := len(cleanups) - 1; i >= 0; i-- {
		cleanups[i]()
	}
}

// SignalOptions configures equality. Equals defaults to true.
type SignalOptions struct {
	Equals *bool
}

func defaultEquals(options *SignalOptions) bool {
	if options == nil || options.Equals == nil {
		return true
	}
	return *options.Equals
}

// CreateSignal returns a reactive getter/setter pair.
func CreateSignal[T any](value T, options ...SignalOptions) (Accessor[T], Setter[T]) {
	var opts *SignalOptions
	if len(options) > 0 {
		opts = &options[0]
	}
	signal := &Signal[T]{Value: value, Equals: defaultEquals(opts)}
	return func() T { return signal.Read() }, func(next T) { signal.Write(next) }
}

// NewSignal exposes a signal as one object for callers that prefer Get/Set.
func NewSignal[T any](value T, options ...SignalOptions) *Signal[T] {
	var opts *SignalOptions
	if len(options) > 0 {
		opts = &options[0]
	}
	return &Signal[T]{Value: value, Equals: defaultEquals(opts)}
}

// CreateMemo returns a cached derived value.
func CreateMemo[T any](compute func() T, options ...SignalOptions) Accessor[T] {
	var opts *SignalOptions
	if len(options) > 0 {
		opts = &options[0]
	}
	memo := &Memo[T]{compute: compute, equals: defaultEquals(opts)}
	memo.Pure = true
	memo.Owner.Owner = CurrentOwner
	memo.Owner.self = memo
	if CurrentOwner != nil {
		CurrentOwner.Owned = append(CurrentOwner.Owned, &memo.Owner)
	}
	memo.Fn = func() {
		next := memo.compute()
		if !memo.equals || !identical(memo.Value, next) {
			memo.Value = next
			markObservers(memo)
		}
	}
	memo.Value = initialComputation(&memo.Computation, compute)
	return func() T { return memo.Read() }
}

// CreateEffect runs after the current batch and re-runs when its dependencies change.
func CreateEffect(fn func()) {
	node := newComputation(CurrentOwner, fn, false)
	node.State = stale
	if updating {
		queue(node)
		return
	}
	runUpdates(func() {
		queue(node)
	})
}

// CreateRenderEffect runs immediately, for bindings that must be applied during rendering.
func CreateRenderEffect(fn func()) {
	node := newComputation(CurrentOwner, fn, false)
	updateComputation(node)
}

// Untrack runs fn without subscribing the current computation to anything it reads.
func Untrack[T any](fn func() T) T {
	previous := Listener
	Listener = nil
	defer func() { Listener = previous }()
	return fn()
}

// Batch applies several writes and notifies once, when the outermost batch ends.
func Batch(fn func()) {
	runUpdates(fn)
}

// CreateRoot creates a detached ownership root. dispose releases everything created inside fn.
func CreateRoot[T any](fn func(dispose func()) T) T {
	root := NewOwner(nil)
	dispose := func() {
		if root.Disposed {
			return
		}
		root.Disposed = true
		disposeOwned(root)
		runCleanups(root)
	}
	previousOwner := CurrentOwner
	previousListener := Listener
	CurrentOwner = root
	Listener = nil
	defer func() {
		CurrentOwner = previousOwner
		Listener = previousListener
	}()
	return fn(dispose)
}

func GetOwner() *Owner { return CurrentOwner }

func GetComputation() *Computation {
	if CurrentOwner == nil {
		return nil
	}
	return CurrentOwner.asComputation()
}

// RunWithOwner runs fn with owner as the current owner and no tracking listener.
func RunWithOwner[T any](owner *Owner, fn func() T) T {
	previousOwner := CurrentOwner
	previousListener := Listener
	CurrentOwner = owner
	Listener = nil
	defer func() {
		CurrentOwner = previousOwner
		Listener = previousListener
	}()
	return fn()
}

// OnCleanup registers a cleanup that runs when the current owner is disposed or re-runs.
func OnCleanup(fn func()) {
	owner := CurrentOwner
	if owner == nil {
		return
	}
	owner.Cleanups = append(owner.Cleanups, fn)
}

// DisposeOwner disposes an owner explicitly.
func DisposeOwner(owner *Owner) {
	if owner == nil || owner.Disposed {
		return
	}
	if c := owner.asComputation(); c != nil {
		disposeComputation(c)
		return
	}
	detachOwner(owner)
	owner.Disposed = true
	disposeOwned(owner)
	runCleanups(owner)
}

type contextEntry[T any] struct {
	owner *Owner
	value T
}

// Context is a value provided to a subtree and read by descendants through the owner chain.
type Context[T any] struct {
	Default T
	entries []contextEntry[T]
}

func CreateContext[T any](defaultValue T) *Context[T] {
	return &Context[T]{Default: defaultValue}
}

func (c *Context[T]) Provide(value T, fn func()) {
	owner := NewOwner(CurrentOwner)
	entry := contextEntry[T]{owner: owner, value: value}
	c.entries = append(c.entries, entry)
	onCleanupOf(owner, func() {
		for i, existing := range c.entries {
			if existing.owner == owner {
				c.entries = append(c.entries[:i], c.entries[i+1:]...)
				return
			}
		}
	})
	RunWithOwner(owner, func() struct{} {
		fn()
		return struct{}{}
	})
}

func (c *Context[T]) Use() T {
	current := CurrentOwner
	for current != nil {
		for _, entry := range c.entries {
			if entry.owner == current {
				return entry.value
			}
		}
		current = current.Owner
	}
	return c.Default
}

func UseContext[T any](context *Context[T]) T {
	return context.Use()
}

func onCleanupOf(owner *Owner, fn func()) {
	owner.Cleanups = append(owner.Cleanups, fn)
}

// Flush runs every queued effect now instead of at the end of the enclosing batch.
func Flush() {
	if !updating {
		runUpdates(func() {})
	}
}
