# Go counter

The TypeScript counter example, written against the Go frontend. Components run once;
signals update only the text node that read them.

```console
cd examples/counter-go
bun run dev
```

`quickgui.config.ts` sets `language: "go"`. `quickgui dev` and `quickgui build` compile with
`CGO_ENABLED=0 go build` and stage the host shared library. `go build` does not compile Rust
or invoke a C compiler. Set `QUICKGUI_HOST_LIB` if you run the binary without that staged copy.
