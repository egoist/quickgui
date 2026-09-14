#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
export CGO_ENABLED=0

unformatted=$(find go examples extensions -type d \( -name .quickgui -o -name node_modules -o -name target \) -prune -o -type f -name '*.go' -print0 | xargs -0 gofmt -l)
if [[ -n "$unformatted" ]]; then
  printf 'Run gofmt on:\n%s\n' "$unformatted" >&2
  exit 1
fi

go -C go run ./cmd/quickguifmt -check . ../examples ../extensions
go -C go/protocol run ../internal/cmd/protocolgen -check
go -C go/ui run ../internal/cmd/optionsgen -check
go -C go/ui run ../internal/cmd/componentsgen -check
bun scripts/generate-style-helpers.ts --check
go -C go test ./...
for module in extensions/*/go.mod; do
  go -C "$(dirname "$module")" test ./...
done
for module in examples/*/go.mod; do
  bun packages/cli/src/cli.ts test --project "$(dirname "$module")"
done
bun packages/cli/src/cli.ts test --project packages/cli/test-fixtures/go-views
