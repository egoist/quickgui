package reactive

import "testing"

func TestMemosCacheAndUpdateInDependencyOrder(t *testing.T) {
	CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		a, setA := CreateSignal(1)
		b, setB := CreateSignal(2)
		computed := 0
		sum := CreateMemo(func() int {
			computed++
			return a() + b()
		})
		doubled := CreateMemo(func() int { return sum() * 2 })
		if sum() != 3 || doubled() != 6 || computed != 1 {
			t.Fatalf("initial sum=%d doubled=%d computed=%d", sum(), doubled(), computed)
		}
		setA(5)
		if doubled() != 14 || computed != 2 {
			t.Fatalf("after setA doubled=%d computed=%d", doubled(), computed)
		}
		Batch(func() {
			setA(1)
			setB(1)
		})
		if computed != 3 || sum() != 2 {
			t.Fatalf("after batch computed=%d sum=%d", computed, sum())
		}
		return struct{}{}
	})
}

func TestDiamondRunsJoiningMemoOnce(t *testing.T) {
	CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		source, setSource := CreateSignal(1)
		left := CreateMemo(func() int { return source() + 1 })
		right := CreateMemo(func() int { return source() * 10 })
		joins := 0
		join := CreateMemo(func() int {
			joins++
			return left() + right()
		})
		if join() != 12 {
			t.Fatalf("initial %d", join())
		}
		setSource(2)
		if join() != 23 || joins != 2 {
			t.Fatalf("join=%d joins=%d", join(), joins)
		}
		return struct{}{}
	})
}

func TestEqualWritesDoNotNotifyUnlessDisabled(t *testing.T) {
	CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		value, setValue := CreateSignal(1)
		eq := false
		force, setForce := CreateSignal(1, SignalOptions{Equals: &eq})
		runs := 0
		forcedRuns := 0
		CreateRenderEffect(func() {
			value()
			runs++
		})
		CreateRenderEffect(func() {
			force()
			forcedRuns++
		})
		setValue(1)
		setForce(1)
		if runs != 1 || forcedRuns != 2 {
			t.Fatalf("runs=%d forced=%d", runs, forcedRuns)
		}
		return struct{}{}
	})
}

func TestStaleControllerRunsBeforeControlledScope(t *testing.T) {
	value, setValue := CreateSignal[*struct{ N int }](nil)
	var seen []int
	CreateRoot(func(dispose func()) struct{} {
		parent := GetOwner()
		var content *Owner
		CreateRenderEffect(func() {
			current := value()
			if content != nil {
				DisposeOwner(content)
				content = nil
			}
			if current == nil {
				return
			}
			owner := NewOwner(parent)
			owner.Controller = GetComputation()
			content = owner
			RunWithOwner(owner, func() struct{} {
				CreateRenderEffect(func() {
					seen = append(seen, value().N)
				})
				return struct{}{}
			})
		})
		for i := 0; i < 4; i++ {
			CreateRenderEffect(func() { _ = value() })
		}
		return struct{}{}
	})
	setValue(&struct{ N int }{N: 1})
	setValue(nil)
	if len(seen) != 1 || seen[0] != 1 {
		t.Fatalf("seen %v", seen)
	}
}

func TestManuallyDisposedChildRoots(t *testing.T) {
	CreateRoot(func(dispose func()) struct{} {
		owner := GetOwner()
		cleanups := 0
		for i := 0; i < 1000; i++ {
			CreateRoot(func(close func()) struct{} {
				OnCleanup(func() { cleanups++ })
				close()
				return struct{}{}
			})
		}
		if len(owner.Owned) != 0 {
			t.Fatalf("owned %d", len(owner.Owned))
		}
		if cleanups != 1000 {
			t.Fatalf("cleanups %d", cleanups)
		}
		dispose()
		if cleanups != 1000 {
			t.Fatalf("double cleanup %d", cleanups)
		}
		return struct{}{}
	})
}

func TestEffectsRunAfterBatchOnce(t *testing.T) {
	CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		a, setA := CreateSignal(1)
		b, setB := CreateSignal(1)
		var seen []int
		CreateEffect(func() {
			seen = append(seen, a()+b())
		})
		if len(seen) != 1 || seen[0] != 2 {
			t.Fatalf("initial %v", seen)
		}
		Batch(func() {
			setA(2)
			setB(2)
		})
		if len(seen) != 2 || seen[1] != 4 {
			t.Fatalf("after batch %v", seen)
		}
		return struct{}{}
	})
}

