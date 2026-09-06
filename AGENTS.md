# Repository guidance

## Architecture

- Implement public framework capabilities in the Rust core whenever possible. JavaScript and Go bindings should adopt and expose the core behavior instead of maintaining a parallel implementation or source of truth; keep logic in a frontend only when it is inherently specific to that runtime or renderer.
- The hosted JavaScript/native and Go/native boundaries must never synchronously wait for native main-thread execution. Enqueue fire-and-forget mutations, allocate constructor handles locally, and expose every result, lifecycle outcome, or native request acceptance through an asynchronous task (a Promise in TypeScript, a callback on the application goroutine in Go).
- The Go frontend is cgo-free. It loads the prebuilt host shared library at runtime so `go build` stays a plain, fast compile and does not link Rust or invoke a C compiler.
