package native

import "sync"

type hostEvent struct {
	kind   string
	window uint32
	target uint32
	flags  uint32
	value  string
	extra  string
	data   []byte
}

var (
	workMu     sync.Mutex
	workQueue  []func()
	workWake   = make(chan struct{}, 1)
	eventMu    sync.Mutex
	eventQueue []hostEvent
)

// Dispatch queues fn for the application goroutine. Event handlers already run there.
func Dispatch(fn func()) {
	workMu.Lock()
	workQueue = append(workQueue, fn)
	workMu.Unlock()
	select {
	case workWake <- struct{}{}:
	default:
	}
}

func enqueueHostEvent(kind string, window, target, flags uint32, value, extra string, data []byte) {
	eventMu.Lock()
	eventQueue = append(eventQueue, hostEvent{
		kind: kind, window: window, target: target, flags: flags,
		value: value, extra: extra, data: data,
	})
	eventMu.Unlock()
	select {
	case workWake <- struct{}{}:
	default:
	}
}

func drainWork() []func() {
	workMu.Lock()
	jobs := workQueue
	workQueue = nil
	workMu.Unlock()
	return jobs
}

func drainEvents() []hostEvent {
	eventMu.Lock()
	events := eventQueue
	eventQueue = nil
	eventMu.Unlock()
	return events
}