func TestRenderEffectsAndUntrack(t *testing.T) {
	CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		tracked, setTracked := CreateSignal(0)
		ignored, setIgnored := CreateSignal(0)
		runs := 0
		CreateRenderEffect(func() {
			tracked()
			Untrack(func() int { return ignored() })
			runs++
		})
		if runs != 1 {
			t.Fatalf("initial %d", runs)
		}
		setIgnored(1)
		if runs != 1 {
			t.Fatalf("untrack subscribed %d", runs)
		}
		setTracked(1)
		if runs != 2 {
			t.Fatalf("tracked %d", runs)
		}
		return struct{}{}
	})
}

func TestNestedComputationsDispose(t *testing.T) {
	var cleanups []string
	var setOuter, setInner Setter[int]
	innerRuns := 0
	dispose := CreateRoot(func(disposeRoot func()) func() {
		outer, writeOuter := CreateSignal(0)
		inner, writeInner := CreateSignal(0)
		setOuter = writeOuter
		setInner = writeInner
		CreateRenderEffect(func() {
			outer()
			OnCleanup(func() { cleanups = append(cleanups, "outer") })
			CreateRenderEffect(func() {
				inner()
				innerRuns++
				OnCleanup(func() { cleanups = append(cleanups, "inner") })
			})
		})
		return disposeRoot
	})
	if innerRuns != 1 {
		t.Fatalf("initial %d", innerRuns)
	}
	setOuter(1)
	if got := join(cleanups); got != "inner,outer" {
		t.Fatalf("after outer %s", got)
	}
	if innerRuns != 2 {
		t.Fatalf("innerRuns %d", innerRuns)
	}
	setInner(1)
	if innerRuns != 3 {
		t.Fatalf("after inner %d", innerRuns)
	}
	if got := join(cleanups); got != "inner,outer,inner" {
		t.Fatalf("rerun cleanups %s", got)
	}
	dispose()
	if got := join(cleanups); got != "inner,outer,inner,inner,outer" {
		t.Fatalf("dispose %s", got)
	}
	setInner(2)
	if innerRuns != 3 {
		t.Fatalf("ran after dispose %d", innerRuns)
	}
}

func TestMemoReadAheadOfBatch(t *testing.T) {
	CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		count, setCount := CreateSignal(1)
		double := CreateMemo(func() int { return count() * 2 })
		observed := 0
		CreateEffect(func() {
			observed = double()
		})
		Batch(func() {
			setCount(2)
			if double() != 4 {
				t.Fatalf("ahead %d", double())
			}
		})
		if observed != 4 {
			t.Fatalf("observed %d", observed)
		}
		return struct{}{}
	})
}

func TestContextProvideDisposesWithComputation(t *testing.T) {
	context := CreateContext(0)
	value, setValue := CreateSignal(0)
	page, setPage := CreateSignal(0)
	runs := 0
	CreateRoot(func(dispose func()) struct{} {
		CreateRenderEffect(func() {
			page()
			context.Provide(1, func() {
				CreateRenderEffect(func() {
					value()
					runs++
				})
			})
		})
		return struct{}{}
	})
	if runs != 1 {
		t.Fatalf("initial %d", runs)
	}
	setValue(1)
	if runs != 2 {
		t.Fatalf("value %d", runs)
	}
	setPage(1)
	if runs != 3 {
		t.Fatalf("page %d", runs)
	}
	setValue(2)
	if runs != 4 {
		t.Fatalf("only current page %d", runs)
	}
}

func TestContextFallback(t *testing.T) {
	CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		theme := CreateContext("light")
		inner := ""
		outer := ""
		theme.Provide("dark", func() {
			CreateRenderEffect(func() { inner = theme.Use() })
		})
		CreateRenderEffect(func() { outer = theme.Use() })
		if inner != "dark" || outer != "light" {
			t.Fatalf("inner=%s outer=%s", inner, outer)
		}
		return struct{}{}
	})
}

func join(parts []string) string {
	out := ""
	for i, part := range parts {
		if i > 0 {
			out += ","
		}
		out += part
	}
	return out
}
