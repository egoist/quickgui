# Codebase review — 2026-09-13

## Findings

### P2: application-service promises survive application destruction

Location: `packages/native/src/system.ts:1609` (`rejectPendingSystemRequests`).

`App.destroy()` calls this cleanup before destroying the native app, but it omits
`pendingAppServiceRequests`. That map owns Dock visibility/attention, activation
policy, moving the application to Applications, and native popup-menu requests.
Destroying the app before its completion event leaves the promise pending and
retains its request closure. Popup-menu entries also retain their disposal closure.

Reproduced against the existing fake binding: start `app.dock.bounce()`, destroy the
app without delivering an `app-service` event, then drain microtasks and wait 20 ms.
The request remains pending. This is binding-level evidence, not a live AppKit test.
The missing cleanup is directly visible by comparing the request map at line 1855
with the cleanup routine and its caller at `packages/native/src/index.ts:1037`.

Recommended correction: delete and reject every pending application-service request
during cleanup, preserving the existing rejection wrappers that dispose popup-menu
resources. Add a lifecycle regression for destruction before service completion,
including a late event after disposal. Run singleton-destroying tests in an isolated
process because the current native tests share their app singleton.

### P2: resource bundles accepted by the writer can exceed the reader's limit

Location: `packages/cli/src/extension-resources.ts:98`.

`packResources` validates a maximum of 128 MiB of decoded file content, then writes
base64 data inside JSON. `unpackResources` applies that same 128 MiB limit to the
decompressed JSON, before decoding base64. Consequently a valid resource bundle
with more than roughly 96 MiB of files can pack successfully and fail on unpack.

Reproduced with a temporary `Example.framework/payload` containing 97 MiB of zero
bytes. Packing succeeded and produced a 131,964-byte gzip archive. Unpacking failed
with `ERR_BUFFER_TOO_LARGE: Cannot create a Buffer larger than 134217728 bytes`.
The temporary fixture was removed after the check. The pack/unpack mismatch also
applies to less-compressible files; compression only made this reproduction small.

Recommended correction: define decoded-content and serialized-envelope limits
separately, account for bounded entry metadata and base64 expansion, and enforce
compatible limits in both directions. Preserve bounded decompression. Add a
round-trip regression near the decoded-content boundary and an oversized-envelope
rejection check.

## Architecture assessment

The repository has a clearly documented ownership model: a retained Rust runtime
and renderer shared by in-process Go and Bun/Solid bindings, with language-specific
construction and reactivity. The performance guide defines useful, testable
invariants for invalidation, identity, bounded resources and clean-window sleep.
Generators keep protocol and style declarations aligned across languages. CI
defines Rust, Go and TypeScript checks plus platform compile gates.

The two reproduced failures illustrate coverage gaps around lifecycle termination
and serialization boundaries even when the normal binding/CLI suites pass. Those
are useful review targets for future changes. This was a focused architecture,
binding-lifecycle and packaging review, supported by the available language suites;
it was not an exhaustive audit of the renderer, unsafe code, or every platform.

## Verification

Dependencies were initially absent. `bun install --frozen-lockfile` installed the
workspace dependencies without changing the tracked lockfile. After installation:

| Check | Result |
| --- | --- |
| `bun run test:js` | 128 passed, 0 failed |
| `bun run test:typescript` | 158 framework tests and 64 Quick Git tests passed |
| `bun run typecheck:js` | Passed |
| `bash scripts/check-go.sh` | Passed, including SDK, generators, examples and compiler fixture |
| `bun scripts/generate-typescript.ts --check` | Passed |
| `bun scripts/generate-style-helpers.ts --check` | Passed |
| `scripts/with-macos-ghostty-zig.sh cargo test -p quickgui-host --lib --locked` | Blocked before Cargo: Zig 0.15.2 required |

No native application was launched, no GPU/visual acceptance was performed, and
Linux/Windows runtime behavior was not verified. The initial review left production
source unchanged. Existing untracked `my-app/` was untouched.

## Fix follow-up

Both findings are fixed on `fix/service-cleanup-resource-bounds`:

- Application destruction deletes and rejects pending application-service requests,
  invoking their existing popup disposal wrappers. An isolated-process regression
  covers pending Dock, activation, installation and popup requests, repeated
  destruction, and late service/popup/menu-action events.
- Resource serialization has a separate bounded envelope allowance for base64,
  per-entry padding, paths and JSON metadata. Both writer and reader enforce it;
  the decoded-content limit remains 128 MiB. Regressions cover an exact-limit
  round trip, oversized decoded content in both directions, and oversized
  decompression rejection before output creation.

The new regressions reproduced the original failures before the production edits.
Afterward, `bun run test:js` passed 131 tests and `bun run test:typescript` passed
159 framework plus 64 Quick Git tests. Type checking, generated TypeScript binding
checks and `git diff --check` also passed. These fixes change TypeScript only;
native Rust/visual acceptance was not rerun.

## Reusable skills

No QuickGUI-specific `SKILL.md` was found in the repository or the local Codex/agent
skill directories before this review. Added:

- [quickgui-app-development](../../.agents/skills/quickgui-app-development/SKILL.md):
  application language routing, current component contracts and build/test workflow.
- [quickgui-core-change](../../.agents/skills/quickgui-core-change/SKILL.md): framework
  ownership, protocol/style generators, lifecycle/packaging review and validation.

Both skills refer to maintained repository documentation rather than duplicating
the API manuals. `AGENTS.md` links them for future sessions.
