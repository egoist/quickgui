package host

// Fake is a host API that answers Call with CallFn and leaves every other method inert.
// Tests install it with [Install] so the native router can run without a shared library.
type Fake struct {
	unloaded
	CallFn func(method, params string) string
}

func (f *Fake) Call(method, params string) string {
	if f != nil && f.CallFn != nil {
		return f.CallFn(method, params)
	}
	return unloaded{}.Call(method, params)
}

// Install sets Current to api and returns a restore function.
func Install(api API) func() {
	previous := Current
	Current = api
	return func() { Current = previous }
}
