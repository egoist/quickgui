# Go counter

The TypeScript counter example, written against the Go frontend. Components run once;
signals update only the text node that read them.

```console
bun run build:native
cd examples/counter-go
CGO_ENABLED=0 go run .
```

`go build` does not compile Rust or invoke a C compiler. The process loads the
prebuilt host shared library (`packages/native/lib/<target>/libquickgui_host.dylib`)
at runtime. Set `QUICKGUI_HOST_LIB` if the library is staged elsewhere.
